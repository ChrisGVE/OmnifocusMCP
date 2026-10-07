//! Wiring of folder and project parameters (upstream #11).
//!
//! Companion to `folder_project_resolution_test.rs`, which runs the shared
//! `JS_RESOLVERS` snippet and whole tool scripts in JavaScriptCore. This file
//! pins, on the script text each tool builds, that every tool taking a
//! `folder` or `project` parameter resolves it through those resolvers and
//! reads the documented `parentFolder`, and that the project_planning prompt
//! copes with the stricter task filter. The advertised schema of those
//! parameters is checked in `folder_project_schema_test.rs`.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    error::OmniFocusError,
    js_helpers::JS_RESOLVERS,
    jxa::JxaRunner,
    prompts::project_planning,
    tools::{
        folders::{create_folder, list_folders},
        projects::{
            create_project, get_project, list_projects, move_project, search_projects,
            update_project,
        },
        tasks::{
            create_task, create_tasks_batch, get_task_counts_with_added_changed,
            list_tasks_with_added_changed, move_task, move_tasks_batch,
            search_tasks_with_added_changed, CreateTaskInput,
        },
    },
};
use serde_json::{json, Value};

// ---------------------------------------------------------------- capture

/// Records every script it is asked to run and returns a fixed payload.
struct CapturingRunner {
    payload: Value,
    scripts: Arc<Mutex<Vec<String>>>,
}

impl CapturingRunner {
    fn new(payload: Value) -> Self {
        Self {
            payload,
            scripts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn scripts(&self) -> Vec<String> {
        self.scripts.lock().expect("script lock").clone()
    }

    fn last_script(&self) -> String {
        self.scripts().last().cloned().expect("a script was run")
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
        Box::pin(async move { Ok(self.payload.clone()) })
    }
}

/// True when `script` reads a `.folder` property (as opposed to
/// `.parentFolder` or `.folders`); OmniJS documents no such property on
/// projects.
fn reads_folder_property(script: &str) -> bool {
    script.match_indices(".folder").any(|(index, _)| {
        let next = script[index + ".folder".len()..].chars().next();
        !matches!(next, Some(c) if c.is_ascii_alphanumeric() || c == '_')
    })
}

/// A script that resolves folder/project values carries the resolvers once,
/// reads no undocumented `.folder` property, and compiles.
fn assert_uses_resolvers(tool: &str, script: &str) {
    assert_eq!(
        script.matches(JS_RESOLVERS).count(),
        1,
        "{tool} must prepend JS_RESOLVERS exactly once"
    );
    assert!(!reads_folder_property(script), "{tool} reads `.folder`");
    common::assert_script_compiles(tool, script);
}

async fn get_task_counts_script(project: Option<&str>) -> String {
    let runner = CapturingRunner::new(json!({
        "total": 0, "available": 0, "completed": 0, "overdue": 0, "dueSoon": 0,
        "flagged": 0, "deferred": 0
    }));
    get_task_counts_with_added_changed(
        &runner, project, None, None, "any", None, None, None, None, None, None, None, None, None,
        None, None, None,
    )
    .await
    .expect("get_task_counts runs");
    runner.last_script()
}

async fn move_project_script(folder: Option<&str>) -> String {
    let runner = CapturingRunner::new(json!({"id": "p3"}));
    move_project(&runner, "p3", folder)
        .await
        .expect("move_project runs");
    runner.last_script()
}

// ---------------------------------------------------------------- wiring

#[tokio::test]
async fn project_reading_tools_report_parent_folder() {
    let runner = CapturingRunner::new(json!([]));
    list_projects(
        &runner,
        Some("Work"),
        "active",
        None,
        None,
        false,
        None,
        "asc",
        5,
    )
    .await
    .expect("list_projects runs");
    let list = runner.last_script();
    assert_uses_resolvers("list_projects", &list);
    assert!(list.contains(
        "const filterFolder = folderFilter === null ? null : resolveFolder(folderFilter);"
    ));
    assert!(!list.contains("if (folderName !== folderFilter)"));

    let runner = CapturingRunner::new(json!([]));
    search_projects(&runner, "a", 5)
        .await
        .expect("search_projects runs");
    let runner_get = CapturingRunner::new(json!({"id": "p1"}));
    get_project(&runner_get, "p1")
        .await
        .expect("get_project runs");
    let runner_update = CapturingRunner::new(json!({"id": "p1"}));
    update_project(
        &runner_update,
        "p1",
        Some("x"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .expect("update_project runs");
    for (tool, script) in [
        ("list_projects", list),
        ("search_projects", runner.last_script()),
        ("get_project", runner_get.last_script()),
        ("update_project", runner_update.last_script()),
    ] {
        assert!(!reads_folder_property(&script), "{tool} reads `.folder`");
        assert!(
            script.contains("folderName: project.parentFolder ? project.parentFolder.name : null"),
            "{tool}"
        );
    }
}

#[tokio::test]
async fn list_folders_counts_projects_by_parent_folder() {
    let runner = CapturingRunner::new(json!([]));
    list_folders(&runner, 5).await.expect("list_folders runs");
    let script = runner.last_script();
    assert!(!reads_folder_property(&script));
    assert!(script.contains("const folder = project.parentFolder;"));
}

#[tokio::test]
async fn task_filters_resolve_the_project() {
    let counts = get_task_counts_script(Some("Errands")).await;
    let runner = CapturingRunner::new(json!([]));
    list_tasks_with_added_changed(
        &runner,
        Some("Errands"),
        None,
        None,
        "any",
        None,
        "all",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        "asc",
        5,
    )
    .await
    .expect("list_tasks runs");
    let list = runner.last_script();
    let runner = CapturingRunner::new(json!([]));
    search_tasks_with_added_changed(
        &runner,
        "x",
        Some("Errands"),
        None,
        None,
        "any",
        None,
        "all",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        "asc",
        5,
    )
    .await
    .expect("search_tasks runs");
    for (tool, script) in [
        ("get_task_counts", counts),
        ("list_tasks", list),
        ("search_tasks", runner.last_script()),
    ] {
        assert_uses_resolvers(tool, &script);
        assert!(
            script.contains(
                "const filterProject = projectFilter === null ? null : resolveProject(projectFilter);"
            ),
            "{tool}"
        );
        assert!(
            !script.contains("if (projectName !== projectFilter)"),
            "{tool}"
        );
    }
}

#[tokio::test]
async fn folder_destinations_resolve_the_folder() {
    let runner = CapturingRunner::new(json!({"id": "p1"}));
    create_project(&runner, "New", Some("Work"), None, None, None, None)
        .await
        .expect("create_project runs");
    let created = runner.last_script();
    let runner = CapturingRunner::new(json!({"id": "f1"}));
    create_folder(&runner, "Sub", Some("Work"))
        .await
        .expect("create_folder runs");
    let sub_folder = runner.last_script();
    let moved = move_project_script(Some("Work")).await;
    for (tool, script, call) in [
        (
            "create_project",
            created,
            "const targetFolder = resolveFolder(folderName);",
        ),
        (
            "create_folder",
            sub_folder,
            "const parentFolder = resolveFolder(parentName);",
        ),
        (
            "move_project",
            moved,
            "const targetFolder = resolveFolder(folderName);",
        ),
    ] {
        assert_uses_resolvers(tool, &script);
        assert!(script.contains(call), "{tool}");
        // The old name-only lookups; the resolver's own byName is the only one left.
        assert!(
            !script.contains("document.flattenedFolders.byName(folderName)"),
            "{tool}"
        );
        assert!(
            !script.contains("document.flattenedFolders.byName(parentName)"),
            "{tool}"
        );
    }
}

#[tokio::test]
async fn project_destinations_resolve_the_project() {
    let created = {
        let runner = CapturingRunner::new(json!({"id": "t1"}));
        create_task(
            &runner,
            "Do",
            Some("Errands"),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("create_task runs");
        runner.last_script()
    };
    let batch = {
        let runner = CapturingRunner::new(json!([]));
        let input = CreateTaskInput {
            name: "Do".to_string(),
            project: Some("Errands".to_string()),
            note: None,
            due_date: None,
            defer_date: None,
            flagged: None,
            tags: None,
            estimated_minutes: None,
        };
        create_tasks_batch(&runner, vec![input])
            .await
            .expect("create_tasks_batch runs");
        runner.last_script()
    };
    let moved = {
        let runner = CapturingRunner::new(json!({"id": "t1"}));
        move_task(&runner, "t1", Some("Errands"), None)
            .await
            .expect("move_task runs");
        runner.last_script()
    };
    let moved_batch = {
        let runner = CapturingRunner::new(json!({}));
        move_tasks_batch(&runner, vec!["t1".to_string()], Some("Errands"), None)
            .await
            .expect("move_tasks_batch runs");
        runner.last_script()
    };
    for (tool, script) in [
        ("create_task", created),
        ("create_tasks_batch", batch.clone()),
        ("move_task", moved),
        ("move_tasks_batch", moved_batch),
    ] {
        assert_uses_resolvers(tool, &script);
        assert!(
            script.contains("const targetProject = resolveProject(projectName);"),
            "{tool}"
        );
        // The old name-only lookup; the resolver's own byName is the only one left.
        assert!(
            !script.contains("document.flattenedProjects.byName(projectName)"),
            "{tool}"
        );
    }
    // Every parent is resolved before the first task is created.
    let resolved_at = batch
        .find("const parents = taskInputs.map(input => resolveParent(input.project));")
        .expect("batch resolves all parents up front");
    let created_at = batch.find("new Task(").expect("batch creates tasks");
    assert!(resolved_at < created_at);
}

// ---------------------------------------------------------------- prompt

/// Answers like OmniFocus for a project that does not exist: both the
/// project lookup and a task filter on that project fail.
struct MissingProjectRunner {
    scripts: Arc<Mutex<Vec<String>>>,
}

impl JxaRunner for MissingProjectRunner {
    fn run_omnijs<'a>(
        &'a self,
        script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        self.scripts
            .lock()
            .expect("script lock")
            .push(script.to_string());
        Box::pin(async {
            Err(OmniFocusError::OmniFocus(
                "Project not found: new-idea".to_string(),
            ))
        })
    }
}

#[tokio::test]
async fn project_planning_for_missing_project_skips_the_task_lookup() {
    let runner = MissingProjectRunner {
        scripts: Arc::new(Mutex::new(Vec::new())),
    };
    let planning = project_planning(&runner, "new-idea")
        .await
        .expect("planning still renders for a missing project");
    assert!(planning.contains("\"status\":\"not_found\""));
    assert!(planning.contains("project_available_tasks_json:\n[]"));
    assert_eq!(runner.scripts.lock().expect("script lock").len(), 1);
}

/// Answers the project lookup with a found project and the task list with an
/// empty list, recording both scripts.
struct FoundProjectRunner {
    scripts: Arc<Mutex<Vec<String>>>,
}

impl JxaRunner for FoundProjectRunner {
    fn run_omnijs<'a>(
        &'a self,
        script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        let mut scripts = self.scripts.lock().expect("script lock");
        scripts.push(script.to_string());
        let payload = if scripts.len() == 1 {
            json!({"id": "project-1", "name": "alpha"})
        } else {
            json!([])
        };
        Box::pin(async move { Ok(payload) })
    }
}

#[tokio::test]
async fn project_planning_lists_tasks_of_the_resolved_project_id() {
    let runner = FoundProjectRunner {
        scripts: Arc::new(Mutex::new(Vec::new())),
    };
    project_planning(&runner, "alpha")
        .await
        .expect("planning renders");
    let scripts = runner.scripts.lock().expect("script lock").clone();
    assert_eq!(scripts.len(), 2, "project lookup, then task list");
    assert!(scripts[1].contains(r#"const projectFilter = "project-1";"#));
}
