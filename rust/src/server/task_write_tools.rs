//! MCP registrations of the tools that change tasks: create (one, a
//! subtask, a batch), duplicate, update, set repetition, complete and
//! reopen, delete (one or a batch), move (one or a batch), and append to a
//! task or project note.
//!
//! Each method maps its parameter struct onto a builder in
//! `crate::tools::tasks` or `crate::tools::utility`, filling in the
//! documented defaults.

use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};

use crate::{
    flexible_tags::tags_as_opt_vec,
    jxa::JxaRunner,
    params::{
        AppendToNoteParams, CreateSubtaskParams, CreateTaskParams, CreateTasksBatchParams,
        DeleteTasksBatchParams, DuplicateTaskParams, MoveTaskParams, MoveTasksBatchParams,
        SetTaskRepetitionParams, TaskIdParams, UpdateTaskParams,
    },
    tools::{
        tasks::{
            complete_task, create_subtask, create_task, create_tasks_batch, delete_task,
            delete_tasks_batch, duplicate_task, move_task, move_tasks_batch, set_task_repetition,
            update_task, CreateTaskInput,
        },
        utility::{append_to_note as append_to_note_tool, uncomplete_task},
    },
};

use super::{as_call_tool_result, to_mcp_error, OmniFocusServer};

#[tool_router(router = task_write_tools, vis = "pub(super)")]
impl<R: JxaRunner + Send + Sync + 'static> OmniFocusServer<R> {
    #[tool(
        description = "duplicate a task with all its properties. if the task has subtasks, they are cloned too by default.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn duplicate_task(
        &self,
        Parameters(params): Parameters<DuplicateTaskParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = duplicate_task(
            self.runner.as_ref(),
            &params.task_id,
            params.include_children.map(bool::from).unwrap_or(true),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "create one task in inbox or a project (id or exact name). accepts name plus optional note, dates, flagged, tags, and estimated minutes. dates take YYYY-MM-DD or an ISO 8601 date-time; a bare date gets your omnifocus default time for that field. returns the created task (id, name, plannedDate).",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn create_task(
        &self,
        Parameters(params): Parameters<CreateTaskParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = create_task(
            self.runner.as_ref(),
            &params.name,
            params.project.as_deref(),
            params.note.as_deref(),
            params.due_date.as_deref(),
            params.defer_date.as_deref(),
            params.planned_date.as_deref(),
            params.flagged.map(bool::from),
            tags_as_opt_vec(params.tags),
            params.estimated_minutes.map(i32::from),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "create multiple tasks in a single omnijs call. each item accepts the same fields as create_task; returns the created tasks (id, name, plannedDate).",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn create_tasks_batch(
        &self,
        Parameters(params): Parameters<CreateTasksBatchParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let tasks = params
            .tasks
            .into_iter()
            .map(|task| CreateTaskInput {
                name: task.name,
                project: task.project,
                note: task.note,
                due_date: task.due_date,
                defer_date: task.defer_date,
                planned_date: task.planned_date,
                flagged: task.flagged.map(bool::from),
                tags: tags_as_opt_vec(task.tags),
                estimated_minutes: task.estimated_minutes.map(i32::from),
            })
            .collect();
        let result = create_tasks_batch(self.runner.as_ref(), tasks)
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "create a subtask under an existing parent task id. supports optional note, dates, flagged, tags, and estimatedMinutes. dates take YYYY-MM-DD or an ISO 8601 date-time; a bare date gets your omnifocus default time for that field. returns the created task plus parent references.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn create_subtask(
        &self,
        Parameters(params): Parameters<CreateSubtaskParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = create_subtask(
            self.runner.as_ref(),
            &params.name,
            &params.parent_task_id,
            params.note.as_deref(),
            params.due_date.as_deref(),
            params.defer_date.as_deref(),
            params.planned_date.as_deref(),
            params.flagged.map(bool::from),
            tags_as_opt_vec(params.tags),
            params.estimated_minutes.map(i32::from),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "mark a task complete by id. use this for done/completed task lifecycle updates.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn complete_task(
        &self,
        Parameters(params): Parameters<TaskIdParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = complete_task(self.runner.as_ref(), &params.task_id)
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "mark a completed task incomplete by id (reopen task). fails if the task is not currently completed.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn uncomplete_task(
        &self,
        Parameters(params): Parameters<TaskIdParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = uncomplete_task(self.runner.as_ref(), &params.task_id)
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "set or clear a task repetition rule by id. pass rule_string plus schedule_type (regularly/from_completion/none), or null rule_string to clear.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn set_task_repetition(
        &self,
        Parameters(params): Parameters<SetTaskRepetitionParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = set_task_repetition(
            self.runner.as_ref(),
            &params.task_id,
            params.rule_string.as_deref(),
            params.schedule_type.as_deref().unwrap_or("regularly"),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "update an existing task by id, modifying only provided fields. supports name, note, due/defer/planned dates, flagged, tags replacement, and estimatedMinutes. dates take YYYY-MM-DD or an ISO 8601 date-time; a bare date gets your omnifocus default time for that field.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn update_task(
        &self,
        Parameters(params): Parameters<UpdateTaskParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = update_task(
            self.runner.as_ref(),
            &params.task_id,
            params.name.as_deref(),
            params.note.as_deref(),
            params.due_date.as_deref(),
            params.defer_date.as_deref(),
            params.planned_date.as_deref(),
            params.flagged.map(bool::from),
            tags_as_opt_vec(params.tags),
            params.estimated_minutes.map(i32::from),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "delete a task by id. destructive operation: use update_task or move_task for edits/reorganization, and never delete then recreate as a substitute for updating. ask for explicit user confirmation before proceeding.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn delete_task(
        &self,
        Parameters(params): Parameters<TaskIdParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = delete_task(self.runner.as_ref(), &params.task_id)
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "delete multiple tasks by id in a single omnijs call. destructive operation: never use batch delete as a shortcut for edits or reorganization. use update_task/move_task instead when preserving history matters. before calling this tool, always show the user the list of tasks to be deleted and ask for explicit confirmation. do not proceed without user approval.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn delete_tasks_batch(
        &self,
        Parameters(params): Parameters<DeleteTasksBatchParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = delete_tasks_batch(self.runner.as_ref(), params.task_ids)
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "move a task without deleting or recreating it. destination modes: (a) provide project to move to a project, (b) provide parent_task_id to move under an existing parent task, or (c) omit both to move to inbox. move_task preserves the original task object and id by default, and delete is never required for reorganization.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn move_task(
        &self,
        Parameters(params): Parameters<MoveTaskParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = move_task(
            self.runner.as_ref(),
            &params.task_id,
            params.project.as_deref(),
            params.parent_task_id.as_deref(),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "move multiple tasks without deleting or recreating them. destination modes: (a) provide project to move tasks to a project, (b) provide parent_task_id to move tasks under an existing parent task, or (c) omit both to move tasks to inbox. runs as one omnijs call per invocation and returns per-task move results. destructive delete confirmation remains a separate workflow.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn move_tasks_batch(
        &self,
        Parameters(params): Parameters<MoveTasksBatchParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = move_tasks_batch(
            self.runner.as_ref(),
            params.task_ids,
            params.project.as_deref(),
            params.parent_task_id.as_deref(),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "append text to a task or project note by object id without replacing existing note content. returns id/name/type and resulting note length.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn append_to_note(
        &self,
        Parameters(params): Parameters<AppendToNoteParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = append_to_note_tool(
            self.runner.as_ref(),
            &params.object_type,
            &params.object_id,
            &params.text,
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }
}
