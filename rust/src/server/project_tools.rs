//! MCP registrations of the project tools: list, count, search, read,
//! create, complete and reopen, delete (one or a batch), move, update and
//! set status.
//!
//! Each method maps its parameter struct onto a builder in
//! `crate::tools::projects`, filling in the documented defaults.

use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};

use crate::{
    flexible_tags::tags_as_opt_vec,
    jxa::JxaRunner,
    params::{
        CreateProjectParams, DeleteProjectsBatchParams, GetProjectCountsParams, ListProjectsParams,
        MoveProjectParams, ProjectIdOrNameParams, SearchProjectsParams, SetProjectStatusParams,
        UpdateProjectParams,
    },
    tools::projects::{
        complete_project, create_project, delete_project, delete_projects_batch, get_project,
        get_project_counts, list_projects, move_project, search_projects, set_project_status,
        uncomplete_project, update_project,
    },
};

use super::{as_call_tool_result, to_mcp_error, OmniFocusServer};

#[tool_router(router = project_tools, vis = "pub(super)")]
impl<R: JxaRunner + Send + Sync + 'static> OmniFocusServer<R> {
    #[tool(
        description = "list projects with status and folder filters. status semantics: completed means finished work (done), dropped means intentionally abandoned/not-doing, on_hold means paused, active means current. completedBefore/completedAfter take YYYY-MM-DD (local midnight) or an ISO 8601 date-time."
    )]
    async fn list_projects(
        &self,
        Parameters(params): Parameters<ListProjectsParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = list_projects(
            self.runner.as_ref(),
            params.folder.as_deref(),
            params.status.as_deref().unwrap_or("active"),
            params.completed_before.as_deref(),
            params.completed_after.as_deref(),
            params.stalled_only.map(bool::from).unwrap_or(false),
            params.sort_by.as_deref(),
            params.sort_order.as_deref().unwrap_or("asc"),
            params.limit.map(i32::from).unwrap_or(100),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "get aggregate project counts by status without listing individual projects."
    )]
    async fn get_project_counts(
        &self,
        Parameters(params): Parameters<GetProjectCountsParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = get_project_counts(self.runner.as_ref(), params.folder.as_deref())
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "search projects by name text using omnifocus matching. returns lightweight project summaries (id, name, status, folderName)."
    )]
    async fn search_projects(
        &self,
        Parameters(params): Parameters<SearchProjectsParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = search_projects(
            self.runner.as_ref(),
            &params.query,
            params.limit.map(i32::from).unwrap_or(100),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "get full details for one project by id or exact name, including counts, status, dates, next-task hints, and root task summaries."
    )]
    async fn get_project(
        &self,
        Parameters(params): Parameters<ProjectIdOrNameParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = get_project(self.runner.as_ref(), &params.project_id_or_name)
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "create a project with optional folder (id or exact name), note, due/defer dates, and sequential mode. dates take YYYY-MM-DD or an ISO 8601 date-time; a bare date gets your omnifocus default time for that field. returns the created project with the same fields as get_project."
    )]
    async fn create_project(
        &self,
        Parameters(params): Parameters<CreateProjectParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = create_project(
            self.runner.as_ref(),
            &params.name,
            params.folder.as_deref(),
            params.note.as_deref(),
            params.due_date.as_deref(),
            params.defer_date.as_deref(),
            params.sequential.map(bool::from),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "mark a project complete by id or name. use this for finished/closed projects (done/completed), not set_project_status(\"dropped\")."
    )]
    async fn complete_project(
        &self,
        Parameters(params): Parameters<ProjectIdOrNameParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = complete_project(self.runner.as_ref(), &params.project_id_or_name)
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "mark a completed project incomplete by id or name (reopen done work back to active)."
    )]
    async fn uncomplete_project(
        &self,
        Parameters(params): Parameters<ProjectIdOrNameParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = uncomplete_project(self.runner.as_ref(), &params.project_id_or_name)
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "delete a project by id or name. IMPORTANT: this permanently removes the project and all its tasks from the database. never use delete+recreate to apply project changes; use update_project/move_project/set_project_status instead. before calling, show the user the project name and task count, and ask for explicit confirmation."
    )]
    async fn delete_project(
        &self,
        Parameters(params): Parameters<ProjectIdOrNameParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = delete_project(self.runner.as_ref(), &params.project_id_or_name)
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "delete multiple projects by id or exact name in a single omnijs call. destructive operation: this permanently removes each matched project and its tasks. use update_project, move_project, or set_project_status for non-destructive changes. before calling, always show the user which projects are targeted and ask for explicit confirmation."
    )]
    async fn delete_projects_batch(
        &self,
        Parameters(params): Parameters<DeleteProjectsBatchParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = delete_projects_batch(self.runner.as_ref(), params.project_ids_or_names)
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "move a project by id or name to a folder or top level (null folder). use this for organization changes without deleting/recreating."
    )]
    async fn move_project(
        &self,
        Parameters(params): Parameters<MoveProjectParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = move_project(
            self.runner.as_ref(),
            &params.project_id_or_name,
            params.folder.as_deref(),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "update a project by id or name, modifying only provided fields. supports name, note, dates, flagged, tags replacement, sequential, completedByChildren, and reviewInterval. dates take YYYY-MM-DD or an ISO 8601 date-time; a bare date gets your omnifocus default time for that field."
    )]
    async fn update_project(
        &self,
        Parameters(params): Parameters<UpdateProjectParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = update_project(
            self.runner.as_ref(),
            &params.project_id_or_name,
            params.name.as_deref(),
            params.note.as_deref(),
            params.due_date.as_deref(),
            params.defer_date.as_deref(),
            params.flagged.map(bool::from),
            tags_as_opt_vec(params.tags),
            params.sequential.map(bool::from),
            params.completed_by_children.map(bool::from),
            params.review_interval.as_deref(),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "set a project's organizational status by id or name. allowed values: active, on_hold, dropped. semantics: dropped means intentionally abandoned/cancelled (not completed); for finished/closed projects use complete_project instead. when presenting planned/finished changes to users, prefer business-meaning labels (project name, folder, current->target status) and include raw ids only as secondary references."
    )]
    async fn set_project_status(
        &self,
        Parameters(params): Parameters<SetProjectStatusParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = set_project_status(
            self.runner.as_ref(),
            &params.project_id_or_name,
            &params.status,
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }
}
