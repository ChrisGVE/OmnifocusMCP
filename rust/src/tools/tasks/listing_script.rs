//! The script shared by `list_tasks` and `search_tasks`.
//!
//! Both tools filter `document.flattenedTasks`, sort the matches and map
//! them to task summaries; `search_tasks` only adds a case-insensitive
//! name/note match. `Listing` holds one validated request and builds its
//! script from four parts: declarations, filter, sort and mapping.

use crate::{
    error::Result,
    js_helpers::{
        JS_DATE_HELPERS, JS_PLANNED_DATE, JS_PROJECT_STATUS, JS_RESOLVERS, JS_TASK_STATUS,
    },
    jxa::escape_for_jxa,
    tools::js_values::js_string_or_null,
};

use super::filters::{FilterLiterals, TaskFilters, PARSED_BOUND_LINES};

/// One validated `list_tasks` or `search_tasks` request.
pub(super) struct Listing<'a> {
    /// The search text; `None` for `list_tasks`.
    pub query: Option<&'a str>,
    pub filters: TaskFilters<'a>,
    /// Canonical tag filter mode, task status and sort order.
    pub tag_filter_mode: &'static str,
    pub status: &'static str,
    pub sort_by: Option<&'a str>,
    pub sort_order: &'static str,
    pub limit: i32,
}

impl Listing<'_> {
    /// The complete OmniJS script for this request.
    pub fn script(&self) -> Result<String> {
        let literals = self.filters.literals(self.tag_filter_mode)?;
        let declarations = self.declarations(&literals);
        let filter = filter_fragment(self.query.is_some());
        let limit = self.limit;
        Ok(format!(
            "{declarations}\n\n{filter}\n\n{SORT_LISTED_TASKS}\n\n\
             const tasks = sortedTasks.slice(0, {limit});\n\n{MAP_LISTED_TASKS}"
        ))
    }

    /// The helper prelude and every constant the filter, sort and mapping
    /// read. A completion date bound widens the status filter to `all`
    /// (unless it already asks for completed tasks) and, without an explicit
    /// sort, sorts by completion date, newest first.
    fn declarations(&self, literals: &FilterLiterals) -> String {
        let completed_range = self.filters.has_completed_range();
        let (sort_by, sort_order) = if completed_range && self.sort_by.is_none() {
            (Some("completionDate"), "desc")
        } else {
            (self.sort_by, self.sort_order)
        };
        let status = if completed_range && self.status != "completed" {
            "all"
        } else {
            self.status
        };
        let query_line = self
            .query
            .map(|query| {
                let query_filter = escape_for_jxa(query.trim());
                format!("const queryFilter = {query_filter}.toLowerCase();\n")
            })
            .unwrap_or_default();
        let selection = literals.selection_lines();
        let status_filter = escape_for_jxa(status);
        let bounds = literals.bound_lines();
        let sort_by_filter = js_string_or_null(sort_by);
        let sort_order_filter = escape_for_jxa(sort_order);
        format!(
            r#"{JS_DATE_HELPERS}
{JS_PLANNED_DATE}
{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_RESOLVERS}
{query_line}{selection}
const statusFilter = {status_filter};
{bounds}
const sortBy = {sort_by_filter};
const sortOrder = {sort_order_filter};
{PARSED_BOUND_LINES}
const includeCompletedForDateFilter = completedBefore !== null || completedAfter !== null;
const supportsPlannedDate = detectPlannedDateSupport(document.flattenedTasks);
requirePlannedDateSupport(supportsPlannedDate, plannedBeforeRaw, plannedAfterRaw, sortBy);
const getPlannedDate = (task) => readPlannedDate(task, supportsPlannedDate);"#
        )
    }
}

/// The filter over every task; with a query, a task must also contain it in
/// its name or note.
fn filter_fragment(has_query: bool) -> String {
    let query_match = if has_query { QUERY_MATCH_LINES } else { "" };
    format!("{FILTER_HEAD}{query_match}{FILTER_PREDICATE}")
}

const FILTER_HEAD: &str = r#"const filteredTasks = document.flattenedTasks
  .filter(task => {
    if (isProjectRootTask(task)) return false;
"#;

const QUERY_MATCH_LINES: &str = r#"    const name = (task.name || "").toLowerCase();
    const note = (task.note || "").toLowerCase();
    if (!(name.includes(queryFilter) || note.includes(queryFilter))) return false;

"#;

/// Project, tag, flagged, status, date bound and duration tests. A
/// completed task passes a non-completed status only when a completion
/// date bound asked for completed tasks.
const FILTER_PREDICATE: &str = r#"    if (filterProject !== null) {
      const containing = task.containingProject;
      if (!containing || containing.id.primaryKey !== filterProject.id.primaryKey) return false;
    }

    if (tagNames !== null && tagNames.length > 0) {
      let tagMatches = false;
      if (tagFilterMode === "all") {
        tagMatches = tagNames.every(tn => task.tags.some(t => t.name === tn));
      } else {
        tagMatches = task.tags.some(t => tagNames.includes(t.name));
      }
      if (!tagMatches) return false;
    }

    if (flaggedFilter !== null && task.flagged !== flaggedFilter) return false;

    let statusMatches = false;
    if (statusFilter === "all") {
      statusMatches = true;
    } else if (statusFilter === "completed") {
      statusMatches = isTaskCompleted(task);
    } else if (isTaskCompleted(task)) {
      statusMatches = includeCompletedForDateFilter;
    } else if (isTaskRemaining(task)) {
      const dueDate = task.dueDate;
      if (statusFilter === "available") {
        statusMatches = isTaskAvailable(task, now);
      } else if (statusFilter === "overdue") {
        statusMatches = dueDate !== null && dueDate < now;
      } else if (statusFilter === "due_soon") {
        statusMatches = dueDate !== null && dueDate >= now && dueDate <= soon;
      } else if (statusFilter === "on_hold") {
        statusMatches = task.containingProject !== null && normalizeProjectStatus(task.containingProject) === "on_hold";
      }
    }
    if (!statusMatches) return false;
    if (dueBefore !== null && !(task.dueDate !== null && task.dueDate < dueBefore)) return false;
    if (dueAfter !== null && !(task.dueDate !== null && task.dueDate > dueAfter)) return false;
    if (deferBefore !== null && !(task.deferDate !== null && task.deferDate < deferBefore)) return false;
    if (deferAfter !== null && !(task.deferDate !== null && task.deferDate > deferAfter)) return false;
    if (completedBefore !== null && !(task.completionDate !== null && task.completionDate < completedBefore)) return false;
    if (completedAfter !== null && !(task.completionDate !== null && task.completionDate > completedAfter)) return false;
    if (addedBefore !== null && !(task.added !== null && task.added <= addedBefore)) return false;
    if (addedAfter !== null && !(task.added !== null && task.added >= addedAfter)) return false;
    if (changedBefore !== null && !(task.modified !== null && task.modified <= changedBefore)) return false;
    if (changedAfter !== null && !(task.modified !== null && task.modified >= changedAfter)) return false;
    if (supportsPlannedDate) {
      const plannedDate = getPlannedDate(task);
      if (plannedBefore !== null && !(plannedDate !== null && plannedDate < plannedBefore)) return false;
      if (plannedAfter !== null && !(plannedDate !== null && plannedDate > plannedAfter)) return false;
    }
    if (maxEstimatedMinutes !== null && !(task.estimatedMinutes !== null && task.estimatedMinutes <= maxEstimatedMinutes)) return false;
    return true;
  });"#;

/// Sorts the matches by `sortBy` in `sortOrder`; tasks without a value
/// for the sort field go last.
const SORT_LISTED_TASKS: &str = r#"const compareValues = (aValue, bValue, isString = false) => {
  if (aValue === null && bValue === null) return 0;
  if (aValue === null) return 1;
  if (bValue === null) return -1;
  let left = aValue;
  let right = bValue;
  if (isString) {
    left = String(aValue).toLowerCase();
    right = String(bValue).toLowerCase();
  }
  if (left < right) return sortOrder === "asc" ? -1 : 1;
  if (left > right) return sortOrder === "asc" ? 1 : -1;
  return 0;
};

const sortedTasks = sortBy === null ? filteredTasks : filteredTasks.slice().sort((a, b) => {
  let aValue = null;
  let bValue = null;
  let isString = false;
  if (sortBy === "dueDate") {
    aValue = a.dueDate;
    bValue = b.dueDate;
  } else if (sortBy === "deferDate") {
    aValue = a.deferDate;
    bValue = b.deferDate;
  } else if (sortBy === "name") {
    aValue = a.name;
    bValue = b.name;
    isString = true;
  } else if (sortBy === "completionDate") {
    aValue = a.completionDate;
    bValue = b.completionDate;
  } else if (sortBy === "estimatedMinutes") {
    aValue = a.estimatedMinutes;
    bValue = b.estimatedMinutes;
  } else if (sortBy === "project") {
    aValue = a.containingProject ? a.containingProject.name : null;
    bValue = b.containingProject ? b.containingProject.name : null;
    isString = true;
  } else if (sortBy === "flagged") {
    aValue = a.flagged;
    bValue = b.flagged;
  } else if (sortBy === "addedDate" || sortBy === "added") {
    aValue = a.added;
    bValue = b.added;
  } else if (sortBy === "changedDate" || sortBy === "modified") {
    aValue = a.modified;
    bValue = b.modified;
  } else if (sortBy === "plannedDate" || sortBy === "planned") {
    aValue = getPlannedDate(a);
    bValue = getPlannedDate(b);
  }
  return compareValues(aValue, bValue, isString);
});"#;

/// Reads planned dates for the summary mapper without requiring them: on a
/// database not migrated for planned dates every `plannedDate` is `null`.
/// Needs `JS_PLANNED_DATE`. Used by reads that take no planned-date filter.
pub(super) const READ_PLANNED_DATES: &str = r#"const supportsPlannedDate = detectPlannedDateSupport(document.flattenedTasks);
const getPlannedDate = (task) => readPlannedDate(task, supportsPlannedDate);"#;

/// Maps `tasks` to the task summary every listing tool returns (`list_tasks`,
/// `search_tasks`, `get_inbox`, `list_subtasks`). Needs `getPlannedDate`.
pub(super) const MAP_LISTED_TASKS: &str = r#"return tasks.map(task => {
  const tags = task.tags.map(taskTag => taskTag.name);
  const plannedDate = getPlannedDate(task);
  return {
    id: task.id.primaryKey,
    name: task.name,
    note: task.note,
    flagged: task.flagged,
    dueDate: task.dueDate ? task.dueDate.toISOString() : null,
    addedDate: task.added ? task.added.toISOString() : null,
    changedDate: task.modified ? task.modified.toISOString() : null,
    deferDate: task.deferDate ? task.deferDate.toISOString() : null,
    completed: task.completed,
    completionDate: task.completionDate ? task.completionDate.toISOString() : null,
    plannedDate: plannedDate ? plannedDate.toISOString() : null,
    projectName: task.containingProject ? task.containingProject.name : null,
    tags: tags,
    estimatedMinutes: task.estimatedMinutes,
    inInbox: task.inInbox,
    hasChildren: task.hasChildren,
    sequential: task.sequential,
    taskStatus: (() => {
      const s = String(task.taskStatus);
      if (s.includes("Available")) return "available";
      if (s.includes("Blocked")) return "blocked";
      if (s.includes("Next")) return "next";
      if (s.includes("DueSoon")) return "due_soon";
      if (s.includes("Overdue")) return "overdue";
      if (s.includes("Completed")) return "completed";
      if (s.includes("Dropped")) return "dropped";
      return "unknown";
    })()
  };
});"#;
