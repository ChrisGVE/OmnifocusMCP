use serde_json::Value;

use crate::{
    error::{OmniFocusError, Result},
    js_helpers::{JS_PROJECT_STATUS, JS_TASK_STATUS},
    jxa::{escape_for_jxa, JxaRunner},
};

pub async fn uncomplete_task<R: JxaRunner>(runner: &R, task_id: &str) -> Result<Value> {
    if task_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "task_id must not be empty.".to_string(),
        ));
    }

    let task_id_value = escape_for_jxa(task_id.trim());
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
const taskId = {task_id_value};
const task = document.flattenedTasks.find(item => item.id.primaryKey === taskId && !isProjectRootTask(item));
if (!task) {{
  throw new Error(`Task not found: ${{taskId}}`);
}}
if (!task.completed) {{
  if (isTaskCompleted(task)) {{
    // Done through a completed container, not its own flag: name the container
    // (the nearest completed ancestor, else the completed project).
    let holder = task.parent;
    while (holder && !holder.completed) holder = holder.parent;
    const holderName = holder ? holder.name
      : (task.containingProject ? task.containingProject.name : null);
    throw new Error(
      `Task ${{taskId}} is completed through its project or a containing task; `
        + `reopen that instead: ${{holderName}}`
    );
  }}
  throw new Error(`Task is not completed: ${{taskId}}`);
}}

task.markIncomplete();

return {{
  id: task.id.primaryKey,
  name: task.name,
  completed: isTaskCompleted(task)
}};"#
    );
    runner.run_omnijs(&script).await
}

pub async fn append_to_note<R: JxaRunner>(
    runner: &R,
    object_type: &str,
    object_id: &str,
    text: &str,
) -> Result<Value> {
    if !matches!(object_type, "task" | "project") {
        return Err(OmniFocusError::Validation(
            "object_type must be one of: task, project.".to_string(),
        ));
    }
    if object_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "object_id must not be empty.".to_string(),
        ));
    }
    if text.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "text must not be empty.".to_string(),
        ));
    }

    let object_type_value = escape_for_jxa(object_type);
    let object_id_value = escape_for_jxa(object_id.trim());
    let text_value = escape_for_jxa(text);
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
const objectType = {object_type_value};
const objectId = {object_id_value};
const textToAppend = {text_value};

let obj;
if (objectType === "task") {{
  obj = document.flattenedTasks.find(item => item.id.primaryKey === objectId && !isProjectRootTask(item));
  if (!obj) {{
    throw new Error(`Task not found: ${{objectId}}`);
  }}
}} else if (objectType === "project") {{
  obj = document.flattenedProjects.find(item => item.id.primaryKey === objectId);
  if (!obj) {{
    throw new Error(`Project not found: ${{objectId}}`);
  }}
}} else {{
  throw new Error(`Invalid object_type: ${{objectType}}`);
}}

obj.appendStringToNote(textToAppend);

return {{
  id: obj.id.primaryKey,
  name: obj.name,
  type: objectType,
  noteLength: obj.note.length
}};"#
    );
    runner.run_omnijs(&script).await
}
