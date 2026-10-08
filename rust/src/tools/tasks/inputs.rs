//! Validation and normalisation of task tool inputs.
//!
//! A few task tool arguments are enumerations that accept aliases (status,
//! tag filter mode, sort order); the `normalize_*` functions map an accepted
//! spelling to its canonical value or return the tool's validation error.
//! The `validate_*` checks are shared by `list`, `search` and `counts`; each
//! of those calls them in its own fixed order, which decides the error a call
//! with several bad arguments reports.

use serde_json::Value;

use crate::{
    error::{OmniFocusError, Result},
    types::TaskResult,
};

pub(super) fn parse_task_list(value: Value) -> Result<Vec<TaskResult>> {
    Ok(serde_json::from_value(value)?)
}

pub(super) fn normalize_tag_filter_mode_input(value: &str) -> Result<&'static str> {
    let normalized_value = value.trim().to_ascii_lowercase();
    match normalized_value.as_str() {
        "any" => Ok("any"),
        "all" => Ok("all"),
        "and" => Ok("all"),
        "or" => Ok("any"),
        _ => Err(OmniFocusError::Validation(format!(
            "tagFilterMode must be one of: any, all. received: {}.",
            serde_json::to_string(value).unwrap_or_else(|_| "\"<invalid>\"".to_string())
        ))),
    }
}

pub(super) fn normalize_task_status_input(value: &str) -> Result<&'static str> {
    let normalized_value = value.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    match normalized_value.as_str() {
        "available" => Ok("available"),
        "due_soon" | "duesoon" => Ok("due_soon"),
        "overdue" => Ok("overdue"),
        "on_hold" | "onhold" => Ok("on_hold"),
        "completed" => Ok("completed"),
        "all" => Ok("all"),
        _ => Err(OmniFocusError::Validation(format!(
            "status must be one of: available, due_soon, overdue, on_hold, completed, all. received: {}.",
            serde_json::to_string(value).unwrap_or_else(|_| "\"<invalid>\"".to_string())
        ))),
    }
}

pub(super) fn normalize_sort_order_input(value: &str) -> Result<&'static str> {
    let normalized_value = value.trim().to_ascii_lowercase();
    match normalized_value.as_str() {
        "asc" => Ok("asc"),
        "desc" => Ok("desc"),
        "ascending" => Ok("asc"),
        "descending" => Ok("desc"),
        _ => Err(OmniFocusError::Validation(format!(
            "sortOrder must be one of: asc, desc. received: {}.",
            serde_json::to_string(value).unwrap_or_else(|_| "\"<invalid>\"".to_string())
        ))),
    }
}

/// Rejects a `limit` below 1.
pub(super) fn validate_limit(limit: i32) -> Result<()> {
    if limit < 1 {
        return Err(OmniFocusError::Validation(
            "limit must be greater than 0.".to_string(),
        ));
    }
    Ok(())
}

/// Rejects a negative `maxEstimatedMinutes`.
pub(super) fn validate_max_estimated_minutes(max_estimated_minutes: Option<i32>) -> Result<()> {
    if let Some(max_minutes) = max_estimated_minutes {
        if max_minutes < 0 {
            return Err(OmniFocusError::Validation(
                "maxEstimatedMinutes must be greater than or equal to 0.".to_string(),
            ));
        }
    }
    Ok(())
}

/// Rejects a `sortBy` that `list_tasks` and `search_tasks` cannot sort by.
pub(super) fn validate_task_sort_by(sort_by: Option<&str>) -> Result<()> {
    if let Some(sort_field) = sort_by {
        if !matches!(
            sort_field,
            "dueDate"
                | "deferDate"
                | "name"
                | "completionDate"
                | "estimatedMinutes"
                | "project"
                | "flagged"
                | "addedDate"
                | "changedDate"
                | "plannedDate"
                | "added"
                | "modified"
                | "planned"
        ) {
            return Err(OmniFocusError::Validation(
                "sortBy must be one of: dueDate, deferDate, name, completionDate, estimatedMinutes, project, flagged, addedDate, changedDate, plannedDate, added, modified, planned.".to_string(),
            ));
        }
    }
    Ok(())
}
