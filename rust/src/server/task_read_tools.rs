//! MCP registrations of the task read tools: the inbox, listing,
//! searching and counting tasks, one task's details, and its subtasks.
//!
//! Each method maps its parameter struct onto a builder in
//! `crate::tools::tasks`, filling in the documented defaults.

use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};

use crate::{
    flexible_tags::tags_as_opt_vec,
    jxa::JxaRunner,
    params::{
        GetTaskCountsParams, LimitParams, ListTasksParams, SearchTasksParams, TaskIdLimitParams,
        TaskIdParams,
    },
    tools::tasks::{
        get_inbox, get_task, get_task_counts_with_added_changed, list_subtasks,
        list_tasks_with_added_changed, search_tasks_with_added_changed,
    },
};

use super::{as_call_tool_result, to_mcp_error, OmniFocusServer};

#[tool_router(router = task_read_tools, vis = "pub(super)")]
impl<R: JxaRunner + Send + Sync + 'static> OmniFocusServer<R> {
    #[tool(
        description = "get inbox tasks from omnifocus. returns unprocessed inbox tasks with id, name, note, flagged, due/defer/completion dates, tags, estimated minutes, and taskStatus. supports limit.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_inbox(
        &self,
        Parameters(params): Parameters<LimitParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = get_inbox(
            self.runner.as_ref(),
            params.limit.map(i32::from).unwrap_or(100),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "list tasks with optional project/tag filters, status, date ranges, and sorting. canonical status values are available, due_soon, overdue, on_hold, completed, and all. date filters take YYYY-MM-DD (local midnight) or an ISO 8601 date-time; changed maps to task.modified. accepted aliases (case-insensitive): sortOrder ascending/descending, status due soon or due-soon and on hold or on-hold, and tagFilterMode and/or. sortBy accepts dueDate, deferDate, name, completionDate, estimatedMinutes, project, flagged, addedDate, changedDate, plannedDate, and aliases added/modified/planned. returns task summaries.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_tasks(
        &self,
        Parameters(params): Parameters<ListTasksParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let ListTasksParams {
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
            added_after,
            added_before,
            changed_after,
            changed_before,
            max_estimated_minutes,
            planned_before,
            planned_after,
            sort_by,
            sort_order,
            limit,
        } = params;
        let result = list_tasks_with_added_changed(
            self.runner.as_ref(),
            project.as_deref(),
            tag.as_deref(),
            tags_as_opt_vec(tags),
            tag_filter_mode.as_deref().unwrap_or("any"),
            flagged.map(bool::from),
            status.as_deref().unwrap_or("available"),
            due_before.as_deref(),
            due_after.as_deref(),
            defer_before.as_deref(),
            defer_after.as_deref(),
            completed_before.as_deref(),
            completed_after.as_deref(),
            added_after.as_deref(),
            added_before.as_deref(),
            changed_after.as_deref(),
            changed_before.as_deref(),
            planned_before.as_deref(),
            planned_after.as_deref(),
            max_estimated_minutes.map(i32::from),
            sort_by.as_deref(),
            sort_order.as_deref().unwrap_or("asc"),
            limit.map(i32::from).unwrap_or(100),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "get aggregate task counts for any filter combination without listing individual tasks. date filters take YYYY-MM-DD (local midnight) or an ISO 8601 date-time; changed means the task's last modified timestamp. tagFilterMode accepts canonical any/all and aliases and/or (case-insensitive). much faster than list_tasks for answering 'how many' questions.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_task_counts(
        &self,
        Parameters(params): Parameters<GetTaskCountsParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = get_task_counts_with_added_changed(
            self.runner.as_ref(),
            params.project.as_deref(),
            params.tag.as_deref(),
            tags_as_opt_vec(params.tags),
            params.tag_filter_mode.as_deref().unwrap_or("any"),
            params.flagged.map(bool::from),
            params.due_before.as_deref(),
            params.due_after.as_deref(),
            params.defer_before.as_deref(),
            params.defer_after.as_deref(),
            params.completed_before.as_deref(),
            params.completed_after.as_deref(),
            params.added_after.as_deref(),
            params.added_before.as_deref(),
            params.changed_after.as_deref(),
            params.changed_before.as_deref(),
            params.planned_before.as_deref(),
            params.planned_after.as_deref(),
            params.max_estimated_minutes.map(i32::from),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "get full details for one task by id. returns list_tasks fields plus children, parentName, sequential, repetitionRule, effective dates, and task status fields.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_task(
        &self,
        Parameters(params): Parameters<TaskIdParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = get_task(self.runner.as_ref(), &params.task_id)
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "list direct subtasks for a parent task id. returns summary fields for direct children only, limited by limit.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_subtasks(
        &self,
        Parameters(params): Parameters<TaskIdLimitParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = list_subtasks(
            self.runner.as_ref(),
            &params.task_id,
            params.limit.map(i32::from).unwrap_or(100),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "search tasks by case-insensitive name/note text with optional filters and sorting. canonical status values are available, due_soon, overdue, on_hold, completed, and all. date filters take YYYY-MM-DD (local midnight) or an ISO 8601 date-time; changed maps to task.modified. accepted aliases (case-insensitive): sortOrder ascending/descending, status due soon or due-soon and on hold or on-hold, and tagFilterMode and/or. sortBy accepts dueDate, deferDate, name, completionDate, estimatedMinutes, project, flagged, addedDate, changedDate, plannedDate, and aliases added/modified/planned. returns task summaries.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn search_tasks(
        &self,
        Parameters(params): Parameters<SearchTasksParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = search_tasks_with_added_changed(
            self.runner.as_ref(),
            &params.query,
            params.project.as_deref(),
            params.tag.as_deref(),
            tags_as_opt_vec(params.tags),
            params.tag_filter_mode.as_deref().unwrap_or("any"),
            params.flagged.map(bool::from),
            params.status.as_deref().unwrap_or("available"),
            params.due_before.as_deref(),
            params.due_after.as_deref(),
            params.defer_before.as_deref(),
            params.defer_after.as_deref(),
            params.completed_before.as_deref(),
            params.completed_after.as_deref(),
            params.added_after.as_deref(),
            params.added_before.as_deref(),
            params.changed_after.as_deref(),
            params.changed_before.as_deref(),
            params.planned_before.as_deref(),
            params.planned_after.as_deref(),
            params.max_estimated_minutes.map(i32::from),
            params.sort_by.as_deref(),
            params.sort_order.as_deref().unwrap_or("asc"),
            params.limit.map(i32::from).unwrap_or(100),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }
}
