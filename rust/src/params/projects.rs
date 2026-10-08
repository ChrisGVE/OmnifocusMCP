//! Parameter structs of the project tools (see `crate::params` for why they
//! carry no doc comments).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    flexible_tags::FlexibleTagList,
    lenient_scalars::{LenientBool, LenientI32},
};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListProjectsParams {
    #[schemars(
        description = "Folder id or exact name. Only projects directly in this folder are included; an unknown folder is an error."
    )]
    pub(crate) folder: Option<String>,
    pub(crate) status: Option<String>,
    #[serde(rename = "completedBefore", alias = "completed_before")]
    pub(crate) completed_before: Option<String>,
    #[serde(rename = "completedAfter", alias = "completed_after")]
    pub(crate) completed_after: Option<String>,
    #[serde(rename = "stalledOnly", alias = "stalled_only")]
    pub(crate) stalled_only: Option<LenientBool>,
    #[serde(rename = "sortBy", alias = "sort_by")]
    pub(crate) sort_by: Option<String>,
    #[serde(rename = "sortOrder", alias = "sort_order")]
    pub(crate) sort_order: Option<String>,
    pub(crate) limit: Option<LenientI32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetProjectCountsParams {
    #[schemars(
        description = "Folder id or exact name. Only projects directly in this folder are included; an unknown folder is an error."
    )]
    pub(crate) folder: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchProjectsParams {
    pub(crate) query: String,
    pub(crate) limit: Option<LenientI32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectIdOrNameParams {
    pub(crate) project_id_or_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeleteProjectsBatchParams {
    pub(crate) project_ids_or_names: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MoveProjectParams {
    pub(crate) project_id_or_name: String,
    #[schemars(
        description = "Folder id or exact name to move into; omit to move to the top level. An unknown folder is an error."
    )]
    pub(crate) folder: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetProjectStatusParams {
    pub(crate) project_id_or_name: String,
    pub(crate) status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateProjectParams {
    pub(crate) name: String,
    #[schemars(
        description = "Folder id or exact name to create the project in; omit for the top level. An unknown folder is an error."
    )]
    pub(crate) folder: Option<String>,
    pub(crate) note: Option<String>,
    #[serde(rename = "dueDate", alias = "due_date")]
    pub(crate) due_date: Option<String>,
    #[serde(rename = "deferDate", alias = "defer_date")]
    pub(crate) defer_date: Option<String>,
    pub(crate) sequential: Option<LenientBool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateProjectParams {
    pub(crate) project_id_or_name: String,
    pub(crate) name: Option<String>,
    pub(crate) note: Option<String>,
    #[serde(rename = "dueDate", alias = "due_date")]
    pub(crate) due_date: Option<String>,
    #[serde(rename = "deferDate", alias = "defer_date")]
    pub(crate) defer_date: Option<String>,
    pub(crate) flagged: Option<LenientBool>,
    pub(crate) tags: Option<FlexibleTagList>,
    pub(crate) sequential: Option<LenientBool>,
    #[serde(rename = "completedByChildren", alias = "completed_by_children")]
    pub(crate) completed_by_children: Option<LenientBool>,
    #[serde(rename = "reviewInterval", alias = "review_interval")]
    #[schemars(
        description = "review interval as \"N unit\": N a whole number of at least 1, unit one of days, weeks, months, years (singular also accepted, case-insensitive), e.g. \"2 weeks\". the project must already have a review interval."
    )]
    pub(crate) review_interval: Option<String>,
}
