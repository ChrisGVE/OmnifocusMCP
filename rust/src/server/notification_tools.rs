//! MCP registrations of the task notification tools: list, add and
//! remove the notifications of one task.

use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};

use crate::{
    jxa::JxaRunner,
    params::{AddNotificationParams, RemoveNotificationParams, TaskIdParams},
    tools::tasks::{add_notification, list_notifications, remove_notification},
};

use super::{as_call_tool_result, to_mcp_error, OmniFocusServer};

#[tool_router(router = notification_tools, vis = "pub(super)")]
impl<R: JxaRunner + Send + Sync + 'static> OmniFocusServer<R> {
    #[tool(
        description = "list active notifications for a task by id. returns notification id, kind, absolute/relative schedule fields, next fire date, and snooze state."
    )]
    async fn list_notifications(
        &self,
        Parameters(params): Parameters<TaskIdParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = list_notifications(self.runner.as_ref(), &params.task_id)
            .await
            .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(
        description = "add one notification to a task by id. provide exactly one of absoluteDate or relativeOffset. absoluteDate takes YYYY-MM-DD (local midnight) or an ISO 8601 date-time. relativeOffset requires a task with an effective due date. returns created notification summary."
    )]
    async fn add_notification(
        &self,
        Parameters(params): Parameters<AddNotificationParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = add_notification(
            self.runner.as_ref(),
            &params.task_id,
            params.absolute_date.as_deref(),
            params.relative_offset.map(f64::from),
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }

    #[tool(description = "remove one notification from a task by task_id and notification_id.")]
    async fn remove_notification(
        &self,
        Parameters(params): Parameters<RemoveNotificationParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = remove_notification(
            self.runner.as_ref(),
            &params.task_id,
            &params.notification_id,
        )
        .await
        .map_err(to_mcp_error)?;
        as_call_tool_result(&result)
    }
}
