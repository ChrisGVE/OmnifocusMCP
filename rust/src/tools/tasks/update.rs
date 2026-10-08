//! Changing an existing task: its fields (`update_task`) and its
//! repetition rule (`set_task_repetition`).

use serde_json::Value;

use crate::{
    error::{OmniFocusError, Result},
    js_helpers::{
        JS_DATE_HELPERS, JS_PLANNED_DATE, JS_PROJECT_STATUS, JS_RESOLVERS, JS_TASK_STATUS,
    },
    jxa::{escape_for_jxa, JxaRunner},
};

#[allow(clippy::too_many_arguments)]
pub async fn update_task<R: JxaRunner>(
    runner: &R,
    task_id: &str,
    name: Option<&str>,
    note: Option<&str>,
    due_date: Option<&str>,
    defer_date: Option<&str>,
    planned_date: Option<&str>,
    flagged: Option<bool>,
    tags: Option<Vec<String>>,
    estimated_minutes: Option<i32>,
) -> Result<Value> {
    if task_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "task_id must not be empty.".to_string(),
        ));
    }
    if let Some(value) = name {
        if value.trim().is_empty() {
            return Err(OmniFocusError::Validation(
                "name must not be empty when provided.".to_string(),
            ));
        }
    }

    let updates = task_updates(
        name,
        note,
        due_date,
        defer_date,
        planned_date,
        flagged,
        tags,
        estimated_minutes,
    );

    let task_id_value = escape_for_jxa(task_id.trim());
    let updates_value = serde_json::to_string(&updates)?;

    let script = format!(
        r#"{JS_DATE_HELPERS}
{JS_PLANNED_DATE}
{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_RESOLVERS}
const taskId = {task_id_value};
const updates = {updates_value};
const task = document.flattenedTasks.find(item => item.id.primaryKey === taskId && !isProjectRootTask(item));
if (!task) {{
  throw new Error(`Task not found: ${{taskId}}`);
}}

{APPLY_TASK_UPDATES}

{UPDATED_TASK_RESULT}"#
    );
    runner.run_omnijs(&script).await
}

/// The fields `update_task` was given, under the keys its script reads; an
/// absent field is left out, so the script leaves it unchanged.
#[allow(clippy::too_many_arguments)]
fn task_updates(
    name: Option<&str>,
    note: Option<&str>,
    due_date: Option<&str>,
    defer_date: Option<&str>,
    planned_date: Option<&str>,
    flagged: Option<bool>,
    tags: Option<Vec<String>>,
    estimated_minutes: Option<i32>,
) -> serde_json::Map<String, Value> {
    let mut updates = serde_json::Map::new();
    if let Some(value) = name {
        updates.insert("name".to_string(), Value::String(value.trim().to_string()));
    }
    if let Some(value) = note {
        updates.insert("note".to_string(), Value::String(value.to_string()));
    }
    if let Some(value) = due_date {
        updates.insert("dueDate".to_string(), Value::String(value.to_string()));
    }
    if let Some(value) = defer_date {
        updates.insert("deferDate".to_string(), Value::String(value.to_string()));
    }
    if let Some(value) = planned_date {
        updates.insert("plannedDate".to_string(), Value::String(value.to_string()));
    }
    if let Some(value) = flagged {
        updates.insert("flagged".to_string(), Value::Bool(value));
    }
    if let Some(value) = tags {
        updates.insert(
            "tags".to_string(),
            Value::Array(value.into_iter().map(Value::String).collect()),
        );
    }
    if let Some(value) = estimated_minutes {
        updates.insert("estimatedMinutes".to_string(), Value::from(value));
    }
    updates
}

/// Parses the given dates and resolves the given tags before changing
/// anything, then applies each given field; `tags` replaces the task's tags.
const APPLY_TASK_UPDATES: &str = r#"const has = (key) => Object.prototype.hasOwnProperty.call(updates, key);
const supportsPlannedDate = detectPlannedDateSupport(document.flattenedTasks);
const parsedDueDate = has("dueDate") ? parseWriteDate(updates.dueDate, "dueDate", "DefaultDueTime", "17:00") : null;
const parsedDeferDate = has("deferDate") ? parseWriteDate(updates.deferDate, "deferDate", "DefaultStartTime", "00:00") : null;
const parsedPlannedDate = has("plannedDate") ? parseWriteDate(updates.plannedDate, "plannedDate", "DefaultPlannedTime", "09:00") : null;
const resolvedTags = has("tags") ? updates.tags.map(tagName => resolveTag(tagName)) : null;

if (parsedPlannedDate !== null && !supportsPlannedDate) {
  throw new Error("plannedDate requires an OmniFocus database migrated to support planned dates");
}

if (has("name")) task.name = updates.name;
if (has("note")) task.note = updates.note;
if (has("dueDate")) task.dueDate = parsedDueDate;
if (has("deferDate")) task.deferDate = parsedDeferDate;
if (has("plannedDate")) setPlannedDate(task, parsedPlannedDate);
if (has("flagged")) task.flagged = updates.flagged;
if (has("estimatedMinutes")) task.estimatedMinutes = updates.estimatedMinutes;

if (has("tags")) {
  const existingTags = task.tags.slice();
  existingTags.forEach(tag => {
    task.removeTag(tag);
  });
  resolvedTags.forEach(tag => task.addTag(tag));
}"#;

/// The summary `update_task` returns, read after every change is applied.
const UPDATED_TASK_RESULT: &str = r#"const plannedDate = readPlannedDate(task, supportsPlannedDate);
return {
  id: task.id.primaryKey,
  name: task.name,
  note: task.note,
  flagged: task.flagged,
  dueDate: task.dueDate ? task.dueDate.toISOString() : null,
  addedDate: task.added ? task.added.toISOString() : null,
  changedDate: task.modified ? task.modified.toISOString() : null,
  deferDate: task.deferDate ? task.deferDate.toISOString() : null,
  plannedDate: plannedDate ? plannedDate.toISOString() : null,
  effectiveDueDate: task.effectiveDueDate ? task.effectiveDueDate.toISOString() : null,
  effectiveDeferDate: task.effectiveDeferDate ? task.effectiveDeferDate.toISOString() : null,
  effectiveFlagged: task.effectiveFlagged,
  completed: isTaskCompleted(task),
  projectName: task.containingProject ? task.containingProject.name : null,
  inInbox: task.inInbox,
  tags: task.tags.map(tag => tag.name),
  sequential: task.sequential,
  estimatedMinutes: task.estimatedMinutes
};"#;

pub async fn set_task_repetition<R: JxaRunner>(
    runner: &R,
    task_id: &str,
    rule_string: Option<&str>,
    schedule_type: &str,
) -> Result<Value> {
    if task_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "task_id must not be empty.".to_string(),
        ));
    }
    if let Some(value) = rule_string {
        if value.trim().is_empty() {
            return Err(OmniFocusError::Validation(
                "rule_string must not be empty when provided.".to_string(),
            ));
        }
    }
    if !matches!(schedule_type, "regularly" | "from_completion" | "none") {
        return Err(OmniFocusError::Validation(
            "schedule_type must be one of: regularly, from_completion, none.".to_string(),
        ));
    }

    let task_id_value = escape_for_jxa(task_id.trim());
    let rule_string_value = rule_string
        .map(|value| escape_for_jxa(value.trim()))
        .unwrap_or_else(|| "null".to_string());
    let schedule_type_value = escape_for_jxa(schedule_type);
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
const taskId = {task_id_value};
const ruleString = {rule_string_value};
const scheduleTypeInput = {schedule_type_value};
const task = document.flattenedTasks.find(item => item.id.primaryKey === taskId && !isProjectRootTask(item));
if (!task) {{
  throw new Error(`Task not found: ${{taskId}}`);
}}

if (ruleString === null) {{
  task.repetitionRule = null;
}} else {{
  const scheduleType = (() => {{
    if (scheduleTypeInput === "regularly") return Task.RepetitionScheduleType.Regularly;
    if (scheduleTypeInput === "from_completion") return Task.RepetitionScheduleType.FromCompletion;
    if (scheduleTypeInput === "none") return Task.RepetitionScheduleType.None;
    throw new Error(`Invalid schedule_type: ${{scheduleTypeInput}}`);
  }})();
  task.repetitionRule = new Task.RepetitionRule(ruleString, null, scheduleType, null, false);
}}

return {{
  id: task.id.primaryKey,
  name: task.name,
  repetitionRule: task.repetitionRule ? task.repetitionRule.ruleString : null
}};"#
    );
    runner.run_omnijs(&script).await
}
