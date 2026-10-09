//! Pieces shared by the batch delete tools for projects, tags and folders.
//!
//! `projects::delete_projects_batch`, `tags::delete_tags_batch` and
//! `folders::delete_folders_batch` validate their list of ids or names the
//! same way and end their scripts with the same summary, so both live here.
//! Tags and folders form trees, so their two scripts are one: see
//! `tree_batch_delete_script`.
//! (`tasks::delete_tasks_batch` takes ids only and reports differently.)

use std::collections::HashSet;

use crate::{
    error::{OmniFocusError, Result},
    js_helpers::JS_RESOLVERS,
};

/// A kind of object that nests in a tree and is deleted in batches.
pub(crate) struct TreeKind {
    /// The kind's name in an ambiguous-name refusal.
    noun: &'static str,
    /// The `JS_RESOLVERS` function listing every object a value names.
    matcher: &'static str,
    /// The live, database-wide list of objects of this kind.
    collection: &'static str,
}

pub(crate) const TAG_TREE: TreeKind = TreeKind {
    noun: "tag",
    matcher: "matchTags",
    collection: "document.flattenedTags",
};

pub(crate) const FOLDER_TREE: TreeKind = TreeKind {
    noun: "folder",
    matcher: "matchFolders",
    collection: "document.flattenedFolders",
};

/// The script that deletes the tags or folders `ids_or_names_json` (a JSON
/// array of already normalised ids or names) names.
///
/// Every entry is resolved before anything is deleted, so each one names
/// what it named when the call was made. An entry that matches nothing is
/// reported as `"not found"`, and a name that several objects share with the
/// `ambiguousNameMessage` refusal (nothing is deleted for it); the others go
/// ahead. The resolved objects
/// are deleted deepest first, so a child is never removed with its parent
/// before its own turn; one already gone (removed with a parent, or named
/// twice) counts as deleted.
pub(crate) fn tree_batch_delete_script(kind: &TreeKind, ids_or_names_json: &str) -> String {
    let TreeKind {
        noun,
        matcher,
        collection,
    } = kind;
    format!(
        r#"{JS_RESOLVERS}
const idsOrNames = {ids_or_names_json};
const findLiveById = (id) => {collection}.find(object => {{
  try {{
    return object.id.primaryKey === id;
  }} catch (e) {{
    return false;
  }}
}});
const treeDepth = (object) => {{
  let depth = 0;
  for (let parent = object.parent; parent; parent = parent.parent) depth += 1;
  return depth;
}};

const results = new Array(idsOrNames.length);
const resolved = [];
idsOrNames.forEach((idOrName, index) => {{
  const matches = {matcher}(idOrName);
  if (matches.length !== 1) {{
    const error = matches.length === 0 ? "not found" : ambiguousNameMessage("{noun}", idOrName, matches);
    results[index] = {{ id_or_name: idOrName, id: null, name: null, deleted: false, error }};
    return;
  }}
  const object = matches[0];
  resolved.push({{ idOrName, index, id: object.id.primaryKey, name: object.name, depth: treeDepth(object) }});
}});

resolved
  .sort((left, right) => right.depth - left.depth || left.index - right.index)
  .forEach(request => {{
    const deletedResult = {{ id_or_name: request.idOrName, id: request.id, name: request.name, deleted: true, error: null }};
    const liveObject = findLiveById(request.id);
    if (!liveObject) {{
      results[request.index] = deletedResult;
      return;
    }}
    try {{
      deleteObject(liveObject);
      results[request.index] = deletedResult;
    }} catch (e) {{
      if (!findLiveById(request.id)) {{
        results[request.index] = deletedResult;
        return;
      }}
      const errorMessage = e && e.message ? String(e.message) : String(e);
      results[request.index] = {{ ...deletedResult, deleted: false, error: errorMessage }};
    }}
  }});

{BATCH_DELETE_SUMMARY}"#
    )
}

/// Trims each id or name and rejects an empty list, an empty entry or a
/// repeated entry. `kind` ("project", "tag" or "folder") names the argument
/// in the error, e.g. `tag_ids_or_names must not contain duplicates: x`.
pub(crate) fn normalize_ids_or_names(kind: &str, values: Vec<String>) -> Result<Vec<String>> {
    if values.is_empty() {
        return Err(OmniFocusError::Validation(format!(
            "{kind}_ids_or_names must contain at least one {kind} id or name."
        )));
    }

    let mut normalized: Vec<String> = Vec::with_capacity(values.len());
    let mut seen: HashSet<String> = HashSet::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(OmniFocusError::Validation(format!(
                "each {kind} id or name must be a non-empty string."
            )));
        }
        if seen.contains(trimmed) {
            return Err(OmniFocusError::Validation(format!(
                "{kind}_ids_or_names must not contain duplicates: {trimmed}"
            )));
        }
        seen.insert(trimmed.to_string());
        normalized.push(trimmed.to_string());
    }
    Ok(normalized)
}

/// Script tail that returns the batch result: counts of deleted and failed
/// entries plus the per-entry `results` array the script built before it.
pub(crate) const BATCH_DELETE_SUMMARY: &str = r#"const deletedCount = results.filter(result => result.deleted).length;
const failedCount = results.length - deletedCount;

return {
  summary: {
    requested: results.length,
    deleted: deletedCount,
    failed: failedCount
  },
  partial_success: deletedCount > 0 && failedCount > 0,
  results: results
};"#;
