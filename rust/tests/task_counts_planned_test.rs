//! Planned-date filters of `get_task_counts`.
//!
//! The tool advertised `plannedBefore` / `plannedAfter`, but the handler never
//! passed them on and the counts script had no planned filter, so a count of
//! "tasks planned before Friday" silently counted every task. These tests pin
//! the whole path, in three steps:
//!
//! 1. a `tools/call` over MCP reaches the script with both bounds (the handler
//!    wiring, which is where the values were dropped);
//! 2. the script parses each bound as a local date and filters on it, the same
//!    way `list_tasks` does;
//! 3. run in JavaScriptCore against a small fake database, the script counts
//!    only the tasks planned inside the bounds.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use omnifocus_mcp::{
    jxa::JxaRunner, server::OmniFocusServer, tools::tasks::get_task_counts_with_added_changed,
};
use rmcp::ServiceExt;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const EMPTY_COUNTS: &str = r#"{"total": 0, "available": 0, "completed": 0, "overdue": 0,
    "dueSoon": 0, "flagged": 0, "deferred": 0}"#;

/// Upper bound on any single exchange with the in-process server.
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(10);

// ---------------------------------------------------------------- capture

/// Records every script it is asked to run and returns empty counts.
#[derive(Clone)]
struct CapturingRunner {
    scripts: Arc<Mutex<Vec<String>>>,
}

impl CapturingRunner {
    fn new() -> Self {
        Self {
            scripts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn last_script(&self) -> String {
        let scripts = self.scripts.lock().expect("script lock");
        scripts.last().cloned().expect("a script was run")
    }
}

impl JxaRunner for CapturingRunner {
    fn run_omnijs<'a>(
        &'a self,
        script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        self.scripts
            .lock()
            .expect("script lock")
            .push(script.to_string());
        Box::pin(
            async move { Ok(serde_json::from_str(EMPTY_COUNTS).expect("EMPTY_COUNTS is JSON")) },
        )
    }
}

/// The counts script for the given planned bounds, every other filter unset.
async fn counts_script(planned_before: Option<&str>, planned_after: Option<&str>) -> String {
    let runner = CapturingRunner::new();
    get_task_counts_with_added_changed(
        &runner,
        None,
        None,
        None,
        "any",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        planned_before,
        planned_after,
        None,
    )
    .await
    .expect("get_task_counts runs");
    runner.last_script()
}

// ---------------------------------------------------------------- MCP round trip

/// Writes one JSON-RPC message as a line.
async fn send<W: AsyncWriteExt + Unpin>(writer: &mut W, message: Value) {
    let mut line = serde_json::to_vec(&message).expect("message serializes");
    line.push(b'\n');
    writer.write_all(&line).await.expect("write to server");
}

/// Reads the next JSON-RPC message line, failing after `EXCHANGE_TIMEOUT`.
async fn receive<R: AsyncBufReadExt + Unpin>(lines: &mut tokio::io::Lines<R>) -> Value {
    let line = tokio::time::timeout(EXCHANGE_TIMEOUT, lines.next_line())
        .await
        .expect("server answers in time")
        .expect("read from server")
        .expect("server keeps the stream open");
    serde_json::from_str(&line).expect("server sends JSON")
}

/// Serves the real `OmniFocusServer` over an in-memory stream, performs the
/// MCP handshake, calls `tool` with `arguments`, and returns the response.
async fn call_tool_over_mcp(runner: CapturingRunner, tool: &str, arguments: Value) -> Value {
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    let serving = tokio::spawn(async move {
        let service = OmniFocusServer::new(runner)
            .serve(tokio::io::split(server_io))
            .await
            .expect("server completes the handshake");
        service.waiting().await.expect("server shuts down cleanly");
    });

    let (client_read, mut client_write) = tokio::io::split(client_io);
    let mut lines = BufReader::new(client_read).lines();
    send(
        &mut client_write,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "0"}}}),
    )
    .await;
    let initialized = receive(&mut lines).await;
    assert_eq!(initialized["id"], 1, "initialize response: {initialized}");
    send(
        &mut client_write,
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    )
    .await;
    send(
        &mut client_write,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": {"name": tool, "arguments": arguments}}),
    )
    .await;
    let response = receive(&mut lines).await;

    drop(client_write);
    drop(lines);
    tokio::time::timeout(EXCHANGE_TIMEOUT, serving)
        .await
        .expect("server stops once the client disconnects")
        .expect("server task does not panic");
    response
}

#[tokio::test]
async fn tool_call_passes_planned_bounds_to_the_script() {
    let runner = CapturingRunner::new();
    let response = call_tool_over_mcp(
        runner.clone(),
        "get_task_counts",
        json!({"plannedBefore": "2026-05-15", "plannedAfter": "2026-05-05"}),
    )
    .await;
    assert_eq!(response["id"], 2, "tools/call response: {response}");
    assert!(
        response.get("error").is_none(),
        "tools/call failed: {response}"
    );
    let script = runner.last_script();
    assert!(script.contains(r#"const plannedBeforeRaw = "2026-05-15";"#));
    assert!(script.contains(r#"const plannedAfterRaw = "2026-05-05";"#));
}

#[tokio::test]
async fn tool_call_accepts_snake_case_planned_aliases() {
    let runner = CapturingRunner::new();
    let response = call_tool_over_mcp(
        runner.clone(),
        "get_task_counts",
        json!({"planned_before": "2026-05-15", "planned_after": "2026-05-05"}),
    )
    .await;
    assert!(
        response.get("error").is_none(),
        "tools/call failed: {response}"
    );
    let script = runner.last_script();
    assert!(script.contains(r#"const plannedBeforeRaw = "2026-05-15";"#));
    assert!(script.contains(r#"const plannedAfterRaw = "2026-05-05";"#));
}

// ---------------------------------------------------------------- script text

#[tokio::test]
async fn script_parses_planned_bounds_as_local_dates() {
    let script = counts_script(Some("2026-05-15"), Some("2026-05-05")).await;
    assert!(script.contains(r#"const plannedBeforeRaw = "2026-05-15";"#));
    assert!(script.contains(r#"const plannedAfterRaw = "2026-05-05";"#));
    assert!(script.contains(
        r#"const plannedBefore = parseOptionalLocalDate(plannedBeforeRaw, "plannedBefore");"#
    ));
    assert!(script.contains(
        r#"const plannedAfter = parseOptionalLocalDate(plannedAfterRaw, "plannedAfter");"#
    ));
}

#[tokio::test]
async fn script_without_planned_bounds_sets_them_to_null() {
    let script = counts_script(None, None).await;
    assert!(script.contains("const plannedBeforeRaw = null;"));
    assert!(script.contains("const plannedAfterRaw = null;"));
}

// ---------------------------------------------------------------- script, run

/// The task statuses the counts script reads, as distinct enum objects.
const FAKE_TASK_STATUS: &str = r#"var Task = { Status: {} };
["Available", "Blocked", "Completed", "Dropped", "DueSoon", "Next", "Overdue"].forEach(
  function (name) { Task.Status[name] = { name: name }; });"#;

/// Four available inbox tasks: planned on 1, 10 and 20 May 2026, and one never
/// planned.
const FAKE_DATABASE: &str = r#"function fakeTask(id, plannedDate) {
  return { id: { primaryKey: id }, name: id, containingProject: null, tags: [],
    flagged: false, completed: false, dueDate: null, deferDate: null, completionDate: null,
    added: null, modified: null, estimatedMinutes: null, plannedDate: plannedDate,
    taskStatus: Task.Status.Available };
}
var document = { flattenedTasks: [
  fakeTask("t1", new Date(2026, 4, 1, 9, 0)),
  fakeTask("t2", new Date(2026, 4, 10, 9, 0)),
  fakeTask("t3", new Date(2026, 4, 20, 9, 0)),
  fakeTask("t4", null)
] };"#;

/// The same tasks on an OmniFocus without planned dates: reading the property
/// throws, as it does on versions that predate it.
const FAKE_DATABASE_WITHOUT_PLANNED_DATES: &str = r#"function fakeTask(id) {
  const task = { id: { primaryKey: id }, name: id, containingProject: null, tags: [],
    flagged: false, completed: false, dueDate: null, deferDate: null, completionDate: null,
    added: null, modified: null, estimatedMinutes: null, taskStatus: Task.Status.Available };
  Object.defineProperty(task, "plannedDate", {
    get: function () { throw new Error("plannedDate is not supported"); }
  });
  return task;
}
var document = { flattenedTasks: [fakeTask("t1"), fakeTask("t2"), fakeTask("t3"), fakeTask("t4")] };"#;

/// Runs a counts script against `database` and returns its JSON or error text.
fn run_against(database: &str, script: &str) -> String {
    common::assert_script_compiles("get_task_counts", script);
    common::run_jsc(
        &format!("{FAKE_TASK_STATUS}\n{database}"),
        &format!(
            "try {{ const result = (function () {{\n{script}\n}})(); print(JSON.stringify(result)); }}\n\
             catch (error) {{ print(\"ERROR: \" + error.message); }}"
        ),
    )
}

/// Runs a counts script against `database` and returns the `total` it reports.
fn total_against(database: &str, script: &str) -> Value {
    let output = run_against(database, script);
    let counts: Value =
        serde_json::from_str(&output).unwrap_or_else(|_| panic!("expected counts, got {output}"));
    counts["total"].clone()
}

#[tokio::test]
async fn both_bounds_count_only_tasks_planned_between_them() {
    let script = counts_script(Some("2026-05-15"), Some("2026-05-05")).await;
    assert_eq!(total_against(FAKE_DATABASE, &script), 1);
}

#[tokio::test]
async fn planned_before_excludes_later_and_unplanned_tasks() {
    let script = counts_script(Some("2026-05-15"), None).await;
    assert_eq!(total_against(FAKE_DATABASE, &script), 2);
}

#[tokio::test]
async fn planned_after_excludes_earlier_and_unplanned_tasks() {
    let script = counts_script(None, Some("2026-05-05")).await;
    assert_eq!(total_against(FAKE_DATABASE, &script), 2);
}

#[tokio::test]
async fn bare_planned_bound_is_local_midnight() {
    // A task planned at 09:00 on 10 May is after "2026-05-10" (local
    // midnight) and before "2026-05-11".
    let after = counts_script(None, Some("2026-05-10")).await;
    assert_eq!(total_against(FAKE_DATABASE, &after), 2);
    let before = counts_script(Some("2026-05-11"), Some("2026-05-10")).await;
    assert_eq!(total_against(FAKE_DATABASE, &before), 1);
}

#[tokio::test]
async fn no_planned_bounds_count_every_task() {
    let script = counts_script(None, None).await;
    assert_eq!(total_against(FAKE_DATABASE, &script), 4);
}

#[tokio::test]
async fn invalid_planned_bound_is_reported_by_field_name() {
    let script = counts_script(Some("not a date"), None).await;
    common::assert_script_compiles("get_task_counts", &script);
    let output = common::run_jsc(
        &format!("{FAKE_TASK_STATUS}\n{FAKE_DATABASE}"),
        &format!(
            "try {{ (function () {{\n{script}\n}})(); print(\"NO ERROR\"); }}\n\
             catch (error) {{ print(\"ERROR: \" + error.message); }}"
        ),
    );
    assert!(
        output.starts_with("ERROR: ") && output.contains("plannedBefore"),
        "expected an error naming plannedBefore, got {output}"
    );
}

#[tokio::test]
async fn planned_bounds_error_where_planned_dates_are_unsupported() {
    let script = counts_script(Some("2026-05-15"), Some("2026-05-05")).await;
    assert_eq!(
        run_against(FAKE_DATABASE_WITHOUT_PLANNED_DATES, &script),
        "ERROR: plannedBefore/plannedAfter require an OmniFocus database migrated to support planned dates"
    );
}
