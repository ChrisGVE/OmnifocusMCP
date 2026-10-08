//! Tag resolution and write validation.
//!
//! Tag-valued write parameters resolve through the shared `JS_RESOLVERS`
//! snippet. Resolution tries an OmniFocus identifier before an exact name and
//! rejects a missing tag. Batch creation prepares every entry before calling
//! `new Task`, so a bad later entry cannot leave earlier tasks behind.

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
        projects::update_project,
        tags::{create_tag, delete_tag, update_tag},
        tasks::{create_subtask, create_task, create_tasks_batch, update_task, CreateTaskInput},
    },
};
use serde_json::{json, Value};

const FAKE_RESOLVER_DATABASE: &str = r#"function byName(name) {
  return this.find(item => item.name === name) || null;
}
var tags = [
  { id: { primaryKey: "tag-1" }, name: "Home" },
  { id: { primaryKey: "tag-2" }, name: "tag-1" }
];
tags.byName = byName;
var Tag = {
  byIdentifier: function (id) {
    return tags.find(tag => tag.id.primaryKey === id) || null;
  }
};
var Folder = { byIdentifier: function () { return null; } };
var Project = { byIdentifier: function () { return null; } };
var document = {
  flattenedTags: tags,
  flattenedFolders: Object.assign([], { byName: byName }),
  flattenedProjects: Object.assign([], { byName: byName })
};"#;

fn resolve_tag(expression: &str) -> String {
    common::run_jsc(
        &format!("{FAKE_RESOLVER_DATABASE}\n{JS_RESOLVERS}"),
        &format!(
            "try {{ print({expression}.id.primaryKey); }} \
             catch (error) {{ print(\"ERROR: \" + error.message); }}"
        ),
    )
}

#[test]
fn tag_resolves_by_identifier_before_exact_name() {
    assert_eq!(resolve_tag(r#"resolveTag("tag-1")"#), "tag-1");
}

#[test]
fn tag_resolves_by_exact_name() {
    assert_eq!(resolve_tag(r#"resolveTag("Home")"#), "tag-1");
}

#[test]
fn missing_tag_throws_tag_not_found() {
    assert_eq!(
        resolve_tag(r#"resolveTag("Missing")"#),
        "ERROR: Tag not found: Missing"
    );
}

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
        Box::pin(async move { Ok(json!({})) })
    }
}

fn assert_before(script: &str, earlier: &str, later: &str) {
    let earlier_at = script
        .find(earlier)
        .unwrap_or_else(|| panic!("missing {earlier:?}"));
    let later_at = script
        .find(later)
        .unwrap_or_else(|| panic!("missing {later:?}"));
    assert!(earlier_at < later_at, "{earlier:?} must precede {later:?}");
}

fn assert_uses_shared_resolver(tool: &str, script: &str) {
    assert_eq!(
        script.matches(JS_RESOLVERS).count(),
        1,
        "{tool} must prepend JS_RESOLVERS exactly once"
    );
    common::assert_script_compiles(tool, script);
}

#[tokio::test]
async fn task_writes_resolve_all_tags_before_mutating() {
    let runner = CapturingRunner::new();

    create_task(
        &runner,
        "Task",
        None,
        None,
        None,
        None,
        None,
        Some(vec!["Home".to_string()]),
        None,
    )
    .await
    .expect("create_task runs");
    let script = runner.last_script();
    assert_uses_shared_resolver("create_task", &script);
    assert_before(&script, "const resolvedTags", "new Task(");

    create_subtask(
        &runner,
        "Child",
        "parent-1",
        None,
        None,
        None,
        None,
        Some(vec!["Home".to_string()]),
        None,
    )
    .await
    .expect("create_subtask runs");
    let script = runner.last_script();
    assert_uses_shared_resolver("create_subtask", &script);
    assert_before(&script, "const resolvedTags", "new Task(");

    update_task(
        &runner,
        "task-1",
        Some("Renamed"),
        None,
        None,
        None,
        None,
        Some(vec!["Home".to_string()]),
        None,
    )
    .await
    .expect("update_task runs");
    let script = runner.last_script();
    assert_uses_shared_resolver("update_task", &script);
    assert_before(&script, "const resolvedTags", "task.name = updates.name");
    assert_before(&script, "const resolvedTags", "task.removeTag(tag)");
}

#[tokio::test]
async fn project_and_tag_writes_use_the_shared_tag_resolver() {
    let runner = CapturingRunner::new();

    update_project(
        &runner,
        "project-1",
        Some("Renamed"),
        None,
        None,
        None,
        None,
        Some(vec!["Home".to_string()]),
        None,
        None,
        None,
    )
    .await
    .expect("update_project runs");
    let script = runner.last_script();
    assert_uses_shared_resolver("update_project", &script);
    assert_before(&script, "const resolvedTags", "project.name = updates.name");
    assert_before(&script, "const resolvedTags", "project.removeTag(tag)");

    create_tag(&runner, "Child", Some("Home"))
        .await
        .expect("create_tag runs");
    let script = runner.last_script();
    assert_uses_shared_resolver("create_tag", &script);
    assert!(script.contains("const parentTag = resolveTag(parentName);"));

    update_tag(&runner, "Home", Some("House"), None)
        .await
        .expect("update_tag runs");
    let script = runner.last_script();
    assert_uses_shared_resolver("update_tag", &script);
    assert!(script.contains("const tag = resolveTag(tagFilter);"));

    delete_tag(&runner, "Home").await.expect("delete_tag runs");
    let script = runner.last_script();
    assert_uses_shared_resolver("delete_tag", &script);
    assert!(script.contains("const tag = resolveTag(tagFilter);"));
}

fn batch_input(name: &str, project: Option<&str>, tags: Option<Vec<&str>>) -> CreateTaskInput {
    CreateTaskInput {
        name: name.to_string(),
        project: project.map(str::to_string),
        note: None,
        due_date: None,
        defer_date: None,
        flagged: None,
        tags: tags.map(|values| values.into_iter().map(str::to_string).collect()),
        estimated_minutes: None,
    }
}

async fn batch_script(tasks: Vec<CreateTaskInput>) -> String {
    let runner = CapturingRunner::new();
    create_tasks_batch(&runner, tasks)
        .await
        .expect("create_tasks_batch builds a script");
    runner.last_script()
}

const BATCH_DATABASE: &str = r#"function byName(name) {
  return this.find(item => item.name === name) || null;
}
var createdTaskNames = [];
var projects = [{ id: { primaryKey: "project-1" }, name: "Work", ending: {} }];
projects.byName = byName;
var tags = [{ id: { primaryKey: "tag-1" }, name: "Home" }];
tags.byName = byName;
var Project = {
  byIdentifier: function (id) {
    return projects.find(project => project.id.primaryKey === id) || null;
  }
};
var Tag = {
  byIdentifier: function (id) {
    return tags.find(tag => tag.id.primaryKey === id) || null;
  }
};
var Folder = { byIdentifier: function () { return null; } };
var document = {
  flattenedProjects: projects,
  flattenedTags: tags,
  flattenedFolders: Object.assign([], { byName: byName })
};
var inbox = { ending: {} };
function Task(name) {
  createdTaskNames.push(name);
  this.id = { primaryKey: "created-" + createdTaskNames.length };
  this.name = name;
  this.tags = [];
  this.addTag = function (tag) { this.tags.push(tag); };
}"#;

fn run_batch(script: &str) -> String {
    common::assert_script_compiles("create_tasks_batch", script);
    common::run_jsc(
        BATCH_DATABASE,
        &format!(
            "try {{ (function () {{\n{script}\n}})(); \
             print(\"CREATED: \" + createdTaskNames.length); }} \
             catch (error) {{ print(\"ERROR: \" + error.message + \
             \"; CREATED: \" + createdTaskNames.length); }}"
        ),
    )
}

#[tokio::test]
async fn batch_unknown_project_names_entry_and_creates_nothing() {
    let script = batch_script(vec![
        batch_input("First", Some("Work"), None),
        batch_input("Second", Some("Missing"), None),
    ])
    .await;
    assert_eq!(
        run_batch(&script),
        "ERROR: tasks[1].project: Project not found: Missing; CREATED: 0"
    );
}

#[tokio::test]
async fn batch_unknown_tag_names_entry_and_creates_nothing() {
    let script = batch_script(vec![
        batch_input("First", None, Some(vec!["Home"])),
        batch_input("Second", None, Some(vec!["Missing"])),
    ])
    .await;
    assert_eq!(
        run_batch(&script),
        "ERROR: tasks[1].tags[0]: Tag not found: Missing; CREATED: 0"
    );
}

#[tokio::test]
async fn batch_invalid_later_date_creates_nothing() {
    let mut second = batch_input("Second", None, None);
    second.due_date = Some("not-a-date".to_string());
    let script = batch_script(vec![batch_input("First", None, None), second]).await;
    let output = run_batch(&script);
    assert!(
        output.contains("tasks[1].dueDate must be YYYY-MM-DD or an ISO 8601 date-time"),
        "unexpected error: {output}"
    );
    assert!(
        output.ends_with("; CREATED: 0"),
        "unexpected output: {output}"
    );
}
