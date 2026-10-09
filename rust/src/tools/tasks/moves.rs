//! Moving one task to a project, under a parent task, or to the inbox,
//! keeping the task itself (and its id) rather than recreating it.
//!
//! `validate_destination` is shared with `move_tasks_batch` in `batch`.

use serde_json::Value;

use crate::{
    error::{OmniFocusError, Result},
    js_helpers::{JS_PROJECT_STATUS, JS_RESOLVERS, JS_TASK_STATUS},
    jxa::{escape_for_jxa, JxaRunner},
    tools::js_values::js_trimmed_string_or_null,
};

pub async fn move_task<R: JxaRunner>(
    runner: &R,
    task_id: &str,
    project: Option<&str>,
    parent_task_id: Option<&str>,
) -> Result<Value> {
    if task_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "task_id must not be empty.".to_string(),
        ));
    }
    validate_destination(project, parent_task_id)?;

    let task_id_value = escape_for_jxa(task_id.trim());
    let project_value = js_trimmed_string_or_null(project);
    let parent_task_id_value = js_trimmed_string_or_null(parent_task_id);
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_RESOLVERS}
const taskId = {task_id_value};
const projectName = {project_value};
const parentTaskId = {parent_task_id_value};
const task = resolveTask(taskId);

const destinationInfo = (() => {{
  if (parentTaskId !== null && parentTaskId !== "") {{
    if (parentTaskId === taskId) {{
      throw new Error("Cannot move a task under itself.");
    }}
    const parentTask = resolveTask(parentTaskId, "Parent task");
    // Climb from the destination through `parent`; a top-level task's parent
    // is its project's root task, which is no task's descendant, so stop there.
    let ancestor = parentTask;
    while (ancestor && !isProjectRootTask(ancestor)) {{
      if (ancestor.id.primaryKey === taskId) {{
        throw new Error("Cannot move a task under its own descendant.");
      }}
      ancestor = ancestor.parent;
    }}
    return {{ mode: "parent", location: parentTask.ending }};
  }}
  if (projectName === null || projectName === "") {{
    return {{ mode: "inbox", location: inbox.ending }};
  }}
  const targetProject = resolveProject(projectName);
  return {{ mode: "project", location: targetProject.ending }};
}})();

const originalTaskId = task.id.primaryKey;
moveTasks([task], destinationInfo.location);
if (task.id.primaryKey !== originalTaskId) {{
  throw new Error("Task move did not preserve task identity.");
}}
const newParent = task.parent;
if (destinationInfo.mode !== "parent" && newParent && !isProjectRootTask(newParent)) {{
  throw new Error("Task move failed: task is still nested under a parent.");
}}

return {{
  id: task.id.primaryKey,
  name: task.name,
  projectName: task.containingProject ? task.containingProject.name : null,
  inInbox: task.inInbox
}};"#
    );
    runner.run_omnijs(&script).await
}

/// Rejects an empty `project` or `parent_task_id`, and both at once: a
/// move goes to one project, one parent task, or (with neither) the inbox.
pub(super) fn validate_destination(
    project: Option<&str>,
    parent_task_id: Option<&str>,
) -> Result<()> {
    if let Some(project_name) = project {
        if project_name.trim().is_empty() {
            return Err(OmniFocusError::Validation(
                "project must not be empty when provided.".to_string(),
            ));
        }
    }
    if let Some(parent_id) = parent_task_id {
        if parent_id.trim().is_empty() {
            return Err(OmniFocusError::Validation(
                "parent_task_id must not be empty when provided.".to_string(),
            ));
        }
    }
    if project.is_some() && parent_task_id.is_some() {
        return Err(OmniFocusError::Validation(
            "provide either project or parent_task_id, not both (destination is ambiguous)."
                .to_string(),
        ));
    }
    Ok(())
}
