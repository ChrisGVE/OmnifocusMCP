//! The batch task tools: create, delete or move many tasks in one OmniJS
//! call.
//!
//! Each script checks every entry before it changes anything where it can
//! (destinations, tags and dates for create; the destination for move), so
//! one bad entry does not leave a half-applied batch.

use std::collections::HashSet;

use serde_json::Value;

use crate::{
    error::{to_json_string, OmniFocusError, Result},
    js_helpers::{
        JS_DATE_HELPERS, JS_PLANNED_DATE, JS_PROJECT_STATUS, JS_RESOLVERS, JS_TASK_STATUS,
    },
    jxa::JxaRunner,
    tools::js_values::{js_string_or_null, js_trimmed_string_or_null},
};

use super::{
    create::CreateTaskInput, inputs::validate_non_negative_minutes, moves::validate_destination,
};

pub async fn create_tasks_batch<R: JxaRunner>(
    runner: &R,
    tasks: Vec<CreateTaskInput>,
) -> Result<Value> {
    if tasks.is_empty() {
        return Err(OmniFocusError::Validation(
            "tasks must contain at least one task definition.".to_string(),
        ));
    }
    for (index, task) in tasks.iter().enumerate() {
        if task.name.trim().is_empty() {
            return Err(OmniFocusError::Validation(
                "each task must include a non-empty name.".to_string(),
            ));
        }
        if let Some(project) = &task.project {
            if project.trim().is_empty() {
                return Err(OmniFocusError::Validation(
                    "project must not be empty when provided.".to_string(),
                ));
            }
        }
        validate_non_negative_minutes(
            &format!("tasks[{index}].estimatedMinutes"),
            task.estimated_minutes,
        )?;
    }

    let normalized: Vec<CreateTaskInput> = tasks
        .into_iter()
        .map(|task| CreateTaskInput {
            name: task.name.trim().to_string(),
            project: task.project.map(|project| project.trim().to_string()),
            note: task.note,
            due_date: task.due_date,
            defer_date: task.defer_date,
            planned_date: task.planned_date,
            flagged: task.flagged,
            tags: task.tags,
            estimated_minutes: task.estimated_minutes,
        })
        .collect();

    let tasks_value = to_json_string(&normalized)?;
    let script = format!(
        r#"{JS_DATE_HELPERS}
{JS_PLANNED_DATE}
{JS_RESOLVERS}
const taskInputs = {tasks_value};

{PREPARE_BATCH_TASKS}

{CREATE_BATCH_TASKS}"#
    );

    runner.run_omnijs(&script).await
}

/// Resolves every destination and tag and parses every date, naming the
/// failing entry (`tasks[i]...`) in any error.
const PREPARE_BATCH_TASKS: &str = r#"const resolveParent = (projectName, index) => {
  if (projectName === null || projectName === "") return inbox.ending;
  try {
    const targetProject = resolveProject(projectName);
    return targetProject.ending;
  } catch (error) {
    throw new Error("tasks[" + index + "].project: " + error.message);
  }
};

// Resolve every destination before creating any task, so one unknown project
// cannot leave part of the batch created.
const parents = taskInputs.map((input, index) => resolveParent(input.project, index));

// Resolve every tag before creating any task. Include both the task and tag
// positions in an error so callers can identify the bad batch entry.
const resolvedTags = taskInputs.map((input, index) => {
  if (input.tags === null || input.tags === undefined) return [];
  return input.tags.map((tagName, tagIndex) => {
    try {
      return resolveTag(tagName);
    } catch (error) {
      throw new Error("tasks[" + index + "].tags[" + tagIndex + "]: " + error.message);
    }
  });
});

const supportsPlannedDate = detectPlannedDateSupport(document.flattenedTasks);

const isPresent = (value) => value !== null && value !== undefined;

// Parse every date before creating any task, so one bad date cannot leave
// part of the batch created.
const parsedDates = taskInputs.map((input, index) => ({
  dueDate: isPresent(input.dueDate)
    ? parseWriteDate(input.dueDate, "tasks[" + index + "].dueDate", "DefaultDueTime", "17:00")
    : null,
  deferDate: isPresent(input.deferDate)
    ? parseWriteDate(input.deferDate, "tasks[" + index + "].deferDate", "DefaultStartTime", "00:00")
    : null,
  plannedDate: isPresent(input.plannedDate)
    ? parseWriteDate(input.plannedDate, "tasks[" + index + "].plannedDate", "DefaultPlannedTime", "09:00")
    : null
}));

if (parsedDates.some(dates => dates.plannedDate !== null) && !supportsPlannedDate) {
  throw new Error("plannedDate requires an OmniFocus database migrated to support planned dates");
}"#;

/// Creates each task from its prepared destination, dates and tags.
const CREATE_BATCH_TASKS: &str = r#"const created = taskInputs.map((input, index) => {
  const task = new Task(input.name, parents[index]);
  const dates = parsedDates[index];

  if (input.note !== null && input.note !== undefined) task.note = input.note;
  if (dates.dueDate !== null) task.dueDate = dates.dueDate;
  if (dates.deferDate !== null) task.deferDate = dates.deferDate;
  if (dates.plannedDate !== null) setPlannedDate(task, dates.plannedDate);
  if (input.flagged !== null && input.flagged !== undefined) task.flagged = input.flagged;
  if (input.estimatedMinutes !== null && input.estimatedMinutes !== undefined) {
    task.estimatedMinutes = input.estimatedMinutes;
  }

  resolvedTags[index].forEach(tag => task.addTag(tag));

  const plannedDate = readPlannedDate(task, supportsPlannedDate);
  return {
    id: task.id.primaryKey,
    name: task.name,
    plannedDate: plannedDate ? plannedDate.toISOString() : null
  };
});

return created;"#;

pub async fn delete_tasks_batch<R: JxaRunner>(runner: &R, task_ids: Vec<String>) -> Result<Value> {
    if task_ids.is_empty() {
        return Err(OmniFocusError::Validation(
            "task_ids must contain at least one task id.".to_string(),
        ));
    }

    let mut normalized_task_ids: Vec<String> = Vec::with_capacity(task_ids.len());
    let mut seen_task_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for task_id in task_ids {
        let normalized_task_id = task_id.trim();
        if normalized_task_id.is_empty() {
            return Err(OmniFocusError::Validation(
                "each task id must be a non-empty string.".to_string(),
            ));
        }
        if seen_task_ids.contains(normalized_task_id) {
            return Err(OmniFocusError::Validation(
                "task_ids must not contain duplicate ids.".to_string(),
            ));
        }
        seen_task_ids.insert(normalized_task_id.to_string());
        normalized_task_ids.push(normalized_task_id.to_string());
    }
    let task_ids_value = to_json_string(&normalized_task_ids)?;
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
const taskIds = {task_ids_value};
const taskById = new Map();
for (const task of document.flattenedTasks) {{
  if (isProjectRootTask(task)) continue;
  try {{
    taskById.set(task.id.primaryKey, task);
  }} catch (e) {{
  }}
}}
const results = taskIds.map(taskId => {{
  const task = taskById.get(taskId);
  if (!task) {{
    return {{
      id: taskId,
      deleted: false,
      error: "not found"
    }};
  }}

  const taskName = task.name;
  deleteObject(task);
  return {{
    id: taskId,
    name: taskName,
    deleted: true
  }};
}});

const deletedCount = results.filter(result => result.deleted).length;
const notFoundCount = results.length - deletedCount;

return {{
  deleted_count: deletedCount,
  not_found_count: notFoundCount,
  results: results
}};"#
    );
    runner.run_omnijs(&script).await
}

pub async fn move_tasks_batch<R: JxaRunner>(
    runner: &R,
    task_ids: Vec<String>,
    project: Option<&str>,
    parent_task_id: Option<&str>,
) -> Result<Value> {
    if task_ids.is_empty() {
        return Err(OmniFocusError::Validation(
            "task_ids must contain at least one task id.".to_string(),
        ));
    }
    validate_destination(project, parent_task_id)?;

    let (normalized_task_ids, seen_task_ids) = normalize_move_task_ids(task_ids)?;
    let normalized_parent_task_id = parent_task_id.map(str::trim);
    if let Some(parent_id) = normalized_parent_task_id {
        if seen_task_ids.contains(parent_id) {
            return Err(OmniFocusError::Validation(
                "parent_task_id must not be included in task_ids (cannot move a task under itself)."
                    .to_string(),
            ));
        }
    }

    let task_ids_value = to_json_string(&normalized_task_ids)?;
    let project_value = js_trimmed_string_or_null(project);
    let parent_task_id_value = js_string_or_null(normalized_parent_task_id);
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_RESOLVERS}
const taskIds = {task_ids_value};
const projectName = {project_value};
const parentTaskId = {parent_task_id_value};
{RESOLVE_BATCH_DESTINATION}

{MOVE_BATCH_TASKS}"#
    );
    runner.run_omnijs(&script).await
}

/// Trims the ids and rejects an empty or repeated one. Also returns the
/// set of ids, which the caller checks the parent task against.
fn normalize_move_task_ids(task_ids: Vec<String>) -> Result<(Vec<String>, HashSet<String>)> {
    let mut normalized_task_ids: Vec<String> = Vec::with_capacity(task_ids.len());
    let mut seen_task_ids: HashSet<String> = HashSet::with_capacity(task_ids.len());
    for task_id in task_ids {
        let normalized_task_id = task_id.trim();
        if normalized_task_id.is_empty() {
            return Err(OmniFocusError::Validation(
                "each task id must be a non-empty string.".to_string(),
            ));
        }
        if !seen_task_ids.insert(normalized_task_id.to_string()) {
            return Err(OmniFocusError::Validation(
                "task_ids must not contain duplicate ids.".to_string(),
            ));
        }
        normalized_task_ids.push(normalized_task_id.to_string());
    }
    Ok((normalized_task_ids, seen_task_ids))
}

/// Indexes the tasks by id and resolves the destination, refusing a parent
/// that is one of the moved tasks or below one of them.
const RESOLVE_BATCH_DESTINATION: &str = r#"const taskById = new Map();
for (const task of document.flattenedTasks) {
  if (isProjectRootTask(task)) continue;
  try {
    taskById.set(task.id.primaryKey, task);
  } catch (e) {
  }
}

const destinationInfo = (() => {
  if (parentTaskId !== null && parentTaskId !== "") {
    const parentTask = resolveTask(parentTaskId, "Parent task");
    // Climb from the destination through `parent`; a top-level task's parent
    // is its project's root task, which is no task's descendant, so stop there.
    let ancestor = parentTask;
    while (ancestor && !isProjectRootTask(ancestor)) {
      if (taskIds.includes(ancestor.id.primaryKey)) {
        throw new Error("Cannot move tasks under their own descendant.");
      }
      ancestor = ancestor.parent;
    }
    return {
      mode: "parent",
      location: parentTask.ending,
      summary: {
        mode: "parent",
        parentTaskId: parentTask.id.primaryKey,
        parentTaskName: parentTask.name
      }
    };
  }
  if (projectName === null || projectName === "") {
    return { mode: "inbox", location: inbox.ending, summary: { mode: "inbox" } };
  }
  const targetProject = resolveProject(projectName);
  return {
    mode: "project",
    location: targetProject.ending,
    summary: { mode: "project", projectName: targetProject.name }
  };
})();"#;

/// Moves the tasks that exist, checks each kept its identity and left its
/// old parent, and reports one result per requested id.
const MOVE_BATCH_TASKS: &str = r#"const existingTasksById = new Map();
for (const taskId of taskIds) {
  const task = taskById.get(taskId);
  if (task) {
    existingTasksById.set(taskId, task);
  }
}

const movableTasks = Array.from(existingTasksById.values());
if (movableTasks.length > 0) {
  const originalTaskIds = new Map();
  for (const [taskId, task] of existingTasksById.entries()) {
    originalTaskIds.set(taskId, task.id.primaryKey);
  }
  moveTasks(movableTasks, destinationInfo.location);
  for (const [taskId, task] of existingTasksById.entries()) {
    if (task.id.primaryKey !== originalTaskIds.get(taskId)) {
      throw new Error("Task move did not preserve task identity.");
    }
    const newParent = task.parent;
    if (destinationInfo.mode !== "parent" && newParent && !isProjectRootTask(newParent)) {
      throw new Error("Task move failed: task is still nested under a parent.");
    }
  }
}

const results = taskIds.map(taskId => {
  const task = existingTasksById.get(taskId);
  if (!task) {
    return {
      id: taskId,
      name: null,
      moved: false,
      destination: destinationInfo.summary,
      error: "Task not found."
    };
  }
  return {
    id: task.id.primaryKey,
    name: task.name,
    moved: true,
    destination: destinationInfo.summary,
    error: null
  };
});

const movedCount = results.filter(result => result.moved).length;
const failedCount = results.length - movedCount;

return {
  requested_count: taskIds.length,
  moved_count: movedCount,
  failed_count: failedCount,
  partial_success: movedCount > 0 && failedCount > 0,
  results: results
};"#;
