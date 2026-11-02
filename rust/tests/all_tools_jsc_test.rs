//! Every registered tool, end to end without OmniFocus (audit CR-006,
//! CR-008, CR-028).
//!
//! Each call goes through the real MCP server, in memory, so the server
//! layer's parameter handling and defaults apply. The tool's real script
//! then runs through `support/omnijs_harness.rs`: inside the production
//! osascript wrapper, in JavaScriptCore, against a strict fake of OmniJS
//! that fails on any member OmniFocus's API reference does not document. A
//! call passes when every script it ran compiled and executed, touched only
//! documented members, and the tool answered with a result of the expected
//! shape.
//!
//! The calls are in `support/tool_rows.rs`, whose `rows()` has exactly one
//! row per registered tool (a test below checks it), and
//! `support/tool_variants.rs`, whose `variants()` take other branches of the
//! same scripts. Both run twice, in UTC and in
//! Europe/Zurich, so local-time code is exercised away from UTC. Every tool
//! must also reject an argument it does not declare.
//!
//! A new tool needs a row (see CONTRIBUTING.md).

mod common;
#[path = "support/mcp_session.rs"]
mod mcp_session;
#[path = "support/omnijs_harness.rs"]
mod omnijs_harness;
#[path = "support/tool_rows.rs"]
mod tool_rows;
#[path = "support/tool_variants.rs"]
mod tool_variants;

use std::{any::Any, collections::BTreeSet, sync::Arc};

use omnifocus_mcp::jxa::JxaRunner;
use serde_json::{json, Value};
use tokio::{sync::Semaphore, task::JoinSet};

use common::{assert_script_compiles, run_jsc, run_jsc_in_time_zone, UTC};
use mcp_session::{
    assert_tool_success, call_tool, protocol_error_message, request, tool_error_text,
};
use omnijs_harness::{ScriptRun, StrictOmniJs};
use tool_rows::{rows, Expect, Row};
use tool_variants::variants;

const ZURICH: &str = "Europe/Zurich";

/// How many `jsc` processes may run at once.
const PARALLEL_RUNS: usize = 8;

/// The tool's JSON result, or why the response is not a successful one.
fn tool_result(response: &Value) -> Result<Value, String> {
    if response.get("error").is_some() {
        return Err(format!("protocol error: {}", response["error"]));
    }
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or_else(|| format!("no text content: {response}"))?;
    if response["result"]["isError"] == true {
        return Err(format!("tool error: {text}"));
    }
    assert_tool_success(response);
    serde_json::from_str(text).map_err(|error| format!("result is not JSON ({error}): {text}"))
}

fn missing_keys(value: &Value, keys: &[&str]) -> Option<String> {
    let Some(object) = value.as_object() else {
        return Some(format!("expected an object, got {value}"));
    };
    let missing: Vec<&str> = keys
        .iter()
        .copied()
        .filter(|key| !object.contains_key(*key))
        .collect();
    (!missing.is_empty()).then(|| format!("missing keys {missing:?} in {value}"))
}

fn list_problem(response: &Value, keys: &[&str]) -> Option<String> {
    let result = match tool_result(response) {
        Ok(result) => result,
        Err(error) => return Some(error),
    };
    match result.as_array() {
        None => Some(format!("expected an array, got {result}")),
        Some(items) if items.is_empty() => Some("expected a non-empty array".to_string()),
        Some(items) => items.iter().find_map(|item| missing_keys(item, keys)),
    }
}

fn known_defect_problem(response: &Value, runs: &[ScriptRun], violation: &str) -> Option<String> {
    let seen: Vec<&String> = runs.iter().flat_map(|run| &run.violations).collect();
    if response["result"]["isError"] != true || seen.len() != 1 || seen[0] != violation {
        return Some(format!(
            "KNOWN DEFECT row no longer fails with {violation:?} (violations {seen:?}); \
             if the script was fixed, expect its result shape instead: {response}"
        ));
    }
    let text = tool_error_text(response);
    (!text.contains(violation)).then(|| format!("error does not name {violation:?}: {text}"))
}

fn shape_problem(response: &Value, runs: &[ScriptRun], expect: Expect) -> Option<String> {
    match expect {
        Expect::Object(keys) => match tool_result(response) {
            Err(error) => Some(error),
            Ok(result) => missing_keys(&result, keys),
        },
        Expect::List(keys) => list_problem(response, keys),
        Expect::KnownDefect { violation } => known_defect_problem(response, runs, violation),
    }
}

fn run_problems(index: usize, run: &ScriptRun, time_zone: &str, known_defect: bool) -> Vec<String> {
    let expected_offset = if time_zone == ZURICH { -60 } else { 0 };
    let mut problems = Vec::new();
    if !run.outer_compiled || !run.inner_compiled || !run.executed {
        problems.push(format!(
            "script {index}: outer compiled {}, inner compiled {}, executed {}: {:?}",
            run.outer_compiled, run.inner_compiled, run.executed, run.error
        ));
    }
    if !run.violations.is_empty() && !known_defect {
        problems.push(format!(
            "script {index}: undocumented OmniJS use {:?}",
            run.violations
        ));
    }
    if !run.gaps.is_empty() {
        problems.push(format!(
            "script {index}: the fake does not model {:?} (extend fixtures/omnijs/fake/)",
            run.gaps
        ));
    }
    if run.january_offset_minutes != expected_offset {
        problems.push(format!(
            "script {index}: ran with UTC offset {} minutes, not in {time_zone}",
            run.january_offset_minutes
        ));
    }
    problems
}

/// Everything wrong with one call, empty when it passes.
fn call_problems(row: &Row, time_zone: &str, response: &Value, runs: &[ScriptRun]) -> Vec<String> {
    let known_defect = matches!(row.expect, Expect::KnownDefect { .. });
    let mut problems = Vec::new();
    if runs.is_empty() {
        problems.push("ran no script".to_string());
    }
    for (index, run) in runs.iter().enumerate() {
        problems.extend(run_problems(index, run, time_zone, known_defect));
    }
    problems.extend(shape_problem(response, runs, row.expect));
    problems
        .into_iter()
        .map(|problem| format!("{} {} [{time_zone}]: {problem}", row.tool, row.arguments))
        .collect()
}

fn panic_text(payload: Box<dyn Any + Send>) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string()))
        .unwrap_or_else(|| "a non-text panic".to_string())
}

/// Makes every call of `rows()` and `variants()` in `time_zone`, and fails
/// listing every problem found.
async fn assert_every_call_runs(time_zone: &'static str) {
    let permits = Arc::new(Semaphore::new(PARALLEL_RUNS));
    let mut calls = JoinSet::new();
    for row in rows().into_iter().chain(variants()) {
        let permits = Arc::clone(&permits);
        calls.spawn(async move {
            let _permit = permits.acquire_owned().await.expect("semaphore open");
            let runner = StrictOmniJs::new(time_zone);
            let response = request(
                runner.clone(),
                "tools/call",
                json!({"name": row.tool, "arguments": row.arguments}),
            )
            .await;
            call_problems(&row, time_zone, &response, &runner.runs())
        });
    }
    let mut problems = Vec::new();
    while let Some(outcome) = calls.join_next().await {
        match outcome {
            Ok(found) => problems.extend(found),
            Err(error) if error.is_panic() => problems.push(panic_text(error.into_panic())),
            Err(error) => problems.push(format!("a call did not finish: {error}")),
        }
    }
    problems.sort();
    assert!(
        problems.is_empty(),
        "{} problem(s):\n{}",
        problems.len(),
        problems.join("\n")
    );
}

#[tokio::test]
async fn every_tool_runs_on_the_strict_fake_in_utc() {
    assert_every_call_runs(UTC).await;
}

#[tokio::test]
async fn every_tool_runs_on_the_strict_fake_in_zurich() {
    assert_every_call_runs(ZURICH).await;
}

#[tokio::test]
async fn the_table_covers_exactly_the_registered_tools() {
    let response = request(StrictOmniJs::new(UTC), "tools/list", json!({})).await;
    let registered: BTreeSet<String> = response["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("tools/list returns tools: {response}"))
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name").to_string())
        .collect();
    let table: Vec<String> = rows().iter().map(|row| row.tool.to_string()).collect();
    let covered: BTreeSet<String> = table.iter().cloned().collect();
    assert_eq!(covered.len(), table.len(), "a tool has two rows in rows()");
    assert_eq!(
        covered, registered,
        "rows() must have exactly one row per registered tool"
    );
    let variant_tools: BTreeSet<String> =
        variants().iter().map(|row| row.tool.to_string()).collect();
    assert!(
        variant_tools.is_subset(&registered),
        "variants() names an unregistered tool: {:?}",
        variant_tools.difference(&registered).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn every_tool_rejects_an_undeclared_argument() {
    for row in rows() {
        let mut arguments = row.arguments.clone();
        arguments["__bogus"] = json!(true);
        let call = call_tool(row.tool, arguments).await;
        let message = protocol_error_message(&call.response);
        assert!(
            message.contains("__bogus"),
            "{}: the error must name the undeclared key: {message}",
            row.tool
        );
        assert!(
            call.scripts.is_empty(),
            "{}: a rejected call must run no script",
            row.tool
        );
    }
}

// The instrument itself: each check below would read green on a broken
// harness if the harness could not see what it claims to see.

#[tokio::test]
async fn the_harness_flags_an_undocumented_read_the_script_catches() {
    let runner = StrictOmniJs::new(UTC);
    let outcome = runner
        .run_omnijs("try { void flattenedTasks[0].containingTask; } catch (error) {}\nreturn 1;")
        .await;
    assert!(
        outcome.is_err(),
        "a caught violation must still fail: {outcome:?}"
    );
    let runs = runner.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(
        runs[0].violations,
        ["read of undocumented Task.containingTask"]
    );
    assert!(runs[0].executed, "the script itself ran to completion");
}

#[tokio::test]
async fn the_harness_reports_a_script_that_does_not_compile() {
    let runner = StrictOmniJs::new(UTC);
    let outcome = runner.run_omnijs("return (;").await;
    let error = outcome.expect_err("a syntax error must fail").to_string();
    let run = &runner.runs()[0];
    assert!(run.outer_compiled, "the osascript program itself is valid");
    assert!(!run.inner_compiled, "the wrapped script must not compile");
    assert!(!run.executed);
    assert!(error.contains("SyntaxError"), "{error}");
}

#[tokio::test]
async fn the_harness_returns_what_a_documented_script_returns() {
    let runner = StrictOmniJs::new(ZURICH);
    let outcome = runner
        .run_omnijs(
            "const task = Task.byIdentifier(\"task-draft\");\n\
             return { name: task.name, project: task.containingProject.name, now: new Date().toISOString() };",
        )
        .await
        .expect("a documented script succeeds");
    assert_eq!(
        outcome,
        json!({"name": "Draft plan", "project": "Launch", "now": "2026-10-09T10:00:00.000Z"})
    );
}

#[tokio::test]
async fn the_harness_agrees_with_the_standalone_compile_check() {
    let recorded = call_tool("get_task", json!({"task_id": "task-draft"})).await;
    assert_eq!(recorded.scripts.len(), 1);
    assert_script_compiles("get_task", &recorded.scripts[0]);

    let runner = StrictOmniJs::new(UTC);
    let response = request(
        runner.clone(),
        "tools/call",
        json!({"name": "get_task", "arguments": {"task_id": "task-draft"}}),
    )
    .await;
    assert_tool_success(&response);
    assert!(runner.runs()[0].inner_compiled);
}

#[test]
fn jsc_runs_in_the_requested_time_zone() {
    // 2026-03-29 is the day Zurich moves from UTC+1 to UTC+2.
    let print = "print([new Date(2026, 2, 28, 12).getTimezoneOffset(), \
                 new Date(2026, 2, 29, 12).getTimezoneOffset()].join(','))";
    assert_eq!(run_jsc_in_time_zone(ZURICH, "", print), "-60,-120");
    assert_eq!(run_jsc_in_time_zone(UTC, "", print), "0,0");
    assert_eq!(run_jsc("", print), "0,0", "run_jsc defaults to UTC");
}
