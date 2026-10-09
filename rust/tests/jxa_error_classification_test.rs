//! Where an OmniFocus failure message comes from decides how it is read
//! (audit CR-011).
//!
//! A call can fail in two places:
//!
//! - **osascript itself** exits non-zero and writes to stderr. That is where
//!   macOS reports that OmniFocus is not running or that Automation access was
//!   refused, so stderr is classified into those friendly messages.
//! - **our script, inside OmniFocus**, throws and the wrapper returns
//!   `{ok: false, error}`. OmniFocus was running and reachable for that to
//!   happen, and the message often echoes user input (a tag name, a status
//!   value). It is passed through unchanged.
//!
//! The envelope message used to go through the same substring checks, so a
//! tag named "Not permitted" was reported as a macOS permission problem, and
//! every message outside a four-entry allow-list got an "OmniFocus operation
//! failed: " prefix.

use omnifocus_mcp::{
    error::OmniFocusError,
    jxa::{friendly_jxa_error, unwrap_omnijs_envelope},
};
use serde_json::json;

const PERMISSION_MESSAGE: &str = "macOS blocked Automation access to OmniFocus. Grant permission in System Settings > Privacy & Security > Automation.";
const NOT_RUNNING_MESSAGE: &str = "OmniFocus is not running. Please open OmniFocus and try again.";

/// The error a failed envelope carrying `message` turns into.
fn envelope_error(message: &str) -> OmniFocusError {
    unwrap_omnijs_envelope(json!({"ok": false, "error": message}))
        .expect_err("an envelope with ok: false is an error")
}

/// Asserts that `message`, thrown inside OmniFocus, reaches the caller as is.
fn assert_passes_through(message: &str) {
    let error = envelope_error(message);
    assert!(
        matches!(error, OmniFocusError::OmniFocus(_)),
        "expected an OmniFocus error, got {error:?}"
    );
    assert_eq!(error.to_string(), message);
}

// ------------------------------------------------ envelope: user text stays

#[test]
fn user_text_reading_not_permitted_is_not_a_permission_error() {
    assert_passes_through(r#"Invalid status: "Not permitted""#);
}

#[test]
fn user_text_naming_apple_events_is_not_a_permission_error() {
    assert_passes_through("Unknown tag: Apple Events");
}

#[test]
fn user_text_reading_not_authorized_is_not_a_permission_error() {
    assert_passes_through("Tag not found: Not authorized");
}

#[test]
fn user_text_saying_omnifocus_is_not_running_is_not_relabelled() {
    assert_passes_through("Project not found: OmniFocus application isn't running");
    assert_passes_through("Invalid name: omnifocus not running");
}

// ------------------------------------------------ envelope: no prefix allow-list

#[test]
fn parent_task_not_found_is_not_prefixed() {
    assert_passes_through("Parent task not found: x");
}

#[test]
fn notification_not_found_is_not_prefixed() {
    assert_passes_through("Notification not found: n1");
}

#[test]
fn any_other_script_error_is_not_prefixed() {
    assert_passes_through("Task is not completed: abc");
    assert_passes_through("Cannot move a task under itself.");
}

#[test]
fn allow_listed_not_found_messages_still_pass_through() {
    assert_passes_through("Task not found: abc");
    assert_passes_through("Project not found: Errands");
    assert_passes_through("Tag not found: home");
    assert_passes_through("Folder not found: Work");
}

#[test]
fn envelope_message_is_trimmed_only() {
    let error = envelope_error("  Task not found: abc \n");
    assert_eq!(error.to_string(), "Task not found: abc");
}

#[test]
fn envelope_without_a_message_is_a_generic_script_error() {
    let error = unwrap_omnijs_envelope(json!({"ok": false, "error": "   "}))
        .expect_err("ok: false is an error");
    assert_eq!(error.to_string(), "OmniFocus script error.");
}

// ------------------------------------------------ osascript stderr: classified

#[test]
fn stderr_refusing_apple_events_is_the_permission_message() {
    assert_eq!(
        friendly_jxa_error(
            "execution error: Error: Error: Not authorized to send Apple events to OmniFocus. (-1743)"
        ),
        PERMISSION_MESSAGE
    );
}

#[test]
fn stderr_with_only_the_permission_error_code_is_the_permission_message() {
    assert_eq!(
        friendly_jxa_error("execution error: Error (-1743)"),
        PERMISSION_MESSAGE
    );
}

#[test]
fn stderr_saying_omnifocus_is_not_running_is_the_not_running_message() {
    assert_eq!(
        friendly_jxa_error(
            "execution error: OmniFocus got an error: Application isn't running. (-600)"
        ),
        NOT_RUNNING_MESSAGE
    );
    assert_eq!(
        friendly_jxa_error("OmniFocus is not running"),
        NOT_RUNNING_MESSAGE
    );
}

#[test]
fn stderr_syntax_error_keeps_its_text() {
    assert_eq!(
        friendly_jxa_error("  syntax error: Unexpected token (-2700)\n"),
        "JXA script syntax error: syntax error: Unexpected token (-2700)"
    );
}

#[test]
fn unrecognised_stderr_is_trimmed_only() {
    assert_eq!(
        friendly_jxa_error("  something else went wrong \n"),
        "something else went wrong"
    );
}
