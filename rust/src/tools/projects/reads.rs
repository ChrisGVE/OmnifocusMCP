//! Project reads other than the filtered list: a name search, aggregate
//! counts by status, and one project's full details.

use serde_json::Value;

use crate::{
    error::{OmniFocusError, Result},
    js_helpers::{JS_PROJECT_STATUS, JS_RESOLVERS, JS_REVIEW_INTERVAL, JS_TASK_STATUS},
    jxa::{escape_for_jxa, JxaRunner},
    types::ProjectCountsResult,
};

pub async fn search_projects<R: JxaRunner>(runner: &R, query: &str, limit: i32) -> Result<Value> {
    if query.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "query must not be empty.".to_string(),
        ));
    }
    if limit < 1 {
        return Err(OmniFocusError::Validation(
            "limit must be greater than 0.".to_string(),
        ));
    }

    let query_value = escape_for_jxa(query.trim());
    let script = format!(
        r#"{JS_PROJECT_STATUS}
const queryValue = {query_value};

return projectsMatching(queryValue)
  .slice(0, {limit})
  .map(project => {{
    return {{
      id: project.id.primaryKey,
      name: project.name,
      status: normalizeProjectStatus(project),
      folderName: project.parentFolder ? project.parentFolder.name : null
    }};
  }});"#
    );

    runner.run_omnijs(&script).await
}

pub async fn get_project_counts<R: JxaRunner>(
    runner: &R,
    folder: Option<&str>,
) -> Result<ProjectCountsResult> {
    if let Some(folder_name) = folder {
        if folder_name.trim().is_empty() {
            return Err(OmniFocusError::Validation(
                "folder must not be empty when provided.".to_string(),
            ));
        }
    }

    let folder_filter = folder
        .map(|value| escape_for_jxa(value.trim()))
        .unwrap_or_else(|| "null".to_string());
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_RESOLVERS}
const folderFilter = {folder_filter};
const filterFolder = folderFilter === null ? null : resolveFolder(folderFilter);

const counts = {{
  total: 0,
  active: 0,
  onHold: 0,
  completed: 0,
  dropped: 0,
  stalled: 0
}};
const now = new Date();

document.flattenedProjects.forEach(project => {{
  if (filterFolder !== null) {{
    const parent = project.parentFolder;
    if (!parent || parent.id.primaryKey !== filterFolder.id.primaryKey) return;
  }}

  const status = normalizeProjectStatus(project);
  const isStalled = isProjectStalled(project, now);

  counts.total += 1;
  if (status === "active") counts.active += 1;
  if (status === "on_hold") counts.onHold += 1;
  if (status === "completed") counts.completed += 1;
  if (status === "dropped") counts.dropped += 1;
  if (isStalled) counts.stalled += 1;
}});

return counts;"#
    );

    let value = runner.run_omnijs(&script).await?;
    Ok(serde_json::from_value(value)?)
}

pub async fn get_project<R: JxaRunner>(runner: &R, project_id_or_name: &str) -> Result<Value> {
    if project_id_or_name.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "project_id_or_name must not be empty.".to_string(),
        ));
    }

    let project_filter = escape_for_jxa(project_id_or_name.trim());
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_REVIEW_INTERVAL}
const projectFilter = {project_filter};
const project = document.flattenedProjects.find(item => {{
  return item.id.primaryKey === projectFilter || item.name === projectFilter;
}});
if (!project) {{
  throw new Error(`Project not found: ${{projectFilter}}`);
}}

const allProjectTasks = document.flattenedTasks.filter(task => {{
  return !isProjectRootTask(task)
    && task.containingProject
    && task.containingProject.id.primaryKey === project.id.primaryKey;
}});
const now = new Date();
const nextTask = project.nextTask;
const isStalled = isProjectStalled(project, now);

const rootTasks = project.tasks.map(task => {{
  return {{
    id: task.id.primaryKey,
    name: task.name,
    note: task.note,
    flagged: task.flagged,
    dueDate: task.dueDate ? task.dueDate.toISOString() : null,
    deferDate: task.deferDate ? task.deferDate.toISOString() : null,
    completed: task.completed,
    tags: task.tags.map(tag => tag.name),
    inInbox: task.inInbox,
    sequential: task.sequential,
    estimatedMinutes: task.estimatedMinutes
  }};
}});

return {{
  id: project.id.primaryKey,
  name: project.name,
  status: normalizeProjectStatus(project),
  folderName: project.parentFolder ? project.parentFolder.name : null,
  taskCount: allProjectTasks.length,
  remainingTaskCount: allProjectTasks.filter(task => isTaskRemaining(task)).length,
  completedTaskCount: allProjectTasks.filter(task => isTaskCompleted(task)).length,
  availableTaskCount: allProjectTasks.filter(task => isTaskAvailable(task, now)).length,
  deferDate: project.deferDate ? project.deferDate.toISOString() : null,
  dueDate: project.dueDate ? project.dueDate.toISOString() : null,
  completionDate: project.completionDate ? project.completionDate.toISOString() : null,
  modified: project.modified ? project.modified.toISOString() : null,
  note: project.note,
  sequential: project.sequential,
  isStalled: isStalled,
  nextTaskId: nextTask ? nextTask.id.primaryKey : null,
  nextTaskName: nextTask ? nextTask.name : null,
  reviewInterval: formatReviewInterval(project.reviewInterval),
  rootTasks: rootTasks
}};"#
    );

    runner.run_omnijs(&script).await
}
