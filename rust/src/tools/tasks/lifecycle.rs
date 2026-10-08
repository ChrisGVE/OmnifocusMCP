//! Ending one task's life: completing it or deleting it.
//!
//! Reopening a completed task is `crate::tools::utility::uncomplete_task`;
//! the batch delete is `delete_tasks_batch` in `batch`.

use serde_json::Value;

use crate::{
    error::{OmniFocusError, Result},
    js_helpers::{JS_PROJECT_STATUS, JS_TASK_STATUS},
    jxa::{escape_for_jxa, JxaRunner},
};

pub async fn complete_task<R: JxaRunner>(runner: &R, task_id: &str) -> Result<Value> {
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

task.markComplete();

return {{
  id: task.id.primaryKey,
  name: task.name,
  completed: task.completed
}};"#
    );
    runner.run_omnijs(&script).await
}

pub async fn delete_task<R: JxaRunner>(runner: &R, task_id: &str) -> Result<Value> {
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

const taskName = task.name;
const childCount = task.children.length;
const warning = childCount > 0
  ? `Deleted task had ${{childCount}} child task(s).`
  : null;

deleteObject(task);

return {{
  id: taskId,
  name: taskName,
  deleted: true,
  warning: warning
}};"#
    );
    runner.run_omnijs(&script).await
}
