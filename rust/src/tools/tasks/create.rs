//! Creating tasks one at a time: a task in the inbox or a project, a subtask
//! under an existing task, and a copy of an existing task.
//!
//! `create_task` and `create_subtask` take the same optional fields; those
//! are declared by `NewTaskFields` and applied by `APPLY_NEW_TASK_FIELDS`.
//! `CreateTaskInput` describes one task of `create_tasks_batch` (in `batch`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    error::{OmniFocusError, Result},
    js_helpers::{
        JS_DATE_HELPERS, JS_PLANNED_DATE, JS_PROJECT_STATUS, JS_RESOLVERS, JS_TASK_STATUS,
    },
    jxa::{escape_for_jxa, JxaRunner},
    tools::js_values::{
        js_bool_or_null, js_number_or_null, js_string_or_null, js_trimmed_string_or_null,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskInput {
    pub name: String,
    pub project: Option<String>,
    pub note: Option<String>,
    #[serde(rename = "dueDate")]
    pub due_date: Option<String>,
    #[serde(rename = "deferDate")]
    pub defer_date: Option<String>,
    #[serde(rename = "plannedDate")]
    pub planned_date: Option<String>,
    pub flagged: Option<bool>,
    pub tags: Option<Vec<String>>,
    #[serde(rename = "estimatedMinutes")]
    pub estimated_minutes: Option<i32>,
}

#[allow(clippy::too_many_arguments)]
pub async fn create_task<R: JxaRunner>(
    runner: &R,
    name: &str,
    project: Option<&str>,
    note: Option<&str>,
    due_date: Option<&str>,
    defer_date: Option<&str>,
    planned_date: Option<&str>,
    flagged: Option<bool>,
    tags: Option<Vec<String>>,
    estimated_minutes: Option<i32>,
) -> Result<Value> {
    if name.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "name must not be empty.".to_string(),
        ));
    }
    if let Some(project_name) = project {
        if project_name.trim().is_empty() {
            return Err(OmniFocusError::Validation(
                "project must not be empty when provided.".to_string(),
            ));
        }
    }

    let task_name = escape_for_jxa(name.trim());
    let project_name = js_trimmed_string_or_null(project);
    let field_declarations = NewTaskFields::new(
        note,
        due_date,
        defer_date,
        planned_date,
        flagged,
        tags,
        estimated_minutes,
    )?
    .declarations();

    let script = format!(
        r#"{JS_DATE_HELPERS}
{JS_PLANNED_DATE}
{JS_RESOLVERS}
const taskName = {task_name};
const projectName = {project_name};
{field_declarations}
const resolvedTags = tagNames === null ? [] : tagNames.map(tagName => resolveTag(tagName));

if (parsedPlannedDate !== null && !supportsPlannedDate) {{
  throw new Error("plannedDate requires an OmniFocus database migrated to support planned dates");
}}

const parent = (() => {{
  if (projectName === null || projectName === "") return inbox.ending;
  const targetProject = resolveProject(projectName);
  return targetProject.ending;
}})();

const task = new Task(taskName, parent);

{APPLY_NEW_TASK_FIELDS}
return {{
  id: task.id.primaryKey,
  name: task.name,
  plannedDate: plannedDate ? plannedDate.toISOString() : null
}};"#
    );

    runner.run_omnijs(&script).await
}

#[allow(clippy::too_many_arguments)]
pub async fn create_subtask<R: JxaRunner>(
    runner: &R,
    name: &str,
    parent_task_id: &str,
    note: Option<&str>,
    due_date: Option<&str>,
    defer_date: Option<&str>,
    planned_date: Option<&str>,
    flagged: Option<bool>,
    tags: Option<Vec<String>>,
    estimated_minutes: Option<i32>,
) -> Result<Value> {
    if name.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "name must not be empty.".to_string(),
        ));
    }
    if parent_task_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "parent_task_id must not be empty.".to_string(),
        ));
    }

    let task_name = escape_for_jxa(name.trim());
    let parent_task_id_value = escape_for_jxa(parent_task_id.trim());
    let field_declarations = NewTaskFields::new(
        note,
        due_date,
        defer_date,
        planned_date,
        flagged,
        tags,
        estimated_minutes,
    )?
    .declarations();

    let script = format!(
        r#"{JS_DATE_HELPERS}
{JS_PLANNED_DATE}
{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_RESOLVERS}
const taskName = {task_name};
const parentTaskId = {parent_task_id_value};
{field_declarations}

const parentTask = document.flattenedTasks.find(item => item.id.primaryKey === parentTaskId && !isProjectRootTask(item));
if (!parentTask) {{
  throw new Error(`Parent task not found: ${{parentTaskId}}`);
}}
const resolvedTags = tagNames === null ? [] : tagNames.map(tagName => resolveTag(tagName));

if (parsedPlannedDate !== null && !supportsPlannedDate) {{
  throw new Error("plannedDate requires an OmniFocus database migrated to support planned dates");
}}

const task = new Task(taskName, parentTask.ending);

{APPLY_NEW_TASK_FIELDS}
return {{
  id: task.id.primaryKey,
  name: task.name,
  parentTaskId: parentTask.id.primaryKey,
  parentTaskName: parentTask.name,
  plannedDate: plannedDate ? plannedDate.toISOString() : null
}};"#
    );

    runner.run_omnijs(&script).await
}

/// The optional fields of a new task or subtask, as JavaScript literals.
struct NewTaskFields {
    note_value: String,
    due_date_value: String,
    defer_date_value: String,
    planned_date_value: String,
    flagged_value: String,
    tags_value: String,
    estimated_minutes_value: String,
}

impl NewTaskFields {
    fn new(
        note: Option<&str>,
        due_date: Option<&str>,
        defer_date: Option<&str>,
        planned_date: Option<&str>,
        flagged: Option<bool>,
        tags: Option<Vec<String>>,
        estimated_minutes: Option<i32>,
    ) -> Result<Self> {
        let tags_value = match tags {
            Some(values) => serde_json::to_string(&values)?,
            None => "null".to_string(),
        };
        Ok(Self {
            note_value: js_string_or_null(note),
            due_date_value: js_string_or_null(due_date),
            defer_date_value: js_string_or_null(defer_date),
            planned_date_value: js_string_or_null(planned_date),
            flagged_value: js_bool_or_null(flagged),
            tags_value,
            estimated_minutes_value: js_number_or_null(estimated_minutes),
        })
    }

    /// Declares each field and parses the dates, so a malformed date fails
    /// the script before anything is created. Each script then checks the
    /// planned date against the database's support for it.
    fn declarations(&self) -> String {
        let NewTaskFields {
            note_value,
            due_date_value,
            defer_date_value,
            planned_date_value,
            flagged_value,
            tags_value,
            estimated_minutes_value,
        } = self;
        format!(
            r#"const noteValue = {note_value};
const dueDateValue = {due_date_value};
const deferDateValue = {defer_date_value};
const plannedDateValue = {planned_date_value};
const flaggedValue = {flagged_value};
const tagNames = {tags_value};
const estimatedMinutesValue = {estimated_minutes_value};
const supportsPlannedDate = detectPlannedDateSupport(document.flattenedTasks);
const parsedDueDate = dueDateValue === null ? null : parseWriteDate(dueDateValue, "dueDate", "DefaultDueTime", "17:00");
const parsedDeferDate = deferDateValue === null ? null : parseWriteDate(deferDateValue, "deferDate", "DefaultStartTime", "00:00");
const parsedPlannedDate = plannedDateValue === null ? null : parseWriteDate(plannedDateValue, "plannedDate", "DefaultPlannedTime", "09:00");"#
        )
    }
}

/// Applies the declared fields and resolved tags to the new `task`, then
/// reads back its planned date for the result.
const APPLY_NEW_TASK_FIELDS: &str = r#"if (noteValue !== null) task.note = noteValue;
if (parsedDueDate !== null) task.dueDate = parsedDueDate;
if (parsedDeferDate !== null) task.deferDate = parsedDeferDate;
if (parsedPlannedDate !== null) setPlannedDate(task, parsedPlannedDate);
if (flaggedValue !== null) task.flagged = flaggedValue;
if (estimatedMinutesValue !== null) task.estimatedMinutes = estimatedMinutesValue;

resolvedTags.forEach(tag => task.addTag(tag));

const plannedDate = readPlannedDate(task, supportsPlannedDate);"#;

pub async fn duplicate_task<R: JxaRunner>(
    runner: &R,
    task_id: &str,
    include_children: bool,
) -> Result<Value> {
    if task_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "task_id must not be empty.".to_string(),
        ));
    }

    let task_id_filter = escape_for_jxa(task_id.trim());
    let include_children_value = if include_children { "true" } else { "false" };
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
const taskId = {task_id_filter};
const includeChildren = {include_children_value};
const task = document.flattenedTasks.find(item => item.id.primaryKey === taskId && !isProjectRootTask(item));
if (!task) {{
  throw new Error(`Task not found: ${{taskId}}`);
}}
const insertionLocation = task.containingProject ? task.containingProject.ending : inbox.ending;

const taskStatusValue = (taskItem) => {{
  const s = String(taskItem.taskStatus);
  if (s.includes("Available")) return "available";
  if (s.includes("Blocked")) return "blocked";
  if (s.includes("Next")) return "next";
  if (s.includes("DueSoon")) return "due_soon";
  if (s.includes("Overdue")) return "overdue";
  if (s.includes("Completed")) return "completed";
  if (s.includes("Dropped")) return "dropped";
  return "unknown";
}};

const plannedDateValue = (taskItem) => {{
  try {{
    return taskItem.plannedDate ? taskItem.plannedDate.toISOString() : null;
  }} catch (e) {{
    return null;
  }}
}};

let duplicatedTask;
if (includeChildren) {{
  const duplicated = duplicateTasks([task], insertionLocation);
  if (!duplicated || duplicated.length === 0) {{
    throw new Error("Failed to duplicate task.");
  }}
  duplicatedTask = duplicated[0];
}} else {{
  duplicatedTask = new Task(task.name, insertionLocation);
  duplicatedTask.note = task.note;
  duplicatedTask.flagged = task.flagged;
  duplicatedTask.dueDate = task.dueDate;
  duplicatedTask.deferDate = task.deferDate;
  duplicatedTask.estimatedMinutes = task.estimatedMinutes;
  task.tags.forEach(tag => duplicatedTask.addTag(tag));
  try {{
    duplicatedTask.plannedDate = task.plannedDate;
  }} catch (e) {{
  }}
}}

{DUPLICATED_TASK_RESULT}"#
    );

    runner.run_omnijs(&script).await
}

/// The summary `duplicate_task` returns for the copy.
const DUPLICATED_TASK_RESULT: &str = r#"return {
  id: duplicatedTask.id.primaryKey,
  name: duplicatedTask.name,
  note: duplicatedTask.note,
  flagged: duplicatedTask.flagged,
  dueDate: duplicatedTask.dueDate ? duplicatedTask.dueDate.toISOString() : null,
  addedDate: duplicatedTask.added ? duplicatedTask.added.toISOString() : null,
  changedDate: duplicatedTask.modified ? duplicatedTask.modified.toISOString() : null,
  deferDate: duplicatedTask.deferDate ? duplicatedTask.deferDate.toISOString() : null,
  completed: duplicatedTask.completed,
  completionDate: duplicatedTask.completionDate ? duplicatedTask.completionDate.toISOString() : null,
  plannedDate: plannedDateValue(duplicatedTask),
  projectName: duplicatedTask.containingProject ? duplicatedTask.containingProject.name : null,
  inInbox: duplicatedTask.inInbox,
  tags: duplicatedTask.tags.map(tag => tag.name),
  estimatedMinutes: duplicatedTask.estimatedMinutes,
  hasChildren: duplicatedTask.hasChildren,
  sequential: duplicatedTask.sequential,
  taskStatus: taskStatusValue(duplicatedTask)
};"#;
