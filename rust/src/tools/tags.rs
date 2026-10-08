use serde_json::Value;

use crate::{
    error::{OmniFocusError, Result},
    js_helpers::{JS_PROJECT_STATUS, JS_RESOLVERS, JS_TASK_STATUS},
    jxa::{escape_for_jxa, JxaRunner},
    tools::{
        batch_delete::{normalize_ids_or_names, BATCH_DELETE_SUMMARY},
        js_values::js_string_or_null,
    },
};

pub async fn list_tags<R: JxaRunner>(
    runner: &R,
    status_filter: &str,
    sort_by: Option<&str>,
    sort_order: &str,
    limit: i32,
) -> Result<Value> {
    validate_list_tags(status_filter, sort_by, sort_order, limit)?;

    let status_filter_value = escape_for_jxa(status_filter);
    let sort_by_value = js_string_or_null(sort_by);
    let sort_order_value = escape_for_jxa(sort_order);
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
const statusFilter = {status_filter_value};
const sortBy = {sort_by_value};
const sortOrder = {sort_order_value};
const now = new Date();

{TAG_TASK_COUNTS}

{NORMALIZE_TAG_STATUS}

const compareValues = (left, right) => {{
  if (left < right) return sortOrder === "asc" ? -1 : 1;
  if (left > right) return sortOrder === "asc" ? 1 : -1;
  return 0;
}};

const filteredTags = document.flattenedTags.filter(tag => {{
  return statusFilter === "all" || normalizeTagStatus(tag) === statusFilter;
}});

const mappedTags = filteredTags.map(tag => {{
  const counts = tagCounts.get(tag.id.primaryKey) || {{ availableTaskCount: 0, totalTaskCount: 0 }};
  return {{
  id: tag.id.primaryKey,
  name: tag.name,
  parent: tag.parent ? tag.parent.name : null,
  availableTaskCount: counts.availableTaskCount,
  totalTaskCount: counts.totalTaskCount,
  status: normalizeTagStatus(tag)
  }};
}});

const sortedTags = sortBy === null ? mappedTags : mappedTags.slice().sort((a, b) => {{
  if (sortBy === "name") {{
    return compareValues(String(a.name).toLowerCase(), String(b.name).toLowerCase());
  }}
  return compareValues(a[sortBy], b[sortBy]);
}});

return sortedTags.slice(0, {limit});"#
    );
    runner.run_omnijs(&script).await
}

/// Rejects an unknown status filter, sort field or sort order, and a limit
/// below 1; the limit is checked first.
fn validate_list_tags(
    status_filter: &str,
    sort_by: Option<&str>,
    sort_order: &str,
    limit: i32,
) -> Result<()> {
    if limit < 1 {
        return Err(OmniFocusError::Validation(
            "limit must be greater than 0.".to_string(),
        ));
    }
    if !matches!(status_filter, "active" | "on_hold" | "dropped" | "all") {
        return Err(OmniFocusError::Validation(
            "statusFilter must be one of: active, on_hold, dropped, all.".to_string(),
        ));
    }
    if let Some(sort_field) = sort_by {
        if !matches!(sort_field, "name" | "availableTaskCount" | "totalTaskCount") {
            return Err(OmniFocusError::Validation(
                "sortBy must be one of: name, availableTaskCount, totalTaskCount.".to_string(),
            ));
        }
    }
    if !matches!(sort_order, "asc" | "desc") {
        return Err(OmniFocusError::Validation(
            "sortOrder must be one of: asc, desc.".to_string(),
        ));
    }
    Ok(())
}

/// Counts, per tag, the tasks carrying it and the available ones among them.
const TAG_TASK_COUNTS: &str = r#"const tagCounts = new Map();
document.flattenedTasks.forEach(task => {
  if (isProjectRootTask(task)) return;
  task.tags.forEach(tag => {
    const tagId = tag.id.primaryKey;
    const current = tagCounts.get(tagId) || { availableTaskCount: 0, totalTaskCount: 0 };
    current.totalTaskCount += 1;
    if (isTaskAvailable(task, now)) current.availableTaskCount += 1;
    tagCounts.set(tagId, current);
  });
});"#;

/// Reads a tag's status as `active`, `on_hold` or `dropped`.
const NORMALIZE_TAG_STATUS: &str = r#"const normalizeTagStatus = (tag) => {
  const rawStatus = String(tag.status || "").toLowerCase();
  const flattened = rawStatus
    .replace(/^\[object_/g, "")
    .replace(/[\[\]{}()]/g, " ")
    .replace(/status/g, " ")
    .replace(/[:.=]/g, " ")
    .replace(/[_-]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  if (flattened.includes("onhold") || /(^|\s)on\s*hold(\s|$)/.test(flattened)) return "on_hold";
  if (flattened.includes("dropped")) return "dropped";
  if (flattened.includes("active")) return "active";
  return "active";
};"#;

pub async fn search_tags<R: JxaRunner>(runner: &R, query: &str, limit: i32) -> Result<Value> {
    if query.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "query must not be empty.".to_string(),
        ));
    }
    if limit < 1 {
        return Err(OmniFocusError::Validation(
            "limit must be greater than 0.".to_string(),
        ));
    }

    let query_value = escape_for_jxa(query.trim());
    let script = format!(
        r#"const queryValue = {query_value};
const normalizeTagStatus = (tag) => {{
  const rawStatus = String(tag.status || "").toLowerCase();
  const flattened = rawStatus
    .replace(/^\[object_/g, "")
    .replace(/[\[\]{{}}()]/g, " ")
    .replace(/status/g, " ")
    .replace(/[:.=]/g, " ")
    .replace(/[_-]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  if (flattened.includes("onhold") || /(^|\s)on\s*hold(\s|$)/.test(flattened)) return "on_hold";
  if (flattened.includes("dropped")) return "dropped";
  if (flattened.includes("active")) return "active";
  return "active";
}};

return tagsMatching(queryValue)
  .slice(0, {limit})
  .map(tag => {{
    return {{
      id: tag.id.primaryKey,
      name: tag.name,
      status: normalizeTagStatus(tag),
      parent: tag.parent ? tag.parent.name : null
    }};
  }});"#
    );
    runner.run_omnijs(&script).await
}

pub async fn create_tag<R: JxaRunner>(
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
    let tag_name = escape_for_jxa(name.trim());
    let parent_name = parent
        .map(|value| escape_for_jxa(value.trim()))
        .unwrap_or_else(|| "null".to_string());
    let script = format!(
        r#"{JS_RESOLVERS}
const tagName = {tag_name};
const parentName = {parent_name};
const tag = (() => {{
  if (parentName === null) return new Tag(tagName);
  const parentTag = resolveTag(parentName);
  return new Tag(tagName, parentTag.ending);
}})();
return {{
  id: tag.id.primaryKey,
  name: tag.name,
  parent: tag.parent ? tag.parent.name : null,
  availableTaskCount: 0,
  totalTaskCount: 0,
  status: "active"
}};"#
    );
    runner.run_omnijs(&script).await
}

pub async fn update_tag<R: JxaRunner>(
    runner: &R,
    tag_name_or_id: &str,
    name: Option<&str>,
    status: Option<&str>,
) -> Result<Value> {
    if tag_name_or_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "tag_name_or_id must not be empty.".to_string(),
        ));
    }
    if let Some(value) = name {
        if value.trim().is_empty() {
            return Err(OmniFocusError::Validation(
                "name must not be empty when provided.".to_string(),
            ));
        }
    }
    if let Some(value) = status {
        if !matches!(value, "active" | "on_hold" | "dropped") {
            return Err(OmniFocusError::Validation(
                "status must be one of: active, on_hold, dropped.".to_string(),
            ));
        }
    }
    if name.is_none() && status.is_none() {
        return Err(OmniFocusError::Validation(
            "at least one field must be provided: name or status.".to_string(),
        ));
    }
    let tag_filter = escape_for_jxa(tag_name_or_id.trim());
    let new_name = name
        .map(|value| escape_for_jxa(value.trim()))
        .unwrap_or_else(|| "null".to_string());
    let status_value = status
        .map(escape_for_jxa)
        .unwrap_or_else(|| "null".to_string());
    let script = format!(
        r#"{JS_RESOLVERS}
const tagFilter = {tag_filter};
const newName = {new_name};
const statusValue = {status_value};
const tag = resolveTag(tagFilter);
if (newName !== null) {{
  tag.name = newName;
}}
if (statusValue !== null) {{
  let targetStatus;
  if (statusValue === "active") {{
    targetStatus = Tag.Status.Active;
  }} else if (statusValue === "on_hold") {{
    targetStatus = Tag.Status.OnHold;
  }} else if (statusValue === "dropped") {{
    targetStatus = Tag.Status.Dropped;
  }} else {{
    throw new Error(`Invalid status: ${{statusValue}}`);
  }}
  tag.status = targetStatus;
}}
const normalizeTagStatus = (tag) => {{
  const rawStatus = String(tag.status || "").toLowerCase();
  const flattened = rawStatus
    .replace(/^\[object_/g, "")
    .replace(/[\[\]{{}}()]/g, " ")
    .replace(/status/g, " ")
    .replace(/[:.=]/g, " ")
    .replace(/[_-]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  if (flattened.includes("onhold") || /(^|\s)on\s*hold(\s|$)/.test(flattened)) return "on_hold";
  if (flattened.includes("dropped")) return "dropped";
  if (flattened.includes("active")) return "active";
  return "active";
}};
return {{ id: tag.id.primaryKey, name: tag.name, status: normalizeTagStatus(tag) }};"#
    );
    runner.run_omnijs(&script).await
}

pub async fn delete_tag<R: JxaRunner>(runner: &R, tag_name_or_id: &str) -> Result<Value> {
    if tag_name_or_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "tag_name_or_id must not be empty.".to_string(),
        ));
    }
    let tag_filter = escape_for_jxa(tag_name_or_id.trim());
    let script = format!(
        r#"{JS_RESOLVERS}
const tagFilter = {tag_filter};

const tag = resolveTag(tagFilter);

const tagId = tag.id.primaryKey;
const tagName = tag.name;
const taskCount = tag.tasks.length;

deleteObject(tag);

return {{
  id: tagId,
  name: tagName,
  deleted: true,
  taskCount: taskCount
}};"#
    );
    runner.run_omnijs(&script).await
}

pub async fn delete_tags_batch<R: JxaRunner>(
    runner: &R,
    tag_ids_or_names: Vec<String>,
) -> Result<Value> {
    let normalized_tag_ids_or_names = normalize_ids_or_names("tag", tag_ids_or_names)?;

    let tag_ids_or_names_value = serde_json::to_string(&normalized_tag_ids_or_names)?;
    let script = format!(
        r#"const tagIdsOrNames = {tag_ids_or_names_value};
const requests = tagIdsOrNames.map((idOrName, index) => ({{ idOrName, index }}));
{TAG_LOOKUP}

{DELETE_TAGS_DEEPEST_FIRST}

{BATCH_DELETE_SUMMARY}"#
    );
    runner.run_omnijs(&script).await
}

/// Snapshots every tag's id, name and parent and defines lookups over it:
/// by id or name, by depth in the tag tree, and back to the live tag.
const TAG_LOOKUP: &str = r#"const tags = document.flattenedTags
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
const tagsById = new Map(tags.map(tag => [tag.id, tag]));

const resolveTag = (idOrName) => {
  const byId = tagsById.get(idOrName);
  if (byId) return byId;
  return tags.find(tag => tag.name === idOrName);
};

const depthCache = new Map();
const getDepth = (tagId, stack = new Set()) => {
  if (depthCache.has(tagId)) return depthCache.get(tagId);
  if (stack.has(tagId)) return 0;
  stack.add(tagId);
  const tag = tagsById.get(tagId);
  let depth = 0;
  if (tag && tag.parentId && tagsById.has(tag.parentId)) {
    depth = getDepth(tag.parentId, stack) + 1;
  }
  stack.delete(tagId);
  depthCache.set(tagId, depth);
  return depth;
};

const existsTagById = (tagId) => {
  return document.flattenedTags.some(tag => {
    try {
      return tag.id.primaryKey === tagId;
    } catch (e) {
      return false;
    }
  });
};

const getLiveTagById = (tagId) => {
  return document.flattenedTags.find(tag => {
    try {
      return tag.id.primaryKey === tagId;
    } catch (e) {
      return false;
    }
  });
};"#;

/// Deletes the resolved tags deepest first, so a child is never deleted
/// with its parent before its own turn, and records one result per request.
const DELETE_TAGS_DEEPEST_FIRST: &str = r#"const results = new Array(requests.length);
const unresolved = [];
const resolved = [];

requests.forEach(request => {
  const tag = resolveTag(request.idOrName);
  if (!tag) {
    unresolved.push(request);
    return;
  }
  resolved.push({
    ...request,
    tag,
    depth: getDepth(tag.id)
  });
});

resolved
  .sort((left, right) => right.depth - left.depth || left.index - right.index)
  .forEach(request => {
    const resolvedId = request.tag.id;
    const resolvedName = request.tag.name;
    const liveTag = getLiveTagById(resolvedId);
    if (!liveTag) {
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
      deleteObject(liveTag);
      results[request.index] = {
        id_or_name: request.idOrName,
        id: resolvedId,
        name: resolvedName,
        deleted: true,
        error: null
      };
    } catch (e) {
      if (!existsTagById(resolvedId)) {
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
