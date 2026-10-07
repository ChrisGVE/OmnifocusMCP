//! Folder and project parameters (upstream #11).
//!
//! Every `folder` and `project` parameter used to be matched by exact name
//! only, a filter given a value that matched nothing silently returned
//! nothing, the scripts read `project.folder` (not part of the OmniJS API;
//! the documented property is `parentFolder`), and move_project echoed the
//! requested folder instead of reporting where the project ended up.
//!
//! The shared `JS_RESOLVERS` snippet resolves a value as an id first and an
//! exact name second, and throws "Folder not found: ..." / "Project not
//! found: ..." when neither matches. These tests run the resolvers and whole
//! tool scripts in JavaScriptCore against a small fake OmniFocus database.
//! The wiring of every tool that takes such a parameter is pinned in
//! `folder_project_wiring_test.rs`, the schema in `folder_project_schema_test.rs`.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    js_helpers::JS_RESOLVERS,
    jxa::JxaRunner,
    tools::{
        projects::{get_project_counts, move_project},
        tasks::get_task_counts_with_added_changed,
    },
};
use serde_json::{json, Value};

// ---------------------------------------------------------------- fake database

/// A small OmniFocus database: folders Work (fW) and Home (fH); projects
/// Alpha (active) and Beta (done) in Work, Gamma (active) in Home, Delta (on
/// hold) at the top level, and two projects both named "Errands" (p5, p6)
/// holding one and two tasks. A third folder (fX) is *named* "fW", so a value
/// that is one folder's id and another's name shows which one wins. Projects
/// carry only the documented `parentFolder`, never `folder`.
const FAKE_DATABASE: &str = r#"function FakeStatus(name) { this.name = name; }
function byName(name) { return this.find(item => item.name === name) || null; }
function fakeFolder(id, name) {
  const folder = { id: { primaryKey: id }, name: name };
  folder.ending = { folder: folder };
  return folder;
}
function fakeProject(id, name, status, parentFolder) {
  return { id: { primaryKey: id }, name: name, status: Project.Status[status],
    parentFolder: parentFolder, flattenedTasks: [], nextTask: null, tasks: [], ending: {} };
}
function fakeTask(id, project) {
  return { id: { primaryKey: id }, name: id, containingProject: project, tags: [],
    flagged: false, completed: false, dueDate: null, deferDate: null, completionDate: null,
    added: null, modified: null, estimatedMinutes: null };
}
var Project = {
  Status: { Active: new FakeStatus("Active"), Done: new FakeStatus("Done"),
    Dropped: new FakeStatus("Dropped"), OnHold: new FakeStatus("OnHold") },
  byIdentifier: function (id) { return projects.find(p => p.id.primaryKey === id) || null; }
};
var Folder = {
  byIdentifier: function (id) { return folders.find(f => f.id.primaryKey === id) || null; }
};
var work = fakeFolder("fW", "Work");
var home = fakeFolder("fH", "Home");
var namedLikeAnId = fakeFolder("fX", "fW");
var folders = [work, home, namedLikeAnId];
folders.byName = byName;
var errandsOne = fakeProject("p5", "Errands", "Active", home);
var errandsTwo = fakeProject("p6", "Errands", "Active", home);
var projects = [
  fakeProject("p1", "Alpha", "Active", work),
  fakeProject("p2", "Beta", "Done", work),
  fakeProject("p3", "Gamma", "Active", home),
  fakeProject("p4", "Delta", "OnHold", null),
  errandsOne,
  errandsTwo
];
projects.byName = byName;
var tasks = [fakeTask("t1", errandsOne), fakeTask("t2", errandsTwo), fakeTask("t3", errandsTwo)];
var document = { flattenedFolders: folders, flattenedProjects: projects, flattenedTasks: tasks };
var library = { ending: { folder: null } };
function moveSections(sections, destination) {
  sections.forEach(section => { section.parentFolder = destination.folder; });
}"#;

/// Runs a complete tool script (which ends in a top-level `return`, as
/// `crate::jxa` wraps it) against the fake database. Returns the JSON result,
/// or "ERROR: <message>" when the script throws.
///
/// The script is compile-checked first, so a broken script fails as such
/// rather than as a puzzling behavioural mismatch.
fn run_against_fake_database(script: &str) -> String {
    common::assert_script_compiles("tool", script);
    common::run_jsc(
        FAKE_DATABASE,
        &format!(
            "try {{ const result = (function () {{\n{script}\n}})(); print(JSON.stringify(result)); }}\n\
             catch (error) {{ print(\"ERROR: \" + error.message); }}"
        ),
    )
}

fn resolve(expression: &str) -> String {
    common::run_jsc(
        &format!("{FAKE_DATABASE}\n{JS_RESOLVERS}"),
        &format!(
            "try {{ print({expression}.id.primaryKey); }} \
             catch (error) {{ print(\"ERROR: \" + error.message); }}"
        ),
    )
}

// ---------------------------------------------------------------- resolvers

#[test]
fn folder_resolves_by_id() {
    assert_eq!(resolve(r#"resolveFolder("fH")"#), "fH");
}

#[test]
fn folder_resolves_by_exact_name() {
    assert_eq!(resolve(r#"resolveFolder("Home")"#), "fH");
}

#[test]
fn folder_id_wins_over_a_folder_named_like_it() {
    assert_eq!(resolve(r#"resolveFolder("fW")"#), "fW");
}

#[test]
fn missing_folder_throws_folder_not_found() {
    assert_eq!(
        resolve(r#"resolveFolder("Nope")"#),
        "ERROR: Folder not found: Nope"
    );
}

#[test]
fn project_resolves_by_id() {
    assert_eq!(resolve(r#"resolveProject("p6")"#), "p6");
}

#[test]
fn project_resolves_by_exact_name() {
    assert_eq!(resolve(r#"resolveProject("Gamma")"#), "p3");
}

#[test]
fn missing_project_throws_project_not_found() {
    assert_eq!(
        resolve(r#"resolveProject("Nope")"#),
        "ERROR: Project not found: Nope"
    );
}

// ---------------------------------------------------------------- capture

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

async fn get_project_counts_script(folder: Option<&str>) -> String {
    let runner = CapturingRunner::new(json!({
        "total": 0, "active": 0, "onHold": 0, "completed": 0, "dropped": 0, "stalled": 0
    }));
    get_project_counts(&runner, folder)
        .await
        .expect("get_project_counts runs");
    runner.last_script()
}

async fn get_task_counts_script(project: Option<&str>) -> String {
    let runner = CapturingRunner::new(json!({
        "total": 0, "available": 0, "completed": 0, "overdue": 0, "dueSoon": 0,
        "flagged": 0, "deferred": 0
    }));
    get_task_counts_with_added_changed(
        &runner, project, None, None, "any", None, None, None, None, None, None, None, None, None,
        None, None, None, None, None,
    )
    .await
    .expect("get_task_counts runs");
    runner.last_script()
}

async fn move_project_script(folder: Option<&str>) -> String {
    let runner = CapturingRunner::new(json!({"id": "p3"}));
    move_project(&runner, "p3", folder)
        .await
        .expect("move_project runs");
    runner.last_script()
}

// ---------------------------------------------------------------- filters, run

#[tokio::test]
async fn folder_filter_matches_by_id_or_name_through_parent_folder() {
    let expected =
        json!({"total": 2, "active": 1, "onHold": 0, "completed": 1, "dropped": 0, "stalled": 0});
    for folder in ["fW", "Work"] {
        let output = run_against_fake_database(&get_project_counts_script(Some(folder)).await);
        let counts: Value = serde_json::from_str(&output)
            .unwrap_or_else(|_| panic!("folder {folder:?}: expected counts, got {output}"));
        assert_eq!(counts, expected, "folder {folder:?}");
    }
}

#[tokio::test]
async fn folder_filter_with_unknown_folder_errors_instead_of_counting_nothing() {
    let output = run_against_fake_database(&get_project_counts_script(Some("Nope")).await);
    assert_eq!(output, "ERROR: Folder not found: Nope");
}

#[tokio::test]
async fn project_filter_by_id_tells_same_named_projects_apart() {
    let by_id = run_against_fake_database(&get_task_counts_script(Some("p6")).await);
    let by_id: Value = serde_json::from_str(&by_id).expect("counts by id");
    assert_eq!(by_id["total"], 2);
    // An exact name selects the first project with that name.
    let by_name = run_against_fake_database(&get_task_counts_script(Some("Errands")).await);
    let by_name: Value = serde_json::from_str(&by_name).expect("counts by name");
    assert_eq!(by_name["total"], 1);
}

#[tokio::test]
async fn project_filter_with_unknown_project_errors_instead_of_counting_nothing() {
    let output = run_against_fake_database(&get_task_counts_script(Some("Nope")).await);
    assert_eq!(output, "ERROR: Project not found: Nope");
}

#[tokio::test]
async fn move_project_reports_where_the_project_ended_up() {
    // Requested by id: an echo of the request would say "fW".
    let output = run_against_fake_database(&move_project_script(Some("fW")).await);
    let moved: Value = serde_json::from_str(&output).expect("move result");
    assert_eq!(moved["folderName"], "Work");
}

#[tokio::test]
async fn move_project_without_folder_moves_to_top_level() {
    let output = run_against_fake_database(&move_project_script(None).await);
    let moved: Value = serde_json::from_str(&output).expect("move result");
    assert_eq!(moved["folderName"], Value::Null);
}

#[tokio::test]
async fn move_project_to_unknown_folder_errors() {
    let output = run_against_fake_database(&move_project_script(Some("Nope")).await);
    assert_eq!(output, "ERROR: Folder not found: Nope");
}
