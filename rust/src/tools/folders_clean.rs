use serde_json::Value;

use crate::{
    error::{to_json_string, OmniFocusError, Result},
    js_helpers::{JS_FOLDER_STATUS, JS_PROJECT_STATUS, JS_RESOLVERS},
    jxa::{escape_for_jxa, JxaRunner},
    tools::batch_delete::{normalize_ids_or_names, BATCH_DELETE_SUMMARY},
};

pub async fn list_folders<R: JxaRunner>(runner: &R, limit: i32) -> Result<Value> {
    if limit < 1 {
        return Err(OmniFocusError::Validation(
            "limit must be greater than 0.".to_string(),
        ));
    }

    let script = format!(
        r#"const folderProjectCounts = new Map();
document.flattenedProjects.forEach(project => {{
  const folder = project.parentFolder;
  if (!folder) return;
  const folderId = folder.id.primaryKey;
  const current = folderProjectCounts.get(folderId) || 0;
  folderProjectCounts.set(folderId, current + 1);
}});

const folders = document.flattenedFolders.slice(0, {limit});
return folders.map(folder => {{
  return {{
    id: folder.id.primaryKey,
    name: folder.name,
    parentName: folder.parent ? folder.parent.name : null,
    projectCount: folderProjectCounts.get(folder.id.primaryKey) || 0
  }};
}});"#
    );
    runner.run_omnijs(&script).await
}

pub async fn create_folder<R: JxaRunner>(
    runner: &R,
    name: &str,
    parent: Option<&str>,
) -> Result<Value> {
    if name.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "name must not be empty.".to_string(),
        ));
    }
    if let Some(parent_name) = parent {
        if parent_name.trim().is_empty() {
            return Err(OmniFocusError::Validation(
                "parent must not be empty when provided.".to_string(),
            ));
        }
    }

    let folder_name = escape_for_jxa(name.trim());
    let parent_name = parent
        .map(|value| escape_for_jxa(value.trim()))
        .unwrap_or_else(|| "null".to_string());
    let script = format!(
        r#"{JS_RESOLVERS}
const folderName = {folder_name};
const parentName = {parent_name};

const folder = (() => {{
  if (parentName === null) return new Folder(folderName);
  const parentFolder = resolveFolder(parentName);
  return new Folder(folderName, parentFolder.ending);
}})();

return {{
  id: folder.id.primaryKey,
  name: folder.name,
  parentName: folder.parent ? folder.parent.name : null
}};"#
    );
    runner.run_omnijs(&script).await
}

pub async fn get_folder<R: JxaRunner>(runner: &R, folder_name_or_id: &str) -> Result<Value> {
    if folder_name_or_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "folder_name_or_id must not be empty.".to_string(),
        ));
    }

    let folder_filter = escape_for_jxa(folder_name_or_id.trim());
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_FOLDER_STATUS}
{JS_RESOLVERS}
const folderFilter = {folder_filter};
const folder = resolveFolder(folderFilter);

return {{
  id: folder.id.primaryKey,
  name: folder.name,
  status: normalizeFolderStatus(folder),
  parentName: folder.parent ? folder.parent.name : null,
  projects: folder.projects.map(project => {{
    return {{
      id: project.id.primaryKey,
      name: project.name,
      status: normalizeProjectStatus(project)
    }};
  }}),
  subfolders: folder.folders.map(subfolder => {{
    return {{
      id: subfolder.id.primaryKey,
      name: subfolder.name
    }};
  }})
}};"#
    );
    runner.run_omnijs(&script).await
}

pub async fn update_folder<R: JxaRunner>(
    runner: &R,
    folder_name_or_id: &str,
    name: Option<&str>,
    status: Option<&str>,
) -> Result<Value> {
    let folder_filter = folder_name_or_id.trim();
    let new_name_value = name.map(str::trim);
    let status_value = status;

    if folder_filter.is_empty() {
        return Err(OmniFocusError::Validation(
            "folder_name_or_id must not be empty.".to_string(),
        ));
    }
    if name.is_some() && new_name_value == Some("") {
        return Err(OmniFocusError::Validation(
            "name must not be empty when provided.".to_string(),
        ));
    }
    if let Some(value) = status_value {
        if !matches!(value, "active" | "dropped") {
            return Err(OmniFocusError::Validation(
                "status must be one of: active, dropped.".to_string(),
            ));
        }
    }
    if new_name_value.is_none() && status_value.is_none() {
        return Err(OmniFocusError::Validation(
            "at least one field must be provided: name or status.".to_string(),
        ));
    }

    let escaped_folder_filter = escape_for_jxa(folder_filter);
    let escaped_name = new_name_value
        .map(escape_for_jxa)
        .unwrap_or_else(|| "null".to_string());
    let escaped_status = status_value
        .map(escape_for_jxa)
        .unwrap_or_else(|| "null".to_string());
    let script = format!(
        r#"{JS_FOLDER_STATUS}
{JS_RESOLVERS}
const folderFilter = {escaped_folder_filter};
const newName = {escaped_name};
const statusValue = {escaped_status};
const folder = resolveFolder(folderFilter);

if (newName !== null) {{
  folder.name = newName;
}}

if (statusValue !== null) {{
  let targetStatus;
  if (statusValue === "active") {{
    targetStatus = Folder.Status.Active;
  }} else if (statusValue === "dropped") {{
    targetStatus = Folder.Status.Dropped;
  }} else {{
    throw new Error(`Invalid status: ${{statusValue}}`);
  }}
  folder.status = targetStatus;
}}

return {{
  id: folder.id.primaryKey,
  name: folder.name,
  status: normalizeFolderStatus(folder)
}};"#
    );
    runner.run_omnijs(&script).await
}

pub async fn delete_folder<R: JxaRunner>(runner: &R, folder_name_or_id: &str) -> Result<Value> {
    let folder_filter = folder_name_or_id.trim();
    if folder_filter.is_empty() {
        return Err(OmniFocusError::Validation(
            "folder_name_or_id must not be empty.".to_string(),
        ));
    }

    let escaped_folder_filter = escape_for_jxa(folder_filter);
    let script = format!(
        r#"{JS_RESOLVERS}
const folderFilter = {escaped_folder_filter};
const folder = resolveFolder(folderFilter);

const folderId = folder.id.primaryKey;
const folderName = folder.name;
const projectCount = folder.projects.length;
const subfolderCount = folder.folders.length;
deleteObject(folder);

return {{
  id: folderId,
  name: folderName,
  deleted: true,
  projectCount: projectCount,
  subfolderCount: subfolderCount
}};"#
    );
    runner.run_omnijs(&script).await
}

pub async fn delete_folders_batch<R: JxaRunner>(
    runner: &R,
    folder_ids_or_names: Vec<String>,
) -> Result<Value> {
    let normalized_folder_ids_or_names = normalize_ids_or_names("folder", folder_ids_or_names)?;

    let folder_ids_or_names_value = to_json_string(&normalized_folder_ids_or_names)?;
    let script = format!(
        r#"const folderIdsOrNames = {folder_ids_or_names_value};
const requests = folderIdsOrNames.map((idOrName, index) => ({{ idOrName, index }}));
{FOLDER_LOOKUP}

{DELETE_FOLDERS_DEEPEST_FIRST}

{BATCH_DELETE_SUMMARY}"#
    );
    runner.run_omnijs(&script).await
}

/// Snapshots every folder's id, name and parent and defines lookups over
/// it: by id or name, by depth in the folder tree, and back to the live
/// folder.
const FOLDER_LOOKUP: &str = r#"const folders = document.flattenedFolders
  .map(item => {
    try {
      return {
        id: item.id.primaryKey,
        name: item.name,
        parentId: item.parent ? item.parent.id.primaryKey : null
      };
    } catch (e) {
      return null;
    }
  })
  .filter(item => item !== null);
const foldersById = new Map(folders.map(folder => [folder.id, folder]));

const resolveFolder = (idOrName) => {
  const byId = foldersById.get(idOrName);
  if (byId) return byId;
  return folders.find(folder => folder.name === idOrName);
};

const depthCache = new Map();
const getDepth = (folderId, stack = new Set()) => {
  if (depthCache.has(folderId)) return depthCache.get(folderId);
  if (stack.has(folderId)) return 0;
  stack.add(folderId);
  const folder = foldersById.get(folderId);
  let depth = 0;
  if (folder && folder.parentId && foldersById.has(folder.parentId)) {
    depth = getDepth(folder.parentId, stack) + 1;
  }
  stack.delete(folderId);
  depthCache.set(folderId, depth);
  return depth;
};

const existsFolderById = (folderId) => {
  return document.flattenedFolders.some(folder => {
    try {
      return folder.id.primaryKey === folderId;
    } catch (e) {
      return false;
    }
  });
};

const getLiveFolderById = (folderId) => {
  return document.flattenedFolders.find(folder => {
    try {
      return folder.id.primaryKey === folderId;
    } catch (e) {
      return false;
    }
  });
};"#;

/// Deletes the resolved folders deepest first, so a subfolder is never
/// deleted with its parent before its own turn, and records one result per
/// request.
const DELETE_FOLDERS_DEEPEST_FIRST: &str = r#"const results = new Array(requests.length);
const unresolved = [];
const resolved = [];

requests.forEach(request => {
  const folder = resolveFolder(request.idOrName);
  if (!folder) {
    unresolved.push(request);
    return;
  }
  resolved.push({
    ...request,
    folder,
    depth: getDepth(folder.id)
  });
});

resolved
  .sort((left, right) => right.depth - left.depth || left.index - right.index)
  .forEach(request => {
    const resolvedId = request.folder.id;
    const resolvedName = request.folder.name;
    const liveFolder = getLiveFolderById(resolvedId);
    if (!liveFolder) {
      results[request.index] = {
        id_or_name: request.idOrName,
        id: resolvedId,
        name: resolvedName,
        deleted: true,
        error: null
      };
      return;
    }
    try {
      deleteObject(liveFolder);
      results[request.index] = {
        id_or_name: request.idOrName,
        id: resolvedId,
        name: resolvedName,
        deleted: true,
        error: null
      };
    } catch (e) {
      if (!existsFolderById(resolvedId)) {
        results[request.index] = {
          id_or_name: request.idOrName,
          id: resolvedId,
          name: resolvedName,
          deleted: true,
          error: null
        };
        return;
      }
      const errorMessage = e && e.message ? String(e.message) : String(e);
      results[request.index] = {
        id_or_name: request.idOrName,
        id: resolvedId,
        name: resolvedName,
        deleted: false,
        error: errorMessage
      };
    }
  });

unresolved.forEach(request => {
  results[request.index] = {
    id_or_name: request.idOrName,
    id: null,
    name: null,
    deleted: false,
    error: "not found"
  };
});"#;
