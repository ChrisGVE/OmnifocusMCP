//! Parameter structs of the tag and folder tools (see `crate::params` for
//! why they carry no doc comments).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::lenient_scalars::LenientI32;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchTagsParams {
    pub(crate) query: String,
    pub(crate) limit: Option<LenientI32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListTagsParams {
    #[serde(rename = "statusFilter", alias = "status_filter")]
    pub(crate) status_filter: Option<String>,
    #[serde(rename = "sortBy", alias = "sort_by")]
    pub(crate) sort_by: Option<String>,
    #[serde(rename = "sortOrder", alias = "sort_order")]
    pub(crate) sort_order: Option<String>,
    pub(crate) limit: Option<LenientI32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateTagParams {
    pub(crate) name: String,
    pub(crate) parent: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateFolderParams {
    pub(crate) name: String,
    #[schemars(
        description = "Folder id or exact name of the parent folder; omit for the top level. An unknown folder is an error."
    )]
    pub(crate) parent: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct FolderNameOrIdParams {
    pub(crate) folder_name_or_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateFolderParams {
    pub(crate) folder_name_or_id: String,
    pub(crate) name: Option<String>,
    pub(crate) status: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateTagParams {
    pub(crate) tag_name_or_id: String,
    pub(crate) name: Option<String>,
    pub(crate) status: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TagNameOrIdParams {
    pub(crate) tag_name_or_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeleteTagsBatchParams {
    pub(crate) tag_ids_or_names: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeleteFoldersBatchParams {
    pub(crate) folder_ids_or_names: Vec<String>,
}
