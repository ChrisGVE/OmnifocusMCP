//! Errors raised when JSON from OmniFocus cannot be used (audit CR-012).
//!
//! Two different things can go wrong with the JSON a script hands back:
//!
//! 1. osascript's stdout is not JSON at all (the script printed something
//!    else, or was cut off) — reported as `MalformedOutput`;
//! 2. the JSON parses, but is not the shape the tool reads it into (a missing
//!    field, a string where a number belongs) — reported as
//!    `UnexpectedResultShape`.
//!
//! Both used to be one variant whose message, "JXA command returned
//! malformed JSON.", dropped serde's explanation, so a shape mismatch was
//! misreported as broken output and nobody could tell which field was wrong.
//! These tests pin that each case has its own variant and that its message
//! carries serde's detail.

use std::{collections::BTreeMap, future::Future, pin::Pin};

use omnifocus_mcp::{
    error::{to_json_string, OmniFocusError},
    jxa::{parse_jxa_output, JxaRunner},
    tools::{
        projects::get_project_counts,
        tasks::{get_inbox, get_task_counts_with_added_changed},
    },
};
use serde_json::{json, Value};

/// Returns the same payload for every script, as OmniFocus would.
struct PayloadRunner {
    payload: Value,
}

impl JxaRunner for PayloadRunner {
    fn run_omnijs<'a>(
        &'a self,
        _script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        Box::pin(async move { Ok(self.payload.clone()) })
    }
}

async fn task_counts_error(payload: Value) -> OmniFocusError {
    let runner = PayloadRunner { payload };
    get_task_counts_with_added_changed(
        &runner, None, None, None, "any", None, None, None, None, None, None, None, None, None,
        None, None, None, None, None,
    )
    .await
    .expect_err("a counts payload of the wrong shape must fail")
}

/// Asserts a shape error and returns its message.
fn shape_message(error: OmniFocusError) -> String {
    assert!(
        matches!(error, OmniFocusError::UnexpectedResultShape(_)),
        "expected a result-shape error, got {error:?}"
    );
    error.to_string()
}

#[tokio::test]
async fn task_list_shape_mismatch_names_the_missing_field() {
    let runner = PayloadRunner {
        payload: json!([{"id": "abc"}]),
    };
    let error = get_inbox(&runner, 10)
        .await
        .expect_err("a task without its fields must fail");
    let message = shape_message(error);
    assert!(
        message.contains("missing field `name`"),
        "serde's detail is lost: {message}"
    );
}

#[tokio::test]
async fn project_counts_shape_mismatch_names_the_missing_field() {
    let runner = PayloadRunner {
        payload: json!({"total": 1}),
    };
    let error = get_project_counts(&runner, None)
        .await
        .expect_err("counts without their fields must fail");
    let message = shape_message(error);
    assert!(
        message.contains("missing field `active`"),
        "serde's detail is lost: {message}"
    );
}

#[tokio::test]
async fn task_counts_shape_mismatch_names_the_wrong_type() {
    let message = shape_message(task_counts_error(json!({"total": "many"})).await);
    assert!(
        message.contains(r#"invalid type: string "many""#),
        "serde's detail is lost: {message}"
    );
}

#[test]
fn stdout_that_is_not_json_is_malformed_output_with_detail() {
    let error = parse_jxa_output("{not json").expect_err("not JSON must fail");
    assert!(
        matches!(error, OmniFocusError::MalformedOutput(_)),
        "expected malformed output, got {error:?}"
    );
    let message = error.to_string();
    assert!(
        message.starts_with("JXA command returned malformed JSON: "),
        "unexpected message: {message}"
    );
    assert!(
        message.contains("key must be a string"),
        "serde's detail is lost: {message}"
    );
}

#[test]
fn empty_stdout_is_reported_as_empty_output() {
    let error = parse_jxa_output("").expect_err("empty output must fail");
    assert_eq!(
        error.to_string(),
        "JXA execution failed: JXA command returned empty output."
    );
}

#[test]
fn data_that_cannot_be_encoded_is_an_encoding_error_with_detail() {
    // JSON object keys must be strings, so a map keyed by a list cannot be
    // encoded: the one way our own data can fail to become JSON.
    let unencodable = BTreeMap::from([(vec![1_u8], 1_i32)]);
    let error = to_json_string(&unencodable).expect_err("a non-string key must fail");
    assert!(
        matches!(error, OmniFocusError::Encoding(_)),
        "expected an encoding error, got {error:?}"
    );
    assert_eq!(
        error.to_string(),
        "Could not encode JSON: key must be a string"
    );
}

#[test]
fn stdout_that_is_json_parses() {
    let value = parse_jxa_output(r#"{"ok": true, "data": 3}"#).expect("valid JSON parses");
    assert_eq!(value, json!({"ok": true, "data": 3}));
}
