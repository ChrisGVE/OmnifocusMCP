//! MCP registrations of the tag tools: search, list, create, update and
//! delete (one or a batch).
//!
//! Each method maps its parameter struct onto a builder in
//! `crate::tools::tags`, filling in the documented defaults.

use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};

use crate::{
    jxa::JxaRunner,
    params::{
        CreateTagParams, DeleteTagsBatchParams, ListTagsParams, SearchTagsParams,
        TagNameOrIdParams, UpdateTagParams,
    },
    tools::tags::{create_tag, delete_tag, delete_tags_batch, list_tags, search_tags, update_tag},
};

use super::{as_call_tool_result, to_mcp_error, OmniFocusServer};

#[tool_router(router = tag_tools, vis = "pub(super)")]
impl<R: JxaRunner + Send + Sync + 'static> OmniFocusServer<R> {
    #[tool(
        description = "search tags by name text using omnifocus matching. returns lightweight tag summaries (id, name, parent)."
    )]
    async fn search_tags(
        &self,
        Parameters(params): Parameters<SearchTagsParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = search_tags(
            self.runner.as_ref(),
            &params.query,
            params.limit.map(i32::from).unwrap_or(100),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "list tags with optional statusFilter and sorting; returns id/name/parent plus availableTaskCount and totalTaskCount."
    )]
    async fn list_tags(
        &self,
        Parameters(params): Parameters<ListTagsParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = list_tags(
            self.runner.as_ref(),
            params.status_filter.as_deref().unwrap_or("all"),
            params.sort_by.as_deref(),
            params.sort_order.as_deref().unwrap_or("asc"),
            params.limit.map(i32::from).unwrap_or(100),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "create a tag with optional parent tag name; returns the created tag (id, name, parent, availableTaskCount, totalTaskCount, status)."
    )]
    async fn create_tag(
        &self,
        Parameters(params): Parameters<CreateTagParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = create_tag(self.runner.as_ref(), &params.name, params.parent.as_deref())
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(description = "update a tag by id or name, modifying provided name and/or status.")]
    async fn update_tag(
        &self,
        Parameters(params): Parameters<UpdateTagParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = update_tag(
            self.runner.as_ref(),
            &params.tag_name_or_id,
            params.name.as_deref(),
            params.status.as_deref(),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "delete a tag by id or name. destructive operation: this removes the tag and unassigns it from linked tasks. use update_tag for non-destructive edits. before calling, ask the user for explicit confirmation."
    )]
    async fn delete_tag(
        &self,
        Parameters(params): Parameters<TagNameOrIdParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = delete_tag(self.runner.as_ref(), &params.tag_name_or_id)
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "delete multiple tags by id or exact name in a single omnijs call. destructive operation: this removes tags and unassigns them from linked tasks. use update_tag for non-destructive edits. before calling, always show the user which tags are targeted and ask for explicit confirmation."
    )]
    async fn delete_tags_batch(
        &self,
        Parameters(params): Parameters<DeleteTagsBatchParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = delete_tags_batch(self.runner.as_ref(), params.tag_ids_or_names)
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }
}
