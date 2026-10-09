//! The tag and folder batch deletes (audit CR-046).
//!
//! Both scripts defined a local `const resolveTag` / `const resolveFolder`
//! with the same names as the shared `JS_RESOLVERS` functions, so a script
//! holding both would not even compile. They now share one builder that
//! looks entries up through `matchTags` / `matchFolders`, while keeping their
//! per-entry behaviour: an entry that matches nothing is reported, not
//! thrown, and the others are still deleted, deepest first, so a child is
//! never taken down with its parent before its own turn.
//!
//! These tests run the whole scripts in JavaScriptCore against a fake tree.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    js_helpers::JS_RESOLVERS,
    jxa::JxaRunner,
    tools::{folders::delete_folders_batch, tags::delete_tags_batch},
};
use serde_json::{json, Value};

/// Tags Home (tag-h) > Errands (tag-e) > Store (tag-s), and Work (tag-w);
/// folders Areas (f-a) > Home (f-h), and Archive (f-x). `deleteObject`
/// records the order of deletions and, as OmniFocus does, removes the
/// object's descendants with it.
const FAKE_TREE: &str = r#"function node(id, name, parent) {
  return { id: { primaryKey: id }, name: name, parent: parent };
}
var home = node("tag-h", "Home", null);
var errands = node("tag-e", "Errands", home);
var store = node("tag-s", "Store", errands);
var work = node("tag-w", "Work", null);
var areas = node("f-a", "Areas", null);
var homeFolder = node("f-h", "Home", areas);
var archive = node("f-x", "Archive", null);
var document = { flattenedTags: [home, errands, store, work],
  flattenedFolders: [areas, homeFolder, archive], flattenedProjects: [] };
function byIdIn(collection) {
  return function (id) {
    return document[collection].find(item => item.id.primaryKey === id) || null;
  };
}
var Tag = { byIdentifier: byIdIn("flattenedTags") };
var Folder = { byIdentifier: byIdIn("flattenedFolders") };
var Project = { byIdentifier: function () { return null; } };
var DELETED = [];
function isWithin(item, ancestor) {
  for (let current = item; current; current = current.parent) {
    if (current === ancestor) return true;
  }
  return false;
}
function deleteObject(object) {
  DELETED.push(object.id.primaryKey);
  for (const collection of ["flattenedTags", "flattenedFolders"]) {
    document[collection] = document[collection].filter(item => !isWithin(item, object));
  }
}"#;

/// Records the script it is asked to run and returns an empty object.
#[derive(Clone)]
struct CapturingRunner {
    scripts: Arc<Mutex<Vec<String>>>,
}

impl CapturingRunner {
    fn new() -> Self {
        Self {
            scripts: Arc::new(Mutex::new(Vec::new())),
        }
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

async fn tags_script(entries: &[&str]) -> String {
    let runner = CapturingRunner::new();
    delete_tags_batch(&runner, entries.iter().map(|e| e.to_string()).collect())
        .await
        .expect("delete_tags_batch builds a script");
    runner.last_script()
}

async fn folders_script(entries: &[&str]) -> String {
    let runner = CapturingRunner::new();
    delete_folders_batch(&runner, entries.iter().map(|e| e.to_string()).collect())
        .await
        .expect("delete_folders_batch builds a script");
    runner.last_script()
}

/// Runs a batch script against the fake tree and returns its result with
/// the deletion order added as `deletedOrder`.
fn run_batch(script: &str) -> Value {
    common::assert_script_compiles("batch delete", script);
    let output = common::run_jsc(
        FAKE_TREE,
        &format!(
            "try {{ const result = (function () {{\n{script}\n}})(); \
             result.deletedOrder = DELETED; print(JSON.stringify(result)); }}\n\
             catch (error) {{ print(\"ERROR: \" + error.message); }}"
        ),
    );
    serde_json::from_str(&output)
        .unwrap_or_else(|_| panic!("expected a batch result, got {output}"))
}

// ---------------------------------------------------------------- shadowing

#[tokio::test]
async fn batch_delete_scripts_compile_alongside_the_shared_resolvers() {
    for (tool, script) in [
        ("delete_tags_batch", tags_script(&["Home"]).await),
        ("delete_folders_batch", folders_script(&["Home"]).await),
    ] {
        common::assert_script_compiles(tool, &format!("{JS_RESOLVERS}\n{script}"));
        assert!(
            !script.contains("const resolveTag") && !script.contains("const resolveFolder"),
            "{tool} must not redefine a shared resolver"
        );
    }
}

// ---------------------------------------------------------------- tags

#[tokio::test]
async fn tag_batch_deletes_deepest_first_and_reports_each_entry_in_order() {
    let result = run_batch(&tags_script(&["Home", "tag-s", "Missing", "Errands"]).await);
    assert_eq!(result["deletedOrder"], json!(["tag-s", "tag-e", "tag-h"]));
    let results = &result["results"];
    assert_eq!(results[0]["id"], "tag-h");
    assert_eq!(results[1]["id"], "tag-s");
    assert_eq!(
        results[2],
        json!({"id_or_name": "Missing", "id": null, "name": null, "deleted": false, "error": "not found"})
    );
    assert_eq!(results[3]["name"], "Errands");
    assert_eq!(
        result["summary"],
        json!({"requested": 4, "deleted": 3, "failed": 1})
    );
    assert_eq!(result["partial_success"], true);
}

#[tokio::test]
async fn tag_batch_counts_an_entry_named_twice_as_deleted_once() {
    // "Work" and "tag-w" name the same tag: the second finds it gone.
    let result = run_batch(&tags_script(&["Work", "tag-w"]).await);
    assert_eq!(result["deletedOrder"], json!(["tag-w"]));
    assert_eq!(
        result["summary"],
        json!({"requested": 2, "deleted": 2, "failed": 0})
    );
}

// ---------------------------------------------------------------- folders

#[tokio::test]
async fn folder_batch_deletes_deepest_first_and_reports_a_missing_entry() {
    let result = run_batch(&folders_script(&["Areas", "f-h", "Nope"]).await);
    assert_eq!(result["deletedOrder"], json!(["f-h", "f-a"]));
    let results = &result["results"];
    assert_eq!(results[0]["id"], "f-a");
    assert_eq!(results[1]["name"], "Home");
    assert_eq!(results[2]["error"], "not found");
    assert_eq!(
        result["summary"],
        json!({"requested": 3, "deleted": 2, "failed": 1})
    );
}

#[tokio::test]
async fn folder_batch_reports_a_delete_that_omnifocus_refused() {
    let script = folders_script(&["Archive"]).await;
    let output = common::run_jsc(
        &format!("{FAKE_TREE}\ndeleteObject = function () {{ throw new Error(\"locked\"); }};"),
        &format!("print(JSON.stringify((function () {{\n{script}\n}})()));"),
    );
    let result: Value = serde_json::from_str(&output).expect("batch result");
    assert_eq!(
        result["results"][0],
        json!({"id_or_name": "Archive", "id": "f-x", "name": "Archive", "deleted": false, "error": "locked"})
    );
}
