use serde_json::Value;

use crate::{
    error::{OmniFocusError, Result},
    js_helpers::{JS_PLANNED_DATE, JS_PROJECT_STATUS, JS_TASK_STATUS},
    jxa::JxaRunner,
    tools::tasks::listing_script::{MAP_LISTED_TASKS, READ_PLANNED_DATES},
};

pub async fn get_forecast<R: JxaRunner>(runner: &R, limit: i32) -> Result<Value> {
    if limit < 1 {
        return Err(OmniFocusError::Validation(
            "limit must be greater than 0.".to_string(),
        ));
    }

    let script = format!(
        r#"{JS_PROJECT_STATUS}
{JS_TASK_STATUS}
{JS_PLANNED_DATE}
{READ_PLANNED_DATES}
const now = new Date();
const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate());
const endOfToday = new Date(startOfToday.getTime() + (24 * 60 * 60 * 1000));
const endOfWeek = new Date(now.getTime() + (7 * 24 * 60 * 60 * 1000));

{MAP_LISTED_TASKS}

const openTasks = document.flattenedTasks.filter(task => {{
  return !isProjectRootTask(task) && isTaskRemaining(task);
}});
const overdue = [];
const dueToday = [];
const flagged = [];
const deferred = [];
const dueThisWeek = [];

const counts = {{
  overdueCount: 0,
  dueTodayCount: 0,
  flaggedCount: 0,
  deferredCount: 0,
  dueThisWeekCount: 0
}};

openTasks.forEach(task => {{
  if (task.dueDate !== null && task.dueDate < startOfToday) {{
    counts.overdueCount += 1;
            if (overdue.length < {limit}) overdue.push(mapListedTask(task));
  }}
  if (task.dueDate !== null && task.dueDate >= startOfToday && task.dueDate < endOfToday) {{
    counts.dueTodayCount += 1;
            if (dueToday.length < {limit}) dueToday.push(mapListedTask(task));
  }}
  if (task.flagged) {{
    counts.flaggedCount += 1;
            if (flagged.length < {limit}) flagged.push(mapListedTask(task));
  }}
  if (task.deferDate !== null && task.deferDate > now) {{
    counts.deferredCount += 1;
            if (deferred.length < {limit}) deferred.push(mapListedTask(task));
  }}
  if (task.dueDate !== null && task.dueDate >= endOfToday && task.dueDate < endOfWeek) {{
    counts.dueThisWeekCount += 1;
            if (dueThisWeek.length < {limit}) dueThisWeek.push(mapListedTask(task));
  }}
}});

return {{
  overdue: overdue,
  dueToday: dueToday,
  flagged: flagged,
  deferred: deferred,
  dueThisWeek: dueThisWeek,
  counts: counts
}};"#
    );

    runner.run_omnijs(&script).await
}
