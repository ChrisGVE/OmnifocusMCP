//! Script-text tests for user date handling (upstream #13).
//!
//! Every tool that accepts a user date must prepend `JS_DATE_HELPERS` to its
//! OmniJS script and parse each date through those helpers — never through a
//! raw `new Date(userValue)`, which reads a bare `YYYY-MM-DD` as UTC. The
//! helpers' behaviour is tested in JavaScriptCore by `js_date_helpers_test.rs`;
//! these tests pin the wiring of each tool to them.

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    js_helpers::JS_DATE_HELPERS,
    jxa::JxaRunner,
    tools::{
        projects::{create_project, list_projects, update_project},
        tasks::{
            add_notification, create_subtask, create_task, create_tasks_batch,
            get_task_counts_with_added_changed, list_tasks_with_added_changed,
            search_tasks_with_added_changed, update_task, CreateTaskInput,
        },
    },
};
use serde_json::{json, Value};

const DUE_WRITE: &str = r#"parseWriteDate(dueDateValue, "dueDate", "DefaultDueTime", "17:00")"#;
const DEFER_WRITE: &str =
    r#"parseWriteDate(deferDateValue, "deferDate", "DefaultStartTime", "00:00")"#;
const DUE_UPDATE: &str = r#"parseWriteDate(updates.dueDate, "dueDate", "DefaultDueTime", "17:00")"#;
const DEFER_UPDATE: &str =
    r#"parseWriteDate(updates.deferDate, "deferDate", "DefaultStartTime", "00:00")"#;

/// Records the last script it was asked to run and returns a fixed payload.
struct CapturingRunner {
    payload: Value,
    script: Arc<Mutex<String>>,
}

impl CapturingRunner {
    fn new(payload: Value) -> Self {
        Self {
            payload,
            script: Arc::new(Mutex::new(String::new())),
        }
    }

    fn script(&self) -> String {
        self.script.lock().expect("script lock").clone()
    }
}

impl JxaRunner for CapturingRunner {
    fn run_omnijs<'a>(
        &'a self,
        script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        Box::pin(async move {
            *self.script.lock().expect("script lock") = script.to_string();
            Ok(self.payload.clone())
        })
    }
}

/// The script must start with the shared helpers and, after them, contain no
/// `new Date(` other than "now" (`new Date()`) or arithmetic on "now".
fn assert_uses_helpers_only(script: &str) {
    assert!(
        script.starts_with(JS_DATE_HELPERS),
        "script must start with JS_DATE_HELPERS:\n{script}"
    );
    let body = &script[JS_DATE_HELPERS.len()..];
    assert!(
        !body.contains("parseOptionalDate"),
        "per-tool parseOptionalDate copy must be gone:\n{body}"
    );
    for (offset, _) in body.match_indices("new Date(") {
        let argument = &body[offset + "new Date(".len()..];
        assert!(
            argument.starts_with(')') || argument.starts_with("now.getTime()"),
            "raw user date parsing remains: {}",
            &body[offset..(offset + 60).min(body.len())]
        );
    }
}

/// Asserts `earlier` appears in `script` before `later` (both must exist).
fn assert_before(script: &str, earlier: &str, later: &str) {
    let earlier_at = script
        .find(earlier)
        .unwrap_or_else(|| panic!("missing {earlier}"));
    let later_at = script
        .find(later)
        .unwrap_or_else(|| panic!("missing {later}"));
    assert!(earlier_at < later_at, "{earlier} must come before {later}");
}

fn assert_filter_bounds(script: &str, fields: &[(&str, &str)]) {
    for (variable, field) in fields {
        let line =
            format!(r#"const {variable} = parseOptionalLocalDate({variable}Raw, "{field}");"#);
        assert!(script.contains(&line), "missing `{line}`");
    }
}

const TASK_FILTER_BOUNDS: [(&str, &str); 10] = [
    ("dueBefore", "dueBefore"),
    ("dueAfter", "dueAfter"),
    ("deferBefore", "deferBefore"),
    ("deferAfter", "deferAfter"),
    ("completedBefore", "completedBefore"),
    ("completedAfter", "completedAfter"),
    ("addedAfter", "added_after"),
    ("addedBefore", "added_before"),
    ("changedAfter", "changed_after"),
    ("changedBefore", "changed_before"),
];

const PLANNED_FILTER_BOUNDS: [(&str, &str); 2] = [
    ("plannedBefore", "plannedBefore"),
    ("plannedAfter", "plannedAfter"),
];

const D: Option<&str> = Some("2026-07-12");

#[tokio::test]
async fn create_task_parses_dates_with_write_helper_before_creating() {
    let runner = CapturingRunner::new(json!({"id": "t1", "name": "x"}));
    create_task(&runner, "x", None, None, D, D, None, None, None)
        .await
        .expect("create_task");
    let script = runner.script();
    assert_uses_helpers_only(&script);
    assert!(script.contains(&format!(
        "const parsedDueDate = dueDateValue === null ? null : {DUE_WRITE};"
    )));
    assert!(script.contains(&format!(
        "const parsedDeferDate = deferDateValue === null ? null : {DEFER_WRITE};"
    )));
    assert!(script.contains("if (parsedDueDate !== null) task.dueDate = parsedDueDate;"));
    assert!(script.contains("if (parsedDeferDate !== null) task.deferDate = parsedDeferDate;"));
    assert_before(&script, "const parsedDeferDate", "new Task(");
}

#[tokio::test]
async fn create_subtask_parses_dates_with_write_helper_before_creating() {
    let runner = CapturingRunner::new(json!({"id": "c1", "name": "x"}));
    create_subtask(&runner, "x", "parent-1", None, D, D, None, None, None)
        .await
        .expect("create_subtask");
    let script = runner.script();
    assert_uses_helpers_only(&script);
    assert!(script.contains(&format!(
        "const parsedDueDate = dueDateValue === null ? null : {DUE_WRITE};"
    )));
    assert!(script.contains(&format!(
        "const parsedDeferDate = deferDateValue === null ? null : {DEFER_WRITE};"
    )));
    assert!(script.contains("if (parsedDueDate !== null) task.dueDate = parsedDueDate;"));
    assert!(script.contains("if (parsedDeferDate !== null) task.deferDate = parsedDeferDate;"));
    assert_before(&script, "const parsedDeferDate", "new Task(");
}

#[tokio::test]
async fn create_tasks_batch_parses_every_date_before_creating_any_task() {
    let runner = CapturingRunner::new(json!([]));
    let input = CreateTaskInput {
        name: "x".to_string(),
        project: None,
        note: None,
        due_date: Some("2026-07-12".to_string()),
        defer_date: Some("2026-07-12".to_string()),
        flagged: None,
        tags: None,
        estimated_minutes: None,
    };
    create_tasks_batch(&runner, vec![input])
        .await
        .expect("create_tasks_batch");
    let script = runner.script();
    assert_uses_helpers_only(&script);
    assert!(script.contains(
        r#"parseWriteDate(input.dueDate, "tasks[" + index + "].dueDate", "DefaultDueTime", "17:00")"#
    ));
    assert!(script.contains(
        r#"parseWriteDate(input.deferDate, "tasks[" + index + "].deferDate", "DefaultStartTime", "00:00")"#
    ));
    assert!(script.contains("if (dates.dueDate !== null) task.dueDate = dates.dueDate;"));
    assert!(script.contains("if (dates.deferDate !== null) task.deferDate = dates.deferDate;"));
    assert_before(&script, "const parsedDates = ", "new Task(");
}

#[tokio::test]
async fn update_task_parses_dates_with_write_helper_before_any_change() {
    let runner = CapturingRunner::new(json!({"id": "t1", "name": "x"}));
    update_task(&runner, "t1", Some("renamed"), None, D, D, None, None, None)
        .await
        .expect("update_task");
    let script = runner.script();
    assert_uses_helpers_only(&script);
    assert!(script.contains(&format!(
        "const parsedDueDate = has(\"dueDate\") ? {DUE_UPDATE} : null;"
    )));
    assert!(script.contains(&format!(
        "const parsedDeferDate = has(\"deferDate\") ? {DEFER_UPDATE} : null;"
    )));
    assert!(script.contains(r#"if (has("dueDate")) task.dueDate = parsedDueDate;"#));
    assert!(script.contains(r#"if (has("deferDate")) task.deferDate = parsedDeferDate;"#));
    assert_before(
        &script,
        "const parsedDeferDate",
        "task.name = updates.name;",
    );
}

#[tokio::test]
async fn create_project_parses_dates_with_write_helper_before_creating() {
    let runner = CapturingRunner::new(json!({"id": "p1"}));
    create_project(&runner, "x", None, None, D, D, None)
        .await
        .expect("create_project");
    let script = runner.script();
    assert_uses_helpers_only(&script);
    assert!(script.contains(&format!(
        "const parsedDueDate = dueDateValue === null ? null : {DUE_WRITE};"
    )));
    assert!(script.contains(&format!(
        "const parsedDeferDate = deferDateValue === null ? null : {DEFER_WRITE};"
    )));
    assert!(script.contains("if (parsedDueDate !== null) project.dueDate = parsedDueDate;"));
    assert!(script.contains("if (parsedDeferDate !== null) project.deferDate = parsedDeferDate;"));
    assert_before(&script, "const parsedDeferDate", "new Project(");
}

#[tokio::test]
async fn update_project_parses_dates_with_write_helper_before_any_change() {
    let runner = CapturingRunner::new(json!({"id": "p1"}));
    update_project(
        &runner,
        "p1",
        Some("renamed"),
        None,
        D,
        D,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .expect("update_project");
    let script = runner.script();
    assert_uses_helpers_only(&script);
    assert!(script.contains(&format!(
        "const parsedDueDate = has(\"dueDate\") ? {DUE_UPDATE} : null;"
    )));
    assert!(script.contains(&format!(
        "const parsedDeferDate = has(\"deferDate\") ? {DEFER_UPDATE} : null;"
    )));
    assert!(script.contains(r#"if (has("dueDate")) project.dueDate = parsedDueDate;"#));
    assert!(script.contains(r#"if (has("deferDate")) project.deferDate = parsedDeferDate;"#));
    assert_before(
        &script,
        "const parsedDeferDate",
        "project.name = updates.name;",
    );
}

#[tokio::test]
async fn add_notification_parses_absolute_date_as_local() {
    let runner = CapturingRunner::new(json!({"id": "n1"}));
    add_notification(&runner, "t1", Some("2026-07-12"), None)
        .await
        .expect("add_notification");
    let script = runner.script();
    assert_uses_helpers_only(&script);
    assert!(script
        .contains(r#"const parsedAbsoluteDate = parseLocalDate(absoluteDate, "absoluteDate");"#));
    assert!(script.contains("notification = task.addNotification(parsedAbsoluteDate);"));
}

#[tokio::test]
async fn get_task_counts_parses_filter_bounds_as_local() {
    let counts = json!({"total": 0, "available": 0, "completed": 0, "overdue": 0,
        "dueSoon": 0, "flagged": 0, "deferred": 0});
    let runner = CapturingRunner::new(counts);
    get_task_counts_with_added_changed(
        &runner, None, None, None, "any", None, D, D, D, D, D, D, D, D, D, D, None,
    )
    .await
    .expect("get_task_counts");
    let script = runner.script();
    assert_uses_helpers_only(&script);
    assert_filter_bounds(&script, &TASK_FILTER_BOUNDS);
}

#[tokio::test]
async fn list_tasks_parses_filter_bounds_as_local() {
    let runner = CapturingRunner::new(json!([]));
    list_tasks_with_added_changed(
        &runner, None, None, None, "any", None, "all", D, D, D, D, D, D, D, D, D, D, D, D, None,
        None, "asc", 10,
    )
    .await
    .expect("list_tasks");
    let script = runner.script();
    assert_uses_helpers_only(&script);
    assert_filter_bounds(&script, &TASK_FILTER_BOUNDS);
    assert_filter_bounds(&script, &PLANNED_FILTER_BOUNDS);
}

#[tokio::test]
async fn search_tasks_parses_filter_bounds_as_local() {
    let runner = CapturingRunner::new(json!([]));
    search_tasks_with_added_changed(
        &runner, "query", None, None, None, "any", None, "all", D, D, D, D, D, D, D, D, D, D, D, D,
        None, None, "asc", 10,
    )
    .await
    .expect("search_tasks");
    let script = runner.script();
    assert_uses_helpers_only(&script);
    assert_filter_bounds(&script, &TASK_FILTER_BOUNDS);
    assert_filter_bounds(&script, &PLANNED_FILTER_BOUNDS);
}

#[tokio::test]
async fn list_projects_parses_completion_bounds_as_local() {
    let runner = CapturingRunner::new(json!([]));
    list_projects(&runner, None, "completed", D, D, false, None, "asc", 10)
        .await
        .expect("list_projects");
    let script = runner.script();
    assert_uses_helpers_only(&script);
    assert_filter_bounds(
        &script,
        &[
            ("completedBefore", "completedBefore"),
            ("completedAfter", "completedAfter"),
        ],
    );
}
