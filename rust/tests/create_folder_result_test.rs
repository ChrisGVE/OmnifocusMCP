//! `create_folder` returns the parent it promises (audit CR-032).
//!
//! The tool description promised "id/name/parent", but the script returned
//! only `{id, name}`, so a client could not confirm where the folder landed.
//! It now reports the parent the way `get_folder` and `list_folders` do: as
//! `parentName`, the parent folder's name, or `null` at the top level.
//!
//! The script runs in JavaScriptCore against fake folders, so the assertions
//! are on what it returns, not on its text.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{jxa::JxaRunner, server::OmniFocusServer, tools::folders::create_folder};
use rmcp::ServerHandler;
use serde_json::{json, Value};

/// One existing folder, "Work" (id `f-work`). `new Folder(name, position)`
/// sets `parent` the way OmniJS does: the folder whose `ending` was passed,
/// or `null` for a top-level folder.
const FAKE_FOLDERS: &str = r#"var FOLDERS = [];
function Folder(name, position) {
  this.id = { primaryKey: "new-" + name };
  this.name = name;
  this.parent = position ? position.folder : null;
  this.ending = { kind: "folder", folder: this };
}
Folder.byIdentifier = function (id) {
  return FOLDERS.find(function (f) { return f.id.primaryKey === id; }) || null;
};
var work = new Folder("Work", null);
work.id = { primaryKey: "f-work" };
FOLDERS.push(work);
var document = { flattenedFolders: FOLDERS };
document.flattenedFolders.byName = function (name) {
  return FOLDERS.find(function (f) { return f.name === name; }) || null;
};"#;

/// Records the script it is asked to run and answers with an empty object.
#[derive(Default)]
struct CapturingRunner {
    scripts: Arc<Mutex<Vec<String>>>,
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

/// Builds the real `create_folder` script and returns what it returns
/// against the fake folders.
async fn create_folder_result(name: &str, parent: Option<&str>) -> Value {
    let runner = CapturingRunner::default();
    create_folder(&runner, name, parent)
        .await
        .expect("create_folder builds its script");
    let script = runner
        .scripts
        .lock()
        .expect("script lock")
        .last()
        .cloned()
        .expect("a script was run");
    common::assert_script_compiles("create_folder", &script);
    let output = common::run_jsc(
        FAKE_FOLDERS,
        &format!("print(JSON.stringify((function () {{\n{script}\n}})()));"),
    );
    serde_json::from_str(&output).unwrap_or_else(|_| panic!("create_folder printed {output}"))
}

#[tokio::test]
async fn a_nested_folder_reports_its_parent_name() {
    let result = create_folder_result("Areas", Some("Work")).await;
    assert_eq!(
        result,
        json!({"id": "new-Areas", "name": "Areas", "parentName": "Work"})
    );
}

#[tokio::test]
async fn a_parent_given_by_id_is_reported_by_name_like_get_folder() {
    let result = create_folder_result("Areas", Some("f-work")).await;
    assert_eq!(result["parentName"], "Work", "{result}");
}

#[tokio::test]
async fn a_top_level_folder_reports_a_null_parent_name() {
    let result = create_folder_result("Areas", None).await;
    assert_eq!(
        result,
        json!({"id": "new-Areas", "name": "Areas", "parentName": null})
    );
}

/// Runner for reading the advertised tool list only.
struct NeverCalledRunner;

impl JxaRunner for NeverCalledRunner {
    fn run_omnijs<'a>(
        &'a self,
        _script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        Box::pin(async { panic!("description tests must not run omnijs") })
    }
}

#[test]
fn the_description_names_the_returned_parent_field() {
    let tool = OmniFocusServer::new(NeverCalledRunner)
        .get_tool("create_folder")
        .expect("create_folder is registered");
    let description = tool.description.as_deref().unwrap_or_default();
    assert!(
        description.contains("id, name, and parentName"),
        "{description}"
    );
}
