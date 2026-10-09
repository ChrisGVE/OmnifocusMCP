//! MCP registrations of the folder tools: list, create, read, update and
//! delete (one or a batch).
//!
//! Each method maps its parameter struct onto a builder in
//! `crate::tools::folders`, filling in the documented defaults.

use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};

use crate::{
    jxa::JxaRunner,
    params::{
        CreateFolderParams, DeleteFoldersBatchParams, FolderNameOrIdParams, LimitParams,
        UpdateFolderParams,
    },
    tools::folders::{
        create_folder, delete_folder as delete_folder_tool,
        delete_folders_batch as delete_folders_batch_tool, get_folder, list_folders,
        update_folder as update_folder_tool,
    },
};

use super::{tool_result, OmniFocusServer};

#[tool_router(router = folder_tools, vis = "pub(super)")]
impl<R: JxaRunner + Send + Sync + 'static> OmniFocusServer<R> {
    #[tool(
        description = "list folders with hierarchy context and project counts. returns id, name, parentName, and projectCount. supports limit.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_folders(
        &self,
        Parameters(params): Parameters<LimitParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = list_folders(
            self.runner.as_ref(),
            params.limit.map(i32::from).unwrap_or(100),
        )
        .await;
        tool_result(result)
    }

    #[tool(
        description = "create a folder with optional parent folder (id or exact name). returns the created folder's id, name, and parentName (the parent folder's name, null at the top level).",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn create_folder(
        &self,
        Parameters(params): Parameters<CreateFolderParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result =
            create_folder(self.runner.as_ref(), &params.name, params.parent.as_deref()).await;
        tool_result(result)
    }

    #[tool(
        description = "get full details for one folder by id or name, including direct child projects and direct subfolders.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_folder(
        &self,
        Parameters(params): Parameters<FolderNameOrIdParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = get_folder(self.runner.as_ref(), &params.folder_name_or_id).await;
        tool_result(result)
    }

    #[tool(
        description = "update a folder by id or name, modifying provided name and/or status.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn update_folder(
        &self,
        Parameters(params): Parameters<UpdateFolderParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = update_folder_tool(
            self.runner.as_ref(),
            &params.folder_name_or_id,
            params.name.as_deref(),
            params.status.as_deref(),
        )
        .await;
        tool_result(result)
    }

    #[tool(
        description = "delete a folder by id or name. warning: this permanently removes the folder. do not use delete+recreate for folder edits or renames; use update_folder instead. contained projects may be moved to top level by omnifocus, so confirm with the user before proceeding.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn delete_folder(
        &self,
        Parameters(params): Parameters<FolderNameOrIdParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result = delete_folder_tool(self.runner.as_ref(), &params.folder_name_or_id).await;
        tool_result(result)
    }

    #[tool(
        description = "delete multiple folders by id or exact name in a single omnijs call. destructive operation: this permanently removes folders and may move contained projects depending on omnifocus behavior. use update_folder for non-destructive edits. before calling, always show the user which folders are targeted and ask for explicit confirmation.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn delete_folders_batch(
        &self,
        Parameters(params): Parameters<DeleteFoldersBatchParams>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let result =
            delete_folders_batch_tool(self.runner.as_ref(), params.folder_ids_or_names).await;
        tool_result(result)
    }
}
