//! The filters shared by `list_tasks`, `search_tasks` and `get_task_counts`,
//! and the script lines that declare them.
//!
//! `TaskFilters` holds the filter arguments as a tool received them, and
//! `FilterLiterals` the same filters rendered as JavaScript literals. The
//! `*_lines` fragments declare those literals as script constants under the
//! same names in every script, so the filter code in `listing_script` and
//! `counts` reads them without caring which tool built the script.

use crate::{
    error::{to_json_string, OmniFocusError, Result},
    jxa::escape_for_jxa,
    tools::js_values::{
        js_bool_or_null, js_number_or_null, js_string_or_null, js_trimmed_string_or_null,
    },
};

/// Task filter arguments, as the tool received them.
pub(super) struct TaskFilters<'a> {
    pub project: Option<&'a str>,
    pub tag: Option<&'a str>,
    pub tags: Option<Vec<String>>,
    pub flagged: Option<bool>,
    pub due_before: Option<&'a str>,
    pub due_after: Option<&'a str>,
    pub defer_before: Option<&'a str>,
    pub defer_after: Option<&'a str>,
    pub completed_before: Option<&'a str>,
    pub completed_after: Option<&'a str>,
    pub added_after: Option<&'a str>,
    pub added_before: Option<&'a str>,
    pub changed_after: Option<&'a str>,
    pub changed_before: Option<&'a str>,
    pub planned_before: Option<&'a str>,
    pub planned_after: Option<&'a str>,
    pub max_estimated_minutes: Option<i32>,
}

impl TaskFilters<'_> {
    /// Rejects an empty `project`, `tag` or `tags` entry, in that order.
    pub fn validate_names(&self) -> Result<()> {
        if let Some(project_name) = self.project {
            if project_name.trim().is_empty() {
                return Err(OmniFocusError::Validation(
                    "project must not be empty when provided.".to_string(),
                ));
            }
        }
        if let Some(tag_name) = self.tag {
            if tag_name.trim().is_empty() {
                return Err(OmniFocusError::Validation(
                    "tag must not be empty when provided.".to_string(),
                ));
            }
        }
        if let Some(tag_names) = &self.tags {
            for tag_name in tag_names {
                if tag_name.trim().is_empty() {
                    return Err(OmniFocusError::Validation(
                        "tags entries must not be empty when provided.".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Whether a completion date bound is set. `listing_script` then also
    /// matches completed tasks and, unless told otherwise, sorts by
    /// completion date, newest first.
    pub fn has_completed_range(&self) -> bool {
        self.completed_before.is_some() || self.completed_after.is_some()
    }

    /// The filters as JavaScript literals. `tag_filter_mode` is the
    /// normalised mode (`any` or `all`).
    pub fn literals(&self, tag_filter_mode: &str) -> Result<FilterLiterals> {
        Ok(FilterLiterals {
            project_filter: js_trimmed_string_or_null(self.project),
            tag_filter_values: self.tag_filter_values_literal()?,
            tag_filter_mode_filter: escape_for_jxa(tag_filter_mode),
            flagged_filter: js_bool_or_null(self.flagged),
            due_before_filter: js_string_or_null(self.due_before),
            due_after_filter: js_string_or_null(self.due_after),
            defer_before_filter: js_string_or_null(self.defer_before),
            defer_after_filter: js_string_or_null(self.defer_after),
            completed_before_filter: js_string_or_null(self.completed_before),
            completed_after_filter: js_string_or_null(self.completed_after),
            added_after_filter: js_string_or_null(self.added_after),
            added_before_filter: js_string_or_null(self.added_before),
            changed_after_filter: js_string_or_null(self.changed_after),
            changed_before_filter: js_string_or_null(self.changed_before),
            planned_before_filter: js_string_or_null(self.planned_before),
            planned_after_filter: js_string_or_null(self.planned_after),
            max_estimated_minutes_filter: js_number_or_null(self.max_estimated_minutes),
        })
    }

    /// `tag` followed by the `tags` entries (each a tag id or exact name),
    /// trimmed and without repeats, as a JSON array; `null` when there are
    /// none.
    fn tag_filter_values_literal(&self) -> Result<String> {
        let mut merged_values: Vec<String> = Vec::new();
        let given_tags = self.tags.iter().flatten().map(String::as_str);
        for value in self.tag.into_iter().chain(given_tags) {
            let normalized_value = value.trim().to_string();
            if !normalized_value.is_empty() && !merged_values.contains(&normalized_value) {
                merged_values.push(normalized_value);
            }
        }
        if merged_values.is_empty() {
            Ok("null".to_string())
        } else {
            to_json_string(&merged_values)
        }
    }
}

/// Task filters rendered as JavaScript literals, ready to splice in.
pub(super) struct FilterLiterals {
    project_filter: String,
    tag_filter_values: String,
    tag_filter_mode_filter: String,
    flagged_filter: String,
    due_before_filter: String,
    due_after_filter: String,
    defer_before_filter: String,
    defer_after_filter: String,
    completed_before_filter: String,
    completed_after_filter: String,
    added_after_filter: String,
    added_before_filter: String,
    changed_after_filter: String,
    changed_before_filter: String,
    planned_before_filter: String,
    planned_after_filter: String,
    max_estimated_minutes_filter: String,
}

impl FilterLiterals {
    /// Declares the project, tag and flagged filters, and
    /// `taskMatchesTagFilter(task)`, the tag test the filter code applies.
    /// The project and every tag value (an id or exact name) are resolved
    /// here, so an unknown or ambiguous one fails the script up front; tasks
    /// are then matched by tag id, never by name.
    pub fn selection_lines(&self) -> String {
        let FilterLiterals {
            project_filter,
            tag_filter_values,
            tag_filter_mode_filter,
            flagged_filter,
            ..
        } = self;
        format!(
            r#"const projectFilter = {project_filter};
const filterProject = projectFilter === null ? null : resolveProject(projectFilter);
const tagFilterValues = {tag_filter_values};
const tagFilterMode = {tag_filter_mode_filter};
const filterTagIds = tagFilterValues === null ? null : tagFilterValues.map(value => resolveTag(value).id.primaryKey);
const taskMatchesTagFilter = (task) => {{
  if (filterTagIds === null) return true;
  const taskTagIds = task.tags.map(tag => tag.id.primaryKey);
  if (tagFilterMode === "all") return filterTagIds.every(tagId => taskTagIds.includes(tagId));
  return filterTagIds.some(tagId => taskTagIds.includes(tagId));
}};
const flaggedFilter = {flagged_filter};"#
        )
    }

    /// Declares each date bound as received (`...Raw`) and the estimated
    /// duration limit. `PARSED_BOUND_LINES` turns the raw bounds into dates.
    pub fn bound_lines(&self) -> String {
        let FilterLiterals {
            due_before_filter,
            due_after_filter,
            defer_before_filter,
            defer_after_filter,
            completed_before_filter,
            completed_after_filter,
            added_after_filter,
            added_before_filter,
            changed_after_filter,
            changed_before_filter,
            planned_before_filter,
            planned_after_filter,
            max_estimated_minutes_filter,
            ..
        } = self;
        format!(
            r#"const dueBeforeRaw = {due_before_filter};
const dueAfterRaw = {due_after_filter};
const deferBeforeRaw = {defer_before_filter};
const deferAfterRaw = {defer_after_filter};
const completedBeforeRaw = {completed_before_filter};
const completedAfterRaw = {completed_after_filter};
const addedAfterRaw = {added_after_filter};
const addedBeforeRaw = {added_before_filter};
const changedAfterRaw = {changed_after_filter};
const changedBeforeRaw = {changed_before_filter};
const plannedBeforeRaw = {planned_before_filter};
const plannedAfterRaw = {planned_after_filter};
const maxEstimatedMinutes = {max_estimated_minutes_filter};"#
        )
    }
}

/// Declares `now`, `soon` (seven days from now) and every date bound parsed
/// from its raw value, so a malformed date fails the script up front.
pub(super) const PARSED_BOUND_LINES: &str = r#"const now = new Date();
const soon = new Date(now.getTime() + (7 * 24 * 60 * 60 * 1000));
const dueBefore = parseOptionalLocalDate(dueBeforeRaw, "dueBefore");
const dueAfter = parseOptionalLocalDate(dueAfterRaw, "dueAfter");
const deferBefore = parseOptionalLocalDate(deferBeforeRaw, "deferBefore");
const deferAfter = parseOptionalLocalDate(deferAfterRaw, "deferAfter");
const completedBefore = parseOptionalLocalDate(completedBeforeRaw, "completedBefore");
const completedAfter = parseOptionalLocalDate(completedAfterRaw, "completedAfter");
const addedAfter = parseOptionalLocalDate(addedAfterRaw, "added_after");
const addedBefore = parseOptionalLocalDate(addedBeforeRaw, "added_before");
const changedAfter = parseOptionalLocalDate(changedAfterRaw, "changed_after");
const changedBefore = parseOptionalLocalDate(changedBeforeRaw, "changed_before");
const plannedBefore = parseOptionalLocalDate(plannedBeforeRaw, "plannedBefore");
const plannedAfter = parseOptionalLocalDate(plannedAfterRaw, "plannedAfter");"#;
