//! MCP registrations of the tools that read OmniFocus's own views: the
//! forecast and the list of perspectives.

use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};

use crate::{
    jxa::JxaRunner,
    params::LimitParams,
    tools::{forecast::get_forecast, perspectives::list_perspectives},
};

use super::{as_call_tool_result, to_mcp_error, OmniFocusServer};

#[tool_router(router = view_tools, vis = "pub(super)")]
impl<R: JxaRunner + Send + Sync + 'static> OmniFocusServer<R> {
    #[tool(
        description = "get forecast sections for overdue, due today, flagged, deferred, and due-this-week tasks."
    )]
    async fn get_forecast(
        &self,
        Parameters(params): Parameters<LimitParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = get_forecast(
            self.runner.as_ref(),
            params.limit.map(i32::from).unwrap_or(100),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "list available perspectives including built-in and custom perspectives, deduplicated by id. supports limit."
    )]
    async fn list_perspectives(
        &self,
        Parameters(params): Parameters<LimitParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = list_perspectives(
            self.runner.as_ref(),
            params.limit.map(i32::from).unwrap_or(100),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }
}
