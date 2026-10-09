use std::{future::Future, pin::Pin};

use omnifocus_mcp::{
    error::OmniFocusError,
    jxa::{
        escape_for_jxa, friendly_jxa_error, run_script_with_runner, unwrap_omnijs_envelope,
        JxaRunner,
    },
};
use serde_json::{json, Value};

#[test]
fn escape_for_jxa_handles_special_strings() {
    assert_eq!(
        escape_for_jxa("He said \"hello\""),
        "\"He said \\\"hello\\\"\""
    );
    assert_eq!(escape_for_jxa(r"C:\Users\test"), r#""C:\\Users\\test""#);
    assert_eq!(escape_for_jxa("line1\nline2"), r#""line1\nline2""#);
    assert_eq!(escape_for_jxa("emoji 😀"), "\"emoji 😀\"");
    assert_eq!(escape_for_jxa("a\0b"), r#""a\u0000b""#);
}

#[test]
fn omnifocus_error_display_messages_match_expected() {
    let jxa = OmniFocusError::JxaExecution("boom".to_string());
    assert_eq!(jxa.to_string(), "JXA execution failed: boom");

    let omni = OmniFocusError::OmniFocus("Task not found: abc".to_string());
    assert_eq!(omni.to_string(), "Task not found: abc");

    let validation = OmniFocusError::Validation("limit must be greater than 0.".to_string());
    assert_eq!(validation.to_string(), "limit must be greater than 0.");

    let timeout = OmniFocusError::Timeout {
        after: std::time::Duration::from_secs(30),
    };
    assert_eq!(timeout.to_string(), "JXA command timed out after 30s.");

    let io = OmniFocusError::Io(std::io::Error::other("denied"));
    assert_eq!(io.to_string(), "I/O error while running JXA: denied");
}

#[test]
fn unknown_jxa_failure_has_single_execution_prefix() {
    let error = OmniFocusError::JxaExecution(friendly_jxa_error("unexpected osascript failure"));

    assert_eq!(
        error.to_string(),
        "JXA execution failed: unexpected osascript failure"
    );
}

#[derive(Clone)]
struct MockRunner {
    payload: Value,
}

impl JxaRunner for MockRunner {
    fn run_omnijs<'a>(
        &'a self,
        _script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        Box::pin(async move { Ok(self.payload.clone()) })
    }
}

#[tokio::test]
async fn mock_runner_returns_canned_data() {
    let runner = MockRunner {
        payload: json!({"id": "abc123", "name": "test task"}),
    };
    let result = run_script_with_runner(&runner, "return 1;").await;
    assert!(result.is_ok());
    assert_eq!(
        result.ok(),
        Some(json!({"id": "abc123", "name": "test task"}))
    );
}

#[test]
fn unwrap_envelope_success_returns_data() {
    let result = unwrap_omnijs_envelope(json!({"ok": true, "data": {"count": 3}}));
    assert!(result.is_ok());
    assert_eq!(result.ok(), Some(json!({"count": 3})));
}

#[test]
fn unwrap_envelope_error_returns_friendly_message() {
    let result = unwrap_omnijs_envelope(json!({"ok": false, "error": "Task not found: abc"}));
    assert!(result.is_err());
    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("Task not found: abc".to_string())
    );
}
