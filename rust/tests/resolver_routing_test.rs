//! Every project, folder and task lookup goes through `JS_RESOLVERS` (audit CR-019).
//!
//! Tools used to carry their own copies of the lookups:
//! `find(item => item.id.primaryKey === x || item.name === x)` for projects
//! and folders, and `flattenedTasks.find(item => item.id.primaryKey === x &&
//! !isProjectRootTask(item))` for tasks. The project and folder copies
//! returned whichever object came first in database order, so an object
//! *named* like another object's id could be picked instead of the object
//! with that id. These tests run the resolvers and whole tool scripts in
//! JavaScriptCore against a small fake OmniFocus database, and check that no
//! tool keeps a private copy.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    js_helpers::{JS_RESOLVERS, JS_TASK_STATUS},
    jxa::JxaRunner,
    tools::{
        folders::{delete_folder, get_folder, update_folder},
        projects::{
            complete_project, delete_project, delete_projects_batch, get_project, move_project,
            set_project_status, uncomplete_project, update_project,
        },
        tasks::{
            add_notification, complete_task, create_subtask, delete_task, duplicate_task, get_task,
            list_notifications, list_subtasks, move_task, move_tasks_batch, remove_notification,
            set_task_repetition, update_task,
        },
        utility::{append_to_note, uncomplete_task},
    },
};
use serde_json::{json, Value};

// ---------------------------------------------------------------- fake database

/// A small OmniFocus database. Project "pS" is *named* "p1" and comes first
/// in database order, before project "p1" (Alpha); folder "fS" is named "f1"
/// and comes before folder "f1" (Work). A lookup that takes the first object
/// whose id or name matches picks the impostor; a resolver tries the id
/// first. Every project has a root task with the project's own id, which
/// `document.flattenedTasks` lists, as OmniFocus documents it does.
const FAKE_DATABASE: &str = r#"function FakeEnum(name) { this.name = name; }
var Project = {
  Status: { Active: new FakeEnum("Active"), Done: new FakeEnum("Done"),
    Dropped: new FakeEnum("Dropped"), OnHold: new FakeEnum("OnHold") },
  byIdentifier: function (id) { return projects.find(p => p.id.primaryKey === id) || null; }
};
var Folder = {
  Status: { Active: new FakeEnum("Active"), Dropped: new FakeEnum("Dropped") },
  byIdentifier: function (id) { return folders.find(f => f.id.primaryKey === id) || null; }
};
var Tag = { byIdentifier: function () { return null; } };
var Task = {
  Status: { Available: new FakeEnum("Available"), Completed: new FakeEnum("Completed"),
    Dropped: new FakeEnum("Dropped") },
  byIdentifier: function (id) { return tasks.find(t => t.id.primaryKey === id) || null; }
};
function fakeFolder(id, name) {
  return { id: { primaryKey: id }, name: name, status: Folder.Status.Active, parent: null,
    projects: [], folders: [], ending: {} };
}
function fakeProject(id, name, folder) {
  const project = { id: { primaryKey: id }, name: name, status: Project.Status.Active,
    completed: false, parentFolder: folder, flattenedTasks: [], tasks: [], ending: {},
    markComplete: function () { this.completed = true; },
    markIncomplete: function () { this.completed = false; } };
  project.rootTask = { id: { primaryKey: id }, name: name, project: project, children: [] };
  return project;
}
function fakeTask(id, project) {
  return { id: { primaryKey: id }, name: "Task " + id, project: null,
    containingProject: project, parent: null, children: [], tags: [], completed: false,
    taskStatus: Task.Status.Available, notifications: [],
    markComplete: function () { this.completed = true; } };
}
var impostorFolder = fakeFolder("fS", "f1");
var work = fakeFolder("f1", "Work");
var folders = [impostorFolder, work];
var impostorProject = fakeProject("pS", "p1", work);
var alpha = fakeProject("p1", "Alpha", work);
var projects = [impostorProject, alpha];
var task = fakeTask("t1", alpha);
var tasks = [impostorProject.rootTask, alpha.rootTask, task];
var deleted = [];
function deleteObject(object) { deleted.push(object.id.primaryKey); }
var library = { ending: {} };
function moveSections() {}
var document = { flattenedFolders: folders, flattenedProjects: projects, flattenedTasks: tasks,
  flattenedTags: [] };"#;

/// Runs a complete tool script (it ends in a top-level `return`, as
/// `crate::jxa` wraps it) against the fake database. Returns the JSON
/// result, or "ERROR: <message>" when the script throws.
fn run_tool_script(script: &str) -> String {
    common::assert_script_compiles("tool", script);
    common::run_jsc(
        FAKE_DATABASE,
        &format!(
            "try {{ const result = (function () {{\n{script}\n}})(); print(JSON.stringify(result)); }}\n\
             catch (error) {{ print(\"ERROR: \" + error.message); }}"
        ),
    )
}

/// Evaluates a resolver call against the fake database and prints the id
/// of what it returned, or "ERROR: <message>".
fn resolve(expression: &str) -> String {
    common::run_jsc(
        &format!("{FAKE_DATABASE}\n{JS_TASK_STATUS}\n{JS_RESOLVERS}"),
        &format!(
            "try {{ print({expression}.id.primaryKey); }} \
             catch (error) {{ print(\"ERROR: \" + error.message); }}"
        ),
    )
}

fn result_id(output: &str) -> Value {
    let value: Value = serde_json::from_str(output)
        .unwrap_or_else(|_| panic!("expected a JSON result, got {output}"));
    value["id"].clone()
}

// ---------------------------------------------------------------- capture

/// Records every script it is asked to run and returns an empty array,
/// which every tool used here accepts as a result.
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
        Box::pin(async { Ok(json!([])) })
    }
}

// ---------------------------------------------------------------- resolveTask

#[test]
fn resolve_task_finds_a_task_by_id() {
    assert_eq!(resolve(r#"resolveTask("t1")"#), "t1");
}

#[test]
fn resolve_task_rejects_a_project_root_task_as_not_found() {
    // "p1" is the id of Alpha's root task: OmniFocus lists it among the
    // tasks, but it is the project, not an action.
    assert_eq!(resolve(r#"resolveTask("p1")"#), "ERROR: Task not found: p1");
}

#[test]
fn resolve_task_reports_a_missing_task() {
    assert_eq!(
        resolve(r#"resolveTask("nope")"#),
        "ERROR: Task not found: nope"
    );
}

#[test]
fn resolve_task_names_the_role_it_was_given() {
    assert_eq!(
        resolve(r#"resolveTask("nope", "Parent task")"#),
        "ERROR: Parent task not found: nope"
    );
}

// ---------------------------------------------------------------- projects run

#[tokio::test]
async fn project_tools_pick_the_project_with_the_id_not_one_named_like_it() {
    let runner = CapturingRunner::new();
    complete_project(&runner, "p1").await.expect("script");
    let complete = run_tool_script(&runner.last_script());
    delete_project(&runner, "p1").await.expect("script");
    let delete = run_tool_script(&runner.last_script());
    set_project_status(&runner, "p1", "on_hold")
        .await
        .expect("script");
    let status = run_tool_script(&runner.last_script());
    move_project(&runner, "p1", None).await.expect("script");
    let moved = run_tool_script(&runner.last_script());
    for (tool, output) in [
        ("complete_project", complete),
        ("delete_project", delete),
        ("set_project_status", status),
        ("move_project", moved),
    ] {
        assert_eq!(result_id(&output), json!("p1"), "{tool}: {output}");
    }
}

#[tokio::test]
async fn uncomplete_project_checks_the_project_with_the_id() {
    let runner = CapturingRunner::new();
    uncomplete_project(&runner, "p1").await.expect("script");
    let script = runner.last_script();
    let output = common::run_jsc(
        &format!("{FAKE_DATABASE}\nalpha.completed = true;"),
        &format!(
            "try {{ print(JSON.stringify((function () {{\n{script}\n}})())); }}\n\
             catch (error) {{ print(\"ERROR: \" + error.message); }}"
        ),
    );
    assert_eq!(result_id(&output), json!("p1"), "{output}");
}

#[tokio::test]
async fn delete_projects_batch_picks_the_project_with_the_id() {
    let runner = CapturingRunner::new();
    delete_projects_batch(&runner, vec!["p1".to_string()])
        .await
        .expect("script");
    let output = run_tool_script(&runner.last_script());
    let result: Value = serde_json::from_str(&output).expect("batch result");
    assert_eq!(result["results"][0]["id"], "p1", "{output}");
    assert_eq!(result["results"][0]["deleted"], true, "{output}");
}

// ---------------------------------------------------------------- folders run

#[tokio::test]
async fn folder_tools_pick_the_folder_with_the_id_not_one_named_like_it() {
    let runner = CapturingRunner::new();
    get_folder(&runner, "f1").await.expect("script");
    let read = run_tool_script(&runner.last_script());
    update_folder(&runner, "f1", Some("Renamed"), None)
        .await
        .expect("script");
    let update = run_tool_script(&runner.last_script());
    delete_folder(&runner, "f1").await.expect("script");
    let delete = run_tool_script(&runner.last_script());
    for (tool, output) in [
        ("get_folder", read),
        ("update_folder", update),
        ("delete_folder", delete),
    ] {
        assert_eq!(result_id(&output), json!("f1"), "{tool}: {output}");
    }
}

#[tokio::test]
async fn folder_and_project_tools_still_report_a_missing_object() {
    let runner = CapturingRunner::new();
    delete_folder(&runner, "Nope").await.expect("script");
    assert_eq!(
        run_tool_script(&runner.last_script()),
        "ERROR: Folder not found: Nope"
    );
    complete_project(&runner, "Nope").await.expect("script");
    assert_eq!(
        run_tool_script(&runner.last_script()),
        "ERROR: Project not found: Nope"
    );
}

// ---------------------------------------------------------------- tasks run

#[tokio::test]
async fn task_tools_refuse_a_project_root_task() {
    let runner = CapturingRunner::new();
    delete_task(&runner, "p1").await.expect("script");
    assert_eq!(
        run_tool_script(&runner.last_script()),
        "ERROR: Task not found: p1"
    );
    complete_task(&runner, "p1").await.expect("script");
    assert_eq!(
        run_tool_script(&runner.last_script()),
        "ERROR: Task not found: p1"
    );
}

#[tokio::test]
async fn task_tools_act_on_the_task_with_the_id() {
    let runner = CapturingRunner::new();
    delete_task(&runner, "t1").await.expect("script");
    let output = run_tool_script(&runner.last_script());
    assert_eq!(result_id(&output), json!("t1"), "{output}");
}

// ---------------------------------------------------------------- wiring

/// The script of every tool that looks up a project or folder by id or
/// name. `append_to_note` is absent on purpose: it takes an id only.
async fn project_and_folder_scripts() -> Vec<(&'static str, String)> {
    let runner = CapturingRunner::new();
    let mut scripts = Vec::new();
    get_project(&runner, "p1").await.expect("script");
    scripts.push(("get_project", runner.last_script()));
    complete_project(&runner, "p1").await.expect("script");
    scripts.push(("complete_project", runner.last_script()));
    uncomplete_project(&runner, "p1").await.expect("script");
    scripts.push(("uncomplete_project", runner.last_script()));
    delete_project(&runner, "p1").await.expect("script");
    scripts.push(("delete_project", runner.last_script()));
    delete_projects_batch(&runner, vec!["p1".to_string()])
        .await
        .expect("script");
    scripts.push(("delete_projects_batch", runner.last_script()));
    set_project_status(&runner, "p1", "active")
        .await
        .expect("script");
    scripts.push(("set_project_status", runner.last_script()));
    update_project(
        &runner,
        "p1",
        None,
        Some("note"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .expect("script");
    scripts.push(("update_project", runner.last_script()));
    move_project(&runner, "p1", None).await.expect("script");
    scripts.push(("move_project", runner.last_script()));
    get_folder(&runner, "f1").await.expect("script");
    scripts.push(("get_folder", runner.last_script()));
    update_folder(&runner, "f1", Some("Renamed"), None)
        .await
        .expect("script");
    scripts.push(("update_folder", runner.last_script()));
    delete_folder(&runner, "f1").await.expect("script");
    scripts.push(("delete_folder", runner.last_script()));
    scripts
}

#[tokio::test]
async fn no_project_or_folder_tool_keeps_its_own_id_or_name_lookup() {
    for (tool, script) in project_and_folder_scripts().await {
        assert!(
            !script.contains("item.name === projectFilter")
                && !script.contains("item.name === folderFilter")
                && !script.contains("item.name === idOrName"),
            "{tool} still matches names itself"
        );
        assert!(
            script.contains("resolveProject(")
                || script.contains("resolveFolder(")
                || script.contains("matchProjects("),
            "{tool} does not use the shared resolvers"
        );
        common::assert_script_compiles(tool, &script);
    }
}

/// The script of every tool that looks up a task by id.
async fn task_scripts() -> Vec<(&'static str, String)> {
    let runner = CapturingRunner::new();
    let mut scripts = Vec::new();
    get_task(&runner, "t1").await.expect("script");
    scripts.push(("get_task", runner.last_script()));
    list_subtasks(&runner, "t1", 10).await.expect("script");
    scripts.push(("list_subtasks", runner.last_script()));
    update_task(
        &runner,
        "t1",
        Some("Renamed"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .expect("script");
    scripts.push(("update_task", runner.last_script()));
    set_task_repetition(&runner, "t1", None, "regularly")
        .await
        .expect("script");
    scripts.push(("set_task_repetition", runner.last_script()));
    complete_task(&runner, "t1").await.expect("script");
    scripts.push(("complete_task", runner.last_script()));
    delete_task(&runner, "t1").await.expect("script");
    scripts.push(("delete_task", runner.last_script()));
    list_notifications(&runner, "t1").await.expect("script");
    scripts.push(("list_notifications", runner.last_script()));
    add_notification(&runner, "t1", None, Some(-60.0))
        .await
        .expect("script");
    scripts.push(("add_notification", runner.last_script()));
    remove_notification(&runner, "t1", "n1")
        .await
        .expect("script");
    scripts.push(("remove_notification", runner.last_script()));
    move_task(&runner, "t1", None, Some("t2"))
        .await
        .expect("script");
    scripts.push(("move_task", runner.last_script()));
    create_subtask(
        &runner, "Child", "t1", None, None, None, None, None, None, None,
    )
    .await
    .expect("script");
    scripts.push(("create_subtask", runner.last_script()));
    duplicate_task(&runner, "t1", false).await.expect("script");
    scripts.push(("duplicate_task", runner.last_script()));
    uncomplete_task(&runner, "t1").await.expect("script");
    scripts.push(("uncomplete_task", runner.last_script()));
    append_to_note(&runner, "task", "t1", "more")
        .await
        .expect("script");
    scripts.push(("append_to_note", runner.last_script()));
    move_tasks_batch(&runner, vec!["t1".to_string()], None, Some("t2"))
        .await
        .expect("script");
    scripts.push(("move_tasks_batch", runner.last_script()));
    scripts
}

#[tokio::test]
async fn no_task_tool_keeps_its_own_task_by_id_scan() {
    for (tool, script) in task_scripts().await {
        assert!(
            !script.contains("document.flattenedTasks.find("),
            "{tool} still scans for its task itself"
        );
        assert!(
            script.contains("resolveTask("),
            "{tool} does not use resolveTask"
        );
        common::assert_script_compiles(tool, &script);
    }
}

#[tokio::test]
async fn parent_task_lookups_keep_their_own_not_found_text() {
    let runner = CapturingRunner::new();
    move_task(&runner, "t1", None, Some("nope"))
        .await
        .expect("script");
    let moved = run_tool_script(&runner.last_script());
    assert_eq!(moved, "ERROR: Parent task not found: nope");
    create_subtask(
        &runner, "Child", "nope", None, None, None, None, None, None, None,
    )
    .await
    .expect("script");
    let created = run_tool_script(&runner.last_script());
    assert_eq!(created, "ERROR: Parent task not found: nope");
}
