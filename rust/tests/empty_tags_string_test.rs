//! An empty `tags` string is rejected, never read as "no tags" (audit CR-022).
//!
//! `tags` accepts a JSON array or a string holding one, for clients that send
//! every argument as a string. An empty (or whitespace-only) string used to
//! deserialize as an empty list, and `update_task` and `update_project` treat
//! an empty list as "replace the tags with none": a stray `"tags": ""` wiped
//! every tag. An empty string is what a client sends by accident, so it is now
//! rejected before any tool runs; the explicit "clear all" is `[]` (or the
//! string `"[]"`), which still works.

#[path = "support/mcp_session.rs"]
mod mcp_session;

use mcp_session::{assert_tool_success, call_tool, protocol_error_message, tool_error_text};
use omnifocus_mcp::flexible_tags::FlexibleTagList;
use serde_json::json;

// ---------------------------------------------------------------- the type

#[test]
fn an_empty_string_is_rejected() {
    let err = serde_json::from_str::<FlexibleTagList>(r#""""#).unwrap_err();
    let message = err.to_string();
    assert!(
        message.contains("tags must not be an empty string"),
        "{message}"
    );
    assert!(
        message.contains("[]"),
        "the message names the explicit clear: {message}"
    );
}

#[test]
fn a_whitespace_only_string_is_rejected() {
    let err = serde_json::from_str::<FlexibleTagList>(r#"" \t ""#).unwrap_err();
    assert!(
        err.to_string().contains("tags must not be an empty string"),
        "{err}"
    );
}

#[test]
fn an_empty_array_still_clears() {
    let tags: FlexibleTagList = serde_json::from_str("[]").expect("[] deserializes");
    assert!(tags.into_vec().is_empty());
}

#[test]
fn a_string_holding_an_empty_array_still_clears() {
    let tags: FlexibleTagList = serde_json::from_str(r#""[]""#).expect("\"[]\" deserializes");
    assert!(tags.into_vec().is_empty());
}

#[test]
fn a_non_string_element_is_rejected_in_either_encoding() {
    assert!(serde_json::from_str::<FlexibleTagList>(r#"["Home", 1]"#).is_err());
    assert!(serde_json::from_str::<FlexibleTagList>(r#""[\"Home\", 1]""#).is_err());
    assert!(serde_json::from_str::<FlexibleTagList>(r#"[null]"#).is_err());
}

// ---------------------------------------------------------------- through the server

/// Every tool that writes tags refuses `"tags": ""` before OmniFocus runs.
#[tokio::test]
async fn every_tag_writing_tool_rejects_an_empty_tags_string_without_running() {
    let cases = [
        ("create_task", json!({"name": "x", "tags": ""})),
        (
            "create_subtask",
            json!({"name": "x", "parent_task_id": "p", "tags": ""}),
        ),
        (
            "create_tasks_batch",
            json!({"tasks": [{"name": "x", "tags": ""}]}),
        ),
        ("update_task", json!({"task_id": "t1", "tags": ""})),
        ("update_task", json!({"task_id": "t1", "tags": "   "})),
        (
            "update_project",
            json!({"project_id_or_name": "p1", "tags": ""}),
        ),
    ];
    for (tool, arguments) in cases {
        let call = call_tool(tool, arguments.clone()).await;
        let message = protocol_error_message(&call.response);
        assert!(
            message.contains("tags must not be an empty string"),
            "{tool} {arguments}: {message}"
        );
        assert!(
            call.scripts.is_empty(),
            "{tool} {arguments} must not reach OmniFocus"
        );
    }
}

#[tokio::test]
async fn update_task_with_an_empty_array_still_clears_every_tag() {
    let call = call_tool("update_task", json!({"task_id": "t1", "tags": []})).await;
    assert_tool_success(&call.response);
    let script = call.scripts.last().expect("update_task ran a script");
    assert!(
        script.contains(r#"const updates = {"tags":[]};"#),
        "the explicit clear reaches the script: {script}"
    );
}

#[tokio::test]
async fn update_project_still_rejects_a_blank_tag_inside_the_array() {
    let call = call_tool(
        "update_project",
        json!({"project_id_or_name": "p1", "tags": ["Home", " "]}),
    )
    .await;
    assert_eq!(
        tool_error_text(&call.response),
        "tags must not contain empty values."
    );
    assert!(call.scripts.is_empty());
}
