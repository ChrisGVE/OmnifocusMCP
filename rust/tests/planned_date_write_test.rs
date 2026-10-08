//! Planned dates on write tools, and the `create_project` / `create_tag`
//! return shapes.
//!
//! `create_task`, `create_subtask`, `create_tasks_batch` and `update_task`
//! accept an optional `plannedDate`. It is parsed like every other write date
//! (a bare `YYYY-MM-DD` becomes that local day at the user's default planned
//! time), and on a database that predates planned dates the call fails before
//! anything changes. When `plannedDate` is absent the property is never
//! written, and the returned task still carries a `plannedDate` field.
//!
//! `create_project` and `create_tag` return the same object shape as
//! `get_project` and a `list_tags` tag respectively, not just an id.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    jxa::JxaRunner,
    tools::{
        projects::create_project,
        tags::create_tag,
        tasks::{create_subtask, create_task, create_tasks_batch, update_task, CreateTaskInput},
    },
};
use serde_json::{json, Value};

const PLANNED_WRITE: &str =
    r#"parseWriteDate(plannedDateValue, "plannedDate", "DefaultPlannedTime", "09:00")"#;

#[derive(Clone)]
struct CapturingRunner {
    script: Arc<Mutex<String>>,
}

impl CapturingRunner {
    fn new() -> Self {
        Self {
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
        *self.script.lock().expect("script lock") = script.to_string();
        Box::pin(async move { Ok(json!({})) })
    }
}

async fn create_task_script(planned: Option<&str>) -> String {
    let runner = CapturingRunner::new();
    create_task(
        &runner, "Planned", None, None, None, None, planned, None, None, None,
    )
    .await
    .expect("create_task builds a script");
    runner.script()
}

// ---------------------------------------------------------------- wiring

#[tokio::test]
async fn create_task_parses_planned_date_as_a_write_date() {
    let script = create_task_script(Some("2026-05-15")).await;
    assert!(script.contains(&format!(
        "const parsedPlannedDate = plannedDateValue === null ? null : {PLANNED_WRITE};"
    )));
    assert!(
        script.contains("if (parsedPlannedDate !== null) setPlannedDate(task, parsedPlannedDate);")
    );
    assert!(script.contains(
        "const supportsPlannedDate = detectPlannedDateSupport(document.flattenedTasks);"
    ));
    assert!(script.contains("if (parsedPlannedDate !== null && !supportsPlannedDate) {"));
    assert!(script.contains("plannedDate: plannedDate ? plannedDate.toISOString() : null"));
    // The support check and the write guard run before the task is created.
    let guard = script
        .find("plannedDate requires an OmniFocus database")
        .unwrap();
    let created = script.find("new Task(").unwrap();
    assert!(guard < created);
}

#[tokio::test]
async fn create_subtask_writes_planned_date() {
    let runner = CapturingRunner::new();
    create_subtask(
        &runner,
        "Child",
        "parent-1",
        None,
        None,
        None,
        Some("2026-05-15"),
        None,
        None,
        None,
    )
    .await
    .expect("create_subtask builds a script");
    let script = runner.script();
    assert!(script.contains(&format!(
        "const parsedPlannedDate = plannedDateValue === null ? null : {PLANNED_WRITE};"
    )));
    assert!(
        script.contains("if (parsedPlannedDate !== null) setPlannedDate(task, parsedPlannedDate);")
    );
    assert!(script.contains("plannedDate: plannedDate ? plannedDate.toISOString() : null"));
}

#[tokio::test]
async fn create_tasks_batch_writes_planned_date() {
    let runner = CapturingRunner::new();
    let input = CreateTaskInput {
        name: "Batch".to_string(),
        project: None,
        note: None,
        due_date: None,
        defer_date: None,
        planned_date: Some("2026-05-15".to_string()),
        flagged: None,
        tags: None,
        estimated_minutes: None,
    };
    create_tasks_batch(&runner, vec![input])
        .await
        .expect("create_tasks_batch builds a script");
    let script = runner.script();
    assert!(script.contains(
        r#"parseWriteDate(input.plannedDate, "tasks[" + index + "].plannedDate", "DefaultPlannedTime", "09:00")"#
    ));
    assert!(
        script.contains("if (dates.plannedDate !== null) setPlannedDate(task, dates.plannedDate);")
    );
    assert!(script.contains(
        "if (parsedDates.some(dates => dates.plannedDate !== null) && !supportsPlannedDate) {"
    ));
    assert!(script.contains("plannedDate: plannedDate ? plannedDate.toISOString() : null"));
}

#[tokio::test]
async fn update_task_writes_planned_date() {
    let runner = CapturingRunner::new();
    update_task(
        &runner,
        "task-1",
        None,
        None,
        None,
        None,
        Some("2026-05-15"),
        None,
        None,
        None,
    )
    .await
    .expect("update_task builds a script");
    let script = runner.script();
    assert!(script.contains(
        r#"const parsedPlannedDate = has("plannedDate") ? parseWriteDate(updates.plannedDate, "plannedDate", "DefaultPlannedTime", "09:00") : null;"#
    ));
    assert!(script.contains(r#"if (has("plannedDate")) setPlannedDate(task, parsedPlannedDate);"#));
    assert!(script.contains("plannedDate: plannedDate ? plannedDate.toISOString() : null"));
    // The write guard runs before any field is modified.
    let guard = script
        .find("plannedDate requires an OmniFocus database")
        .unwrap();
    let first_mutation = script.find("task.name = updates.name").unwrap();
    assert!(guard < first_mutation);
}

// ---------------------------------------------------------------- run in jsc

/// A database with planned-date support and no existing tasks, plus a counter
/// of the tasks the script creates.
const SUPPORTED_DATABASE: &str = r#"var createdTasks = [];
function Task(name, parent) {
  createdTasks.push(this);
  this.id = { primaryKey: "task-" + createdTasks.length };
  this.name = name;
  this.note = null;
  this.dueDate = null;
  this.deferDate = null;
  this.plannedDate = null;
  this.flagged = false;
  this.estimatedMinutes = null;
  this.tags = [];
  this.addTag = function (tag) { this.tags.push(tag); };
}
var inbox = { ending: {} };
var emptyByName = [];
emptyByName.byName = function () { return null; };
var document = {
  flattenedTasks: [],
  flattenedProjects: emptyByName,
  flattenedFolders: emptyByName,
  flattenedTags: emptyByName
};
var Project = { byIdentifier: function () { return null; } };
var Tag = { byIdentifier: function () { return null; } };
var Folder = { byIdentifier: function () { return null; } };"#;

/// A database without planned-date support: reading the property on an
/// existing task throws, as does assigning it on a newly created task.
const UNSUPPORTED_DATABASE: &str = r#"var createdTasks = [];
function Task(name, parent) {
  createdTasks.push(this);
  this.id = { primaryKey: "task-" + createdTasks.length };
  this.name = name;
  this.note = null;
  this.dueDate = null;
  this.deferDate = null;
  this.flagged = false;
  this.estimatedMinutes = null;
  this.tags = [];
  this.addTag = function (tag) { this.tags.push(tag); };
  Object.defineProperty(this, "plannedDate", {
    get: function () { throw new Error("plannedDate is not supported"); },
    set: function () { throw new Error("plannedDate is not supported"); }
  });
}
var existing = {
  id: { primaryKey: "existing-1" }, name: "existing", note: null, dueDate: null,
  deferDate: null, flagged: false, estimatedMinutes: null, tags: [],
  addTag: function () {}
};
Object.defineProperty(existing, "plannedDate", {
  get: function () { throw new Error("plannedDate is not supported"); }
});
var inbox = { ending: {} };
var emptyByName = [];
emptyByName.byName = function () { return null; };
var document = {
  flattenedTasks: [existing],
  flattenedProjects: emptyByName,
  flattenedFolders: emptyByName,
  flattenedTags: emptyByName
};
var Project = { byIdentifier: function () { return null; } };
var Tag = { byIdentifier: function () { return null; } };
var Folder = { byIdentifier: function () { return null; } };"#;

/// Runs a captured `create_task` script against `database` and returns either
/// the JSON result plus the number of created tasks, or the error.
fn run_create_task(database: &str, script: &str) -> String {
    common::assert_script_compiles("create_task", script);
    common::run_jsc(
        database,
        &format!(
            "try {{ const result = (function () {{\n{script}\n}})(); \
             print(JSON.stringify({{ result: result, created: createdTasks.length }})); }} \
             catch (error) {{ print(\"ERROR: \" + error.message + \
             \"; CREATED: \" + createdTasks.length); }}"
        ),
    )
}

#[tokio::test]
async fn create_task_writes_the_planned_date_on_a_supported_database() {
    let script = create_task_script(Some("2026-05-15")).await;
    let output = run_create_task(SUPPORTED_DATABASE, &script);
    let value: Value =
        serde_json::from_str(&output).unwrap_or_else(|_| panic!("expected JSON, got {output}"));
    assert_eq!(value["created"], 1);
    assert_eq!(value["result"]["plannedDate"], "2026-05-15T09:00:00.000Z");
}

#[tokio::test]
async fn create_task_fails_before_creating_on_an_unmigrated_database() {
    let script = create_task_script(Some("2026-05-15")).await;
    assert_eq!(
        run_create_task(UNSUPPORTED_DATABASE, &script),
        "ERROR: plannedDate requires an OmniFocus database migrated to support planned dates; CREATED: 0"
    );
}

#[tokio::test]
async fn create_task_without_planned_date_does_not_touch_it() {
    let script = create_task_script(None).await;
    let output = run_create_task(UNSUPPORTED_DATABASE, &script);
    let value: Value =
        serde_json::from_str(&output).unwrap_or_else(|_| panic!("expected JSON, got {output}"));
    assert_eq!(value["created"], 1);
    assert_eq!(value["result"]["plannedDate"], Value::Null);
}

// ---------------------------------------------------------------- return shapes

#[tokio::test]
async fn create_project_returns_the_get_project_shape() {
    let runner = CapturingRunner::new();
    create_project(&runner, "New", None, None, None, None, None)
        .await
        .expect("create_project builds a script");
    let script = runner.script();
    common::assert_script_compiles("create_project", &script);
    for line in [
        "id: project.id.primaryKey,",
        "name: project.name,",
        "status: normalizeProjectStatus(project),",
        "folderName: project.parentFolder ? project.parentFolder.name : null,",
        "taskCount: 0,",
        "remainingTaskCount: 0,",
        "completedTaskCount: 0,",
        "availableTaskCount: 0,",
        "deferDate: project.deferDate ? project.deferDate.toISOString() : null,",
        "dueDate: project.dueDate ? project.dueDate.toISOString() : null,",
        "completionDate: null,",
        "modified: project.task.modified ? project.task.modified.toISOString() : null,",
        "note: project.note,",
        "sequential: project.sequential,",
        "isStalled: false,",
        "nextTaskId: null,",
        "nextTaskName: null,",
        "reviewInterval: null,",
        "rootTasks: []",
    ] {
        assert!(
            script.contains(line),
            "create_project missing {line}\n{script}"
        );
    }
}

#[tokio::test]
async fn create_tag_returns_the_list_tags_shape() {
    let runner = CapturingRunner::new();
    create_tag(&runner, "Home", None)
        .await
        .expect("create_tag builds a script");
    let script = runner.script();
    common::assert_script_compiles("create_tag", &script);
    for line in [
        "id: tag.id.primaryKey,",
        "name: tag.name,",
        "parent: tag.parent ? tag.parent.name : null,",
        "availableTaskCount: 0,",
        "totalTaskCount: 0,",
        "status: \"active\"",
    ] {
        assert!(script.contains(line), "create_tag missing {line}\n{script}");
    }
}
