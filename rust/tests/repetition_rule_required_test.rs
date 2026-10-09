//! `set_task_repetition` requires `rule_string`; only an explicit `null`
//! clears the repetition (audit CR-023).
//!
//! `rule_string` used to be optional, and an absent value cleared the rule
//! exactly like `null`. So `{task_id, schedule_type: "from_completion"}`,
//! which reads as "switch this task to repeat from completion", silently
//! removed its repetition. The key must now be present: a string sets the
//! rule, `null` clears it, and a missing key is a tool error naming the field
//! that runs nothing.

#[path = "support/mcp_session.rs"]
mod mcp_session;

use std::{future::Future, pin::Pin};

use mcp_session::{assert_tool_success, call_tool, protocol_error_message, tool_error_text};
use omnifocus_mcp::{jxa::JxaRunner, server::OmniFocusServer};
use rmcp::ServerHandler;
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

fn set_task_repetition_tool() -> rmcp::model::Tool {
    OmniFocusServer::new(NeverCalledRunner)
        .get_tool("set_task_repetition")
        .expect("set_task_repetition is registered")
}

// ---------------------------------------------------------------- advertised contract

#[test]
fn schema_lists_rule_string_as_required_and_nullable() {
    let schema = Value::Object((*set_task_repetition_tool().input_schema).clone());
    let required: Vec<&str> = schema["required"]
        .as_array()
        .unwrap_or_else(|| panic!("schema has a required list: {schema}"))
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(required.contains(&"rule_string"), "{schema}");
    assert!(required.contains(&"task_id"), "{schema}");

    let rule_string = &schema["properties"]["rule_string"];
    let types = rule_string["type"].clone();
    assert_eq!(types, json!(["string", "null"]), "{rule_string}");
    // A default of null would advertise "omit it to clear", the very bug.
    assert!(rule_string.get("default").is_none(), "{rule_string}");
    let description = rule_string["description"]
        .as_str()
        .unwrap_or_else(|| panic!("rule_string has a description: {rule_string}"));
    assert!(
        description.contains("required; null clears"),
        "{description}"
    );
}

#[test]
fn tool_description_says_rule_string_is_required() {
    let tool = set_task_repetition_tool();
    let description = tool.description.as_deref().unwrap_or_default();
    assert!(
        description.contains("rule_string is required"),
        "{description}"
    );
    assert!(
        description.contains("null rule_string clears"),
        "{description}"
    );
}

// ---------------------------------------------------------------- behaviour

#[tokio::test]
async fn a_missing_rule_string_is_a_tool_error_that_runs_nothing() {
    let call = call_tool(
        "set_task_repetition",
        json!({"task_id": "t1", "schedule_type": "from_completion"}),
    )
    .await;
    assert_eq!(
        tool_error_text(&call.response),
        "rule_string is required: pass a repetition rule to set, or null to clear the repetition."
    );
    assert!(call.scripts.is_empty(), "nothing may reach OmniFocus");
}

#[tokio::test]
async fn an_explicit_null_clears_the_repetition() {
    let call = call_tool(
        "set_task_repetition",
        json!({"task_id": "t1", "rule_string": null}),
    )
    .await;
    assert_tool_success(&call.response);
    let script = call.scripts.last().expect("a script ran");
    assert!(script.contains("const ruleString = null;"), "{script}");
}

#[tokio::test]
async fn a_rule_string_sets_the_repetition_with_the_schedule_type() {
    let call = call_tool(
        "set_task_repetition",
        json!({"task_id": "t1", "rule_string": "FREQ=WEEKLY", "schedule_type": "from_completion"}),
    )
    .await;
    assert_tool_success(&call.response);
    let script = call.scripts.last().expect("a script ran");
    assert!(
        script.contains(r#"const ruleString = "FREQ=WEEKLY";"#),
        "{script}"
    );
    assert!(
        script.contains(r#"const scheduleTypeInput = "from_completion";"#),
        "{script}"
    );
}

#[tokio::test]
async fn a_rule_string_of_the_wrong_type_is_rejected_before_the_tool_runs() {
    let call = call_tool(
        "set_task_repetition",
        json!({"task_id": "t1", "rule_string": 5}),
    )
    .await;
    let message = protocol_error_message(&call.response);
    assert!(message.contains("expected a string"), "{message}");
    assert!(call.scripts.is_empty());
}
