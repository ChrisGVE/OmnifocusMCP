//! Parameter structs of the task tools (see `crate::params` for why they
//! carry no doc comments).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
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

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetTaskRepetitionParams {
    pub(crate) task_id: String,
    pub(crate) rule_string: Option<String>,
    pub(crate) schedule_type: Option<String>,
}
