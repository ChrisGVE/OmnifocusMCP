//! Task writes reject a negative `estimatedMinutes` before any script is
//! built (audit CR-037).
//!
//! The read filter `maxEstimatedMinutes` already refused negatives, but
//! `create_task`, `create_subtask`, `update_task` and `create_tasks_batch`
//! passed any integer straight to OmniFocus. Each now checks the value in
//! Rust first: 0 is accepted, below 0 is a validation error, and the batch
//! names the failing entry (`tasks[i].estimatedMinutes`) without creating
//! anything.

#[path = "support/mcp_session.rs"]
mod mcp_session;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use mcp_session::{assert_tool_success, call_tool, protocol_error_message, tool_error_text};
use omnifocus_mcp::{
    error::OmniFocusError,
    jxa::JxaRunner,
    tools::tasks::{create_subtask, create_task, create_tasks_batch, update_task, CreateTaskInput},
};
use serde_json::{json, Value};

const NEGATIVE_MINUTES: &str = "estimatedMinutes must be greater than or equal to 0.";

/// Records each script it is asked to run and answers with an empty object.
#[derive(Default)]
struct CapturingRunner {
    scripts: Arc<Mutex<Vec<String>>>,
}

impl CapturingRunner {
    fn script_count(&self) -> usize {
        self.scripts.lock().expect("script lock").len()
    }

    fn last_script(&self) -> String {
        self.scripts
            .lock()
            .expect("script lock")
            .last()
            .cloned()
            .expect("a script was run")
    }
}

impl JxaRunner for CapturingRunner {
    fn run_omnijs<'a>(
        &'a self,
        script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        self.scripts
            .lock()
            .expect("script lock")
            .push(script.to_string());
        Box::pin(async { Ok(json!({})) })
    }
}

fn validation_message(result: omnifocus_mcp::error::Result<Value>) -> String {
    match result {
        Err(OmniFocusError::Validation(message)) => message,
        other => panic!("expected a validation error, got {other:?}"),
    }
}

fn batch_item(name: &str, estimated_minutes: Option<i32>) -> CreateTaskInput {
    CreateTaskInput {
        name: name.to_string(),
        project: None,
        note: None,
        due_date: None,
        defer_date: None,
        planned_date: None,
        flagged: None,
        tags: None,
        estimated_minutes,
    }
}

// ---------------------------------------------------------------- rejected

#[tokio::test]
async fn create_task_rejects_negative_minutes_without_running() {
    let runner = CapturingRunner::default();
    let result = create_task(
        &runner,
        "x",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(-1),
    )
    .await;
    assert_eq!(validation_message(result), NEGATIVE_MINUTES);
    assert_eq!(runner.script_count(), 0);
}

#[tokio::test]
async fn create_subtask_rejects_negative_minutes_without_running() {
    let runner = CapturingRunner::default();
    let result = create_subtask(
        &runner,
        "x",
        "parent",
        None,
        None,
        None,
        None,
        None,
        None,
        Some(-1),
    )
    .await;
    assert_eq!(validation_message(result), NEGATIVE_MINUTES);
    assert_eq!(runner.script_count(), 0);
}

#[tokio::test]
async fn update_task_rejects_negative_minutes_without_running() {
    let runner = CapturingRunner::default();
    let result = update_task(
        &runner,
        "t1",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(-30),
    )
    .await;
    assert_eq!(validation_message(result), NEGATIVE_MINUTES);
    assert_eq!(runner.script_count(), 0);
}

#[tokio::test]
async fn create_tasks_batch_names_the_entry_with_negative_minutes_and_creates_nothing() {
    let runner = CapturingRunner::default();
    let result = create_tasks_batch(
        &runner,
        vec![
            batch_item("first", Some(15)),
            batch_item("second", Some(-5)),
            batch_item("third", None),
        ],
    )
    .await;
    assert_eq!(
        validation_message(result),
        "tasks[1].estimatedMinutes must be greater than or equal to 0."
    );
    assert_eq!(runner.script_count(), 0);
}

#[tokio::test]
async fn the_minimum_i32_is_rejected_too() {
    let runner = CapturingRunner::default();
    let result = update_task(
        &runner,
        "t1",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(i32::MIN),
    )
    .await;
    assert_eq!(validation_message(result), NEGATIVE_MINUTES);
}

// ---------------------------------------------------------------- the boundary is accepted

#[tokio::test]
async fn zero_minutes_reaches_every_write_script() {
    let runner = CapturingRunner::default();
    create_task(
        &runner,
        "x",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(0),
    )
    .await
    .expect("create_task accepts 0");
    assert!(runner
        .last_script()
        .contains("const estimatedMinutesValue = 0;"));

    create_subtask(
        &runner,
        "x",
        "parent",
        None,
        None,
        None,
        None,
        None,
        None,
        Some(0),
    )
    .await
    .expect("create_subtask accepts 0");
    assert!(runner
        .last_script()
        .contains("const estimatedMinutesValue = 0;"));

    update_task(
        &runner,
        "t1",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(0),
    )
    .await
    .expect("update_task accepts 0");
    assert!(runner
        .last_script()
        .contains(r#"const updates = {"estimatedMinutes":0};"#));

    create_tasks_batch(&runner, vec![batch_item("only", Some(0))])
        .await
        .expect("create_tasks_batch accepts 0");
    assert!(runner.last_script().contains(r#""estimatedMinutes":0"#));
}

// ---------------------------------------------------------------- through the server

#[tokio::test]
async fn a_negative_value_is_a_tool_error_and_a_non_number_a_protocol_error() {
    let negative = call_tool(
        "create_tasks_batch",
        json!({"tasks": [{"name": "a"}, {"name": "b", "estimatedMinutes": "-10"}]}),
    )
    .await;
    assert_eq!(
        tool_error_text(&negative.response),
        "tasks[1].estimatedMinutes must be greater than or equal to 0."
    );
    assert!(negative.scripts.is_empty());

    let not_a_number = call_tool(
        "update_task",
        json!({"task_id": "t1", "estimatedMinutes": "soon"}),
    )
    .await;
    protocol_error_message(&not_a_number.response);
    assert!(not_a_number.scripts.is_empty());

    let zero = call_tool(
        "update_task",
        json!({"task_id": "t1", "estimatedMinutes": 0}),
    )
    .await;
    assert_tool_success(&zero.response);
}
