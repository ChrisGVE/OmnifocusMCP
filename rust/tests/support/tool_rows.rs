//! The calls `tests/all_tools_jsc_test.rs` makes, one row per registered
//! tool; `tool_variants.rs` adds calls that take other branches. Arguments
//! name objects of `fixtures/omnijs/scenario.js`.
//!
//! Test files include it with `#[path = "support/tool_rows.rs"] mod
//! tool_rows;`.

use serde_json::{json, Value};

/// What a call's tool must answer.
#[derive(Clone, Copy, Debug)]
pub enum Expect {
    /// A JSON object with at least these keys.
    Object(&'static [&'static str]),
    /// A non-empty JSON array whose every item has at least these keys.
    List(&'static [&'static str]),
    /// A defect this harness found and nobody has fixed yet: the tool fails
    /// today because its script touches exactly this undocumented member.
    /// When the script is fixed the row fails, and its expectation must
    /// become the result shape.
    KnownDefect { violation: &'static str },
}

pub struct Row {
    pub tool: &'static str,
    pub arguments: Value,
    pub expect: Expect,
}

pub fn row(tool: &'static str, arguments: Value, expect: Expect) -> Row {
    Row {
        tool,
        arguments,
        expect,
    }
}

pub const TASK_SUMMARY: &[&str] = &["id", "name", "taskStatus"];
pub const BATCH_RESULT: &[&str] = &["results"];

/// Exactly one row per registered tool.
pub fn rows() -> Vec<Row> {
    [
        task_read_rows(),
        task_write_rows(),
        task_move_and_delete_rows(),
        notification_rows(),
        project_rows(),
        tag_rows(),
        folder_rows(),
        view_rows(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn task_read_rows() -> Vec<Row> {
    use Expect::{List, Object};
    vec![
        row("get_inbox", json!({"limit": 10}), List(TASK_SUMMARY)),
        row(
            "list_tasks",
            json!({"project": "Launch", "status": "all", "sortBy": "dueDate", "limit": 10}),
            List(TASK_SUMMARY),
        ),
        row(
            "get_task_counts",
            json!({"project": "project-launch"}),
            Object(&[
                "total",
                "available",
                "completed",
                "overdue",
                "dueSoon",
                "flagged",
                "deferred",
            ]),
        ),
        row(
            "get_task",
            json!({"task_id": "task-draft"}),
            Object(&[
                "id",
                "name",
                "children",
                "parentName",
                "taskStatus",
                "repetitionRule",
            ]),
        ),
        row(
            "list_subtasks",
            json!({"task_id": "task-draft"}),
            List(TASK_SUMMARY),
        ),
        row(
            "search_tasks",
            json!({"query": "plan", "status": "all"}),
            List(TASK_SUMMARY),
        ),
    ]
}

fn task_write_rows() -> Vec<Row> {
    use Expect::{List, Object};
    vec![
        row(
            "create_task",
            json!({"name": "New task", "project": "Launch", "note": "n",
                   "dueDate": "2026-10-15", "deferDate": "2026-10-14T08:00:00Z",
                   "plannedDate": "2026-10-14", "flagged": true, "tags": ["Home"],
                   "estimatedMinutes": 15}),
            Object(&["id", "name", "plannedDate"]),
        ),
        row(
            "create_tasks_batch",
            json!({"tasks": [{"name": "Batch A", "project": "Errands"}, {"name": "Batch B"}]}),
            List(&["id", "name"]),
        ),
        row(
            "create_subtask",
            json!({"name": "Subtask", "parent_task_id": "task-draft", "dueDate": "2026-10-10"}),
            Object(&["id", "name", "parentTaskId"]),
        ),
        row(
            "duplicate_task",
            json!({"task_id": "task-draft", "includeChildren": true}),
            Object(&["id", "name", "hasChildren"]),
        ),
        row(
            "complete_task",
            json!({"task_id": "task-review"}),
            Object(&["id", "name", "completed"]),
        ),
        row(
            "uncomplete_task",
            json!({"task_id": "task-ship"}),
            Object(&["id", "name", "completed"]),
        ),
        row(
            "set_task_repetition",
            json!({"task_id": "task-milk", "rule_string": "FREQ=WEEKLY", "schedule_type": "regularly"}),
            Object(&["id", "name", "repetitionRule"]),
        ),
        row(
            "update_task",
            json!({"task_id": "task-draft", "name": "Draft v2", "dueDate": "2026-10-11",
                   "plannedDate": "2026-10-10", "tags": ["Office"], "estimatedMinutes": 45}),
            Object(&["id", "name", "dueDate", "tags"]),
        ),
        row(
            "append_to_note",
            json!({"object_type": "task", "object_id": "task-read", "text": "more"}),
            Object(&["id", "name", "type"]),
        ),
    ]
}

fn task_move_and_delete_rows() -> Vec<Row> {
    use Expect::Object;
    vec![
        row(
            "delete_task",
            json!({"task_id": "task-scrap"}),
            Object(&["id", "name", "deleted"]),
        ),
        row(
            "delete_tasks_batch",
            json!({"task_ids": ["task-scrap-2", "task-scrap-3"]}),
            Object(&["deleted_count", "results"]),
        ),
        // The parent-task destination is the branch with the descendant
        // guard, whose `containingTask` read once shipped undetected.
        row(
            "move_task",
            json!({"task_id": "task-review", "parent_task_id": "task-draft"}),
            Object(&["id", "name", "projectName", "inInbox"]),
        ),
        row(
            "move_tasks_batch",
            json!({"task_ids": ["task-read", "task-call"], "parent_task_id": "task-draft"}),
            Object(&["moved_count", "results"]),
        ),
    ]
}

fn notification_rows() -> Vec<Row> {
    use Expect::{List, Object};
    vec![
        row(
            "list_notifications",
            json!({"task_id": "task-draft"}),
            List(&[
                "id",
                "kind",
                "absoluteFireDate",
                "relativeFireOffset",
                "nextFireDate",
                "isSnoozed",
            ]),
        ),
        row(
            "add_notification",
            json!({"task_id": "task-draft", "absoluteDate": "2026-10-10T09:00:00Z"}),
            Object(&["id", "kind", "absoluteFireDate"]),
        ),
        row(
            "remove_notification",
            json!({"task_id": "task-draft", "notification_id": "notification-absolute"}),
            Object(&["taskId", "notificationId", "removed"]),
        ),
    ]
}

fn project_rows() -> Vec<Row> {
    use Expect::{List, Object};
    vec![
        row(
            "list_projects",
            json!({"folder": "Work", "sortBy": "dueDate", "limit": 10}),
            List(&["id", "name", "status"]),
        ),
        row(
            "get_project_counts",
            json!({}),
            Object(&[
                "total",
                "active",
                "onHold",
                "completed",
                "dropped",
                "stalled",
            ]),
        ),
        row(
            "search_projects",
            json!({"query": "la"}),
            List(&["id", "name", "status"]),
        ),
        row(
            "get_project",
            json!({"project_id_or_name": "project-launch"}),
            Object(&[
                "id",
                "name",
                "status",
                "reviewInterval",
                "nextTaskId",
                "rootTasks",
            ]),
        ),
        row(
            "create_project",
            json!({"name": "New project", "folder": "Work", "note": "n",
                   "dueDate": "2026-11-01", "deferDate": "2026-10-15", "sequential": true}),
            Object(&["id", "name", "status", "folderName"]),
        ),
        row(
            "complete_project",
            json!({"project_id_or_name": "Launch"}),
            Object(&["id", "name", "completed"]),
        ),
        row(
            "uncomplete_project",
            json!({"project_id_or_name": "project-old"}),
            Object(&["id", "name", "status"]),
        ),
        row(
            "delete_project",
            json!({"project_id_or_name": "project-scrap"}),
            Object(&["id", "name", "deleted"]),
        ),
        row(
            "delete_projects_batch",
            json!({"project_ids_or_names": ["project-scrap-2", "Scrap Project"]}),
            Object(BATCH_RESULT),
        ),
        row(
            "move_project",
            json!({"project_id_or_name": "Errands", "folder": "Work"}),
            Object(&["id", "name", "folderName"]),
        ),
        row(
            "update_project",
            json!({"project_id_or_name": "Launch", "name": "Launch 2", "note": "n",
                   "dueDate": "2026-10-25", "flagged": true, "tags": ["Office"],
                   "sequential": true, "reviewInterval": "2 weeks"}),
            Object(&["id", "name", "reviewInterval", "tags"]),
        ),
        row(
            "set_project_status",
            json!({"project_id_or_name": "Launch", "status": "on_hold"}),
            Object(&["id", "name", "status"]),
        ),
    ]
}

fn tag_rows() -> Vec<Row> {
    use Expect::{List, Object};
    vec![
        row(
            "search_tags",
            json!({"query": "o"}),
            List(&["id", "name", "status"]),
        ),
        row(
            "list_tags",
            json!({"limit": 10}),
            List(&[
                "id",
                "name",
                "status",
                "availableTaskCount",
                "totalTaskCount",
            ]),
        ),
        row(
            "create_tag",
            json!({"name": "New tag", "parent": "Office"}),
            Object(&["id", "name", "parent"]),
        ),
        row(
            "update_tag",
            json!({"tag_name_or_id": "tag-home", "name": "House", "status": "on_hold"}),
            Object(&["id", "name", "status"]),
        ),
        row(
            "delete_tag",
            json!({"tag_name_or_id": "tag-scrap"}),
            Object(&["id", "name", "deleted"]),
        ),
        row(
            "delete_tags_batch",
            json!({"tag_ids_or_names": ["tag-scrap-2"]}),
            Object(BATCH_RESULT),
        ),
    ]
}

fn folder_rows() -> Vec<Row> {
    use Expect::{List, Object};
    vec![
        row(
            "list_folders",
            json!({"limit": 10}),
            List(&["id", "name", "parentName"]),
        ),
        row(
            "create_folder",
            json!({"name": "New folder", "parent": "Work"}),
            Object(&["id", "name", "parentName"]),
        ),
        row(
            "get_folder",
            json!({"folder_name_or_id": "folder-work"}),
            Object(&["id", "name", "projects", "subfolders"]),
        ),
        row(
            "update_folder",
            json!({"folder_name_or_id": "folder-clients", "name": "Customers", "status": "dropped"}),
            Object(&["id", "name", "status"]),
        ),
        row(
            "delete_folder",
            json!({"folder_name_or_id": "folder-scrap"}),
            Object(&["id", "name", "deleted"]),
        ),
        row(
            "delete_folders_batch",
            json!({"folder_ids_or_names": ["folder-scrap-2"]}),
            Object(BATCH_RESULT),
        ),
    ]
}

fn view_rows() -> Vec<Row> {
    use Expect::{KnownDefect, Object};
    vec![
        row(
            "get_forecast",
            json!({"limit": 10}),
            Object(&["counts", "overdue", "dueThisWeek", "deferred"]),
        ),
        // KNOWN DEFECT NEW (found by this harness): list_perspectives reads
        // `id` (then `identifier`) on built-in perspectives, whose class
        // (`OFMBuiltInPerspectiveIdentifier.Wrapper`) documents only `name`,
        // and probes `document.perspectives`, which `Document` does not
        // document. OmniFocus answers `undefined`, so the tool falls back to
        // the name, but it rests on undocumented reads.
        row(
            "list_perspectives",
            json!({"limit": 10}),
            KnownDefect {
                violation: "read of undocumented Perspective.BuiltIn.id",
            },
        ),
    ]
}
