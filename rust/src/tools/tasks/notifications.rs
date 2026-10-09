//! A task's notifications: listing them, adding one at an absolute date or
//! at an offset from the due date, and removing one.

use serde_json::Value;

use crate::{
    error::{OmniFocusError, Result},
    js_helpers::{
        JS_DATE_HELPERS, JS_NOTIFICATION_SUMMARY, JS_PROJECT_STATUS, JS_RESOLVERS, JS_TASK_STATUS,
    },
    jxa::{escape_for_jxa, JxaRunner},
};

pub async fn list_notifications<R: JxaRunner>(runner: &R, task_id: &str) -> Result<Value> {
    if task_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "task_id must not be empty.".to_string(),
        ));
    }

    let task_id_filter = escape_for_jxa(task_id.trim());
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_NOTIFICATION_SUMMARY}
{JS_RESOLVERS}
const taskId = {task_id_filter};
const task = resolveTask(taskId);
return task.notifications.map(summarizeNotification);"#
    );

    runner.run_omnijs(&script).await
}

pub async fn add_notification<R: JxaRunner>(
    runner: &R,
    task_id: &str,
    absolute_date: Option<&str>,
    relative_offset: Option<f64>,
) -> Result<Value> {
    if task_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "task_id must not be empty.".to_string(),
        ));
    }

    if absolute_date.is_some() == relative_offset.is_some() {
        return Err(OmniFocusError::Validation(
            "exactly one of absoluteDate or relativeOffset must be provided.".to_string(),
        ));
    }
    if let Some(absolute_date_value) = absolute_date {
        if absolute_date_value.trim().is_empty() {
            return Err(OmniFocusError::Validation(
                "absoluteDate must not be empty when provided.".to_string(),
            ));
        }
    }

    let task_id_filter = escape_for_jxa(task_id.trim());
    let absolute_date_value = absolute_date
        .map(|value| escape_for_jxa(value.trim()))
        .unwrap_or_else(|| "null".to_string());
    let relative_offset_value = relative_offset
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string());
    let script = format!(
        r#"{JS_DATE_HELPERS}
{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_NOTIFICATION_SUMMARY}
{JS_RESOLVERS}
const taskId = {task_id_filter};
const absoluteDate = {absolute_date_value};
const relativeOffset = {relative_offset_value};
const task = resolveTask(taskId);
let notification = null;
if (absoluteDate !== null) {{
  const parsedAbsoluteDate = parseLocalDate(absoluteDate, "absoluteDate");
  notification = task.addNotification(parsedAbsoluteDate);
}} else {{
  if (task.effectiveDueDate === null) {{
    throw new Error("relativeOffset requires a task with an effective due date.");
  }}
  notification = task.addNotification(relativeOffset);
}}
if (!notification) {{
  throw new Error("Failed to create notification.");
}}
return summarizeNotification(notification);"#
    );

    runner.run_omnijs(&script).await
}

pub async fn remove_notification<R: JxaRunner>(
    runner: &R,
    task_id: &str,
    notification_id: &str,
) -> Result<Value> {
    if task_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "task_id must not be empty.".to_string(),
        ));
    }
    if notification_id.trim().is_empty() {
        return Err(OmniFocusError::Validation(
            "notification_id must not be empty.".to_string(),
        ));
    }

    let task_id_filter = escape_for_jxa(task_id.trim());
    let notification_id_filter = escape_for_jxa(notification_id.trim());
    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_RESOLVERS}
const taskId = {task_id_filter};
const notificationId = {notification_id_filter};
const task = resolveTask(taskId);
const notification = task.notifications.find(item => item.id.primaryKey === notificationId);
if (!notification) {{
  throw new Error(`Notification not found: ${{notificationId}}`);
}}
const removedNotificationId = notification.id.primaryKey;
task.removeNotification(notification);
return {{
  taskId: task.id.primaryKey,
  notificationId: removedNotificationId,
  removed: true
}};"#
    );

    runner.run_omnijs(&script).await
}
