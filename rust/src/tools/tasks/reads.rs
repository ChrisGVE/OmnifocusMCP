//! Single-purpose task reads: the inbox, one task's full details, and the
//! direct subtasks of a task.
//!
//! Filtered listings live in `list` and `search`; counts in `counts`.

use serde_json::Value;

use crate::{
    error::{OmniFocusError, Result},
    js_helpers::{JS_PLANNED_DATE, JS_PROJECT_STATUS, JS_TASK_STATUS},
    jxa::{escape_for_jxa, JxaRunner},
    types::TaskResult,
};

use super::{
    inputs::parse_task_list,
    listing_script::{MAP_LISTED_TASKS, READ_PLANNED_DATES},
};

pub async fn get_inbox<R: JxaRunner>(runner: &R, limit: i32) -> Result<Vec<TaskResult>> {
    if limit < 1 {
        return Err(OmniFocusError::Validation(
            "limit must be greater than 0.".to_string(),
        ));
    }

    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_PLANNED_DATE}
{READ_PLANNED_DATES}
const tasks = inbox
  .filter(task => !task.completed)
  .slice(0, {limit});

{MAP_LISTED_TASKS}"#
    );

    let value = runner.run_omnijs(&script).await?;
    parse_task_list(value)
}

pub async fn get_task<R: JxaRunner>(runner: &R, task_id: &str) -> Result<Value> {
    if task_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "task_id must not be empty.".to_string(),
        ));
    }

    let task_id_filter = escape_for_jxa(task_id.trim());
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_PLANNED_DATE}
{READ_PLANNED_DATES}
const taskId = {task_id_filter};
const task = document.flattenedTasks.find(item => item.id.primaryKey === taskId && !isProjectRootTask(item));
if (!task) {{
  throw new Error(`Task not found: ${{taskId}}`);
}}

const children = task.children.map(child => {{
  return {{
    id: child.id.primaryKey,
    name: child.name,
    completed: isTaskCompleted(child)
  }};
}});

const repetitionRule = task.repetitionRule ? task.repetitionRule.ruleString : null;
const plannedDate = (() => {{
  try {{
    const value = task.plannedDate;
    return value === undefined ? null : value;
  }} catch (e) {{
    return null;
  }}
}})();
const effectivePlannedDate = (() => {{
  try {{
    const value = task.effectivePlannedDate;
    return value === undefined ? null : value;
  }} catch (e) {{
    return null;
  }}
}})();

{TASK_DETAILS_RESULT}"#
    );

    runner.run_omnijs(&script).await
}

/// The object `get_task` returns, read from `task`, `children`,
/// `repetitionRule`, `plannedDate` and `effectivePlannedDate`.
const TASK_DETAILS_RESULT: &str = r#"return {
  id: task.id.primaryKey,
  name: task.name,
  note: task.note,
  flagged: task.flagged,
  dueDate: task.dueDate ? task.dueDate.toISOString() : null,
  deferDate: task.deferDate ? task.deferDate.toISOString() : null,
  effectiveDueDate: task.effectiveDueDate ? task.effectiveDueDate.toISOString() : null,
  effectiveDeferDate: task.effectiveDeferDate ? task.effectiveDeferDate.toISOString() : null,
  effectiveFlagged: task.effectiveFlagged,
  completed: isTaskCompleted(task),
  completionDate: task.completionDate ? task.completionDate.toISOString() : null,
  addedDate: task.added ? task.added.toISOString() : null,
  changedDate: task.modified ? task.modified.toISOString() : null,
  modified: task.modified ? task.modified.toISOString() : null,
  plannedDate: plannedDate ? plannedDate.toISOString() : null,
  effectivePlannedDate: effectivePlannedDate ? effectivePlannedDate.toISOString() : null,
  taskStatus: (() => {
    const s = String(task.taskStatus);
    if (s.includes("Available")) return "available";
    if (s.includes("Blocked")) return "blocked";
    if (s.includes("Next")) return "next";
    if (s.includes("DueSoon")) return "due_soon";
    if (s.includes("Overdue")) return "overdue";
    if (s.includes("Completed")) return "completed";
    if (s.includes("Dropped")) return "dropped";
    return "unknown";
  })(),
  projectName: task.containingProject ? task.containingProject.name : null,
  inInbox: task.inInbox,
  tags: task.tags.map(tag => tag.name),
  estimatedMinutes: task.estimatedMinutes,
  children: children,
  parentName: task.parent ? task.parent.name : null,
  sequential: task.sequential,
  repetitionRule: repetitionRule
};"#;

pub async fn list_subtasks<R: JxaRunner>(
    runner: &R,
    task_id: &str,
    limit: i32,
) -> Result<Vec<TaskResult>> {
    if task_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "task_id must not be empty.".to_string(),
        ));
    }
    if limit < 1 {
        return Err(OmniFocusError::Validation(
            "limit must be greater than 0.".to_string(),
        ));
    }

    let task_id_filter = escape_for_jxa(task_id.trim());
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_PLANNED_DATE}
{READ_PLANNED_DATES}
const taskId = {task_id_filter};
const task = document.flattenedTasks.find(item => item.id.primaryKey === taskId && !isProjectRootTask(item));
if (!task) {{
  throw new Error(`Task not found: ${{taskId}}`);
}}

const tasks = task.children.slice(0, {limit});
{MAP_LISTED_TASKS}"#
    );

    let value = runner.run_omnijs(&script).await?;
    parse_task_list(value)
}
