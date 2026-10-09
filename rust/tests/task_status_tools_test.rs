//! Task status in the tools that filter or count tasks, run in JavaScriptCore.
//!
//! Every tool here used to treat any task not marked completed as open, so
//! dropped tasks, blocked tasks and tasks in completed, dropped or on-hold
//! projects leaked into `available`, `overdue`, `due_soon`, the task counts,
//! the project and tag counts, the stalled test and the forecast. Each test
//! builds the tool's real script, runs it against one fake database holding
//! every one of those cases, and checks what comes out.
//!
//! The fake database (all dates relative to the real clock, as the scripts
//! read it):
//!
//! | task                 | project            | state                          |
//! | -------------------- | ------------------ | ------------------------------ |
//! | `control`            | `p-active`         | due, actionable, next task     |
//! | `blocked`            | `p-active`         | due, `Blocked`                 |
//! | `deferred`           | `p-active`         | deferred into the future       |
//! | `finished`           | `p-active`         | completed                      |
//! | `in-done-project`    | `p-done`           | due, actionable status         |
//! | `in-dropped-project` | `p-dropped`        | due, actionable status         |
//! | `in-on-hold-project` | `p-hold`           | due, actionable status         |
//! | `dropped-on-hold`    | `p-hold`           | `Dropped`                      |
//! | `dropped`            | `p-only-dropped`   | due, deferred, `Dropped`       |
//! | `stuck`              | `p-stuck`          | `Blocked`, project has no next |
//! | `inbox`              | none               | `Available`                    |
//!
//! "Due" is two days ago (`overdue` variant) or in two days (`due soon`
//! variant), with the matching `Overdue` / `DueSoon` status. Every task but
//! `deferred`, `stuck`, `dropped-on-hold` and `inbox` carries the active tag
//! `errand`.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    js_helpers::{JS_PROJECT_STATUS, JS_TASK_STATUS},
    jxa::JxaRunner,
    tools::{
        forecast::get_forecast,
        projects::{get_project, get_project_counts, list_projects, update_project},
        tags::list_tags,
        tasks::{
            get_task, get_task_counts_with_added_changed, list_tasks_with_added_changed,
            search_tasks_with_added_changed,
        },
    },
};
use serde_json::{json, Value};

// ---------------------------------------------------------------- capture

/// Records the script it is asked to run and answers with a fixed payload.
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
        let scripts = self.scripts.lock().expect("script lock");
        scripts.last().cloned().expect("a script was run")
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
        let payload = self.payload.clone();
        Box::pin(async move { Ok(payload) })
    }
}

// ---------------------------------------------------------------- fake database

/// When the due tasks fall due.
#[derive(Clone, Copy)]
enum Due {
    /// Two days ago, status `Overdue`.
    Overdue,
    /// In two days, status `DueSoon`.
    Soon,
}

const FAKE_DATABASE: &str = r#"function FakeEnum(kind, name) { this.kind = kind; this.name = name; }
FakeEnum.prototype.toString = function () { return "[object " + this.kind + ": " + this.name + "]"; };
function fakeEnum(kind, names) {
  const members = {};
  names.forEach(function (name) { members[name] = new FakeEnum(kind, name); });
  return members;
}
var Task = { Status: fakeEnum("Task.Status",
  ["Available", "Blocked", "Completed", "Dropped", "DueSoon", "Next", "Overdue"]),
  byIdentifier: function (id) { return allTasks.find(t => t.id.primaryKey === id) || null; } };
var Project = { Status: fakeEnum("Project.Status", ["Active", "Done", "Dropped", "OnHold"]),
  byIdentifier: function (id) { return allProjects.find(p => p.id.primaryKey === id) || null; } };
var Tag = { Status: fakeEnum("Tag.Status", ["Active", "OnHold", "Dropped"]) };

var DAY = 24 * 60 * 60 * 1000;
var PAST = new Date(Date.now() - 20 * DAY);
var FUTURE = new Date(Date.now() + 30 * DAY);
var DUE = new Date(Date.now() + FAKE_DUE_DAYS * DAY);
var DUE_STATUS = Task.Status[FAKE_DUE_STATUS];

var errand = { id: { primaryKey: "errand" }, name: "errand", parent: null, status: Tag.Status.Active };
var allTasks = [];
var allProjects = [];

function fakeProject(id, status) {
  const project = { id: { primaryKey: id }, name: id, status: Project.Status[status],
    parentFolder: null, nextTask: null, flattenedTasks: [], tasks: [], tags: [],
    deferDate: null, dueDate: null, completionDate: null, task: { modified: null }, note: "",
    flagged: false, sequential: false, containsSingletonActions: false,
    completedByChildren: false, reviewInterval: null };
  allProjects.push(project);
  return project;
}
function fakeTask(id, project, fields) {
  const task = { id: { primaryKey: id }, name: "task " + id, note: "", flagged: false,
    completed: false, taskStatus: Task.Status.Available, containingProject: project,
    tags: [], dueDate: null, deferDate: null, effectiveDeferDate: null,
    completionDate: null, effectiveCompletionDate: null, dropDate: null,
    effectiveDropDate: null, added: null, modified: null, plannedDate: null,
    estimatedMinutes: null, hasChildren: false };
  for (const key in fields) task[key] = fields[key];
  if (project !== null) {
    project.flattenedTasks.push(task);
    project.tasks.push(task);
  }
  allTasks.push(task);
  return task;
}
function dueTask(id, project, fields) {
  const base = { taskStatus: DUE_STATUS, dueDate: DUE, tags: [errand] };
  for (const key in fields) base[key] = fields[key];
  return fakeTask(id, project, base);
}
function projectOnlyTask(id, project, fields) {
  const task = fakeTask(id, project, fields);
  allTasks.pop();
  return task;
}
function projectRootTask(id, project) {
  const task = { id: { primaryKey: id }, name: "task " + id, note: "", flagged: false,
    completed: false, taskStatus: DUE_STATUS, containingProject: project, project: project,
    tags: [errand], dueDate: DUE, deferDate: null, effectiveDeferDate: null,
    completionDate: null, effectiveCompletionDate: null, dropDate: null,
    effectiveDropDate: null, added: null, modified: null, plannedDate: null,
    estimatedMinutes: null, hasChildren: false };
  allTasks.push(task);
  return task;
}

var pActive = fakeProject("p-active", "Active");
var pDone = fakeProject("p-done", "Done");
var pDropped = fakeProject("p-dropped", "Dropped");
var pHold = fakeProject("p-hold", "OnHold");
var pOnlyDropped = fakeProject("p-only-dropped", "Active");
var pStuck = fakeProject("p-stuck", "Active");
var pSingleAvailable = fakeProject("p-single-available", "Active");
var pSingleBlocked = fakeProject("p-single-blocked", "Active");
pSingleAvailable.containsSingletonActions = true;
pSingleBlocked.containsSingletonActions = true;

pActive.nextTask = dueTask("control", pActive, {});
dueTask("blocked", pActive, { taskStatus: Task.Status.Blocked });
fakeTask("deferred", pActive, { taskStatus: Task.Status.Blocked, deferDate: FUTURE,
  effectiveDeferDate: FUTURE });
fakeTask("finished", pActive, { completed: true, taskStatus: Task.Status.Completed,
  completionDate: PAST, effectiveCompletionDate: PAST, tags: [errand] });
dueTask("in-done-project", pDone, {});
dueTask("in-dropped-project", pDropped, {});
dueTask("in-on-hold-project", pHold, {});
dueTask("dropped", pOnlyDropped, { taskStatus: Task.Status.Dropped, deferDate: FUTURE,
  effectiveDeferDate: FUTURE, dropDate: PAST, effectiveDropDate: PAST });
fakeTask("stuck", pStuck, { taskStatus: Task.Status.Blocked });
projectOnlyTask("single-available", pSingleAvailable, {});
projectOnlyTask("single-blocked", pSingleBlocked, { taskStatus: Task.Status.Blocked });
fakeTask("dropped-on-hold", pHold, { taskStatus: Task.Status.Dropped, dropDate: PAST,
  effectiveDropDate: PAST });
fakeTask("inbox", null, {});
projectRootTask("project-root", pActive);

var document = { flattenedTasks: allTasks, flattenedProjects: allProjects,
  flattenedTags: [errand] };"#;

/// Runs a complete tool script against the fake database and returns what it
/// returned. Fails with the script's error message if it throws.
fn run_against_database(tool: &str, script: &str, due: Due) -> Value {
    common::assert_script_compiles(tool, script);
    let (days, status) = match due {
        Due::Overdue => (-2, "Overdue"),
        Due::Soon => (2, "DueSoon"),
    };
    let prelude = format!(
        "var FAKE_DUE_DAYS = {days};\nvar FAKE_DUE_STATUS = \"{status}\";\n{FAKE_DATABASE}"
    );
    let output = common::run_jsc(
        &prelude,
        &format!(
            "try {{ const result = (function () {{\n{script}\n}})(); print(JSON.stringify(result)); }}\n\
             catch (error) {{ print(\"ERROR: \" + error.message); }}"
        ),
    );
    serde_json::from_str(&output).unwrap_or_else(|_| panic!("{tool} returned {output}"))
}

/// The `id` of every object in a returned array, in order.
fn ids(items: &Value) -> Vec<String> {
    items
        .as_array()
        .expect("an array of objects")
        .iter()
        .map(|item| item["id"].as_str().expect("an id").to_string())
        .collect()
}

/// The returned object whose `id` is `id`.
fn by_id<'a>(items: &'a Value, id: &str) -> &'a Value {
    items
        .as_array()
        .expect("an array of objects")
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("{id} missing from {items}"))
}

/// Every script that decides task status prepends the shared snippets once.
fn assert_uses_shared_task_status(tool: &str, script: &str) {
    assert_eq!(
        script.matches(JS_TASK_STATUS).count(),
        1,
        "{tool} must prepend JS_TASK_STATUS exactly once"
    );
    assert_eq!(
        script.matches(JS_PROJECT_STATUS).count(),
        1,
        "{tool} must prepend JS_PROJECT_STATUS exactly once"
    );
}

// ---------------------------------------------------------------- list_tasks / search_tasks

#[tokio::test]
async fn get_task_lookup_rejects_project_root_tasks() {
    let runner = CapturingRunner::new(json!({"id": "project-root"}));
    get_task(&runner, "project-root")
        .await
        .expect("get_task captures its script");
    let script = runner.last_script();
    assert_uses_shared_task_status("get_task", &script);
    // resolveTask (JS_RESOLVERS) refuses a project's root task.
    assert!(script.contains("const task = resolveTask(taskId);"));
}

async fn list_tasks_ids(status: &str, due: Due) -> Vec<String> {
    let runner = CapturingRunner::new(json!([]));
    list_tasks_with_added_changed(
        &runner, None, None, None, "any", None, status, None, None, None, None, None, None, None,
        None, None, None, None, None, None, None, "asc", 100,
    )
    .await
    .expect("list_tasks runs");
    let script = runner.last_script();
    assert_uses_shared_task_status("list_tasks", &script);
    ids(&run_against_database("list_tasks", &script, due))
}

async fn search_tasks_ids(status: &str, due: Due) -> Vec<String> {
    let runner = CapturingRunner::new(json!([]));
    search_tasks_with_added_changed(
        &runner, "task", None, None, None, "any", None, status, None, None, None, None, None, None,
        None, None, None, None, None, None, None, None, "asc", 100,
    )
    .await
    .expect("search_tasks runs");
    let script = runner.last_script();
    assert_uses_shared_task_status("search_tasks", &script);
    ids(&run_against_database("search_tasks", &script, due))
}

const AVAILABLE: [&str; 2] = ["control", "inbox"];
const REMAINING_DUE: [&str; 3] = ["control", "blocked", "in-on-hold-project"];

#[tokio::test]
async fn list_tasks_available_is_actionable_tasks_in_active_projects_only() {
    assert_eq!(list_tasks_ids("available", Due::Overdue).await, AVAILABLE);
}

#[tokio::test]
async fn list_tasks_overdue_excludes_dropped_and_finished_projects() {
    assert_eq!(list_tasks_ids("overdue", Due::Overdue).await, REMAINING_DUE);
}

#[tokio::test]
async fn list_tasks_due_soon_excludes_dropped_and_finished_projects() {
    assert_eq!(list_tasks_ids("due_soon", Due::Soon).await, REMAINING_DUE);
}

#[tokio::test]
async fn list_tasks_on_hold_is_remaining_tasks_of_on_hold_projects() {
    assert_eq!(
        list_tasks_ids("on_hold", Due::Overdue).await,
        ["in-on-hold-project"]
    );
}

#[tokio::test]
async fn list_tasks_completed_includes_tasks_completed_by_their_project() {
    assert_eq!(
        list_tasks_ids("completed", Due::Overdue).await,
        ["finished", "in-done-project"]
    );
}

#[tokio::test]
async fn list_and_search_tasks_exclude_project_root_tasks() {
    let listed = list_tasks_ids("all", Due::Overdue).await;
    let searched = search_tasks_ids("all", Due::Overdue).await;
    assert_eq!(listed.len(), 11);
    assert_eq!(searched.len(), 11);
    assert!(!listed.contains(&"project-root".to_string()));
    assert!(!searched.contains(&"project-root".to_string()));
}

#[tokio::test]
async fn search_tasks_available_is_actionable_tasks_in_active_projects_only() {
    assert_eq!(search_tasks_ids("available", Due::Overdue).await, AVAILABLE);
}

#[tokio::test]
async fn search_tasks_overdue_excludes_dropped_and_finished_projects() {
    assert_eq!(
        search_tasks_ids("overdue", Due::Overdue).await,
        REMAINING_DUE
    );
}

#[tokio::test]
async fn search_tasks_due_soon_excludes_dropped_and_finished_projects() {
    assert_eq!(search_tasks_ids("due_soon", Due::Soon).await, REMAINING_DUE);
}

#[tokio::test]
async fn search_tasks_on_hold_is_remaining_tasks_of_on_hold_projects() {
    assert_eq!(
        search_tasks_ids("on_hold", Due::Overdue).await,
        ["in-on-hold-project"]
    );
}

// ---------------------------------------------------------------- get_task_counts

async fn task_counts(due: Due) -> Value {
    let runner = CapturingRunner::new(json!({"total": 0, "available": 0, "completed": 0,
        "overdue": 0, "dueSoon": 0, "flagged": 0, "deferred": 0}));
    get_task_counts_with_added_changed(
        &runner, None, None, None, "any", None, None, None, None, None, None, None, None, None,
        None, None, None, None, None,
    )
    .await
    .expect("get_task_counts runs");
    let script = runner.last_script();
    assert_uses_shared_task_status("get_task_counts", &script);
    run_against_database("get_task_counts", &script, due)
}

#[tokio::test]
async fn task_counts_use_the_list_tasks_definitions() {
    let counts = task_counts(Due::Overdue).await;
    assert_eq!(
        counts,
        json!({"total": 11, "available": 2, "completed": 2, "overdue": 3, "dueSoon": 0,
            "flagged": 0, "deferred": 1}),
        "available as list_tasks, overdue and deferred over remaining tasks only"
    );
}

#[tokio::test]
async fn task_counts_due_soon_counts_remaining_tasks_only() {
    let counts = task_counts(Due::Soon).await;
    assert_eq!(counts["dueSoon"], 3);
    assert_eq!(counts["overdue"], 0);
}

// ---------------------------------------------------------------- projects

async fn projects(status: &str, stalled_only: bool) -> Value {
    let runner = CapturingRunner::new(json!([]));
    list_projects(
        &runner,
        None,
        status,
        None,
        None,
        stalled_only,
        None,
        "asc",
        100,
    )
    .await
    .expect("list_projects runs");
    let script = runner.last_script();
    assert_uses_shared_task_status("list_projects", &script);
    run_against_database("list_projects", &script, Due::Overdue)
}

#[tokio::test]
async fn list_projects_remaining_counts_exclude_dropped_tasks() {
    let active = projects("active", false).await;
    assert_eq!(by_id(&active, "p-active")["remainingTaskCount"], 3);
    assert_eq!(by_id(&active, "p-only-dropped")["remainingTaskCount"], 0);
    assert_eq!(by_id(&active, "p-only-dropped")["taskCount"], 1);
    assert_eq!(by_id(&active, "p-stuck")["remainingTaskCount"], 1);
    assert_eq!(
        by_id(&projects("on_hold", false).await, "p-hold")["remainingTaskCount"],
        1
    );
}

#[tokio::test]
async fn list_projects_tasks_of_done_or_dropped_projects_are_not_remaining() {
    let completed = projects("completed", false).await;
    assert_eq!(by_id(&completed, "p-done")["remainingTaskCount"], 0);
    let dropped = projects("dropped", false).await;
    assert_eq!(by_id(&dropped, "p-dropped")["remainingTaskCount"], 0);
}

#[tokio::test]
async fn project_whose_only_open_task_is_dropped_is_not_stalled() {
    let active = projects("active", false).await;
    assert_eq!(by_id(&active, "p-only-dropped")["isStalled"], false);
    assert_eq!(by_id(&active, "p-stuck")["isStalled"], true);
    assert_eq!(by_id(&active, "p-active")["isStalled"], false);
    assert_eq!(by_id(&active, "p-single-available")["isStalled"], false);
    assert_eq!(by_id(&active, "p-single-blocked")["isStalled"], true);
}

#[tokio::test]
async fn stalled_only_lists_only_truly_stalled_projects() {
    assert_eq!(
        ids(&projects("active", true).await),
        ["p-stuck", "p-single-blocked"]
    );
}

#[tokio::test]
async fn project_counts_stalled_ignores_dropped_tasks() {
    let runner = CapturingRunner::new(json!({"total": 0, "active": 0, "onHold": 0,
        "completed": 0, "dropped": 0, "stalled": 0}));
    get_project_counts(&runner, None)
        .await
        .expect("get_project_counts runs");
    let script = runner.last_script();
    assert_uses_shared_task_status("get_project_counts", &script);
    let counts = run_against_database("get_project_counts", &script, Due::Overdue);
    assert_eq!(counts["stalled"], 2);
    assert_eq!(counts["active"], 5);
}

async fn project_details(project: &str) -> Value {
    let runner = CapturingRunner::new(json!({"id": project}));
    get_project(&runner, project)
        .await
        .expect("get_project runs");
    let script = runner.last_script();
    assert_uses_shared_task_status("get_project", &script);
    run_against_database("get_project", &script, Due::Overdue)
}

#[tokio::test]
async fn get_project_available_count_excludes_blocked_and_deferred_tasks() {
    let details = project_details("p-active").await;
    assert_eq!(details["taskCount"], 4);
    assert_eq!(details["remainingTaskCount"], 3);
    assert_eq!(details["completedTaskCount"], 1);
    assert_eq!(details["availableTaskCount"], 1);
    assert_eq!(details["isStalled"], false);
}

#[tokio::test]
async fn get_project_on_hold_project_has_remaining_but_no_available_tasks() {
    let details = project_details("p-hold").await;
    assert_eq!(details["remainingTaskCount"], 1);
    assert_eq!(details["availableTaskCount"], 0);
}

#[tokio::test]
async fn get_project_done_project_has_no_remaining_tasks() {
    let details = project_details("p-done").await;
    assert_eq!(details["remainingTaskCount"], 0);
    assert_eq!(details["completedTaskCount"], 1);
    assert_eq!(details["availableTaskCount"], 0);
}

#[tokio::test]
async fn get_project_only_dropped_tasks_is_not_stalled() {
    let details = project_details("p-only-dropped").await;
    assert_eq!(details["remainingTaskCount"], 0);
    assert_eq!(details["availableTaskCount"], 0);
    assert_eq!(details["isStalled"], false);
}

#[tokio::test]
async fn get_project_uses_single_action_availability_for_stalled_state() {
    assert_eq!(
        project_details("p-single-available").await["isStalled"],
        false
    );
    assert_eq!(project_details("p-single-blocked").await["isStalled"], true);
}

#[tokio::test]
async fn update_project_remaining_count_excludes_tasks_of_a_dropped_project() {
    let runner = CapturingRunner::new(json!({"id": "p-dropped"}));
    update_project(
        &runner,
        "p-dropped",
        None,
        Some("reviewed"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .expect("update_project runs");
    let script = runner.last_script();
    assert_uses_shared_task_status("update_project", &script);
    let updated = run_against_database("update_project", &script, Due::Overdue);
    assert_eq!(updated["taskCount"], 1);
    assert_eq!(updated["remainingTaskCount"], 0);
}

// ---------------------------------------------------------------- tags

#[tokio::test]
async fn list_tags_available_count_is_available_tasks_only() {
    let runner = CapturingRunner::new(json!([]));
    list_tags(&runner, "all", None, "asc", 100)
        .await
        .expect("list_tags runs");
    let script = runner.last_script();
    assert_uses_shared_task_status("list_tags", &script);
    let tags = run_against_database("list_tags", &script, Due::Overdue);
    let errand = by_id(&tags, "errand");
    assert_eq!(errand["totalTaskCount"], 7);
    assert_eq!(
        errand["availableTaskCount"], 1,
        "only `control` is available"
    );
}

// ---------------------------------------------------------------- forecast

async fn forecast(due: Due) -> Value {
    let runner = CapturingRunner::new(json!({}));
    get_forecast(&runner, 100).await.expect("get_forecast runs");
    let script = runner.last_script();
    assert_uses_shared_task_status("get_forecast", &script);
    run_against_database("get_forecast", &script, due)
}

#[tokio::test]
async fn forecast_overdue_and_deferred_hold_remaining_tasks_only() {
    let result = forecast(Due::Overdue).await;
    assert_eq!(ids(&result["overdue"]), REMAINING_DUE);
    assert_eq!(result["counts"]["overdueCount"], 3);
    assert_eq!(ids(&result["deferred"]), ["deferred"]);
    assert_eq!(result["counts"]["deferredCount"], 1);
}

#[tokio::test]
async fn forecast_due_this_week_holds_remaining_tasks_only() {
    let result = forecast(Due::Soon).await;
    assert_eq!(ids(&result["dueThisWeek"]), REMAINING_DUE);
    assert_eq!(result["counts"]["dueThisWeekCount"], 3);
}
