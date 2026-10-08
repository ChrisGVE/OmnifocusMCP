//! Pieces shared by the batch delete tools for projects, tags and folders.
//!
//! `projects::delete_projects_batch`, `tags::delete_tags_batch` and
//! `folders::delete_folders_batch` validate their list of ids or names the
//! same way and end their scripts with the same summary, so both live here.
//! (`tasks::delete_tasks_batch` takes ids only and reports differently.)

use std::collections::HashSet;

use crate::error::{OmniFocusError, Result};

/// Trims each id or name and rejects an empty list, an empty entry or a
/// repeated entry. `kind` ("project", "tag" or "folder") names the argument
/// in the error, e.g. `tag_ids_or_names must not contain duplicates: x`.
pub(crate) fn normalize_ids_or_names(kind: &str, values: Vec<String>) -> Result<Vec<String>> {
    if values.is_empty() {
        return Err(OmniFocusError::Validation(format!(
            "{kind}_ids_or_names must contain at least one {kind} id or name."
        )));
    }

    let mut normalized: Vec<String> = Vec::with_capacity(values.len());
    let mut seen: HashSet<String> = HashSet::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(OmniFocusError::Validation(format!(
                "each {kind} id or name must be a non-empty string."
            )));
        }
        if seen.contains(trimmed) {
            return Err(OmniFocusError::Validation(format!(
                "{kind}_ids_or_names must not contain duplicates: {trimmed}"
            )));
        }
        seen.insert(trimmed.to_string());
        normalized.push(trimmed.to_string());
    }
    Ok(normalized)
}

/// Script tail that returns the batch result: counts of deleted and failed
/// entries plus the per-entry `results` array the script built before it.
pub(crate) const BATCH_DELETE_SUMMARY: &str = r#"const deletedCount = results.filter(result => result.deleted).length;
const failedCount = results.length - deletedCount;

return {
  summary: {
    requested: results.length,
    deleted: deletedCount,
    failed: failedCount
  },
  partial_success: deletedCount > 0 && failedCount > 0,
  results: results
};"#;
