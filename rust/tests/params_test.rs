//! Tool/prompt parameter wire-contract tests.
//!
//! Two defects reported upstream (issues #8 and #11) are pinned here:
//! 1. Clients that serialize every argument as a string (`"30"`, `"true"`) must be
//!    accepted for numeric and boolean fields, while the advertised JSON schema keeps
//!    the native `integer` / `number` / `boolean` types.
//! 2. Unknown keys (e.g. `folderId` instead of `folder`) must be rejected instead of
//!    being silently dropped, and every tool schema must say so with
//!    `additionalProperties: false`.
//!
//! Deserialization goes through `serde_json::from_value`, the same call rmcp's
//! `Parameters<T>` extractor makes on the tool-call `arguments` object.

use std::{future::Future, pin::Pin};

use omnifocus_mcp::{
    jxa::JxaRunner,
    server::{
        AddNotificationParams, BatchCreateTaskInput, CreateProjectParams, CreateSubtaskParams,
        CreateTaskParams, CreateTasksBatchParams, DuplicateTaskParams, GetTaskCountsParams,
        LimitParams, ListProjectsParams, ListTagsParams, ListTasksParams, OmniFocusServer,
        ProjectPlanningPromptParams, SearchProjectsParams, SearchTagsParams, SearchTasksParams,
        TaskIdLimitParams, UpdateProjectParams, UpdateTaskParams,
    },
};
use rmcp::ServerHandler;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};

/// Runner for schema inspection only; no tool is ever invoked.
struct NeverCalledRunner;

impl JxaRunner for NeverCalledRunner {
    fn run_omnijs<'a>(
        &'a self,
        _script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        Box::pin(async { panic!("schema tests must not run omnijs") })
    }
}

fn parse<T: DeserializeOwned>(input: Value) -> Result<T, serde_json::Error> {
    serde_json::from_value(input)
}

/// Deserializes `input` as `T`, re-serializes it, and checks each `expected` key
/// carries the native JSON value (so `"30"` must come back as `30`).
fn assert_coerced<T: DeserializeOwned + Serialize>(input: Value, expected: Value) {
    let parsed: T =
        parse(input.clone()).unwrap_or_else(|error| panic!("{input} should deserialize: {error}"));
    let wire = serde_json::to_value(&parsed).expect("re-serialize params");
    for (key, want) in expected.as_object().expect("expected is an object") {
        assert_eq!(&wire[key], want, "field `{key}` from {input}");
    }
}

// ---- Issue reproductions ------------------------------------------------------

#[test]
fn update_task_accepts_string_estimated_minutes() {
    assert_coerced::<UpdateTaskParams>(
        json!({"task_id": "abc123", "estimatedMinutes": "30"}),
        json!({"estimatedMinutes": 30}),
    );
}

#[test]
fn create_project_accepts_string_sequential() {
    assert_coerced::<CreateProjectParams>(
        json!({"name": "Project", "sequential": "true"}),
        json!({"sequential": true}),
    );
}

#[test]
fn list_projects_accepts_string_limit() {
    assert_coerced::<ListProjectsParams>(json!({"limit": "5"}), json!({"limit": 5}));
}

// ---- Every lenient field, struct by struct --------------------------------------

#[test]
fn limit_params_coerce_strings() {
    assert_coerced::<LimitParams>(json!({"limit": "5"}), json!({"limit": 5}));
}

#[test]
fn list_tasks_params_coerce_strings() {
    assert_coerced::<ListTasksParams>(
        json!({"flagged": "true", "maxEstimatedMinutes": "30", "limit": "10"}),
        json!({"flagged": true, "maxEstimatedMinutes": 30, "limit": 10}),
    );
}

#[test]
fn get_task_counts_params_coerce_strings() {
    assert_coerced::<GetTaskCountsParams>(
        json!({"flagged": "false", "maxEstimatedMinutes": "15"}),
        json!({"flagged": false, "maxEstimatedMinutes": 15}),
    );
}

#[test]
fn task_id_limit_params_coerce_strings() {
    assert_coerced::<TaskIdLimitParams>(
        json!({"task_id": "t1", "limit": "7"}),
        json!({"limit": 7}),
    );
}

#[test]
fn add_notification_params_coerce_strings() {
    assert_coerced::<AddNotificationParams>(
        json!({"task_id": "t1", "relativeOffset": "-3600"}),
        json!({"relativeOffset": -3600.0}),
    );
}

#[test]
fn duplicate_task_params_coerce_strings() {
    assert_coerced::<DuplicateTaskParams>(
        json!({"task_id": "t1", "includeChildren": "false"}),
        json!({"includeChildren": false}),
    );
}

#[test]
fn search_tasks_params_coerce_strings() {
    assert_coerced::<SearchTasksParams>(
        json!({"query": "q", "flagged": "TRUE", "maxEstimatedMinutes": "45", "limit": "3"}),
        json!({"flagged": true, "maxEstimatedMinutes": 45, "limit": 3}),
    );
}

#[test]
fn create_task_params_coerce_strings() {
    assert_coerced::<CreateTaskParams>(
        json!({"name": "n", "flagged": "true", "estimatedMinutes": "30"}),
        json!({"flagged": true, "estimatedMinutes": 30}),
    );
}

#[test]
fn create_subtask_params_coerce_strings() {
    assert_coerced::<CreateSubtaskParams>(
        json!({"name": "n", "parent_task_id": "p", "flagged": "true", "estimatedMinutes": "20"}),
        json!({"flagged": true, "estimatedMinutes": 20}),
    );
}

#[test]
fn batch_create_task_input_coerces_strings() {
    assert_coerced::<BatchCreateTaskInput>(
        json!({"name": "n", "flagged": "false", "estimatedMinutes": "5"}),
        json!({"flagged": false, "estimatedMinutes": 5}),
    );
}

#[test]
fn create_tasks_batch_params_coerce_nested_strings() {
    assert_coerced::<CreateTasksBatchParams>(
        json!({"tasks": [{"name": "n", "flagged": "true", "estimatedMinutes": "30"}]}),
        json!({"tasks": [{
            "name": "n", "project": null, "note": null, "dueDate": null, "deferDate": null,
            "plannedDate": null, "flagged": true, "tags": null, "estimatedMinutes": 30
        }]}),
    );
}

#[test]
fn update_task_params_coerce_strings() {
    assert_coerced::<UpdateTaskParams>(
        json!({"task_id": "t", "flagged": "true", "estimatedMinutes": "30"}),
        json!({"flagged": true, "estimatedMinutes": 30}),
    );
}

#[test]
fn list_projects_params_coerce_strings() {
    assert_coerced::<ListProjectsParams>(
        json!({"stalledOnly": "true", "limit": "5"}),
        json!({"stalledOnly": true, "limit": 5}),
    );
}

#[test]
fn search_projects_params_coerce_strings() {
    assert_coerced::<SearchProjectsParams>(
        json!({"query": "q", "limit": "4"}),
        json!({"limit": 4}),
    );
}

#[test]
fn search_tags_params_coerce_strings() {
    assert_coerced::<SearchTagsParams>(json!({"query": "q", "limit": "4"}), json!({"limit": 4}));
}

#[test]
fn list_tags_params_coerce_strings() {
    assert_coerced::<ListTagsParams>(json!({"limit": "9"}), json!({"limit": 9}));
}

#[test]
fn create_project_params_coerce_strings() {
    assert_coerced::<CreateProjectParams>(
        json!({"name": "n", "sequential": "false"}),
        json!({"sequential": false}),
    );
}

#[test]
fn update_project_params_coerce_strings() {
    assert_coerced::<UpdateProjectParams>(
        json!({
            "project_id_or_name": "p",
            "flagged": "true",
            "sequential": "false",
            "completedByChildren": "true"
        }),
        json!({"flagged": true, "sequential": false, "completedByChildren": true}),
    );
}

// ---- Absent values and aliases ------------------------------------------------

#[test]
fn omitted_lenient_fields_are_none() {
    let params: UpdateTaskParams = parse(json!({"task_id": "t"})).expect("minimal update");
    assert!(params.estimated_minutes.is_none());
    assert!(params.flagged.is_none());
}

#[test]
fn null_lenient_fields_are_none() {
    let params: UpdateTaskParams =
        parse(json!({"task_id": "t", "estimatedMinutes": null, "flagged": null}))
            .expect("explicit nulls");
    assert!(params.estimated_minutes.is_none());
    assert!(params.flagged.is_none());
}

#[test]
fn snake_case_alias_accepts_string_value() {
    assert_coerced::<UpdateTaskParams>(
        json!({"task_id": "t", "estimated_minutes": "30"}),
        json!({"estimatedMinutes": 30}),
    );
}

#[test]
fn snake_case_alias_accepts_string_boolean() {
    assert_coerced::<ListProjectsParams>(
        json!({"stalled_only": "true"}),
        json!({"stalledOnly": true}),
    );
}

#[test]
fn snake_case_date_alias_still_accepted() {
    assert_coerced::<CreateProjectParams>(
        json!({"name": "n", "due_date": "2026-06-01T10:00:00Z"}),
        json!({"dueDate": "2026-06-01T10:00:00Z"}),
    );
}

#[test]
fn non_numeric_string_is_still_an_error() {
    assert!(parse::<UpdateTaskParams>(json!({"task_id": "t", "estimatedMinutes": "abc"})).is_err());
}

// ---- Advertised schemas -------------------------------------------------------

fn advertised_schema(tool: &str) -> Value {
    let server = OmniFocusServer::new(NeverCalledRunner);
    let tool_def = server
        .get_tool(tool)
        .unwrap_or_else(|| panic!("tool `{tool}` is registered"));
    Value::Object((*tool_def.input_schema).clone())
}

#[test]
fn create_task_schema_keeps_native_types() {
    let schema = serde_json::to_value(schemars::schema_for!(CreateTaskParams)).unwrap();
    assert_eq!(
        schema["properties"]["estimatedMinutes"],
        json!({"type": ["integer", "null"], "format": "int32"})
    );
    assert_eq!(
        schema["properties"]["flagged"],
        json!({"type": ["boolean", "null"]})
    );
}

#[test]
fn every_lenient_field_advertises_its_native_type() {
    let integer = json!({"type": ["integer", "null"], "format": "int32"});
    let number = json!({"type": ["number", "null"], "format": "double"});
    let boolean = json!({"type": ["boolean", "null"]});
    let cases: &[(&str, &str, &Value)] = &[
        ("get_inbox", "limit", &integer),
        ("list_tasks", "flagged", &boolean),
        ("list_tasks", "maxEstimatedMinutes", &integer),
        ("list_tasks", "limit", &integer),
        ("get_task_counts", "flagged", &boolean),
        ("get_task_counts", "maxEstimatedMinutes", &integer),
        ("list_subtasks", "limit", &integer),
        ("add_notification", "relativeOffset", &number),
        ("duplicate_task", "includeChildren", &boolean),
        ("search_tasks", "flagged", &boolean),
        ("search_tasks", "maxEstimatedMinutes", &integer),
        ("search_tasks", "limit", &integer),
        ("create_task", "flagged", &boolean),
        ("create_task", "estimatedMinutes", &integer),
        ("create_subtask", "flagged", &boolean),
        ("create_subtask", "estimatedMinutes", &integer),
        ("update_task", "flagged", &boolean),
        ("update_task", "estimatedMinutes", &integer),
        ("list_projects", "stalledOnly", &boolean),
        ("list_projects", "limit", &integer),
        ("search_projects", "limit", &integer),
        ("search_tags", "limit", &integer),
        ("list_tags", "limit", &integer),
        ("create_project", "sequential", &boolean),
        ("update_project", "flagged", &boolean),
        ("update_project", "sequential", &boolean),
        ("update_project", "completedByChildren", &boolean),
    ];
    for (tool, field, expected) in cases {
        let schema = advertised_schema(tool);
        assert_eq!(&schema["properties"][field], *expected, "{tool}.{field}");
    }
    let batch = advertised_schema("create_tasks_batch");
    let item = &batch["$defs"]["BatchCreateTaskInput"]["properties"];
    assert_eq!(item["flagged"], boolean, "BatchCreateTaskInput.flagged");
    assert_eq!(
        item["estimatedMinutes"], integer,
        "BatchCreateTaskInput.estimatedMinutes"
    );
}

// ---- Unknown keys -------------------------------------------------------------

fn assert_unknown_field_rejected<T: DeserializeOwned>(input: Value, field: &str) {
    match parse::<T>(input.clone()) {
        Ok(_) => panic!("{input} should be rejected for unknown field `{field}`"),
        Err(error) => assert!(
            error
                .to_string()
                .contains(&format!("unknown field `{field}`")),
            "unexpected error for {input}: {error}"
        ),
    }
}

#[test]
fn create_project_rejects_folder_id() {
    assert_unknown_field_rejected::<CreateProjectParams>(
        json!({"name": "n", "folderId": "f1"}),
        "folderId",
    );
}

#[test]
fn update_task_rejects_misspelled_key() {
    assert_unknown_field_rejected::<UpdateTaskParams>(
        json!({"task_id": "t", "estimatedMinute": 30}),
        "estimatedMinute",
    );
}

#[test]
fn batch_create_task_input_rejects_unknown_key() {
    assert_unknown_field_rejected::<BatchCreateTaskInput>(
        json!({"name": "n", "priority": "high"}),
        "priority",
    );
}

#[test]
fn create_tasks_batch_rejects_unknown_key_in_nested_task() {
    assert_unknown_field_rejected::<CreateTasksBatchParams>(
        json!({"tasks": [{"name": "n", "priority": "high"}]}),
        "priority",
    );
}

#[test]
fn project_planning_prompt_rejects_unknown_key() {
    assert_unknown_field_rejected::<ProjectPlanningPromptParams>(
        json!({"project": "p", "folder": "f"}),
        "folder",
    );
}

/// Every tool registered on the server; keep in step with `server.rs`.
const ALL_TOOLS: &[&str] = &[
    "get_inbox",
    "list_tasks",
    "get_task_counts",
    "get_task",
    "list_subtasks",
    "list_notifications",
    "add_notification",
    "duplicate_task",
    "remove_notification",
    "search_tasks",
    "create_task",
    "create_tasks_batch",
    "create_subtask",
    "complete_task",
    "uncomplete_task",
    "set_task_repetition",
    "update_task",
    "delete_task",
    "delete_tasks_batch",
    "move_task",
    "move_tasks_batch",
    "append_to_note",
    "list_projects",
    "get_project_counts",
    "search_projects",
    "get_project",
    "create_project",
    "complete_project",
    "uncomplete_project",
    "delete_project",
    "delete_projects_batch",
    "move_project",
    "update_project",
    "set_project_status",
    "search_tags",
    "list_tags",
    "create_tag",
    "update_tag",
    "delete_tag",
    "delete_tags_batch",
    "list_folders",
    "create_folder",
    "get_folder",
    "update_folder",
    "delete_folder",
    "delete_folders_batch",
    "get_forecast",
    "list_perspectives",
];

#[test]
fn create_task_schema_forbids_additional_properties() {
    let schema = serde_json::to_value(schemars::schema_for!(CreateTaskParams)).unwrap();
    assert_eq!(schema["additionalProperties"], json!(false));
}

#[test]
fn every_tool_schema_forbids_additional_properties() {
    for tool in ALL_TOOLS {
        let schema = advertised_schema(tool);
        assert_eq!(
            schema["additionalProperties"],
            json!(false),
            "tool `{tool}`"
        );
    }
    let batch = advertised_schema("create_tasks_batch");
    assert_eq!(
        batch["$defs"]["BatchCreateTaskInput"]["additionalProperties"],
        json!(false),
        "nested BatchCreateTaskInput"
    );
}
