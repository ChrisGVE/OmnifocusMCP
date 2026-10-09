//! A name that several objects share is refused (audit CR-003).
//!
//! Duplicate names are normal in OmniFocus (two "Errands" projects in two
//! folders), and every id-or-name parameter used to act on the first match
//! in database order: `delete_project("Errands")` deleted whichever came
//! first. The shared resolvers now refuse such a name everywhere, reads
//! included, with `Ambiguous <kind> name "<x>": <N> matches (<ids>); pass an
//! id.` An id always names exactly one object. The batch deletes report an
//! ambiguous entry in its result and delete nothing for it.
//!
//! These tests run whole tool scripts in JavaScriptCore against a fake
//! database.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    jxa::JxaRunner,
    tools::{
        folders::{delete_folder, delete_folders_batch, get_folder, update_folder},
        projects::{
            complete_project, delete_project, delete_projects_batch, get_project_counts,
            move_project, set_project_status, uncomplete_project,
        },
        tags::{delete_tag, delete_tags_batch, update_tag},
        tasks::get_task_counts_with_added_changed,
    },
};
use serde_json::{json, Value};

const PROJECTS_ERRANDS: &str =
    r#"Ambiguous project name "Errands": 2 matches (p5, p6); pass an id."#;
const FOLDERS_HOME: &str = r#"Ambiguous folder name "Home": 2 matches (f1, f2); pass an id."#;
const TAGS_URGENT: &str = r#"Ambiguous tag name "Urgent": 2 matches (t1, t2); pass an id."#;

/// Projects Alpha (p1) and two named "Errands" (p5, p6); folders Work (fW)
/// and two named "Home" (f1, f2); tags Calls (tc) and two named "Urgent"
/// (t1, t2). `deleteObject` records what it deleted.
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
var Tag = {
  Status: { Active: new FakeEnum("Active"), OnHold: new FakeEnum("OnHold"),
    Dropped: new FakeEnum("Dropped") },
  byIdentifier: function (id) { return tags.find(t => t.id.primaryKey === id) || null; }
};
var Task = { Status: { Available: new FakeEnum("Available") },
  byIdentifier: function () { return null; } };
function fakeFolder(id, name) {
  return { id: { primaryKey: id }, name: name, status: Folder.Status.Active, parent: null,
    projects: [], folders: [], ending: {} };
}
function fakeProject(id, name) {
  return { id: { primaryKey: id }, name: name, status: Project.Status.Active, completed: true,
    parentFolder: null, flattenedTasks: [], tasks: [], nextTask: null, ending: {},
    containsSingletonActions: false,
    markComplete: function () { this.completed = true; },
    markIncomplete: function () { this.completed = false; } };
}
function fakeTag(id, name) {
  return { id: { primaryKey: id }, name: name, status: Tag.Status.Active, parent: null, tasks: [] };
}
var folders = [fakeFolder("fW", "Work"), fakeFolder("f1", "Home"), fakeFolder("f2", "Home")];
var projects = [fakeProject("p1", "Alpha"), fakeProject("p5", "Errands"),
  fakeProject("p6", "Errands")];
var tags = [fakeTag("tc", "Calls"), fakeTag("t1", "Urgent"), fakeTag("t2", "Urgent")];
var DELETED = [];
function deleteObject(object) { DELETED.push(object.id.primaryKey); }
var library = { ending: {} };
function moveSections() {}
var document = { flattenedFolders: folders, flattenedProjects: projects, flattenedTags: tags,
  flattenedTasks: [] };"#;

/// Runs a complete tool script against the fake database. Returns the JSON
/// result with what was deleted added as `deletedIds`, or
/// "ERROR: <message>; DELETED: <ids>" when the script throws.
fn run_tool_script(script: &str) -> String {
    common::assert_script_compiles("tool", script);
    common::run_jsc(
        FAKE_DATABASE,
        &format!(
            "try {{ const result = (function () {{\n{script}\n}})(); \
             if (result && typeof result === \"object\") result.deletedIds = DELETED; \
             print(JSON.stringify(result)); }}\n\
             catch (error) {{ print(\"ERROR: \" + error.message + \"; DELETED: \" + DELETED.join(\",\")); }}"
        ),
    )
}

/// What `run_tool_script` prints for a call refused before deleting
/// anything (its output is trimmed, so the empty list leaves no space).
fn refused(message: &str) -> String {
    format!("ERROR: {message}; DELETED:")
}

/// Records every script it is asked to run and returns an empty object.
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

    /// Runs the script of the call just made against the fake database.
    fn run_last(&self) -> String {
        let script = self
            .scripts
            .lock()
            .expect("script lock")
            .last()
            .cloned()
            .expect("a script was run");
        run_tool_script(&script)
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
        Box::pin(async {
            Ok(json!({
                "total": 0, "active": 0, "onHold": 0, "completed": 0, "dropped": 0,
                "stalled": 0, "available": 0, "overdue": 0, "dueSoon": 0, "flagged": 0,
                "deferred": 0
            }))
        })
    }
}

fn parse(output: &str) -> Value {
    serde_json::from_str(output).unwrap_or_else(|_| panic!("expected a JSON result, got {output}"))
}

// ---------------------------------------------------------------- single writes

#[tokio::test]
async fn delete_project_refuses_a_shared_name_and_deletes_nothing() {
    let runner = CapturingRunner::new();
    delete_project(&runner, "Errands").await.expect("script");
    assert_eq!(runner.run_last(), refused(PROJECTS_ERRANDS));
}

#[tokio::test]
async fn project_state_changes_refuse_a_shared_name() {
    let runner = CapturingRunner::new();
    complete_project(&runner, "Errands").await.expect("script");
    assert_eq!(runner.run_last(), refused(PROJECTS_ERRANDS));
    uncomplete_project(&runner, "Errands")
        .await
        .expect("script");
    assert_eq!(runner.run_last(), refused(PROJECTS_ERRANDS));
    set_project_status(&runner, "Errands", "dropped")
        .await
        .expect("script");
    assert_eq!(runner.run_last(), refused(PROJECTS_ERRANDS));
    move_project(&runner, "Errands", None)
        .await
        .expect("script");
    assert_eq!(runner.run_last(), refused(PROJECTS_ERRANDS));
}

#[tokio::test]
async fn delete_project_by_id_works_when_names_collide() {
    let runner = CapturingRunner::new();
    delete_project(&runner, "p6").await.expect("script");
    let result = parse(&runner.run_last());
    assert_eq!(result["id"], "p6");
    assert_eq!(result["deletedIds"], json!(["p6"]));
}

#[tokio::test]
async fn folder_writes_refuse_a_shared_name_and_accept_an_id() {
    let runner = CapturingRunner::new();
    delete_folder(&runner, "Home").await.expect("script");
    assert_eq!(runner.run_last(), refused(FOLDERS_HOME));
    update_folder(&runner, "Home", Some("House"), None)
        .await
        .expect("script");
    assert_eq!(runner.run_last(), refused(FOLDERS_HOME));
    delete_folder(&runner, "f2").await.expect("script");
    assert_eq!(parse(&runner.run_last())["deletedIds"], json!(["f2"]));
}

#[tokio::test]
async fn tag_writes_refuse_a_shared_name_and_accept_an_id() {
    let runner = CapturingRunner::new();
    delete_tag(&runner, "Urgent").await.expect("script");
    assert_eq!(runner.run_last(), refused(TAGS_URGENT));
    update_tag(&runner, "Urgent", Some("Soon"), None)
        .await
        .expect("script");
    assert_eq!(runner.run_last(), refused(TAGS_URGENT));
    delete_tag(&runner, "t1").await.expect("script");
    assert_eq!(parse(&runner.run_last())["deletedIds"], json!(["t1"]));
}

// ---------------------------------------------------------------- batch writes

/// The per-entry result a batch delete reports for an ambiguous name.
fn ambiguous_entry(id_or_name: &str, message: &str) -> Value {
    json!({"id_or_name": id_or_name, "id": null, "name": null, "deleted": false, "error": message})
}

#[tokio::test]
async fn delete_projects_batch_reports_a_shared_name_and_deletes_the_rest() {
    let runner = CapturingRunner::new();
    delete_projects_batch(&runner, vec!["Errands".to_string(), "Alpha".to_string()])
        .await
        .expect("script");
    let result = parse(&runner.run_last());
    assert_eq!(
        result["results"][0],
        ambiguous_entry("Errands", PROJECTS_ERRANDS)
    );
    assert_eq!(result["results"][1]["deleted"], true);
    assert_eq!(result["deletedIds"], json!(["p1"]));
    assert_eq!(result["partial_success"], true);
}

#[tokio::test]
async fn delete_folders_batch_reports_a_shared_name_and_deletes_the_rest() {
    let runner = CapturingRunner::new();
    delete_folders_batch(&runner, vec!["Home".to_string(), "f1".to_string()])
        .await
        .expect("script");
    let result = parse(&runner.run_last());
    assert_eq!(result["results"][0], ambiguous_entry("Home", FOLDERS_HOME));
    assert_eq!(result["results"][1]["id"], "f1");
    assert_eq!(result["deletedIds"], json!(["f1"]));
}

#[tokio::test]
async fn delete_tags_batch_reports_a_shared_name_and_deletes_the_rest() {
    let runner = CapturingRunner::new();
    delete_tags_batch(&runner, vec!["Urgent".to_string(), "Calls".to_string()])
        .await
        .expect("script");
    let result = parse(&runner.run_last());
    assert_eq!(result["results"][0], ambiguous_entry("Urgent", TAGS_URGENT));
    assert_eq!(result["deletedIds"], json!(["tc"]));
    assert_eq!(
        result["summary"],
        json!({"requested": 2, "deleted": 1, "failed": 1})
    );
}

// ---------------------------------------------------------------- reads

#[tokio::test]
async fn reads_refuse_a_shared_name_too() {
    let runner = CapturingRunner::new();
    get_folder(&runner, "Home").await.expect("script");
    assert_eq!(runner.run_last(), refused(FOLDERS_HOME));
    get_project_counts(&runner, Some("Home"))
        .await
        .expect("script");
    assert_eq!(runner.run_last(), refused(FOLDERS_HOME));
    get_task_counts_with_added_changed(
        &runner,
        Some("Errands"),
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
        None,
        None,
        None,
    )
    .await
    .expect("script");
    assert_eq!(runner.run_last(), refused(PROJECTS_ERRANDS));
}
