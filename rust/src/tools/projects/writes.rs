//! Creating and changing projects: create, move to a folder, set the
//! organisational status, and update fields.

use serde_json::Value;

use crate::{
    error::{OmniFocusError, Result},
    js_helpers::{
        JS_DATE_HELPERS, JS_PROJECT_STATUS, JS_RESOLVERS, JS_REVIEW_INTERVAL, JS_TASK_STATUS,
    },
    jxa::{escape_for_jxa, JxaRunner},
    review_interval::{parse_review_interval, ReviewInterval},
    tools::js_values::{js_bool_or_null, js_string_or_null, js_trimmed_string_or_null},
};

pub async fn create_project<R: JxaRunner>(
    runner: &R,
    name: &str,
    folder: Option<&str>,
    note: Option<&str>,
    due_date: Option<&str>,
    defer_date: Option<&str>,
    sequential: Option<bool>,
) -> Result<Value> {
    if name.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "name must not be empty.".to_string(),
        ));
    }
    if let Some(folder_name) = folder {
        if folder_name.trim().is_empty() {
            return Err(OmniFocusError::Validation(
                "folder must not be empty when provided.".to_string(),
            ));
        }
    }

    let project_name = escape_for_jxa(name.trim());
    let folder_name = js_trimmed_string_or_null(folder);
    let note_value = js_string_or_null(note);
    let due_date_value = js_string_or_null(due_date);
    let defer_date_value = js_string_or_null(defer_date);
    let sequential_value = js_bool_or_null(sequential);

    let script = format!(
        r#"{JS_DATE_HELPERS}
{JS_PROJECT_STATUS}
{JS_RESOLVERS}
const projectName = {project_name};
const folderName = {folder_name};
const noteValue = {note_value};
const dueDateValue = {due_date_value};
const deferDateValue = {defer_date_value};
const sequentialValue = {sequential_value};
const parsedDueDate = dueDateValue === null ? null : parseWriteDate(dueDateValue, "dueDate", "DefaultDueTime", "17:00");
const parsedDeferDate = deferDateValue === null ? null : parseWriteDate(deferDateValue, "deferDate", "DefaultStartTime", "00:00");

const project = (() => {{
  if (folderName === null) return new Project(projectName);
  const targetFolder = resolveFolder(folderName);
  return new Project(projectName, targetFolder.ending);
}})();

if (noteValue !== null) project.note = noteValue;
if (parsedDueDate !== null) project.dueDate = parsedDueDate;
if (parsedDeferDate !== null) project.deferDate = parsedDeferDate;
if (sequentialValue !== null) project.sequential = sequentialValue;

{CREATED_PROJECT_RESULT}"#
    );

    runner.run_omnijs(&script).await
}

/// The new project in the shape `get_project` returns: no tasks yet.
const CREATED_PROJECT_RESULT: &str = r#"return {
  id: project.id.primaryKey,
  name: project.name,
  status: normalizeProjectStatus(project),
  folderName: project.parentFolder ? project.parentFolder.name : null,
  taskCount: 0,
  remainingTaskCount: 0,
  completedTaskCount: 0,
  availableTaskCount: 0,
  deferDate: project.deferDate ? project.deferDate.toISOString() : null,
  dueDate: project.dueDate ? project.dueDate.toISOString() : null,
  completionDate: null,
  modified: project.modified ? project.modified.toISOString() : null,
  note: project.note,
  sequential: project.sequential,
  isStalled: false,
  nextTaskId: null,
  nextTaskName: null,
  reviewInterval: null,
  rootTasks: []
};"#;

pub async fn move_project<R: JxaRunner>(
    runner: &R,
    project_id_or_name: &str,
    folder: Option<&str>,
) -> Result<Value> {
    if project_id_or_name.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "project_id_or_name must not be empty.".to_string(),
        ));
    }
    if let Some(folder_name) = folder {
        if folder_name.trim().is_empty() {
            return Err(OmniFocusError::Validation(
                "folder must not be empty when provided.".to_string(),
            ));
        }
    }

    let project_filter = escape_for_jxa(project_id_or_name.trim());
    let folder_name = folder
        .map(|value| escape_for_jxa(value.trim()))
        .unwrap_or_else(|| "null".to_string());
    let script = format!(
        r#"{JS_RESOLVERS}
const projectFilter = {project_filter};
const folderName = {folder_name};
const project = document.flattenedProjects.find(item => {{
  return item.id.primaryKey === projectFilter || item.name === projectFilter;
}});
if (!project) {{
  throw new Error(`Project not found: ${{projectFilter}}`);
}}

const destination = (() => {{
  if (folderName === null) return library.ending;
  const targetFolder = resolveFolder(folderName);
  return targetFolder.ending;
}})();

moveSections([project], destination);

// Report where the project is now, not what was asked for.
return {{
  id: project.id.primaryKey,
  name: project.name,
  folderName: project.parentFolder ? project.parentFolder.name : null
}};"#
    );

    runner.run_omnijs(&script).await
}

pub async fn set_project_status<R: JxaRunner>(
    runner: &R,
    project_id_or_name: &str,
    status: &str,
) -> Result<Value> {
    if project_id_or_name.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "project_id_or_name must not be empty.".to_string(),
        ));
    }
    if !matches!(status, "active" | "on_hold" | "dropped") {
        return Err(OmniFocusError::Validation(
            "status must be one of: active, on_hold, dropped.".to_string(),
        ));
    }

    let project_filter = escape_for_jxa(project_id_or_name.trim());
    let status_value = escape_for_jxa(status);
    let script = format!(
        r#"const projectFilter = {project_filter};
const statusValue = {status_value};
const project = document.flattenedProjects.find(item => {{
  return item.id.primaryKey === projectFilter || item.name === projectFilter;
}});
if (!project) {{
  throw new Error(`Project not found: ${{projectFilter}}`);
}}

let targetStatus;
if (statusValue === "active") {{
  targetStatus = Project.Status.Active;
}} else if (statusValue === "on_hold") {{
  targetStatus = Project.Status.OnHold;
}} else if (statusValue === "dropped") {{
  targetStatus = Project.Status.Dropped;
}} else {{
  throw new Error(`Invalid status: ${{statusValue}}`);
}}

project.status = targetStatus;

return {{
  id: project.id.primaryKey,
  name: project.name,
  status: statusValue
}};"#
    );

    runner.run_omnijs(&script).await
}

#[allow(clippy::too_many_arguments)]
pub async fn update_project<R: JxaRunner>(
    runner: &R,
    project_id_or_name: &str,
    name: Option<&str>,
    note: Option<&str>,
    due_date: Option<&str>,
    defer_date: Option<&str>,
    flagged: Option<bool>,
    tags: Option<Vec<String>>,
    sequential: Option<bool>,
    completed_by_children: Option<bool>,
    review_interval: Option<&str>,
) -> Result<Value> {
    validate_project_update(project_id_or_name, name, tags.as_deref())?;
    let review_interval = review_interval.map(parse_review_interval).transpose()?;

    let updates = project_updates(
        name,
        note,
        due_date,
        defer_date,
        flagged,
        tags,
        sequential,
        completed_by_children,
        review_interval,
    )?;

    let project_filter = escape_for_jxa(project_id_or_name.trim());
    let updates_value = serde_json::to_string(&updates)?;
    let script = format!(
        r#"{JS_DATE_HELPERS}
{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_REVIEW_INTERVAL}
{JS_RESOLVERS}
const projectFilter = {project_filter};
const updates = {updates_value};
const project = document.flattenedProjects.find(item => {{
  return item.id.primaryKey === projectFilter || item.name === projectFilter;
}});
if (!project) {{
  throw new Error(`Project not found: ${{projectFilter}}`);
}}

const has = (key) => Object.prototype.hasOwnProperty.call(updates, key);
const parsedDueDate = has("dueDate") ? parseWriteDate(updates.dueDate, "dueDate", "DefaultDueTime", "17:00") : null;
const parsedDeferDate = has("deferDate") ? parseWriteDate(updates.deferDate, "deferDate", "DefaultStartTime", "00:00") : null;
const resolvedTags = has("tags") ? updates.tags.map(tagName => resolveTag(tagName)) : null;
// Prepared before any field changes, so a project without an interval fails
// the call with nothing modified.
const preparedReviewInterval = has("reviewInterval") ? updatedReviewInterval(project, updates.reviewInterval) : null;

if (has("name")) project.name = updates.name;
if (has("note")) project.note = updates.note;
if (has("dueDate")) project.dueDate = parsedDueDate;
if (has("deferDate")) project.deferDate = parsedDeferDate;
if (has("flagged")) project.flagged = updates.flagged;
if (has("sequential")) project.sequential = updates.sequential;
if (has("completedByChildren")) project.completedByChildren = updates.completedByChildren;
if (has("reviewInterval")) project.reviewInterval = preparedReviewInterval;
if (has("tags")) {{
  const existingTags = project.tags.slice();
  existingTags.forEach(tag => {{
    project.removeTag(tag);
  }});
  resolvedTags.forEach(tag => {{
    project.addTag(tag);
  }});
}}

{UPDATED_PROJECT_RESULT}"#
    );
    runner.run_omnijs(&script).await
}

/// Rejects an empty project id or name, an empty new name, and an empty
/// tag, in that order.
fn validate_project_update(
    project_id_or_name: &str,
    name: Option<&str>,
    tags: Option<&[String]>,
) -> Result<()> {
    if project_id_or_name.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "project_id_or_name must not be empty.".to_string(),
        ));
    }
    if let Some(value) = name {
        if value.trim().is_empty() {
            return Err(OmniFocusError::Validation(
                "name must not be empty when provided.".to_string(),
            ));
        }
    }
    if let Some(values) = &tags {
        if values.iter().any(|value| value.trim().is_empty()) {
            return Err(OmniFocusError::Validation(
                "tags must not contain empty values.".to_string(),
            ));
        }
    }
    Ok(())
}

/// The fields `update_project` was given, under the keys its script reads;
/// an absent field is left out, so the script leaves it unchanged.
#[allow(clippy::too_many_arguments)]
fn project_updates(
    name: Option<&str>,
    note: Option<&str>,
    due_date: Option<&str>,
    defer_date: Option<&str>,
    flagged: Option<bool>,
    tags: Option<Vec<String>>,
    sequential: Option<bool>,
    completed_by_children: Option<bool>,
    review_interval: Option<ReviewInterval>,
) -> Result<serde_json::Map<String, Value>> {
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
    if let Some(value) = flagged {
        updates.insert("flagged".to_string(), Value::Bool(value));
    }
    if let Some(values) = tags {
        updates.insert(
            "tags".to_string(),
            Value::Array(
                values
                    .into_iter()
                    .map(|value| Value::String(value.trim().to_string()))
                    .collect(),
            ),
        );
    }
    if let Some(value) = sequential {
        updates.insert("sequential".to_string(), Value::Bool(value));
    }
    if let Some(value) = completed_by_children {
        updates.insert("completedByChildren".to_string(), Value::Bool(value));
    }
    if let Some(value) = review_interval {
        updates.insert("reviewInterval".to_string(), serde_json::to_value(value)?);
    }
    Ok(updates)
}

/// The summary `update_project` returns, read after every change is applied.
const UPDATED_PROJECT_RESULT: &str = r#"const allProjectTasks = document.flattenedTasks.filter(task => {
  return !isProjectRootTask(task)
    && task.containingProject
    && task.containingProject.id.primaryKey === project.id.primaryKey;
});
return {
  id: project.id.primaryKey,
  name: project.name,
  status: normalizeProjectStatus(project),
  folderName: project.parentFolder ? project.parentFolder.name : null,
  taskCount: allProjectTasks.length,
  remainingTaskCount: allProjectTasks.filter(task => isTaskRemaining(task)).length,
  deferDate: project.deferDate ? project.deferDate.toISOString() : null,
  dueDate: project.dueDate ? project.dueDate.toISOString() : null,
  note: project.note,
  flagged: project.flagged,
  sequential: project.sequential,
  completedByChildren: project.completedByChildren,
  tags: project.tags.map(tag => tag.name),
  reviewInterval: formatReviewInterval(project.reviewInterval)
};"#;
