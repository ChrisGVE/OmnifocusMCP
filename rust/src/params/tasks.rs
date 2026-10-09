//! Parameter structs of the task tools (see `crate::params` for why they
//! carry no doc comments).

use schemars::{JsonSchema, Schema};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::{
    error::{OmniFocusError, Result},
    flexible_tags::FlexibleTagList,
    lenient_scalars::{LenientBool, LenientF64, LenientI32},
};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListTasksParams {
    #[schemars(
        description = "Project id or exact name. Only tasks in this project are included; an unknown project is an error."
    )]
    pub(crate) project: Option<String>,
    pub(crate) tag: Option<String>,
    pub(crate) tags: Option<FlexibleTagList>,
    #[serde(rename = "tagFilterMode", alias = "tag_filter_mode")]
    #[schemars(description = "tag matching mode: any/all. aliases: and/or (case-insensitive).")]
    pub(crate) tag_filter_mode: Option<String>,
    pub(crate) flagged: Option<LenientBool>,
    #[schemars(
        description = "task status filter: available, due_soon, overdue, on_hold, completed, all. aliases: due soon/due-soon, on hold/on-hold."
    )]
    pub(crate) status: Option<String>,
    #[serde(rename = "dueBefore", alias = "due_before")]
    pub(crate) due_before: Option<String>,
    #[serde(rename = "dueAfter", alias = "due_after")]
    pub(crate) due_after: Option<String>,
    #[serde(rename = "deferBefore", alias = "defer_before")]
    pub(crate) defer_before: Option<String>,
    #[serde(rename = "deferAfter", alias = "defer_after")]
    pub(crate) defer_after: Option<String>,
    #[serde(rename = "completedBefore", alias = "completed_before")]
    pub(crate) completed_before: Option<String>,
    #[serde(rename = "completedAfter", alias = "completed_after")]
    pub(crate) completed_after: Option<String>,
    pub(crate) added_after: Option<String>,
    pub(crate) added_before: Option<String>,
    pub(crate) changed_after: Option<String>,
    pub(crate) changed_before: Option<String>,
    #[serde(rename = "plannedBefore", alias = "planned_before")]
    pub(crate) planned_before: Option<String>,
    #[serde(rename = "plannedAfter", alias = "planned_after")]
    pub(crate) planned_after: Option<String>,
    #[serde(rename = "maxEstimatedMinutes", alias = "max_estimated_minutes")]
    pub(crate) max_estimated_minutes: Option<LenientI32>,
    #[serde(rename = "sortBy", alias = "sort_by")]
    pub(crate) sort_by: Option<String>,
    #[serde(rename = "sortOrder", alias = "sort_order")]
    #[schemars(description = "sort direction: asc/desc. aliases: ascending/descending.")]
    pub(crate) sort_order: Option<String>,
    pub(crate) limit: Option<LenientI32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetTaskCountsParams {
    #[schemars(
        description = "Project id or exact name. Only tasks in this project are included; an unknown project is an error."
    )]
    pub(crate) project: Option<String>,
    pub(crate) tag: Option<String>,
    pub(crate) tags: Option<FlexibleTagList>,
    #[serde(rename = "tagFilterMode", alias = "tag_filter_mode")]
    #[schemars(description = "tag matching mode: any/all. aliases: and/or (case-insensitive).")]
    pub(crate) tag_filter_mode: Option<String>,
    pub(crate) flagged: Option<LenientBool>,
    #[serde(rename = "dueBefore", alias = "due_before")]
    pub(crate) due_before: Option<String>,
    #[serde(rename = "dueAfter", alias = "due_after")]
    pub(crate) due_after: Option<String>,
    #[serde(rename = "deferBefore", alias = "defer_before")]
    pub(crate) defer_before: Option<String>,
    #[serde(rename = "deferAfter", alias = "defer_after")]
    pub(crate) defer_after: Option<String>,
    #[serde(rename = "completedBefore", alias = "completed_before")]
    pub(crate) completed_before: Option<String>,
    #[serde(rename = "completedAfter", alias = "completed_after")]
    pub(crate) completed_after: Option<String>,
    pub(crate) added_after: Option<String>,
    pub(crate) added_before: Option<String>,
    pub(crate) changed_after: Option<String>,
    pub(crate) changed_before: Option<String>,
    #[serde(rename = "plannedBefore", alias = "planned_before")]
    pub(crate) planned_before: Option<String>,
    #[serde(rename = "plannedAfter", alias = "planned_after")]
    pub(crate) planned_after: Option<String>,
    #[serde(rename = "maxEstimatedMinutes", alias = "max_estimated_minutes")]
    pub(crate) max_estimated_minutes: Option<LenientI32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskIdParams {
    pub(crate) task_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskIdLimitParams {
    pub(crate) task_id: String,
    pub(crate) limit: Option<LenientI32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddNotificationParams {
    pub(crate) task_id: String,
    #[serde(rename = "absoluteDate", alias = "absolute_date")]
    pub(crate) absolute_date: Option<String>,
    #[serde(rename = "relativeOffset", alias = "relative_offset")]
    pub(crate) relative_offset: Option<LenientF64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DuplicateTaskParams {
    pub(crate) task_id: String,
    #[serde(rename = "includeChildren", alias = "include_children")]
    pub(crate) include_children: Option<LenientBool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RemoveNotificationParams {
    pub(crate) task_id: String,
    pub(crate) notification_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchTasksParams {
    pub(crate) query: String,
    #[schemars(
        description = "Project id or exact name. Only tasks in this project are included; an unknown project is an error."
    )]
    pub(crate) project: Option<String>,
    pub(crate) tag: Option<String>,
    pub(crate) tags: Option<FlexibleTagList>,
    #[serde(rename = "tagFilterMode", alias = "tag_filter_mode")]
    #[schemars(description = "tag matching mode: any/all. aliases: and/or (case-insensitive).")]
    pub(crate) tag_filter_mode: Option<String>,
    pub(crate) flagged: Option<LenientBool>,
    #[schemars(
        description = "task status filter: available, due_soon, overdue, on_hold, completed, all. aliases: due soon/due-soon, on hold/on-hold."
    )]
    pub(crate) status: Option<String>,
    #[serde(rename = "dueBefore", alias = "due_before")]
    pub(crate) due_before: Option<String>,
    #[serde(rename = "dueAfter", alias = "due_after")]
    pub(crate) due_after: Option<String>,
    #[serde(rename = "deferBefore", alias = "defer_before")]
    pub(crate) defer_before: Option<String>,
    #[serde(rename = "deferAfter", alias = "defer_after")]
    pub(crate) defer_after: Option<String>,
    #[serde(rename = "completedBefore", alias = "completed_before")]
    pub(crate) completed_before: Option<String>,
    #[serde(rename = "completedAfter", alias = "completed_after")]
    pub(crate) completed_after: Option<String>,
    pub(crate) added_after: Option<String>,
    pub(crate) added_before: Option<String>,
    pub(crate) changed_after: Option<String>,
    pub(crate) changed_before: Option<String>,
    #[serde(rename = "maxEstimatedMinutes", alias = "max_estimated_minutes")]
    pub(crate) max_estimated_minutes: Option<LenientI32>,
    #[serde(rename = "plannedBefore", alias = "planned_before")]
    pub(crate) planned_before: Option<String>,
    #[serde(rename = "plannedAfter", alias = "planned_after")]
    pub(crate) planned_after: Option<String>,
    #[serde(rename = "sortBy", alias = "sort_by")]
    pub(crate) sort_by: Option<String>,
    #[serde(rename = "sortOrder", alias = "sort_order")]
    #[schemars(description = "sort direction: asc/desc. aliases: ascending/descending.")]
    pub(crate) sort_order: Option<String>,
    pub(crate) limit: Option<LenientI32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateTaskParams {
    pub name: String,
    #[schemars(
        description = "Project id or exact name to create the task in; omit for the inbox. An unknown project is an error."
    )]
    pub project: Option<String>,
    pub note: Option<String>,
    #[serde(rename = "dueDate", alias = "due_date")]
    pub due_date: Option<String>,
    #[serde(rename = "deferDate", alias = "defer_date")]
    pub defer_date: Option<String>,
    #[serde(rename = "plannedDate", alias = "planned_date")]
    pub planned_date: Option<String>,
    pub flagged: Option<LenientBool>,
    pub tags: Option<FlexibleTagList>,
    #[serde(rename = "estimatedMinutes", alias = "estimated_minutes")]
    pub estimated_minutes: Option<LenientI32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateSubtaskParams {
    pub(crate) name: String,
    pub(crate) parent_task_id: String,
    pub(crate) note: Option<String>,
    #[serde(rename = "dueDate", alias = "due_date")]
    pub(crate) due_date: Option<String>,
    #[serde(rename = "deferDate", alias = "defer_date")]
    pub(crate) defer_date: Option<String>,
    #[serde(rename = "plannedDate", alias = "planned_date")]
    pub(crate) planned_date: Option<String>,
    pub(crate) flagged: Option<LenientBool>,
    pub(crate) tags: Option<FlexibleTagList>,
    #[serde(rename = "estimatedMinutes", alias = "estimated_minutes")]
    pub(crate) estimated_minutes: Option<LenientI32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateTasksBatchParams {
    pub(crate) tasks: Vec<BatchCreateTaskInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BatchCreateTaskInput {
    pub name: String,
    #[schemars(
        description = "Project id or exact name to create the task in; omit for the inbox. An unknown project is an error."
    )]
    pub project: Option<String>,
    pub note: Option<String>,
    #[serde(rename = "dueDate", alias = "due_date")]
    pub due_date: Option<String>,
    #[serde(rename = "deferDate", alias = "defer_date")]
    pub defer_date: Option<String>,
    #[serde(rename = "plannedDate", alias = "planned_date")]
    pub planned_date: Option<String>,
    pub flagged: Option<LenientBool>,
    pub tags: Option<FlexibleTagList>,
    #[serde(rename = "estimatedMinutes", alias = "estimated_minutes")]
    pub estimated_minutes: Option<LenientI32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateTaskParams {
    pub task_id: String,
    pub name: Option<String>,
    pub note: Option<String>,
    #[serde(rename = "dueDate", alias = "due_date")]
    pub due_date: Option<String>,
    #[serde(rename = "deferDate", alias = "defer_date")]
    pub defer_date: Option<String>,
    #[serde(rename = "plannedDate", alias = "planned_date")]
    pub planned_date: Option<String>,
    pub flagged: Option<LenientBool>,
    pub tags: Option<FlexibleTagList>,
    #[serde(rename = "estimatedMinutes", alias = "estimated_minutes")]
    pub estimated_minutes: Option<LenientI32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MoveTaskParams {
    pub(crate) task_id: String,
    #[schemars(
        description = "Project id or exact name to move into. An unknown project is an error."
    )]
    pub(crate) project: Option<String>,
    pub(crate) parent_task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MoveTasksBatchParams {
    pub(crate) task_ids: Vec<String>,
    #[schemars(
        description = "Project id or exact name to move into. An unknown project is an error."
    )]
    pub(crate) project: Option<String>,
    pub(crate) parent_task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppendToNoteParams {
    pub(crate) object_type: String,
    pub(crate) object_id: String,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeleteTasksBatchParams {
    pub(crate) task_ids: Vec<String>,
}

// `rule_string` has three wire states, and they must not collapse: a string
// sets the rule, an explicit `null` clears it, and an absent key is an error.
// Absent used to clear like `null`, so `{task_id, schedule_type}` silently
// removed a repetition (audit CR-023). The outer `Option` records presence
// (`serde(default)` makes absent `None`), the inner one `null`. The schema
// lists the key as required (`require_rule_string`) and, because
// `skip_serializing_if` suppresses it, advertises no `default: null`.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(transform = require_rule_string)]
pub(crate) struct SetTaskRepetitionParams {
    pub(crate) task_id: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_nullable_string"
    )]
    #[schemars(
        with = "Option<String>",
        description = "required; null clears. an iCalendar recurrence rule such as FREQ=WEEKLY;INTERVAL=1 sets the repetition; null removes it. omitting the key is an error."
    )]
    pub(crate) rule_string: Option<Option<String>>,
    pub(crate) schedule_type: Option<String>,
}

impl SetTaskRepetitionParams {
    /// The rule to set (`Some`) or `None` to clear; an absent `rule_string`
    /// is a validation error naming the field, so it never clears by default.
    pub(crate) fn rule_string(&self) -> Result<Option<&str>> {
        match &self.rule_string {
            Some(rule) => Ok(rule.as_deref()),
            None => Err(OmniFocusError::Validation(
                "rule_string is required: pass a repetition rule to set, or null to clear the repetition."
                    .to_string(),
            )),
        }
    }
}

/// Deserializes a present `rule_string`: `null` becomes `Some(None)`, a string
/// `Some(Some(_))`. Only called when the key is present.
fn present_nullable_string<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Option<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Some)
}

/// Adds `rule_string` to the schema's `required` list.
fn require_rule_string(schema: &mut Schema) {
    let required = schema
        .ensure_object()
        .entry("required")
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(names) = required {
        names.push(Value::String("rule_string".to_string()));
    }
}
