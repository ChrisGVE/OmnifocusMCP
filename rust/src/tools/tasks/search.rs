//! `search_tasks`: the tasks whose name or note contains a text,
//! case-insensitively, with the same filters and sorting as `list_tasks`.
//!
//! The three public functions are one tool at successive API generations:
//! `search_tasks_with_added_changed` takes every filter, and the other two
//! pass `None` for the filters they predate. The script is built by
//! `listing_script`, which `list_tasks` shares.

use crate::{
    error::{OmniFocusError, Result},
    jxa::JxaRunner,
    types::TaskResult,
};

use super::{
    filters::TaskFilters,
    inputs::{
        normalize_sort_order_input, normalize_tag_filter_mode_input, normalize_task_status_input,
        parse_task_list, validate_limit, validate_max_estimated_minutes, validate_task_sort_by,
    },
    listing_script::Listing,
};

#[allow(clippy::too_many_arguments)]
pub async fn search_tasks_with_added_changed<R: JxaRunner>(
    runner: &R,
    query: &str,
    project: Option<&str>,
    tag: Option<&str>,
    tags: Option<Vec<String>>,
    tag_filter_mode: &str,
    flagged: Option<bool>,
    status: &str,
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
    sort_by: Option<&str>,
    sort_order: &str,
    limit: i32,
) -> Result<Vec<TaskResult>> {
    if query.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "query must not be empty.".to_string(),
        ));
    }
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
    let normalized_status = normalize_task_status_input(status)?;
    validate_task_sort_by(sort_by)?;
    let normalized_sort_order = normalize_sort_order_input(sort_order)?;
    validate_max_estimated_minutes(max_estimated_minutes)?;
    validate_limit(limit)?;

    let listing = Listing {
        query: Some(query),
        filters,
        tag_filter_mode: normalized_tag_filter_mode,
        status: normalized_status,
        sort_by,
        sort_order: normalized_sort_order,
        limit,
    };
    let value = runner.run_omnijs(&listing.script()?).await?;
    parse_task_list(value)
}

#[allow(clippy::too_many_arguments)]
pub async fn search_tasks_with_planned<R: JxaRunner>(
    runner: &R,
    query: &str,
    project: Option<&str>,
    tag: Option<&str>,
    tags: Option<Vec<String>>,
    tag_filter_mode: &str,
    flagged: Option<bool>,
    status: &str,
    due_before: Option<&str>,
    due_after: Option<&str>,
    defer_before: Option<&str>,
    defer_after: Option<&str>,
    completed_before: Option<&str>,
    completed_after: Option<&str>,
    planned_before: Option<&str>,
    planned_after: Option<&str>,
    max_estimated_minutes: Option<i32>,
    sort_by: Option<&str>,
    sort_order: &str,
    limit: i32,
) -> Result<Vec<TaskResult>> {
    search_tasks_with_added_changed(
        runner,
        query,
        project,
        tag,
        tags,
        tag_filter_mode,
        flagged,
        status,
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
        planned_before,
        planned_after,
        max_estimated_minutes,
        sort_by,
        sort_order,
        limit,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn search_tasks<R: JxaRunner>(
    runner: &R,
    query: &str,
    project: Option<&str>,
    tag: Option<&str>,
    tags: Option<Vec<String>>,
    tag_filter_mode: &str,
    flagged: Option<bool>,
    status: &str,
    due_before: Option<&str>,
    due_after: Option<&str>,
    defer_before: Option<&str>,
    defer_after: Option<&str>,
    completed_before: Option<&str>,
    completed_after: Option<&str>,
    max_estimated_minutes: Option<i32>,
    sort_by: Option<&str>,
    sort_order: &str,
    limit: i32,
) -> Result<Vec<TaskResult>> {
    search_tasks_with_added_changed(
        runner,
        query,
        project,
        tag,
        tags,
        tag_filter_mode,
        flagged,
        status,
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
        sort_by,
        sort_order,
        limit,
    )
    .await
}
