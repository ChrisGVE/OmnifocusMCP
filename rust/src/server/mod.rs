//! The MCP server: tool, prompt and resource registration around the
//! OmniJS script builders in `crate::tools`.
//!
//! `OmniFocusServer` owns the runner that executes the scripts. Its tools
//! are registered per area, one `#[tool_router]` block per submodule
//! (`task_read_tools`, `task_write_tools`, `notification_tools`,
//! `project_tools`, `tag_tools`, `folder_tools`, `view_tools`), and
//! `tool_router` adds those routers into the one rmcp dispatches through.
//! Prompts and resources are registered here. The parameter structs live in
//! `crate::params`; the public ones are re-exported here.

mod folder_tools;
mod notification_tools;
mod project_tools;
mod tag_tools;
mod task_read_tools;
mod task_write_tools;
mod view_tools;

use rmcp::{
    handler::server::{
        router::{prompt::PromptRouter, tool::ToolRouter},
        wrapper::Parameters,
    },
    model::{
        CallToolResult, Content, GetPromptRequestParams, GetPromptResult, Implementation,
        ListPromptsResult, ListResourcesResult, PaginatedRequestParams, PromptMessage,
        PromptMessageRole, RawResource, ReadResourceRequestParams, ReadResourceResult,
        ResourceContents, ServerCapabilities, ServerInfo,
    },
    prompt, prompt_handler, prompt_router, tool_handler, ErrorData as McpError, ServerHandler,
};
use rmcp::{service::RequestContext, RoleServer};
use serde::Serialize;
use std::sync::Arc;

use crate::{
    error::OmniFocusError,
    jxa::JxaRunner,
    prompts::{daily_review, inbox_processing, project_planning, weekly_review},
    resources::{
        inbox_resource, projects_resource, today_resource, INBOX_RESOURCE_URI,
        PROJECTS_RESOURCE_URI, TODAY_RESOURCE_URI,
    },
};

pub use crate::params::{
    AddNotificationParams, BatchCreateTaskInput, CreateProjectParams, CreateSubtaskParams,
    CreateTaskParams, CreateTasksBatchParams, DuplicateTaskParams, GetTaskCountsParams,
    LimitParams, ListProjectsParams, ListTagsParams, ListTasksParams, ProjectPlanningPromptParams,
    SearchProjectsParams, SearchTagsParams, SearchTasksParams, TaskIdLimitParams,
    UpdateProjectParams, UpdateTaskParams,
};

#[derive(Clone)]
pub struct OmniFocusServer<R: JxaRunner + Send + Sync + 'static> {
    runner: Arc<R>,
    runner_dyn: Arc<dyn JxaRunner>,
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
}

impl<R: JxaRunner + Send + Sync + 'static> OmniFocusServer<R> {
    pub fn new(runner: R) -> Self {
        let runner = Arc::new(runner);
        let runner_dyn: Arc<dyn JxaRunner> = runner.clone();
        Self {
            runner,
            runner_dyn,
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        }
    }

    /// Every tool, gathered from the per-area routers of the submodules.
    fn tool_router() -> ToolRouter<Self> {
        Self::task_read_tools()
            + Self::task_write_tools()
            + Self::notification_tools()
            + Self::project_tools()
            + Self::tag_tools()
            + Self::folder_tools()
            + Self::view_tools()
    }
}

fn to_mcp_error(error: OmniFocusError) -> McpError {
    match error {
        OmniFocusError::Validation(message) => McpError::invalid_params(message, None),
        _ => McpError::internal_error(error.to_string(), None),
    }
}

fn as_call_tool_result<T: Serialize>(value: &T) -> std::result::Result<CallToolResult, McpError> {
    let text = serde_json::to_string(value)
        .map_err(|error| McpError::internal_error(error.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

#[prompt_router]
impl<R: JxaRunner + Send + Sync + 'static> OmniFocusServer<R> {
    #[prompt(description = "daily planning prompt with due-soon, overdue, and flagged tasks.")]
    async fn daily_review(&self) -> std::result::Result<Vec<PromptMessage>, McpError> {
        let text = daily_review(self.runner.as_ref())
            .await
            .map_err(to_mcp_error)?;
        Ok(vec![PromptMessage::new_text(PromptMessageRole::User, text)])
    }

    #[prompt(description = "weekly review prompt with active projects and next-action coverage.")]
    async fn weekly_review(&self) -> std::result::Result<Vec<PromptMessage>, McpError> {
        let text = weekly_review(self.runner.as_ref())
            .await
            .map_err(to_mcp_error)?;
        Ok(vec![PromptMessage::new_text(PromptMessageRole::User, text)])
    }

    #[prompt(
        description = "inbox processing prompt that drives one-by-one clarification decisions."
    )]
    async fn inbox_processing(&self) -> std::result::Result<Vec<PromptMessage>, McpError> {
        let text = inbox_processing(self.runner.as_ref())
            .await
            .map_err(to_mcp_error)?;
        Ok(vec![PromptMessage::new_text(PromptMessageRole::User, text)])
    }

    #[prompt(
        description = "project planning prompt that turns a project into actionable next steps."
    )]
    async fn project_planning(
        &self,
        Parameters(params): Parameters<ProjectPlanningPromptParams>,
    ) -> std::result::Result<Vec<PromptMessage>, McpError> {
        let text = project_planning(self.runner.as_ref(), &params.project)
            .await
            .map_err(to_mcp_error)?;
        Ok(vec![PromptMessage::new_text(PromptMessageRole::User, text)])
    }
}

#[tool_handler(router = self.tool_router)]
#[prompt_handler(router = self.prompt_router)]
impl<R: JxaRunner + Send + Sync + 'static> ServerHandler for OmniFocusServer<R> {
    fn get_info(&self) -> ServerInfo {
        let _ = &self.runner_dyn;
        ServerInfo {
            instructions: Some(
                "OmniFocus MCP server exposing tools, resources, and prompts. treat conversations as active omnifocus workflows: ground responses in omnifocus data, engage users with concise clarifying questions when needed, propose concrete next actions, and offer to apply approved changes via tool calls. communicate at business-meaning level first: show object names and context (project/folder/status/counts), and use raw ids only as secondary references. project lifecycle semantics: complete_project is for finished/closed work; set_project_status(\"dropped\") means intentionally abandoned/cancelled; set_project_status(\"on_hold\") means paused. for user-requested changes, preserve existing objects by default and prefer update/move tools; never delete and recreate tasks/projects/folders as a shortcut unless the user explicitly asks for deletion. ask explicit confirmation before destructive operations and report resulting object ids after writes."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
            // rmcp's default identity comes from rmcp's own build
            // environment ("rmcp" and rmcp's version), so name this crate.
            server_info: Implementation {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                ..Implementation::default()
            },
            ..Default::default()
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<ListResourcesResult, McpError> {
        use rmcp::model::AnnotateAble;

        Ok(ListResourcesResult::with_all_items(vec![
            RawResource::new(INBOX_RESOURCE_URI, "Inbox tasks").no_annotation(),
            RawResource::new(TODAY_RESOURCE_URI, "Today forecast").no_annotation(),
            RawResource::new(PROJECTS_RESOURCE_URI, "Active projects").no_annotation(),
        ]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<ReadResourceResult, McpError> {
        let uri = request.uri;
        let content = match uri.as_str() {
            INBOX_RESOURCE_URI => inbox_resource(self.runner.as_ref())
                .await
                .map_err(to_mcp_error)?,
            TODAY_RESOURCE_URI => today_resource(self.runner.as_ref())
                .await
                .map_err(to_mcp_error)?,
            PROJECTS_RESOURCE_URI => projects_resource(self.runner.as_ref())
                .await
                .map_err(to_mcp_error)?,
            _ => {
                return Err(McpError::invalid_params(
                    format!("Unknown resource URI: {uri}"),
                    None,
                ));
            }
        };

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(content, uri)],
        })
    }
}
