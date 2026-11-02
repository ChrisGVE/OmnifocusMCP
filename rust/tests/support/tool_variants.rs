//! Calls of `tests/all_tools_jsc_test.rs` that take other branches of the
//! scripts `tool_rows::rows()` already runs: other destinations, other
//! filters, objects with contents. Arguments name objects of
//! `fixtures/omnijs/scenario.js`.
//!
//! Test files include it with `#[path = "support/tool_variants.rs"] mod
//! tool_variants;`, next to `tool_rows`.

use serde_json::json;

use crate::tool_rows::{row, Expect, Row, BATCH_RESULT, TASK_SUMMARY};

/// Calls that take other branches of the scripts `rows()` already runs.
pub fn variants() -> Vec<Row> {
    [task_variants(), filter_variants(), container_variants()]
        .into_iter()
        .flatten()
        .collect()
}

fn task_variants() -> Vec<Row> {
    use Expect::Object;
    vec![
        row(
            "move_task",
            json!({"task_id": "task-review"}),
            Object(&["id", "inInbox"]),
        ),
        row(
            "move_task",
            json!({"task_id": "task-call", "project": "Errands"}),
            Object(&["id", "projectName"]),
        ),
        row(
            "move_tasks_batch",
            json!({"task_ids": ["task-call"], "project": "Launch"}),
            Object(&["moved_count", "results"]),
        ),
        row(
            "move_tasks_batch",
            json!({"task_ids": ["task-review"]}),
            Object(&["moved_count", "results"]),
        ),
        row(
            "add_notification",
            json!({"task_id": "task-draft", "relativeOffset": -30}),
            Object(&["id", "kind", "relativeFireOffset"]),
        ),
        row(
            "append_to_note",
            json!({"object_type": "project", "object_id": "project-launch", "text": "more"}),
            Object(&["id", "type"]),
        ),
        row(
            "duplicate_task",
            json!({"task_id": "task-draft", "includeChildren": false}),
            Object(&["id", "name"]),
        ),
        row(
            "create_task",
            json!({"name": "Inbox task"}),
            Object(&["id", "name"]),
        ),
        row(
            "get_task",
            json!({"task_id": "task-water"}),
            Object(&["id", "repetitionRule"]),
        ),
        row(
            "complete_task",
            json!({"task_id": "task-water"}),
            Object(&["id", "completed"]),
        ),
        row(
            "set_task_repetition",
            json!({"task_id": "task-water", "rule_string": null}),
            Object(&["id", "repetitionRule"]),
        ),
        row(
            "set_task_repetition",
            json!({"task_id": "task-milk", "rule_string": "FREQ=DAILY", "schedule_type": "from_completion"}),
            Object(&["id", "repetitionRule"]),
        ),
        row(
            "delete_tasks_batch",
            json!({"task_ids": ["task-scrap", "task-missing"]}),
            Object(&["deleted_count", "not_found_count", "results"]),
        ),
    ]
}

fn filter_variants() -> Vec<Row> {
    use Expect::{List, Object};
    vec![
        row(
            "list_tasks",
            json!({"tags": ["Home", "Desk"], "tagFilterMode": "all", "status": "available",
                   "sortBy": "plannedDate", "plannedBefore": "2026-10-10"}),
            List(TASK_SUMMARY),
        ),
        row(
            "list_tasks",
            json!({"status": "completed", "completedAfter": "2026-09-01",
                   "completedBefore": "2026-10-09", "sortBy": "completionDate", "sortOrder": "desc"}),
            List(TASK_SUMMARY),
        ),
        row(
            "search_tasks",
            json!({"query": "plan", "status": "all", "flagged": true, "deferAfter": "2026-10-10"}),
            List(TASK_SUMMARY),
        ),
        row(
            "get_task_counts",
            json!({"tag": "Home", "dueBefore": "2026-10-11", "added_after": "2026-01-01",
                   "changed_before": "2026-12-31", "maxEstimatedMinutes": 60}),
            Object(&["total"]),
        ),
        row(
            "list_projects",
            json!({"status": "completed", "completedAfter": "2026-08-01", "sortBy": "completionDate"}),
            List(&["id", "name", "status"]),
        ),
        row(
            "get_project_counts",
            json!({"folder": "Clients"}),
            Object(&["total"]),
        ),
        row(
            "list_tags",
            json!({"statusFilter": "on_hold", "sortBy": "totalTaskCount", "sortOrder": "desc"}),
            List(&["id", "name", "status"]),
        ),
    ]
}

fn container_variants() -> Vec<Row> {
    use Expect::Object;
    vec![
        row(
            "update_folder",
            json!({"folder_name_or_id": "folder-clients", "status": "active"}),
            Object(&["id", "status"]),
        ),
        row(
            "set_project_status",
            json!({"project_id_or_name": "Errands", "status": "dropped"}),
            Object(&["id", "status"]),
        ),
        row(
            "delete_tag",
            json!({"tag_name_or_id": "Home"}),
            Object(&["id", "deleted"]),
        ),
        row(
            "delete_project",
            json!({"project_id_or_name": "Launch"}),
            Object(&["id", "deleted"]),
        ),
        row(
            "delete_folder",
            json!({"folder_name_or_id": "Work"}),
            Object(&["id", "deleted"]),
        ),
        row(
            "move_project",
            json!({"project_id_or_name": "Launch"}),
            Object(&["id", "name"]),
        ),
        row(
            "create_folder",
            json!({"name": "Top folder"}),
            Object(&["id", "name"]),
        ),
        row(
            "create_tag",
            json!({"name": "Top tag"}),
            Object(&["id", "name"]),
        ),
        row(
            "delete_tags_batch",
            json!({"tag_ids_or_names": ["tag-scrap", "tag-missing"]}),
            Object(BATCH_RESULT),
        ),
    ]
}
