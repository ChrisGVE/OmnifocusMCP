//! `move_task` and `move_tasks_batch` refuse to move a task under itself or
//! under one of its own descendants, and confirm a move out of a parent task
//! really left it.
//!
//! Both guards used to walk `ancestor.containingTask`, a property OmniJS does
//! not have: it reads `undefined`, so the walk stopped after the destination
//! itself and moving a task under its child or grandchild was not refused, and
//! the post-move "still nested" check never fired. OmniJS documents
//! `Task.parent` ("The parent Task which contains this task"); a top-level
//! task's parent is its project's root task, which belongs to no task tree.
//!
//! Each script runs in JavaScriptCore against a fake task tree, so the
//! assertions are on what the tool does, not on its text. The fakes only
//! define documented OmniJS properties, so a script reading one that does not
//! exist sees `undefined`, as it would in OmniFocus.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    error::OmniFocusError,
    jxa::JxaRunner,
    tools::tasks::{move_task, move_tasks_batch},
};
use serde_json::{json, Value};

/// Project "Work" holds `a`, whose child is `b`, whose child is `c`, and an
/// unrelated top-level task `d`; `i` sits in the inbox. Top-level tasks have
/// the project's root task as their parent, the way OmniJS reports them.
///
/// `moveTasks` records what it moved in `MOVED` and re-parents each task the
/// way OmniFocus would; with `MOVE_LEAVES_NESTED` it moves nothing, standing in
/// for a move OmniFocus did not carry out.
const FAKE_TREE: &str = r#"var Task = { Status: {} };
var Tag = { Status: {}, byIdentifier: function () { return null; } };
var work = { id: { primaryKey: "p1" }, name: "Work" };
work.ending = { kind: "project", project: work };
var PROJECTS = [work];
var Project = { Status: {}, byIdentifier: function (id) {
  return PROJECTS.find(function (p) { return p.id.primaryKey === id; }) || null;
} };
var root = { id: { primaryKey: "p1" }, name: "Work", parent: null, project: work,
  containingProject: work, inInbox: false };
work.task = root;
function fakeTask(id, parent, project) {
  const task = { id: { primaryKey: id }, name: "task " + id, parent: parent, project: null,
    containingProject: project, inInbox: project === null };
  task.ending = { kind: "task", task: task };
  return task;
}
var a = fakeTask("a", root, work);
var b = fakeTask("b", a, work);
var c = fakeTask("c", b, work);
var d = fakeTask("d", root, work);
var i = fakeTask("i", null, null);
var inbox = [i];
inbox.ending = { kind: "inbox" };
var document = { flattenedTasks: [root, a, b, c, d, i], flattenedProjects: PROJECTS };
document.flattenedProjects.byName = function (name) {
  return PROJECTS.find(function (p) { return p.name === name; }) || null;
};
var MOVED = [];
function moveTasks(tasks, location) {
  for (const task of tasks) {
    MOVED.push(task.id.primaryKey);
    if (MOVE_LEAVES_NESTED) continue;
    if (location.kind === "task") {
      task.parent = location.task;
      task.containingProject = location.task.containingProject;
      task.inInbox = location.task.inInbox;
    } else if (location.kind === "project") {
      task.parent = location.project.task;
      task.containingProject = location.project;
      task.inInbox = false;
    } else {
      task.parent = null;
      task.containingProject = null;
      task.inInbox = true;
    }
  }
}"#;

/// Records the script it is asked to run and returns an empty object.
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
        Box::pin(async move { Ok(json!({})) })
    }
}

/// What a script did against the fake tree: the error it threw (or `null`),
/// what it returned, and the ids `moveTasks` was asked to move.
fn run_against_tree(tool: &str, script: &str, move_leaves_nested: bool) -> Value {
    common::assert_script_compiles(tool, script);
    let prelude = format!("var MOVE_LEAVES_NESTED = {move_leaves_nested};\n{FAKE_TREE}");
    let output = common::run_jsc(
        &prelude,
        &format!(
            "let outcome;\n\
             try {{ outcome = {{ error: null, result: (function () {{\n{script}\n}})() }}; }}\n\
             catch (error) {{ outcome = {{ error: error.message, result: null }}; }}\n\
             outcome.moved = MOVED;\n\
             print(JSON.stringify(outcome));"
        ),
    );
    serde_json::from_str(&output).unwrap_or_else(|_| panic!("{tool} printed {output}"))
}

async fn move_task_outcome(
    task_id: &str,
    project: Option<&str>,
    parent: Option<&str>,
    move_leaves_nested: bool,
) -> Value {
    let runner = CapturingRunner::new();
    move_task(&runner, task_id, project, parent)
        .await
        .expect("move_task builds its script");
    run_against_tree("move_task", &runner.last_script(), move_leaves_nested)
}

async fn move_batch_outcome(
    task_ids: &[&str],
    project: Option<&str>,
    parent: Option<&str>,
    move_leaves_nested: bool,
) -> Value {
    let runner = CapturingRunner::new();
    let ids = task_ids.iter().map(|id| id.to_string()).collect();
    move_tasks_batch(&runner, ids, project, parent)
        .await
        .expect("move_tasks_batch builds its script");
    run_against_tree(
        "move_tasks_batch",
        &runner.last_script(),
        move_leaves_nested,
    )
}

fn assert_refused(outcome: &Value, message: &str) {
    assert_eq!(outcome["error"], message, "{outcome}");
    assert_eq!(outcome["moved"], json!([]), "nothing may move: {outcome}");
}

const SINGLE_UNDER_ITSELF: &str = "Cannot move a task under itself.";
const SINGLE_UNDER_DESCENDANT: &str = "Cannot move a task under its own descendant.";
const BATCH_UNDER_DESCENDANT: &str = "Cannot move tasks under their own descendant.";
const STILL_NESTED: &str = "Task move failed: task is still nested under a parent.";

#[tokio::test]
async fn move_task_refuses_a_task_under_itself() {
    let outcome = move_task_outcome("a", None, Some("a"), false).await;
    assert_refused(&outcome, SINGLE_UNDER_ITSELF);
}

#[tokio::test]
async fn move_task_refuses_a_task_under_its_direct_child() {
    let outcome = move_task_outcome("a", None, Some("b"), false).await;
    assert_refused(&outcome, SINGLE_UNDER_DESCENDANT);
}

#[tokio::test]
async fn move_task_refuses_a_task_under_its_grandchild() {
    let outcome = move_task_outcome("a", None, Some("c"), false).await;
    assert_refused(&outcome, SINGLE_UNDER_DESCENDANT);
}

/// The walk climbs from `c` through `b` and `a` to the project's root task
/// and stops there without refusing.
#[tokio::test]
async fn move_task_moves_a_task_under_an_unrelated_nested_task() {
    let outcome = move_task_outcome("d", None, Some("c"), false).await;
    assert_eq!(outcome["error"], Value::Null, "{outcome}");
    assert_eq!(outcome["moved"], json!(["d"]));
    assert_eq!(outcome["result"]["id"], "d");
    assert_eq!(outcome["result"]["projectName"], "Work");
}

/// A top-level task's parent is the project's root task, which is not a
/// nesting parent, so a move to a project passes the post-move check.
#[tokio::test]
async fn move_task_moves_a_subtask_to_the_top_of_a_project() {
    let outcome = move_task_outcome("c", Some("Work"), None, false).await;
    assert_eq!(outcome["error"], Value::Null, "{outcome}");
    assert_eq!(outcome["moved"], json!(["c"]));
    assert_eq!(outcome["result"]["projectName"], "Work");
    assert_eq!(outcome["result"]["inInbox"], false);
}

#[tokio::test]
async fn move_task_moves_a_subtask_to_the_inbox() {
    let outcome = move_task_outcome("c", None, None, false).await;
    assert_eq!(outcome["error"], Value::Null, "{outcome}");
    assert_eq!(outcome["result"]["inInbox"], true);
    assert_eq!(outcome["result"]["projectName"], Value::Null);
}

#[tokio::test]
async fn move_task_reports_a_subtask_left_under_its_parent() {
    let outcome = move_task_outcome("c", None, None, true).await;
    assert_eq!(outcome["error"], STILL_NESTED, "{outcome}");
}

/// The batch refuses a parent that is one of the moved tasks before any
/// script runs.
#[tokio::test]
async fn move_tasks_batch_refuses_a_task_under_itself() {
    let runner = CapturingRunner::new();
    let error = move_tasks_batch(&runner, vec!["a".to_string()], None, Some("a"))
        .await
        .expect_err("a task cannot be its own parent");
    assert!(
        matches!(&error, OmniFocusError::Validation(message)
            if message == "parent_task_id must not be included in task_ids (cannot move a task under itself)."),
        "{error:?}"
    );
    assert!(runner.scripts.lock().expect("script lock").is_empty());
}

#[tokio::test]
async fn move_tasks_batch_refuses_a_task_under_its_direct_child() {
    let outcome = move_batch_outcome(&["d", "a"], None, Some("b"), false).await;
    assert_refused(&outcome, BATCH_UNDER_DESCENDANT);
}

#[tokio::test]
async fn move_tasks_batch_refuses_a_task_under_its_grandchild() {
    let outcome = move_batch_outcome(&["a"], None, Some("c"), false).await;
    assert_refused(&outcome, BATCH_UNDER_DESCENDANT);
}

#[tokio::test]
async fn move_tasks_batch_moves_tasks_under_an_unrelated_nested_task() {
    let outcome = move_batch_outcome(&["d", "i"], None, Some("c"), false).await;
    assert_eq!(outcome["error"], Value::Null, "{outcome}");
    assert_eq!(outcome["moved"], json!(["d", "i"]));
    assert_eq!(outcome["result"]["moved_count"], 2);
    assert_eq!(outcome["result"]["failed_count"], 0);
}

#[tokio::test]
async fn move_tasks_batch_moves_subtasks_to_the_top_of_a_project() {
    let outcome = move_batch_outcome(&["b", "c"], Some("Work"), None, false).await;
    assert_eq!(outcome["error"], Value::Null, "{outcome}");
    assert_eq!(outcome["result"]["moved_count"], 2);
}

#[tokio::test]
async fn move_tasks_batch_reports_a_subtask_left_under_its_parent() {
    let outcome = move_batch_outcome(&["c"], None, None, true).await;
    assert_eq!(outcome["error"], STILL_NESTED, "{outcome}");
}
