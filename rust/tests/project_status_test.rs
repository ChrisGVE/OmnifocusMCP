//! Project status naming (upstream #10).
//!
//! `project.status` is a `Project.Status` enum value (`Active`, `Done`,
//! `Dropped`, `OnHold`). Five tools used to carry their own copy of a
//! normaliser that matched the value's *text* and fell back to "active", so a
//! completed project — whose text reads "Done", not "completed" — was reported
//! as active. The shared `JS_PROJECT_STATUS` snippet compares against the enum
//! members instead and names anything else "unknown".
//!
//! The snippet's behaviour is run in JavaScriptCore against a fake
//! `Project.Status` enum; the wiring of every tool that reports a project
//! status is pinned on the script text each one builds.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    js_helpers::JS_PROJECT_STATUS,
    jxa::JxaRunner,
    tools::{
        folders::get_folder,
        projects::{
            get_project, get_project_counts, list_projects, search_projects, update_project,
        },
        tasks::{list_tasks_with_added_changed, search_tasks_with_added_changed},
    },
};
use serde_json::{json, Value};

// ---------------------------------------------------------------- behaviour

/// A fake of OmniJS's `Project.Status`: four distinct enum objects whose
/// string form mimics OmniFocus's ("[object Project.Status: Done]").
const FAKE_PROJECT_STATUS: &str = r#"function FakeStatus(name) { this.name = name; }
FakeStatus.prototype.toString = function () { return "[object Project.Status: " + this.name + "]"; };
var Project = { Status: {
  Active: new FakeStatus("Active"),
  Done: new FakeStatus("Done"),
  Dropped: new FakeStatus("Dropped"),
  OnHold: new FakeStatus("OnHold")
} };"#;

/// Returns what `normalizeProjectStatus` reports for a project whose status is
/// the JS expression `status_expression`.
fn status_name(status_expression: &str) -> String {
    let prelude = format!("{FAKE_PROJECT_STATUS}\n{JS_PROJECT_STATUS}");
    common::run_jsc(
        &prelude,
        &format!("print(normalizeProjectStatus({{ status: {status_expression} }}));"),
    )
}

#[test]
fn done_project_is_completed() {
    // Upstream #10 repro: this used to come back as "active".
    assert_eq!(status_name("Project.Status.Done"), "completed");
}

#[test]
fn active_project_is_active() {
    assert_eq!(status_name("Project.Status.Active"), "active");
}

#[test]
fn dropped_project_is_dropped() {
    assert_eq!(status_name("Project.Status.Dropped"), "dropped");
}

#[test]
fn on_hold_project_is_on_hold() {
    assert_eq!(status_name("Project.Status.OnHold"), "on_hold");
}

#[test]
fn unrecognised_status_object_is_unknown_not_active() {
    // Same text as Done, but not the enum member: identity decides, not text.
    assert_eq!(status_name(r#"new FakeStatus("Done")"#), "unknown");
}

#[test]
fn status_text_is_not_mistaken_for_the_enum() {
    assert_eq!(status_name(r#""active""#), "unknown");
}

#[test]
fn missing_status_is_unknown() {
    assert_eq!(status_name("null"), "unknown");
    assert_eq!(status_name("undefined"), "unknown");
}

// ---------------------------------------------------------------- wiring

/// Text that only the removed per-tool normalisers contained.
const OLD_NORMALISER_MARKERS: [&str; 3] = [
    "const normalizeProjectStatus = (",
    r#"if (flattened.includes("completed")) return "completed";"#,
    r#".replace(/^\[object_/g, "")"#,
];

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

    fn last_script(&self) -> String {
        self.scripts
            .lock()
            .expect("script lock")
            .last()
            .cloned()
            .expect("a script was run")
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

/// Every project-status script carries the shared snippet exactly once, none
/// of the old normaliser text, and still compiles.
fn assert_uses_shared_status(tool: &str, script: &str) {
    assert_eq!(
        script.matches(JS_PROJECT_STATUS).count(),
        1,
        "{tool} must prepend JS_PROJECT_STATUS exactly once"
    );
    for marker in OLD_NORMALISER_MARKERS {
        assert!(
            !script.contains(marker),
            "{tool} still contains old normaliser text {marker:?}"
        );
    }
    common::assert_script_compiles(tool, script);
}

#[tokio::test]
async fn list_projects_filters_and_reports_with_shared_status() {
    let runner = CapturingRunner::new(json!([]));
    list_projects(
        &runner,
        None,
        "completed",
        None,
        None,
        false,
        None,
        "asc",
        5,
    )
    .await
    .expect("list_projects runs");
    let script = runner.last_script();
    assert_uses_shared_status("list_projects", &script);
    assert!(script.contains("if (normalizeProjectStatus(project) !== statusFilter) return false;"));
    assert!(script.contains("status: normalizeProjectStatus(project),"));
}

#[tokio::test]
async fn list_projects_stalled_detection_uses_shared_status() {
    let runner = CapturingRunner::new(json!([]));
    list_projects(&runner, None, "active", None, None, true, None, "asc", 5)
        .await
        .expect("list_projects runs");
    let script = runner.last_script();
    assert_uses_shared_status("list_projects", &script);
    assert!(script.contains(r#"const isStalled = normalizeProjectStatus(project) === "active""#));
}

#[tokio::test]
async fn search_projects_reports_shared_status() {
    let runner = CapturingRunner::new(json!([]));
    search_projects(&runner, "launch", 5)
        .await
        .expect("search_projects runs");
    let script = runner.last_script();
    assert_uses_shared_status("search_projects", &script);
    assert!(script.contains("status: normalizeProjectStatus(project),"));
}

#[tokio::test]
async fn get_project_counts_buckets_by_shared_status() {
    let runner = CapturingRunner::new(json!({
        "total": 0, "active": 0, "onHold": 0, "completed": 0, "dropped": 0, "stalled": 0
    }));
    get_project_counts(&runner, None)
        .await
        .expect("get_project_counts runs");
    let script = runner.last_script();
    assert_uses_shared_status("get_project_counts", &script);
    assert!(script.contains("const status = normalizeProjectStatus(project);"));
    assert!(script.contains(r#"if (status === "completed") counts.completed += 1;"#));
}

#[tokio::test]
async fn get_project_reports_shared_status() {
    let runner = CapturingRunner::new(json!({"id": "p1"}));
    get_project(&runner, "p1").await.expect("get_project runs");
    let script = runner.last_script();
    assert_uses_shared_status("get_project", &script);
    assert!(script.contains("status: normalizeProjectStatus(project),"));
}

#[tokio::test]
async fn update_project_reports_shared_status() {
    let runner = CapturingRunner::new(json!({"id": "p1"}));
    update_project(
        &runner,
        "p1",
        Some("Renamed"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .expect("update_project runs");
    let script = runner.last_script();
    assert_uses_shared_status("update_project", &script);
    assert!(script.contains("status: normalizeProjectStatus(project),"));
}

/// The `on_hold` task filter selects tasks whose project is on hold. It must
/// read the project status through the shared snippet, not its own text match.
fn assert_on_hold_task_filter_uses_shared_status(tool: &str, script: &str) {
    assert_eq!(
        script.matches(JS_PROJECT_STATUS).count(),
        1,
        "{tool} must prepend JS_PROJECT_STATUS exactly once"
    );
    assert!(script.contains(
        r#"statusMatches = task.containingProject !== null && normalizeProjectStatus(task.containingProject) === "on_hold";"#
    ));
    assert!(!script.contains("const projectStatus = task.containingProject ? String("));
    common::assert_script_compiles(tool, script);
}

#[tokio::test]
async fn list_tasks_on_hold_filter_uses_shared_status() {
    let runner = CapturingRunner::new(json!([]));
    list_tasks_with_added_changed(
        &runner, None, None, None, "any", None, "on_hold", None, None, None, None, None, None,
        None, None, None, None, None, None, None, None, "asc", 5,
    )
    .await
    .expect("list_tasks runs");
    assert_on_hold_task_filter_uses_shared_status("list_tasks", &runner.last_script());
}

#[tokio::test]
async fn search_tasks_on_hold_filter_uses_shared_status() {
    let runner = CapturingRunner::new(json!([]));
    search_tasks_with_added_changed(
        &runner, "plan", None, None, None, "any", None, "on_hold", None, None, None, None, None,
        None, None, None, None, None, None, None, None, None, "asc", 5,
    )
    .await
    .expect("search_tasks runs");
    assert_on_hold_task_filter_uses_shared_status("search_tasks", &runner.last_script());
}

#[tokio::test]
async fn get_folder_reports_child_projects_with_project_status() {
    let runner = CapturingRunner::new(json!({"id": "f1"}));
    get_folder(&runner, "f1").await.expect("get_folder runs");
    let script = runner.last_script();
    assert_eq!(script.matches(JS_PROJECT_STATUS).count(), 1);
    // Child projects use the project enum, not the folder normaliser.
    assert!(script.contains("status: normalizeProjectStatus(project)"));
    assert!(!script.contains("status: normalizeStatus(project.status)"));
    common::assert_script_compiles("get_folder", &script);
}
