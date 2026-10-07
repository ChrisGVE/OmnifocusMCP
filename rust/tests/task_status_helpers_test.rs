//! Task availability (`JS_TASK_STATUS`), run in JavaScriptCore.
//!
//! The task filters and counts used to ask only whether a task was marked
//! completed. A dropped task, a blocked task, and every task inside a
//! completed, dropped or on-hold project therefore counted as open. The shared
//! snippet answers two questions from OmniFocus's own documented state:
//!
//! - `isTaskRemaining(task)`: not completed and not dropped, including
//!   *effectively* (a container completed or dropped, or the project done or
//!   dropped);
//! - `isTaskAvailable(task, now)`: remaining, with an actionable
//!   `Task.Status`, in an active project (or none), not deferred into the
//!   future, and with no on-hold tag.
//!
//! Each case below sets only the fact it is about, so every guard in the
//! snippet is exercised on its own. The enums are fakes whose members are
//! distinct objects, like OmniJS's.

mod common;

use omnifocus_mcp::js_helpers::{JS_PROJECT_STATUS, JS_TASK_STATUS};

/// Fakes of `Task.Status`, `Project.Status` and `Tag.Status`, a fixed `NOW`,
/// and a task builder whose defaults describe a plain available inbox task.
const FAKES: &str = r#"function FakeEnum(kind, name) { this.kind = kind; this.name = name; }
FakeEnum.prototype.toString = function () { return "[object " + this.kind + ": " + this.name + "]"; };
function fakeEnum(kind, names) {
  const members = {};
  names.forEach(function (name) { members[name] = new FakeEnum(kind, name); });
  return members;
}
var Task = { Status: fakeEnum("Task.Status",
  ["Available", "Blocked", "Completed", "Dropped", "DueSoon", "Next", "Overdue"]) };
var Project = { Status: fakeEnum("Project.Status", ["Active", "Done", "Dropped", "OnHold"]) };
var Tag = { Status: fakeEnum("Tag.Status", ["Active", "OnHold", "Dropped"]) };
var NOW = new Date(Date.UTC(2026, 4, 10, 12, 0));
var PAST = new Date(Date.UTC(2026, 4, 1, 12, 0));
var FUTURE = new Date(Date.UTC(2026, 5, 1, 12, 0));
function inProject(status) { return { status: Project.Status[status] }; }
function withTag(status) { return { status: Tag.Status[status] }; }
function fakeTask(fields) {
  const task = { completed: false, taskStatus: Task.Status.Available,
    effectiveCompletionDate: null, effectiveDropDate: null, effectiveDeferDate: null,
    containingProject: null, tags: [] };
  for (const key in fields) task[key] = fields[key];
  return task;
}"#;

/// `[isTaskRemaining, isTaskAvailable]` for the task built from `fields`, a
/// JS object literal of the properties that differ from the defaults.
fn verdicts(fields: &str) -> String {
    let prelude = format!("{FAKES}\n{JS_PROJECT_STATUS}\n{JS_TASK_STATUS}");
    common::run_jsc(
        &prelude,
        &format!(
            "const task = fakeTask({fields});\n\
             print(JSON.stringify([isTaskRemaining(task), isTaskAvailable(task, NOW)]));"
        ),
    )
}

#[test]
fn snippet_compiles_alongside_the_project_status_snippet_it_needs() {
    common::assert_script_compiles(
        "JS_TASK_STATUS",
        &format!("{JS_PROJECT_STATUS}\n{JS_TASK_STATUS}"),
    );
}

const REMAINING_AND_AVAILABLE: &str = "[true,true]";
const REMAINING_NOT_AVAILABLE: &str = "[true,false]";
const NEITHER: &str = "[false,false]";

// ---------------------------------------------------------------- Task.Status

#[test]
fn available_inbox_task_is_remaining_and_available() {
    assert_eq!(verdicts("{}"), REMAINING_AND_AVAILABLE);
}

#[test]
fn next_task_is_available() {
    assert_eq!(
        verdicts("{ taskStatus: Task.Status.Next }"),
        REMAINING_AND_AVAILABLE
    );
}

#[test]
fn due_soon_task_is_available() {
    assert_eq!(
        verdicts("{ taskStatus: Task.Status.DueSoon }"),
        REMAINING_AND_AVAILABLE
    );
}

#[test]
fn overdue_task_is_available() {
    assert_eq!(
        verdicts("{ taskStatus: Task.Status.Overdue }"),
        REMAINING_AND_AVAILABLE
    );
}

#[test]
fn blocked_task_is_remaining_but_not_available() {
    assert_eq!(
        verdicts("{ taskStatus: Task.Status.Blocked }"),
        REMAINING_NOT_AVAILABLE
    );
}

#[test]
fn completed_task_is_neither() {
    assert_eq!(
        verdicts("{ completed: true, taskStatus: Task.Status.Completed }"),
        NEITHER
    );
}

#[test]
fn completed_flag_alone_makes_a_task_not_remaining() {
    assert_eq!(verdicts("{ completed: true }"), NEITHER);
}

#[test]
fn dropped_task_is_neither() {
    assert_eq!(verdicts("{ taskStatus: Task.Status.Dropped }"), NEITHER);
}

#[test]
fn completed_status_alone_makes_a_task_not_remaining() {
    // A task inside a completed action group: not marked completed itself.
    assert_eq!(verdicts("{ taskStatus: Task.Status.Completed }"), NEITHER);
}

#[test]
fn unrecognised_task_status_is_remaining_but_not_available() {
    assert_eq!(
        verdicts("{ taskStatus: new FakeEnum(\"Task.Status\", \"Someday\") }"),
        REMAINING_NOT_AVAILABLE
    );
}

// ---------------------------------------------------------------- effective dates

#[test]
fn effective_drop_date_makes_a_task_not_remaining() {
    // Dropped through a container: the task's own status still reads available.
    assert_eq!(verdicts("{ effectiveDropDate: PAST }"), NEITHER);
}

#[test]
fn effective_completion_date_makes_a_task_not_remaining() {
    assert_eq!(verdicts("{ effectiveCompletionDate: PAST }"), NEITHER);
}

#[test]
fn task_deferred_into_the_future_is_not_available() {
    // Even when OmniFocus reports a due status instead of Blocked.
    assert_eq!(
        verdicts("{ taskStatus: Task.Status.Overdue, effectiveDeferDate: FUTURE }"),
        REMAINING_NOT_AVAILABLE
    );
}

#[test]
fn task_deferred_to_the_past_is_available() {
    assert_eq!(
        verdicts("{ effectiveDeferDate: PAST }"),
        REMAINING_AND_AVAILABLE
    );
}

#[test]
fn missing_effective_properties_read_as_unset() {
    // An OmniFocus that predates a property returns undefined, not null.
    assert_eq!(
        verdicts(
            "{ effectiveCompletionDate: undefined, effectiveDropDate: undefined, \
             effectiveDeferDate: undefined }"
        ),
        REMAINING_AND_AVAILABLE
    );
}

// ---------------------------------------------------------------- containing project

#[test]
fn task_in_active_project_is_available() {
    assert_eq!(
        verdicts("{ containingProject: inProject(\"Active\") }"),
        REMAINING_AND_AVAILABLE
    );
}

#[test]
fn task_in_done_project_is_neither() {
    // Project.status does not reach the tasks it holds: the task keeps its
    // own state, so the project has to be read.
    assert_eq!(
        verdicts("{ containingProject: inProject(\"Done\"), taskStatus: Task.Status.Overdue }"),
        NEITHER
    );
}

#[test]
fn task_in_dropped_project_is_neither() {
    assert_eq!(
        verdicts("{ containingProject: inProject(\"Dropped\"), taskStatus: Task.Status.Overdue }"),
        NEITHER
    );
}

#[test]
fn task_in_on_hold_project_is_remaining_but_not_available() {
    assert_eq!(
        verdicts("{ containingProject: inProject(\"OnHold\"), taskStatus: Task.Status.Overdue }"),
        REMAINING_NOT_AVAILABLE
    );
}

#[test]
fn task_in_project_with_unrecognised_status_is_remaining_but_not_available() {
    assert_eq!(
        verdicts(
            "{ containingProject: { status: new FakeEnum(\"Project.Status\", \"Archived\") } }"
        ),
        REMAINING_NOT_AVAILABLE
    );
}

// ---------------------------------------------------------------- tags

#[test]
fn task_with_on_hold_tag_is_remaining_but_not_available() {
    assert_eq!(
        verdicts(
            "{ taskStatus: Task.Status.Overdue, tags: [withTag(\"Active\"), withTag(\"OnHold\")] }"
        ),
        REMAINING_NOT_AVAILABLE
    );
}

#[test]
fn task_with_active_tags_is_available() {
    assert_eq!(
        verdicts("{ tags: [withTag(\"Active\")] }"),
        REMAINING_AND_AVAILABLE
    );
}
