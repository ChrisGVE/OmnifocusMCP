//! Advertised schema of folder and project parameters (upstream #11).
//!
//! Every `folder` and `project` parameter accepts an id or an exact name and
//! refuses a value that matches nothing (see `folder_project_resolution_test.rs`).
//! Clients only learn that from the tool schema, so each such field must say
//! so in its description.

use std::{future::Future, pin::Pin};

use omnifocus_mcp::{
    jxa::JxaRunner,
    server::{
        BatchCreateTaskInput, CreateProjectParams, CreateTaskParams, GetTaskCountsParams,
        ListProjectsParams, ListTasksParams, OmniFocusServer, ProjectPlanningPromptParams,
        SearchTasksParams,
    },
};
use rmcp::ServerHandler;
use serde_json::Value;

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

/// The description a registered tool advertises for one of its fields.
fn advertised_description(tool: &str, field: &str) -> String {
    let server = OmniFocusServer::new(NeverCalledRunner);
    let tool_def = server
        .get_tool(tool)
        .unwrap_or_else(|| panic!("tool `{tool}` is registered"));
    let schema = Value::Object((*tool_def.input_schema).clone());
    schema["properties"][field]["description"]
        .as_str()
        .unwrap_or_else(|| panic!("{tool}.{field} has no description"))
        .to_string()
}

/// The description a param struct's schema gives one of its fields; covers
/// the nested batch input and the prompt params, which no tool lists directly.
fn struct_description<T: schemars::JsonSchema>(field: &str) -> String {
    let schema = serde_json::to_value(schemars::schema_for!(T)).expect("schema serializes");
    schema["properties"][field]["description"]
        .as_str()
        .unwrap_or_else(|| panic!("{field} has no description"))
        .to_string()
}

#[test]
fn every_folder_and_project_field_says_id_or_exact_name() {
    let cases = [
        ("list_projects", "folder", "Folder id or exact name"),
        ("get_project_counts", "folder", "Folder id or exact name"),
        ("create_project", "folder", "Folder id or exact name"),
        ("move_project", "folder", "Folder id or exact name"),
        ("create_folder", "parent", "Folder id or exact name"),
        ("list_tasks", "project", "Project id or exact name"),
        ("get_task_counts", "project", "Project id or exact name"),
        ("search_tasks", "project", "Project id or exact name"),
        ("create_task", "project", "Project id or exact name"),
        ("move_task", "project", "Project id or exact name"),
        ("move_tasks_batch", "project", "Project id or exact name"),
    ];
    for (tool, field, expected) in cases {
        let description = advertised_description(tool, field);
        assert!(
            description.starts_with(expected),
            "{tool}.{field}: {description}"
        );
    }
    let structs = [
        struct_description::<ListProjectsParams>("folder"),
        struct_description::<CreateProjectParams>("folder"),
        struct_description::<ListTasksParams>("project"),
        struct_description::<GetTaskCountsParams>("project"),
        struct_description::<SearchTasksParams>("project"),
        struct_description::<CreateTaskParams>("project"),
        struct_description::<BatchCreateTaskInput>("project"),
        struct_description::<ProjectPlanningPromptParams>("project"),
    ];
    for description in structs {
        assert!(description.contains("id or exact name"), "{description}");
    }
}
