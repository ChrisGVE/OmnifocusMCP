//! The parameter structs of the MCP tools and prompts.
//!
//! rmcp deserialises each call's arguments into one of these structs and
//! derives the tool's advertised JSON schema from it, so field names, serde
//! renames and aliases, and `schemars` descriptions are the wire contract
//! (`tests/params_test.rs` pins it). A doc comment on a struct or field would
//! become a schema description too, which is why they carry none.
//!
//! The structs are grouped like the tools they serve. `crate::server`
//! registers the tools and re-exports the public structs, so paths such as
//! `crate::server::ListTasksParams` keep working.

mod projects;
mod tags_and_folders;
mod tasks;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::lenient_scalars::LenientI32;

pub use projects::{
    CreateProjectParams, ListProjectsParams, SearchProjectsParams, UpdateProjectParams,
};
pub(crate) use projects::{
    DeleteProjectsBatchParams, GetProjectCountsParams, MoveProjectParams, ProjectIdOrNameParams,
    SetProjectStatusParams,
};
pub(crate) use tags_and_folders::{
    CreateFolderParams, CreateTagParams, DeleteFoldersBatchParams, DeleteTagsBatchParams,
    FolderNameOrIdParams, TagNameOrIdParams, UpdateFolderParams, UpdateTagParams,
};
pub use tags_and_folders::{ListTagsParams, SearchTagsParams};
pub use tasks::{
    AddNotificationParams, BatchCreateTaskInput, CreateSubtaskParams, CreateTaskParams,
    CreateTasksBatchParams, DuplicateTaskParams, GetTaskCountsParams, ListTasksParams,
    SearchTasksParams, TaskIdLimitParams, UpdateTaskParams,
};
pub(crate) use tasks::{
    AppendToNoteParams, DeleteTasksBatchParams, MoveTaskParams, MoveTasksBatchParams,
    RemoveNotificationParams, SetTaskRepetitionParams, TaskIdParams,
};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LimitParams {
    pub(crate) limit: Option<LenientI32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectPlanningPromptParams {
    #[schemars(description = "Project id or exact name.")]
    pub(crate) project: String,
}
