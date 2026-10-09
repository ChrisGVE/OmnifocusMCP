//! A project's end of life: completing it, reopening a completed one, and
//! deleting one or many (with all their tasks).

use serde_json::Value;

use crate::{
    error::{to_json_string, OmniFocusError, Result},
    js_helpers::{JS_PROJECT_STATUS, JS_RESOLVERS, JS_TASK_STATUS},
    jxa::{escape_for_jxa, JxaRunner},
    tools::batch_delete::{normalize_ids_or_names, BATCH_DELETE_SUMMARY},
};

pub async fn complete_project<R: JxaRunner>(runner: &R, project_id_or_name: &str) -> Result<Value> {
    if project_id_or_name.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "project_id_or_name must not be empty.".to_string(),
        ));
    }

    let project_filter = escape_for_jxa(project_id_or_name.trim());
    let script = format!(
        r#"{JS_RESOLVERS}
const projectFilter = {project_filter};
const project = resolveProject(projectFilter);

project.markComplete();

return {{
  id: project.id.primaryKey,
  name: project.name,
  completed: true
}};"#
    );

    runner.run_omnijs(&script).await
}

pub async fn uncomplete_project<R: JxaRunner>(
    runner: &R,
    project_id_or_name: &str,
) -> Result<Value> {
    if project_id_or_name.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "project_id_or_name must not be empty.".to_string(),
        ));
    }

    let project_filter = escape_for_jxa(project_id_or_name.trim());
    let script = format!(
        r#"{JS_RESOLVERS}
const projectFilter = {project_filter};
const project = resolveProject(projectFilter);
if (!project.completed) {{
  throw new Error(`Project is not completed: ${{projectFilter}}`);
}}

project.markIncomplete();

return {{
  id: project.id.primaryKey,
  name: project.name,
  status: "active"
}};"#
    );

    runner.run_omnijs(&script).await
}

pub async fn delete_project<R: JxaRunner>(runner: &R, project_id_or_name: &str) -> Result<Value> {
    if project_id_or_name.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "project_id_or_name must not be empty.".to_string(),
        ));
    }

    let project_filter = escape_for_jxa(project_id_or_name.trim());
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_RESOLVERS}
const projectFilter = {project_filter};
const project = resolveProject(projectFilter);

const projectId = project.id.primaryKey;
const projectName = project.name;
const taskCount = document.flattenedTasks.filter(task => {{
  return !isProjectRootTask(task)
    && task.containingProject
    && task.containingProject.id.primaryKey === projectId;
}}).length;

deleteObject(project);

return {{
  id: projectId,
  name: projectName,
  deleted: true,
  taskCount: taskCount
}};"#
    );

    runner.run_omnijs(&script).await
}

pub async fn delete_projects_batch<R: JxaRunner>(
    runner: &R,
    project_ids_or_names: Vec<String>,
) -> Result<Value> {
    let normalized_project_ids_or_names = normalize_ids_or_names("project", project_ids_or_names)?;

    let project_ids_or_names_value = to_json_string(&normalized_project_ids_or_names)?;
    let script = format!(
        r#"{JS_RESOLVERS}
const projectIdsOrNames = {project_ids_or_names_value};
// Every entry is resolved before anything is deleted, so each one names
// what it named when the call was made.
const requests = projectIdsOrNames.map(idOrName => ({{ idOrName, matches: matchProjects(idOrName) }}));
const results = requests.map(({{ idOrName, matches }}) => {{
  if (matches.length === 0) {{
    return {{
      id_or_name: idOrName,
      id: null,
      name: null,
      deleted: false,
      error: "not found"
    }};
  }}

  const project = matches[0];
  const resolvedId = project.id.primaryKey;
  const resolvedName = project.name;
  try {{
    deleteObject(project);
    return {{
      id_or_name: idOrName,
      id: resolvedId,
      name: resolvedName,
      deleted: true,
      error: null
    }};
  }} catch (e) {{
    const errorMessage = e && e.message ? String(e.message) : String(e);
    return {{
      id_or_name: idOrName,
      id: resolvedId,
      name: resolvedName,
      deleted: false,
      error: errorMessage
    }};
  }}
}});

{BATCH_DELETE_SUMMARY}"#
    );
    runner.run_omnijs(&script).await
}
