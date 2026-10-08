//! Folder status normalization.
//!
//! Folder statuses are OmniFocus enum values, not stable display strings.
//! The shared helper compares enum members directly and reports an unfamiliar
//! value as `unknown` rather than silently treating it as active.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    js_helpers::JS_FOLDER_STATUS,
    jxa::JxaRunner,
    tools::folders::{get_folder, update_folder},
};
use serde_json::{json, Value};

const FAKE_FOLDER_STATUS: &str = r#"function FakeStatus(name) { this.name = name; }
var Folder = { Status: {
  Active: new FakeStatus("Active"),
  Dropped: new FakeStatus("Dropped")
} };"#;

fn status_name(status_expression: &str) -> String {
    common::run_jsc(
        &format!("{FAKE_FOLDER_STATUS}\n{JS_FOLDER_STATUS}"),
        &format!("print(normalizeFolderStatus({{ status: {status_expression} }}));"),
    )
}

#[test]
fn active_folder_is_active() {
    assert_eq!(status_name("Folder.Status.Active"), "active");
}

#[test]
fn dropped_folder_is_dropped() {
    assert_eq!(status_name("Folder.Status.Dropped"), "dropped");
}

#[test]
fn unrecognised_folder_status_is_unknown() {
    assert_eq!(status_name(r#"new FakeStatus("Active")"#), "unknown");
    assert_eq!(status_name("null"), "unknown");
}

struct CapturingRunner {
    script: Arc<Mutex<String>>,
}

impl CapturingRunner {
    fn new() -> Self {
        Self {
            script: Arc::new(Mutex::new(String::new())),
        }
    }

    fn script(&self) -> String {
        self.script.lock().expect("script lock").clone()
    }
}

impl JxaRunner for CapturingRunner {
    fn run_omnijs<'a>(
        &'a self,
        script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        *self.script.lock().expect("script lock") = script.to_string();
        Box::pin(async move { Ok(json!({})) })
    }
}

fn assert_uses_shared_folder_status(tool: &str, script: &str) {
    assert_eq!(
        script.matches(JS_FOLDER_STATUS).count(),
        1,
        "{tool} must prepend JS_FOLDER_STATUS exactly once"
    );
    assert!(!script.contains("const normalizeStatus = ("));
    assert!(!script.contains("const normalizeFolderStatus = ("));
    common::assert_script_compiles(tool, script);
}

#[tokio::test]
async fn get_folder_reports_shared_folder_and_project_statuses() {
    let runner = CapturingRunner::new();
    get_folder(&runner, "folder-1")
        .await
        .expect("get_folder runs");
    let script = runner.script();
    assert_uses_shared_folder_status("get_folder", &script);
    assert!(script.contains("status: normalizeFolderStatus(folder)"));
    assert!(script.contains("status: normalizeProjectStatus(project)"));
    assert!(!script.contains("normalizeFolderStatus(project)"));
}

#[tokio::test]
async fn update_folder_reports_shared_folder_status() {
    let runner = CapturingRunner::new();
    update_folder(&runner, "folder-1", None, Some("active"))
        .await
        .expect("update_folder runs");
    let script = runner.script();
    assert_uses_shared_folder_status("update_folder", &script);
    assert!(script.contains("status: normalizeFolderStatus(folder)"));
}
