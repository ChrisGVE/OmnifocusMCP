//! Project review intervals (upstream #12).
//!
//! OmniJS's `project.reviewInterval` is a `Project.ReviewInterval` value
//! object: to change it you read it, set `steps` and `unit`, and assign it
//! back. update_project used to assign a plain `{steps, unit}` object, which
//! OmniFocus rejects, and every read tool reported the interval as
//! `String(interval)`, i.e. "[object Project.ReviewInterval]".
//!
//! Three layers are tested here: the Rust parser that validates the user's
//! "N unit" text before any script runs; the shared JS helpers
//! (`JS_REVIEW_INTERVAL`), run in JavaScriptCore against a fake interval; and
//! the wiring of each tool that writes or reports an interval.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    error::OmniFocusError,
    js_helpers::JS_REVIEW_INTERVAL,
    jxa::JxaRunner,
    review_interval::{parse_review_interval, ReviewInterval},
    server::UpdateProjectParams,
    tools::projects::{get_project, list_projects, update_project},
};
use serde_json::{json, Value};

// ---------------------------------------------------------------- parser

fn parsed(input: &str) -> ReviewInterval {
    parse_review_interval(input).unwrap_or_else(|error| panic!("{input:?} should parse: {error}"))
}

fn interval(steps: u32, unit: &'static str) -> ReviewInterval {
    ReviewInterval { steps, unit }
}

#[test]
fn singular_and_plural_units_become_plural() {
    let cases = [
        ("1 day", interval(1, "days")),
        ("3 days", interval(3, "days")),
        ("1 week", interval(1, "weeks")),
        ("2 weeks", interval(2, "weeks")),
        ("1 month", interval(1, "months")),
        ("6 months", interval(6, "months")),
        ("1 year", interval(1, "years")),
        ("2 years", interval(2, "years")),
    ];
    for (input, expected) in cases {
        assert_eq!(parsed(input), expected, "{input:?}");
    }
}

#[test]
fn unit_is_case_insensitive() {
    assert_eq!(parsed("2 WEEKS"), interval(2, "weeks"));
    assert_eq!(parsed("1 Month"), interval(1, "months"));
}

#[test]
fn surrounding_and_inner_whitespace_is_tolerated() {
    assert_eq!(parsed("  3 months  "), interval(3, "months"));
    assert_eq!(parsed("1\tyear"), interval(1, "years"));
    assert_eq!(parsed("2   weeks"), interval(2, "weeks"));
}

#[test]
fn singular_count_with_plural_unit_is_accepted() {
    // The read side reports "1 weeks" (OmniFocus units are plural), so that
    // text must be accepted back as input.
    assert_eq!(parsed("1 weeks"), interval(1, "weeks"));
}

#[test]
fn large_step_counts_are_accepted() {
    assert_eq!(parsed("365 days"), interval(365, "days"));
}

#[test]
fn invalid_intervals_are_rejected_as_validation_errors() {
    let rejected = [
        "",
        "   ",
        "weeks",
        "2",
        "0 weeks",
        "00 days",
        "-1 weeks",
        "+2 weeks",
        "1.5 weeks",
        "2 minutes",
        "2 hours",
        "2 fortnights",
        "P1W",
        "two weeks",
        "2weeks",
        "2 weeks extra",
        "4294967296 days",
    ];
    for input in rejected {
        match parse_review_interval(input) {
            Err(OmniFocusError::Validation(message)) => assert!(
                message.contains(&format!("{input:?}")),
                "error for {input:?} must quote the received value: {message}"
            ),
            other => panic!("{input:?} must be a validation error, got {other:?}"),
        }
    }
}

#[test]
fn rejection_message_states_the_accepted_form() {
    let Err(OmniFocusError::Validation(message)) = parse_review_interval("2 hours") else {
        panic!("2 hours must be rejected");
    };
    assert!(message.contains("reviewInterval"), "{message}");
    assert!(message.contains("N unit"), "{message}");
    assert!(message.contains("days, weeks, months, years"), "{message}");
}

// ---------------------------------------------------------------- JS helpers

/// A fake OmniJS project whose `reviewInterval` behaves like the real value
/// object: every read returns a fresh copy, and only an assignment changes
/// what the project holds.
const FAKE_PROJECT: &str = r#"function FakeReviewInterval(steps, unit) { this.steps = steps; this.unit = unit; }
FakeReviewInterval.prototype.toString = function () { return "[object Project.ReviewInterval]"; };
function fakeProject(stored) {
  return {
    assignments: 0,
    get reviewInterval() { return stored === null ? null : new FakeReviewInterval(stored.steps, stored.unit); },
    set reviewInterval(value) { this.assignments += 1; stored = { steps: value.steps, unit: value.unit }; }
  };
}"#;

fn run_review_js(snippet: &str) -> String {
    common::run_jsc(&format!("{FAKE_PROJECT}\n{JS_REVIEW_INTERVAL}"), snippet)
}

#[test]
fn format_reports_steps_and_unit() {
    assert_eq!(
        run_review_js(r#"print(formatReviewInterval(new FakeReviewInterval(2, "weeks")));"#),
        "2 weeks"
    );
}

#[test]
fn format_of_a_single_step_uses_the_singular_unit() {
    assert_eq!(
        run_review_js(r#"print(formatReviewInterval(new FakeReviewInterval(1, "weeks")));"#),
        "1 week"
    );
}

#[test]
fn format_of_a_single_step_round_trips_through_the_parser() {
    let reported =
        run_review_js(r#"print(formatReviewInterval(new FakeReviewInterval(1, "months")));"#);
    let parsed = parse_review_interval(&reported).expect("reported text must parse back");
    assert_eq!((parsed.steps, parsed.unit), (1, "months"));
}

#[test]
fn format_of_absent_interval_is_null() {
    assert_eq!(
        run_review_js("print(formatReviewInterval(null) === null && formatReviewInterval(undefined) === null);"),
        "true"
    );
}

#[test]
fn update_returns_the_projects_own_interval_type_with_new_values() {
    let output = run_review_js(
        r#"const project = fakeProject({ steps: 1, unit: "days" });
const updated = updatedReviewInterval(project, { steps: 3, unit: "months" });
print(updated instanceof FakeReviewInterval);
print(updated.steps + " " + updated.unit);
print(project.assignments);"#,
    );
    // A Project.ReviewInterval (not a plain object), carrying the new values,
    // and nothing assigned to the project yet.
    assert_eq!(output, "true\n3 months\n0");
}

#[test]
fn update_of_project_without_interval_throws() {
    let output = run_review_js(
        r#"try { updatedReviewInterval(fakeProject(null), { steps: 1, unit: "weeks" }); print("NO ERROR"); }
catch (error) { print("ERROR: " + error.message); }"#,
    );
    assert_eq!(output, "ERROR: Project has no review interval to update");
}

// ---------------------------------------------------------------- wiring

/// Records every script it is asked to run and returns a fixed payload.
struct CapturingRunner {
    payload: Value,
    scripts: Arc<Mutex<Vec<String>>>,
}

impl CapturingRunner {
    fn new(payload: Value) -> Self {
        Self {
            payload,
            scripts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn scripts(&self) -> Vec<String> {
        self.scripts.lock().expect("script lock").clone()
    }

    fn last_script(&self) -> String {
        self.scripts().last().cloned().expect("a script was run")
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
        Box::pin(async move { Ok(self.payload.clone()) })
    }
}

const REPORTED_INTERVAL: &str = "reviewInterval: formatReviewInterval(project.reviewInterval)";

/// Every tool that reports an interval formats it through the shared helper.
fn assert_reports_formatted_interval(tool: &str, script: &str) {
    assert_eq!(
        script.matches(JS_REVIEW_INTERVAL).count(),
        1,
        "{tool} must prepend JS_REVIEW_INTERVAL exactly once"
    );
    assert!(script.contains(REPORTED_INTERVAL), "{tool}");
    assert!(!script.contains("String(reviewInterval"), "{tool}");
    common::assert_script_compiles(tool, script);
}

async fn update_review_interval(
    runner: &CapturingRunner,
    value: &str,
) -> Result<Value, OmniFocusError> {
    update_project(
        runner,
        "p1",
        Some("Renamed"),
        Some("new note"),
        None,
        None,
        None,
        None,
        None,
        None,
        Some(value),
    )
    .await
}

#[tokio::test]
async fn update_project_sends_validated_interval_and_prepares_it_before_any_change() {
    let runner = CapturingRunner::new(json!({"id": "p1"}));
    update_review_interval(&runner, " 2 Weeks ")
        .await
        .expect("update_project runs");
    let script = runner.last_script();
    assert_reports_formatted_interval("update_project", &script);
    assert!(script.contains(r#""reviewInterval":{"steps":2,"unit":"weeks"}"#));
    assert!(!script.contains("parseReviewInterval"));

    let prepare = r#"const preparedReviewInterval = has("reviewInterval") ? updatedReviewInterval(project, updates.reviewInterval) : null;"#;
    let first_change = r#"if (has("name")) project.name = updates.name;"#;
    let prepared_at = script.find(prepare).expect("interval is prepared");
    let changed_at = script.find(first_change).expect("name is changed");
    assert!(
        prepared_at < changed_at,
        "the interval must be prepared before any project field changes"
    );
    assert!(script
        .contains("if (has(\"reviewInterval\")) project.reviewInterval = preparedReviewInterval;"));
}

#[tokio::test]
async fn update_project_rejects_invalid_interval_before_running_any_script() {
    // Previously name and note were changed before the interval failed.
    let runner = CapturingRunner::new(json!({"id": "p1"}));
    let error = update_review_interval(&runner, "2 hours")
        .await
        .expect_err("hours are not a review unit");
    assert!(matches!(error, OmniFocusError::Validation(_)), "{error:?}");
    assert!(runner.scripts().is_empty(), "no script may run");
}

#[tokio::test]
async fn list_projects_reports_formatted_interval() {
    let runner = CapturingRunner::new(json!([]));
    list_projects(&runner, None, "active", None, None, false, None, "asc", 5)
        .await
        .expect("list_projects runs");
    assert_reports_formatted_interval("list_projects", &runner.last_script());
}

#[tokio::test]
async fn get_project_reports_formatted_interval() {
    let runner = CapturingRunner::new(json!({"id": "p1"}));
    get_project(&runner, "p1").await.expect("get_project runs");
    assert_reports_formatted_interval("get_project", &runner.last_script());
}

// ---------------------------------------------------------------- schema

#[test]
fn review_interval_field_describes_its_format() {
    let schema = serde_json::to_value(schemars::schema_for!(UpdateProjectParams)).unwrap();
    let description = schema["properties"]["reviewInterval"]["description"]
        .as_str()
        .expect("reviewInterval has a description");
    assert!(description.contains("N unit"), "{description}");
    assert!(
        description.contains("days, weeks, months, years"),
        "{description}"
    );
}
