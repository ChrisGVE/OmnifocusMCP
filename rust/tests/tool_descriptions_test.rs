//! Tool descriptions state the 2.0.0 date and folder/project rules.
//!
//! A client learns how to call a tool from its description, and no date
//! field carries a description of its own. Since 2.0.0 every user date is
//! read by the shared OmniJS date helpers: a bare `YYYY-MM-DD` is a local day,
//! which a filter reads as local midnight and a write gives the OmniFocus
//! default time for that field (`js_date_helpers_test.rs` tests the helpers).
//! Folder and project parameters take an id or an exact name
//! (`folder_project_resolution_test.rs`). These tests pin that each affected
//! tool description says so, and that the pre-2.0.0 wording is gone.

use std::{future::Future, pin::Pin};

use omnifocus_mcp::{jxa::JxaRunner, server::OmniFocusServer};
use rmcp::ServerHandler;
use serde_json::Value;

/// Runner for description inspection only; no tool is ever invoked.
struct NeverCalledRunner;

impl JxaRunner for NeverCalledRunner {
    fn run_omnijs<'a>(
        &'a self,
        _script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        Box::pin(async { panic!("description tests must not run omnijs") })
    }
}

/// Tools whose date parameters are filters or a point in time to fire at.
const LOCAL_MIDNIGHT_TOOLS: [&str; 5] = [
    "list_tasks",
    "search_tasks",
    "get_task_counts",
    "list_projects",
    "add_notification",
];

/// Tools that write a task or project due/defer date.
const DEFAULT_TIME_TOOLS: [&str; 5] = [
    "create_task",
    "create_subtask",
    "update_task",
    "create_project",
    "update_project",
];

fn description(tool: &str) -> String {
    let server = OmniFocusServer::new(NeverCalledRunner);
    let tool_def = server
        .get_tool(tool)
        .unwrap_or_else(|| panic!("tool `{tool}` is registered"));
    tool_def
        .description
        .unwrap_or_else(|| panic!("tool `{tool}` has a description"))
        .into_owned()
}

#[test]
fn date_filters_say_a_bare_date_is_local_midnight() {
    for tool in LOCAL_MIDNIGHT_TOOLS {
        let text = description(tool);
        assert!(
            text.contains("YYYY-MM-DD (local midnight)"),
            "{tool}: {text}"
        );
        assert!(text.contains("ISO 8601 date-time"), "{tool}: {text}");
    }
}

#[test]
fn date_writes_say_a_bare_date_gets_the_default_time() {
    for tool in DEFAULT_TIME_TOOLS {
        let text = description(tool);
        assert!(text.contains("YYYY-MM-DD"), "{tool}: {text}");
        assert!(
            text.contains("your omnifocus default time for that field"),
            "{tool}: {text}"
        );
    }
}

#[test]
fn no_description_demands_iso_8601_only() {
    for tool in LOCAL_MIDNIGHT_TOOLS.iter().chain(DEFAULT_TIME_TOOLS.iter()) {
        let text = description(tool);
        assert!(!text.contains("must be ISO 8601"), "{tool}: {text}");
    }
}

#[test]
fn create_task_takes_a_project_id_or_exact_name() {
    let text = description("create_task");
    assert!(text.contains("project (id or exact name)"), "{text}");
    assert!(!text.contains("named project"), "{text}");
}

#[test]
fn create_folder_takes_a_parent_id_or_exact_name() {
    let text = description("create_folder");
    assert!(text.contains("parent folder (id or exact name)"), "{text}");
    assert!(!text.contains("folder name"), "{text}");
}
