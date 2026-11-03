//! `get_task_counts`: how many tasks match a filter, by state, without
//! listing them.
//!
//! The filters are the ones `list_tasks` takes, minus status and sorting;
//! they are declared by the shared fragments in `filters`. The counting pass
//! itself is `COUNT_MATCHING_TASKS`.

use crate::{
    error::{from_result_value, Result},
    js_helpers::{
        JS_DATE_HELPERS, JS_PLANNED_DATE, JS_PROJECT_STATUS, JS_RESOLVERS, JS_TASK_STATUS,
    },
    jxa::JxaRunner,
    types::TaskCountsResult,
};

use super::{
    filters::{FilterLiterals, TaskFilters, PARSED_BOUND_LINES, TASK_MATCHES_FILTERS},
    inputs::{normalize_tag_filter_mode_input, validate_max_estimated_minutes},
};

#[allow(clippy::too_many_arguments)]
pub async fn get_task_counts_with_added_changed<R: JxaRunner>(
    runner: &R,
    project: Option<&str>,
    tag: Option<&str>,
    tags: Option<Vec<String>>,
    tag_filter_mode: &str,
    flagged: Option<bool>,
    due_before: Option<&str>,
    due_after: Option<&str>,
    defer_before: Option<&str>,
    defer_after: Option<&str>,
    completed_before: Option<&str>,
    completed_after: Option<&str>,
    added_after: Option<&str>,
    added_before: Option<&str>,
    changed_after: Option<&str>,
    changed_before: Option<&str>,
    planned_before: Option<&str>,
    planned_after: Option<&str>,
    max_estimated_minutes: Option<i32>,
) -> Result<TaskCountsResult> {
    let filters = TaskFilters {
        project,
        tag,
        tags,
        flagged,
        due_before,
        due_after,
        defer_before,
        defer_after,
        completed_before,
        completed_after,
        added_after,
        added_before,
        changed_after,
        changed_before,
        planned_before,
        planned_after,
        max_estimated_minutes,
    };
    filters.validate_names()?;
    let normalized_tag_filter_mode = normalize_tag_filter_mode_input(tag_filter_mode)?;
    validate_max_estimated_minutes(max_estimated_minutes)?;

    let script = counts_script(&filters.literals(normalized_tag_filter_mode)?);
    let value = runner.run_omnijs(&script).await?;
    from_result_value(value)
}

#[allow(clippy::too_many_arguments)]
pub async fn get_task_counts<R: JxaRunner>(
    runner: &R,
    project: Option<&str>,
    tag: Option<&str>,
    tags: Option<Vec<String>>,
    tag_filter_mode: &str,
    flagged: Option<bool>,
    due_before: Option<&str>,
    due_after: Option<&str>,
    defer_before: Option<&str>,
    defer_after: Option<&str>,
    completed_before: Option<&str>,
    completed_after: Option<&str>,
    max_estimated_minutes: Option<i32>,
) -> Result<TaskCountsResult> {
    get_task_counts_with_added_changed(
        runner,
        project,
        tag,
        tags,
        tag_filter_mode,
        flagged,
        due_before,
        due_after,
        defer_before,
        defer_after,
        completed_before,
        completed_after,
        None,
        None,
        None,
        None,
        None,
        None,
        max_estimated_minutes,
    )
    .await
}

/// The counting script: the filter declarations, then one pass over every
/// task.
fn counts_script(literals: &FilterLiterals) -> String {
    let selection = literals.selection_lines();
    let bounds = literals.bound_lines();
    format!(
        r#"{JS_DATE_HELPERS}
{JS_PLANNED_DATE}
{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_RESOLVERS}
{selection}
{bounds}
{PARSED_BOUND_LINES}
const statusFilter = "all";
const includeCompletedForDateFilter = false;
const supportsPlannedDate = detectPlannedDateSupport(document.flattenedTasks);
requirePlannedDateSupport(supportsPlannedDate, plannedBeforeRaw, plannedAfterRaw, null);
const getPlannedDate = (task) => readPlannedDate(task, supportsPlannedDate);

{TASK_MATCHES_FILTERS}
{COUNT_MATCHING_TASKS}"#
    )
}

/// Counts every task that passes the filters: all of them, the flagged and
/// completed ones, and, among the remaining ones, the available, deferred,
/// overdue and due-within-seven-days ones.
const COUNT_MATCHING_TASKS: &str = r#"const counts = {
  total: 0,
  available: 0,
  completed: 0,
  overdue: 0,
  dueSoon: 0,
  flagged: 0,
  deferred: 0
};

for (const task of document.flattenedTasks) {
  if (!taskMatchesFilters(task)) continue;

  counts.total += 1;
  if (task.flagged) counts.flagged += 1;
  if (isTaskCompleted(task)) {
    counts.completed += 1;
    continue;
  }
  if (!isTaskRemaining(task)) continue;
  if (isTaskAvailable(task, now)) counts.available += 1;
  if (isDeferred(task, now)) counts.deferred += 1;
  if (isOverdue(task, now)) counts.overdue += 1;
  if (isDueSoon(task, now, dueSoonSeconds)) counts.dueSoon += 1;
}
return counts;"#;
