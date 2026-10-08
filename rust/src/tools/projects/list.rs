//! `list_projects`: the projects with one status, optionally only those in
//! one folder, completed within a date range, or stalled, sorted and limited.

use serde_json::Value;

use crate::{
    error::{OmniFocusError, Result},
    js_helpers::{
        JS_DATE_HELPERS, JS_PROJECT_STATUS, JS_RESOLVERS, JS_REVIEW_INTERVAL, JS_TASK_STATUS,
    },
    jxa::{escape_for_jxa, JxaRunner},
    tools::js_values::{js_string_or_null, js_trimmed_string_or_null},
};

#[allow(clippy::too_many_arguments)]
pub async fn list_projects<R: JxaRunner>(
    runner: &R,
    folder: Option<&str>,
    status: &str,
    completed_before: Option<&str>,
    completed_after: Option<&str>,
    stalled_only: bool,
    sort_by: Option<&str>,
    sort_order: &str,
    limit: i32,
) -> Result<Value> {
    validate_list_projects(folder, status, sort_by, sort_order, limit)?;

    let mut effective_status = status;
    if completed_before.is_some() || completed_after.is_some() {
        effective_status = "completed";
    }
    if stalled_only {
        effective_status = "active";
    }

    let mut effective_sort_by = sort_by;
    let mut effective_sort_order = sort_order;
    if (completed_before.is_some() || completed_after.is_some()) && effective_sort_by.is_none() {
        effective_sort_by = Some("completionDate");
        effective_sort_order = "desc";
    }

    let folder_filter = js_trimmed_string_or_null(folder);
    let status_filter = escape_for_jxa(effective_status);
    let completed_before_filter = js_string_or_null(completed_before);
    let completed_after_filter = js_string_or_null(completed_after);
    let stalled_only_filter = if stalled_only { "true" } else { "false" };
    let sort_by_filter = js_string_or_null(effective_sort_by);
    let sort_order_filter = escape_for_jxa(effective_sort_order);
    let script = format!(
        r#"{JS_DATE_HELPERS}
{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_REVIEW_INTERVAL}
{JS_RESOLVERS}
const folderFilter = {folder_filter};
const filterFolder = folderFilter === null ? null : resolveFolder(folderFilter);
const statusFilter = {status_filter};
const completedBeforeRaw = {completed_before_filter};
const completedAfterRaw = {completed_after_filter};
const stalledOnly = {stalled_only_filter};
const sortBy = {sort_by_filter};
const sortOrder = {sort_order_filter};

{SELECT_AND_MAP_PROJECTS}

{SORT_PROJECTS}

return sortedProjects.slice(0, {limit});"#
    );

    runner.run_omnijs(&script).await
}

/// Rejects an empty folder, an unknown status, sort field or sort order,
/// and a limit below 1, in that order.
fn validate_list_projects(
    folder: Option<&str>,
    status: &str,
    sort_by: Option<&str>,
    sort_order: &str,
    limit: i32,
) -> Result<()> {
    if limit < 1 {
        return Err(OmniFocusError::Validation(
            "limit must be greater than 0.".to_string(),
        ));
    }
    if let Some(folder_name) = folder {
        if folder_name.trim().is_empty() {
            return Err(OmniFocusError::Validation(
                "folder must not be empty when provided.".to_string(),
            ));
        }
    }
    if !matches!(status, "active" | "on_hold" | "completed" | "dropped") {
        return Err(OmniFocusError::Validation(
            "status must be one of: active, on_hold, completed, dropped.".to_string(),
        ));
    }
    if let Some(sort_field) = sort_by {
        if !matches!(
            sort_field,
            "name" | "dueDate" | "completionDate" | "taskCount"
        ) {
            return Err(OmniFocusError::Validation(
                "sortBy must be one of: name, dueDate, completionDate, taskCount.".to_string(),
            ));
        }
    }
    if !matches!(sort_order, "asc" | "desc") {
        return Err(OmniFocusError::Validation(
            "sortOrder must be one of: asc, desc.".to_string(),
        ));
    }
    Ok(())
}

/// Counts each project's tasks, keeps the projects that pass the folder,
/// status, completion date and stalled filters, and maps them to summaries.
const SELECT_AND_MAP_PROJECTS: &str = r#"const completedBefore = parseOptionalLocalDate(completedBeforeRaw, "completedBefore");
const completedAfter = parseOptionalLocalDate(completedAfterRaw, "completedAfter");
const now = new Date();

const projectCounts = new Map();
document.flattenedTasks.forEach(task => {
  if (isProjectRootTask(task)) return;
  const project = task.containingProject;
  if (!project) return;
  const projectId = project.id.primaryKey;
  const current = projectCounts.get(projectId) || { taskCount: 0, remainingTaskCount: 0 };
  current.taskCount += 1;
  if (isTaskRemaining(task)) current.remainingTaskCount += 1;
  projectCounts.set(projectId, current);
});

const projects = document.flattenedProjects
  .filter(project => {
    const isStalled = isProjectStalled(project, now);
    if (filterFolder !== null) {
      const parent = project.parentFolder;
      if (!parent || parent.id.primaryKey !== filterFolder.id.primaryKey) return false;
    }
    if (normalizeProjectStatus(project) !== statusFilter) return false;
    if (completedBefore !== null && !(project.completionDate !== null && project.completionDate < completedBefore)) return false;
    if (completedAfter !== null && !(project.completionDate !== null && project.completionDate > completedAfter)) return false;
    if (stalledOnly && !isStalled) return false;
    return true;
  });

const mappedProjects = projects.map(project => {
  const projectId = project.id.primaryKey;
  const counts = projectCounts.get(projectId) || { taskCount: 0, remainingTaskCount: 0 };
  const nextTask = project.nextTask;
  const isStalled = isProjectStalled(project, now);
  return {
    id: projectId,
    name: project.name,
    status: normalizeProjectStatus(project),
    folderName: project.parentFolder ? project.parentFolder.name : null,
    taskCount: counts.taskCount,
    remainingTaskCount: counts.remainingTaskCount,
    deferDate: project.deferDate ? project.deferDate.toISOString() : null,
    dueDate: project.dueDate ? project.dueDate.toISOString() : null,
    completionDate: project.completionDate ? project.completionDate.toISOString() : null,
    note: project.note,
    sequential: project.sequential,
    isStalled: isStalled,
    nextTaskId: nextTask ? nextTask.id.primaryKey : null,
    nextTaskName: nextTask ? nextTask.name : null,
    reviewInterval: formatReviewInterval(project.reviewInterval)
  };
});"#;

/// Sorts the summaries by `sortBy` in `sortOrder`; a missing value sorts last.
const SORT_PROJECTS: &str = r#"const compareValues = (left, right) => {
  if (left < right) return sortOrder === "asc" ? -1 : 1;
  if (left > right) return sortOrder === "asc" ? 1 : -1;
  return 0;
};

const sortedProjects = sortBy === null ? mappedProjects : mappedProjects.slice().sort((a, b) => {
  let aValue = null;
  let bValue = null;
  if (sortBy === "name") {
    aValue = a.name;
    bValue = b.name;
  } else if (sortBy === "dueDate") {
    aValue = a.dueDate;
    bValue = b.dueDate;
  } else if (sortBy === "completionDate") {
    aValue = a.completionDate;
    bValue = b.completionDate;
  } else if (sortBy === "taskCount") {
    aValue = a.taskCount;
    bValue = b.taskCount;
  }

  if (aValue === null) return 1;
  if (bValue === null) return -1;

  if (sortBy === "name") {
    return compareValues(String(aValue).toLowerCase(), String(bValue).toLowerCase());
  }
  return compareValues(aValue, bValue);
});"#;
