//! Planned-date requests on databases without planned-date support.
//!
//! Reading `plannedDate` throws on databases that have not been migrated for
//! the feature. Filters and sorts must fail explicitly there; an absent
//! planned-date request retains the compatibility behaviour of returning
//! `null` planned dates.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    jxa::JxaRunner,
    tools::tasks::{list_tasks_with_added_changed, search_tasks_with_added_changed},
};
use serde_json::{json, Value};

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
        Box::pin(async move { Ok(json!([])) })
    }
}

async fn list_script(
    planned_before: Option<&str>,
    planned_after: Option<&str>,
    sort_by: Option<&str>,
) -> String {
    let runner = CapturingRunner::new();
    list_tasks_with_added_changed(
        &runner,
        None,
        None,
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
        planned_before,
        planned_after,
        None,
        sort_by,
        "asc",
        10,
    )
    .await
    .expect("list_tasks builds a script");
    runner.script()
}

async fn search_script(
    planned_before: Option<&str>,
    planned_after: Option<&str>,
    sort_by: Option<&str>,
) -> String {
    let runner = CapturingRunner::new();
    search_tasks_with_added_changed(
        &runner,
        "task",
        None,
        None,
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
        planned_before,
        planned_after,
        None,
        sort_by,
        "asc",
        10,
    )
    .await
    .expect("search_tasks builds a script");
    runner.script()
}

const DATABASE_WITHOUT_PLANNED_DATES: &str = r#"var Task = { Status: {} };
["Available", "Blocked", "Completed", "Dropped", "DueSoon", "Next", "Overdue"]
  .forEach(function (name) { Task.Status[name] = { name: name }; });
var Project = { Status: {
  Active: { name: "Active" }, Done: { name: "Done" },
  Dropped: { name: "Dropped" }, OnHold: { name: "OnHold" }
} };
var Folder = { byIdentifier: function () { return null; } };
var Tag = { byIdentifier: function () { return null; }, Status: { OnHold: {} } };
var task = {
  id: { primaryKey: "task-1" }, name: "task", note: "", containingProject: null,
  tags: [], flagged: false, completed: false, dueDate: null, deferDate: null,
  completionDate: null, added: null, modified: null, estimatedMinutes: null,
  inInbox: true, hasChildren: false, sequential: false,
  taskStatus: Task.Status.Available
};
Object.defineProperty(task, "plannedDate", {
  get: function () { throw new Error("plannedDate is not supported"); }
});
var emptyByName = [];
emptyByName.byName = function () { return null; };
var document = {
  flattenedTasks: [task], flattenedProjects: emptyByName,
  flattenedFolders: emptyByName, flattenedTags: emptyByName
};"#;

fn run_script(tool: &str, script: &str) -> String {
    common::assert_script_compiles(tool, script);
    common::run_jsc(
        DATABASE_WITHOUT_PLANNED_DATES,
        &format!(
            "try {{ (function () {{\n{script}\n}})(); print(\"NO ERROR\"); }} \
             catch (error) {{ print(\"ERROR: \" + error.message); }}"
        ),
    )
}

#[tokio::test]
async fn list_tasks_names_each_unsupported_planned_filter() {
    let before = list_script(Some("2026-05-15"), None, None).await;
    assert_eq!(
        run_script("list_tasks", &before),
        "ERROR: plannedBefore requires an OmniFocus database migrated to support planned dates"
    );

    let after = list_script(None, Some("2026-05-05"), None).await;
    assert_eq!(
        run_script("list_tasks", &after),
        "ERROR: plannedAfter requires an OmniFocus database migrated to support planned dates"
    );
}

#[tokio::test]
async fn search_tasks_rejects_both_unsupported_planned_filters() {
    let script = search_script(Some("2026-05-15"), Some("2026-05-05"), None).await;
    assert_eq!(
        run_script("search_tasks", &script),
        "ERROR: plannedBefore/plannedAfter require an OmniFocus database migrated to support planned dates"
    );
}

#[tokio::test]
async fn planned_date_sort_requires_database_support() {
    for sort_by in ["plannedDate", "planned"] {
        let script = list_script(None, None, Some(sort_by)).await;
        assert_eq!(
            run_script("list_tasks", &script),
            "ERROR: sortBy requires an OmniFocus database migrated to support planned dates",
            "sort {sort_by}"
        );
    }
}

#[tokio::test]
async fn absent_planned_filter_or_sort_keeps_compatibility_behavior() {
    let script = search_script(None, None, None).await;
    assert_eq!(run_script("search_tasks", &script), "NO ERROR");
}
