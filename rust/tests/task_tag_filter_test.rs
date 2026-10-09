//! The `tag` / `tags` filters of `list_tasks`, `search_tasks` and
//! `get_task_counts` (audit CR-009).
//!
//! The filters compared each task tag's *name* with the raw values, so a
//! tag id matched nothing and a typo silently returned `[]` or a count of
//! 0, against the documented rule that a reference may be an id and a value
//! matching nothing is an error. Each value now resolves through
//! `resolveTag` (id or exact name; unknown or shared names fail) and tasks
//! are matched by tag id.
//!
//! These tests run the whole scripts in JavaScriptCore against a fake
//! database.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    jxa::JxaRunner,
    tools::tasks::{
        get_task_counts_with_added_changed, list_tasks_with_added_changed,
        search_tasks_with_added_changed,
    },
};
use serde_json::{json, Value};

/// Tags Home (tag-h), Errands (tag-e) and two named "Urgent" (t1, t2).
/// Tasks: a [Home], b [Home, Errands], c [Errands], d [no tag], every one
/// named "Buy …" so a search for "buy" finds them all.
const FAKE_DATABASE: &str = r#"function FakeEnum(name) { this.name = name; }
var Task = { Status: { Available: new FakeEnum("Available"), Blocked: new FakeEnum("Blocked"),
  Completed: new FakeEnum("Completed"), Dropped: new FakeEnum("Dropped"),
  DueSoon: new FakeEnum("DueSoon"), Next: new FakeEnum("Next"),
  Overdue: new FakeEnum("Overdue") }, byIdentifier: function () { return null; } };
var Project = { Status: { Active: new FakeEnum("Active") },
  byIdentifier: function () { return null; } };
var Tag = { Status: { Active: new FakeEnum("Active"), OnHold: new FakeEnum("OnHold") },
  byIdentifier: function (id) { return tags.find(t => t.id.primaryKey === id) || null; } };
function fakeTag(id, name) { return { id: { primaryKey: id }, name: name, status: Tag.Status.Active }; }
var home = fakeTag("tag-h", "Home");
var errands = fakeTag("tag-e", "Errands");
var tags = [home, errands, fakeTag("t1", "Urgent"), fakeTag("t2", "Urgent")];
function fakeTask(id, taskTags) {
  return { id: { primaryKey: id }, name: "Buy " + id, note: "", flagged: false, completed: false,
    taskStatus: Task.Status.Available, containingProject: null, project: null, tags: taskTags,
    dueDate: null, deferDate: null, effectiveDeferDate: null, completionDate: null,
    effectiveCompletionDate: null, effectiveDropDate: null, added: null, modified: null,
    plannedDate: null, estimatedMinutes: null, hasChildren: false, inInbox: true,
    sequential: false };
}
var document = { flattenedTags: tags, flattenedProjects: [], flattenedFolders: [],
  flattenedTasks: [fakeTask("a", [home]), fakeTask("b", [home, errands]),
    fakeTask("c", [errands]), fakeTask("d", [])] };"#;

/// Records the script it is asked to run and returns `payload`.
#[derive(Clone)]
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

/// Runs a script against the fake database: its JSON result, or
/// "ERROR: <message>" when it throws.
fn run(script: &str) -> String {
    common::assert_script_compiles("tag filter", script);
    common::run_jsc(
        FAKE_DATABASE,
        &format!(
            "try {{ print(JSON.stringify((function () {{\n{script}\n}})())); }}\n\
             catch (error) {{ print(\"ERROR: \" + error.message); }}"
        ),
    )
}

/// The ids of the tasks a list result holds, or the error it printed.
fn listed_ids(output: &str) -> Result<Vec<String>, String> {
    let Ok(Value::Array(tasks)) = serde_json::from_str::<Value>(output) else {
        return Err(output.to_string());
    };
    Ok(tasks
        .iter()
        .map(|task| task["id"].as_str().expect("an id").to_string())
        .collect())
}

async fn list_tasks_output(tag: Option<&str>, tags: Option<Vec<&str>>, mode: &str) -> String {
    let runner = CapturingRunner::new(json!([]));
    list_tasks_with_added_changed(
        &runner,
        None,
        tag,
        tags.map(|values| values.into_iter().map(str::to_string).collect()),
        mode,
        None,
        "all",
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
        Some("name"),
        "asc",
        100,
    )
    .await
    .expect("list_tasks builds a script");
    run(&runner.last_script())
}

async fn task_counts_output(tag: Option<&str>) -> String {
    let runner = CapturingRunner::new(json!({
        "total": 0, "available": 0, "completed": 0, "overdue": 0, "dueSoon": 0,
        "flagged": 0, "deferred": 0
    }));
    get_task_counts_with_added_changed(
        &runner, None, tag, None, "any", None, None, None, None, None, None, None, None, None,
        None, None, None, None, None,
    )
    .await
    .expect("get_task_counts builds a script");
    run(&runner.last_script())
}

// ---------------------------------------------------------------- list_tasks

#[tokio::test]
async fn tag_filter_accepts_a_tag_id() {
    let output = list_tasks_output(Some("tag-e"), None, "any").await;
    assert_eq!(listed_ids(&output), Ok(vec!["b".into(), "c".into()]));
}

#[tokio::test]
async fn tag_filter_still_accepts_an_exact_name() {
    let output = list_tasks_output(Some("Home"), None, "any").await;
    assert_eq!(listed_ids(&output), Ok(vec!["a".into(), "b".into()]));
}

#[tokio::test]
async fn tags_filter_mixes_ids_and_names_in_all_mode() {
    let output = list_tasks_output(None, Some(vec!["tag-h", "Errands"]), "all").await;
    assert_eq!(listed_ids(&output), Ok(vec!["b".into()]));
}

#[tokio::test]
async fn an_unknown_tag_is_an_error_not_an_empty_list() {
    let output = list_tasks_output(None, Some(vec!["Home", "Nope"]), "any").await;
    assert_eq!(output, "ERROR: Tag not found: Nope");
}

#[tokio::test]
async fn a_tag_name_two_tags_share_is_refused() {
    let output = list_tasks_output(Some("Urgent"), None, "any").await;
    assert_eq!(
        output,
        r#"ERROR: Ambiguous tag name "Urgent": 2 matches (t1, t2); pass an id."#
    );
}

// ---------------------------------------------------------------- search_tasks

#[tokio::test]
async fn search_tasks_tag_filter_accepts_a_tag_id() {
    let runner = CapturingRunner::new(json!([]));
    search_tasks_with_added_changed(
        &runner,
        "buy",
        None,
        Some("tag-h"),
        None,
        "any",
        None,
        "all",
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
        Some("name"),
        "asc",
        100,
    )
    .await
    .expect("search_tasks builds a script");
    let output = run(&runner.last_script());
    assert_eq!(listed_ids(&output), Ok(vec!["a".into(), "b".into()]));
}

// ---------------------------------------------------------------- get_task_counts

#[tokio::test]
async fn task_counts_tag_filter_accepts_a_tag_id() {
    let counts: Value =
        serde_json::from_str(&task_counts_output(Some("tag-e")).await).expect("counts result");
    assert_eq!(counts["total"], 2);
}

#[tokio::test]
async fn task_counts_with_an_unknown_tag_is_an_error_not_zero() {
    assert_eq!(
        task_counts_output(Some("Nope")).await,
        "ERROR: Tag not found: Nope"
    );
}
