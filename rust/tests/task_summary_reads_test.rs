//! `get_inbox` and `list_subtasks` return the same task summary as `list_tasks`.
//!
//! Both tools used to build their own copy of the summary, and the copies had
//! drifted: `list_subtasks` emitted no `projectName`, `plannedDate` or
//! `completionDate`, and `get_inbox` no `plannedDate`. The Rust side fills a
//! missing field with `null`, so the gap was silent — seen live on nested
//! subtasks of a project, which reported `projectName: null`.
//!
//! Each script runs in JavaScriptCore against a small fake database, so the
//! assertions are on what the tool returns, not on its text.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    jxa::JxaRunner,
    tools::{
        tasks::{get_inbox, list_subtasks},
        utility::uncomplete_task,
    },
};
use serde_json::{json, Value};

/// One project holding a parent task with a completed, planned child, an
/// inbox task with a planned date, and a completed project whose open task was
/// left unmarked (OmniFocus keeps that task's own `completed` false). `PLANNED_THROWS` simulates a database not
/// migrated for planned dates, where reading `plannedDate` throws.
const FAKE_DATABASE: &str = r#"function FakeEnum(name) { this.name = name; }
FakeEnum.prototype.toString = function () { return "[object Task.Status: " + this.name + "]"; };
var Task = { Status: { Available: new FakeEnum("Available"), Completed: new FakeEnum("Completed") } };
var Project = { Status: { Active: new FakeEnum("Active"), Done: new FakeEnum("Done") } };
var PLANNED = new Date(Date.UTC(2026, 9, 9, 7, 0));
var DONE = new Date(Date.UTC(2026, 9, 1, 12, 0));
function fakeTask(id, fields) {
  const task = { id: { primaryKey: id }, name: "task " + id, note: "", flagged: false,
    dueDate: null, deferDate: null, added: null, modified: null, completed: false,
    completionDate: null, containingProject: null, project: null, tags: [],
    estimatedMinutes: null, inInbox: false, hasChildren: false, sequential: false,
    taskStatus: Task.Status.Available, children: [] };
  for (const key in fields) task[key] = fields[key];
  if (PLANNED_THROWS) {
    Object.defineProperty(task, "plannedDate", { get: function () { throw new Error("no planned"); } });
  } else if (!("plannedDate" in task)) {
    task.plannedDate = null;
  }
  return task;
}
var education = { id: { primaryKey: "p1" }, name: "Education" };
var child = fakeTask("child", { containingProject: education, completed: true,
  completionDate: DONE, taskStatus: Task.Status.Completed, plannedDate: PLANNED });
var parent = fakeTask("parent", { containingProject: education, hasChildren: true,
  children: [child] });
var inboxTask = fakeTask("inbox", { inInbox: true, plannedDate: PLANNED });
var inbox = [inboxTask];
var closed = { id: { primaryKey: "p2" }, name: "Closed", status: Project.Status.Done };
var leftover = fakeTask("leftover", { containingProject: closed });
var closedParent = fakeTask("closed-parent", { containingProject: closed, hasChildren: true,
  children: [leftover] });
var document = { flattenedTasks: [parent, child, inboxTask, closedParent, leftover] };"#;

/// Records the script it is asked to run and returns an empty list.
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
        Box::pin(async move { Ok(json!([])) })
    }
}

/// Runs `script` against the fake database and returns its single result.
fn run_against_database(tool: &str, script: &str, planned_throws: bool) -> Value {
    common::assert_script_compiles(tool, script);
    let prelude = format!("var PLANNED_THROWS = {planned_throws};\n{FAKE_DATABASE}");
    let output = common::run_jsc(
        &prelude,
        &format!(
            "try {{ const result = (function () {{\n{script}\n}})(); print(JSON.stringify(result)); }}\n\
             catch (error) {{ print(\"ERROR: \" + error.message); }}"
        ),
    );
    let items: Value =
        serde_json::from_str(&output).unwrap_or_else(|_| panic!("{tool} returned {output}"));
    assert_eq!(items.as_array().map(Vec::len), Some(1), "{tool}: {items}");
    items[0].clone()
}

async fn subtasks_script_of(parent: &str) -> String {
    let runner = CapturingRunner::new();
    list_subtasks(&runner, parent, 10)
        .await
        .expect("list_subtasks runs");
    runner.last_script()
}

async fn subtasks_script() -> String {
    let runner = CapturingRunner::new();
    list_subtasks(&runner, "parent", 10)
        .await
        .expect("list_subtasks runs");
    runner.last_script()
}

async fn inbox_script() -> String {
    let runner = CapturingRunner::new();
    get_inbox(&runner, 10).await.expect("get_inbox runs");
    runner.last_script()
}

#[tokio::test]
async fn list_subtasks_reports_project_planned_and_completion_dates() {
    let subtask = run_against_database("list_subtasks", &subtasks_script().await, false);
    assert_eq!(subtask["id"], "child");
    assert_eq!(subtask["projectName"], "Education");
    assert_eq!(subtask["plannedDate"], "2026-10-09T07:00:00.000Z");
    assert_eq!(subtask["completionDate"], "2026-10-01T12:00:00.000Z");
}

#[tokio::test]
async fn get_inbox_reports_planned_date() {
    let task = run_against_database("get_inbox", &inbox_script().await, false);
    assert_eq!(task["id"], "inbox");
    assert_eq!(task["plannedDate"], "2026-10-09T07:00:00.000Z");
}

#[tokio::test]
async fn both_report_no_planned_date_on_an_unmigrated_database() {
    let subtask = run_against_database("list_subtasks", &subtasks_script().await, true);
    assert_eq!(subtask["plannedDate"], Value::Null);
    let task = run_against_database("get_inbox", &inbox_script().await, true);
    assert_eq!(task["plannedDate"], Value::Null);
}

/// `completed` means what the `completed` status filter means: done directly,
/// through a containing task, or through a completed project. A task left open
/// in a project that was then completed is done, though its own flag is false.
#[tokio::test]
async fn a_task_in_a_completed_project_reports_completed() {
    let leftover = run_against_database(
        "list_subtasks",
        &subtasks_script_of("closed-parent").await,
        false,
    );
    assert_eq!(leftover["id"], "leftover");
    assert_eq!(leftover["completed"], true);
}

#[tokio::test]
async fn an_open_task_in_an_active_project_reports_not_completed() {
    let task = run_against_database("get_inbox", &inbox_script().await, false);
    assert_eq!(task["completed"], false);
}

/// Reopening such a task means reopening its project; `uncomplete_task` says
/// so instead of claiming the task is not completed.
#[tokio::test]
async fn uncompleting_a_task_closed_by_its_project_names_the_project() {
    let runner = CapturingRunner::new();
    uncomplete_task(&runner, "leftover")
        .await
        .expect("uncomplete_task runs");
    let script = runner.last_script();
    common::assert_script_compiles("uncomplete_task", &script);
    let prelude = format!("var PLANNED_THROWS = false;\n{FAKE_DATABASE}");
    let output = common::run_jsc(
        &prelude,
        &format!(
            "try {{ (function () {{\n{script}\n}})(); print(\"no error\"); }}\n\
             catch (error) {{ print(error.message); }}"
        ),
    );
    assert_eq!(
        output,
        "Task leftover is completed through its project or a containing task; \
         reopen that instead: Closed"
    );
}
