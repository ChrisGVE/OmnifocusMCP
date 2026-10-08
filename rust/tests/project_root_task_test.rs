//! What a project reports about itself through its root task.
//!
//! OmniJS models a project as a `Project` wrapped around a root `Task`
//! (`project.task`). Two project fields leaked through that seam:
//!
//! - `project.nextTask` returns the root task itself when the project has no
//!   actionable child — seen live on a completed project, which reported its
//!   own id and name as `nextTaskId`/`nextTaskName`. The shared
//!   `projectNextTask(project)` helper treats the root task as "no next task",
//!   and stall detection uses it too.
//! - `Project` has no `modified` property (OmniFocus's interface documentation
//!   lists it only on dated objects such as `Task`), so `project.modified` read
//!   `undefined` and every project reported `modified: null`. The date lives on
//!   the root task: `project.task.modified`.
//!
//! The helper's behaviour runs in JavaScriptCore against fakes; the wiring of
//! each project tool is pinned on the script text it builds.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    js_helpers::{JS_PROJECT_STATUS, JS_TASK_STATUS},
    jxa::JxaRunner,
    tools::projects::{create_project, get_project, list_projects},
};
use serde_json::{json, Value};

// ---------------------------------------------------------------- behaviour

/// Fake enums, a fixed `NOW`, and an active project whose `nextTask` is set
/// by each case. A root task is recognised by its `project` back-reference.
const FAKES: &str = r#"function FakeEnum(name) { this.name = name; }
var Task = { Status: { Available: new FakeEnum("Available"), Blocked: new FakeEnum("Blocked"),
  Completed: new FakeEnum("Completed"), Dropped: new FakeEnum("Dropped"),
  DueSoon: new FakeEnum("DueSoon"), Next: new FakeEnum("Next"), Overdue: new FakeEnum("Overdue") } };
var Project = { Status: { Active: new FakeEnum("Active"), Done: new FakeEnum("Done"),
  Dropped: new FakeEnum("Dropped"), OnHold: new FakeEnum("OnHold") } };
var Tag = { Status: { Active: new FakeEnum("Active"), OnHold: new FakeEnum("OnHold") } };
var NOW = new Date(Date.UTC(2026, 4, 10, 12, 0));
const child = { name: "child", completed: false, taskStatus: Task.Status.Available,
  effectiveCompletionDate: null, effectiveDropDate: null, effectiveDeferDate: null,
  containingProject: null, tags: [] };
const project = { status: Project.Status.Active, containsSingletonActions: false,
  flattenedTasks: [child], nextTask: null };
const root = { name: "the project", project: project };
child.containingProject = project;"#;

fn run(snippet: &str) -> String {
    let prelude = format!("{FAKES}\n{JS_PROJECT_STATUS}\n{JS_TASK_STATUS}");
    common::run_jsc(&prelude, snippet)
}

#[test]
fn next_task_is_the_child_omnifocus_names() {
    assert_eq!(
        run("project.nextTask = child; print(projectNextTask(project).name);"),
        "child"
    );
}

#[test]
fn next_task_is_none_when_omnifocus_returns_the_root_task() {
    assert_eq!(
        run("project.nextTask = root; print(projectNextTask(project));"),
        "null"
    );
}

#[test]
fn next_task_is_none_when_omnifocus_returns_none() {
    assert_eq!(run("print(projectNextTask(project));"), "null");
}

#[test]
fn project_with_remaining_tasks_and_only_its_root_as_next_task_is_stalled() {
    assert_eq!(
        run("project.nextTask = root; print(isProjectStalled(project, NOW));"),
        "true"
    );
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

/// The next task comes from the helper, never straight from `project.nextTask`.
fn assert_next_task_from_helper(tool: &str, script: &str) {
    assert!(
        script.contains("const nextTask = projectNextTask(project);"),
        "{tool} must read the next task through projectNextTask"
    );
    assert!(
        !script.contains("const nextTask = project.nextTask;"),
        "{tool} still reads project.nextTask directly"
    );
}

/// The modified date is the root task's; `Project` has none of its own.
fn assert_modified_from_root_task(tool: &str, script: &str) {
    assert!(
        script.contains(
            "modified: project.task.modified ? project.task.modified.toISOString() : null,"
        ),
        "{tool} must report project.task.modified"
    );
    assert!(
        !script.contains("project.modified"),
        "{tool} still reads the nonexistent project.modified"
    );
}

#[tokio::test]
async fn get_project_reports_next_task_and_modified_through_the_root_task() {
    let runner = CapturingRunner::new(json!({}));
    get_project(&runner, "Errands")
        .await
        .expect("get_project runs");
    let script = runner.last_script();
    assert_next_task_from_helper("get_project", &script);
    assert_modified_from_root_task("get_project", &script);
    common::assert_script_compiles("get_project", &script);
}

#[tokio::test]
async fn list_projects_reports_next_task_through_the_helper() {
    let runner = CapturingRunner::new(json!([]));
    list_projects(&runner, None, "active", None, None, false, None, "asc", 5)
        .await
        .expect("list_projects runs");
    let script = runner.last_script();
    assert_next_task_from_helper("list_projects", &script);
    common::assert_script_compiles("list_projects", &script);
}

#[tokio::test]
async fn create_project_reports_modified_through_the_root_task() {
    let runner = CapturingRunner::new(json!({}));
    create_project(&runner, "Errands", None, None, None, None, None)
        .await
        .expect("create_project runs");
    let script = runner.last_script();
    assert_modified_from_root_task("create_project", &script);
    common::assert_script_compiles("create_project", &script);
}
