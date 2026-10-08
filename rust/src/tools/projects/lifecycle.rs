//! A project's end of life: completing it, reopening a completed one, and
//! deleting one or many (with all their tasks).

use serde_json::Value;

use crate::{
    error::{OmniFocusError, Result},
    js_helpers::{JS_PROJECT_STATUS, JS_TASK_STATUS},
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
        r#"const projectFilter = {project_filter};
const project = document.flattenedProjects.find(item => {{
  return item.id.primaryKey === projectFilter || item.name === projectFilter;
}});
if (!project) {{
  throw new Error(`Project not found: ${{projectFilter}}`);
}}

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
        r#"const projectFilter = {project_filter};
const project = document.flattenedProjects.find(item => {{
  return item.id.primaryKey === projectFilter || item.name === projectFilter;
}});
if (!project) {{
  throw new Error(`Project not found: ${{projectFilter}}`);
}}
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
const projectFilter = {project_filter};
const project = document.flattenedProjects.find(item => {{
  return item.id.primaryKey === projectFilter || item.name === projectFilter;
}});
if (!project) {{
  throw new Error(`Project not found: ${{projectFilter}}`);
}}

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

    let project_ids_or_names_value = serde_json::to_string(&normalized_project_ids_or_names)?;
    let script = format!(
        r#"const projectIdsOrNames = {project_ids_or_names_value};
const projects = document.flattenedProjects
  .map(item => {{
    try {{
      return {{
        id: item.id.primaryKey,
        name: item.name,
        ref: item
      }};
    }} catch (e) {{
      return null;
    }}
  }})
  .filter(item => item !== null);
const results = projectIdsOrNames.map(idOrName => {{
  const project = projects.find(item => {{
    return item.id === idOrName || item.name === idOrName;
  }});
  if (project === undefined) {{
    return {{
      id_or_name: idOrName,
      id: null,
      name: null,
      deleted: false,
      error: "not found"
    }};
  }}

  const resolvedId = project.id;
  const resolvedName = project.name;
  try {{
    deleteObject(project.ref);
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
