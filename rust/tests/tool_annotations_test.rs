//! MCP tool annotations advertised in `tools/list`.
//!
//! A tool without annotations falls back to the MCP defaults:
//! `readOnlyHint: false`, `destructiveHint: true`, `idempotentHint: false`,
//! `openWorldHint: true`. Under those defaults `list_tasks` looks exactly as
//! dangerous as `delete_project`, so a client that asks before destructive
//! calls has to ask about every call, and users approve the whole server at
//! once, deletes included.
//!
//! `EXPECTED` below is the specification: one row per tool, every hint
//! stated. The tests read the real `tools/list` response over an in-memory
//! MCP session, so they check what a client receives, not what a Rust value
//! holds. How each tool was classified, from what its OmniJS script does:
//!
//! - Read-only: the script only reads the database.
//! - Destructive: the tool can overwrite or remove existing content, as the
//!   spec reads `destructiveHint: false` as "performs only additive
//!   updates". That is every delete and `remove_notification`, plus the
//!   tools that replace user-entered values: `update_task`,
//!   `update_project`, `update_tag`, `update_folder`, and
//!   `set_task_repetition` (replaces or clears the rule). Creates, appends,
//!   moves, `complete_*`/`uncomplete_*` and `set_project_status` add
//!   content or change a reversible state, and lose nothing.
//! - Idempotent: repeating the identical call has no further effect. A
//!   tool is NOT idempotent when a repeat creates another object
//!   (`create_*`, `duplicate_task`, `add_notification`, `append_to_note`),
//!   when completing a repeating item advances it to its next occurrence
//!   (`complete_task`, `complete_project`), or when the target is looked up
//!   by id or name and the call can make a second, same-named object the
//!   next match (`update_project`, `update_tag`, `update_folder` renaming
//!   by name; the project, tag and folder deletes). A call that fails on
//!   repeat without changing anything (`uncomplete_*`, `delete_task` by id)
//!   is idempotent.
//! - Open world: never; every tool works on the local OmniFocus database.

use std::{collections::BTreeSet, future::Future, pin::Pin, time::Duration};

use omnifocus_mcp::{jxa::JxaRunner, server::OmniFocusServer};
use rmcp::ServiceExt;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Upper bound on any single exchange with the in-process server.
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(10);

/// Listing tools runs no script; any script run is a test failure.
struct NeverCalledRunner;

impl JxaRunner for NeverCalledRunner {
    fn run_omnijs<'a>(
        &'a self,
        _script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        Box::pin(async { panic!("tools/list must not run omnijs") })
    }
}

/// The hints one tool must advertise.
struct Hints {
    read_only: bool,
    destructive: bool,
    idempotent: bool,
    open_world: bool,
}

/// A read: changes nothing, so it is also non-destructive and idempotent.
const READ: Hints = Hints {
    read_only: true,
    destructive: false,
    idempotent: true,
    open_world: false,
};

/// Adds content or changes a reversible state; a repeat has a further
/// effect.
const CHANGE: Hints = Hints {
    read_only: false,
    destructive: false,
    idempotent: false,
    open_world: false,
};

/// Adds content or changes a reversible state; a repeat has no further
/// effect.
const CHANGE_IDEMPOTENT: Hints = Hints {
    read_only: false,
    destructive: false,
    idempotent: true,
    open_world: false,
};

/// Overwrites or removes existing content, addressed by id; a repeat
/// changes nothing more.
const DESTRUCTIVE_IDEMPOTENT: Hints = Hints {
    read_only: false,
    destructive: true,
    idempotent: true,
    open_world: false,
};

/// Overwrites or removes existing content, addressed by id or name; a
/// repeat by name can reach a second object of the same name.
const DESTRUCTIVE: Hints = Hints {
    read_only: false,
    destructive: true,
    idempotent: false,
    open_world: false,
};

/// Every registered tool and the hints it must advertise.
const EXPECTED: [(&str, Hints); 48] = [
    // tasks: reads
    ("get_inbox", READ),
    ("list_tasks", READ),
    ("get_task_counts", READ),
    ("get_task", READ),
    ("list_subtasks", READ),
    ("search_tasks", READ),
    // tasks: writes
    ("duplicate_task", CHANGE),
    ("create_task", CHANGE),
    ("create_tasks_batch", CHANGE),
    ("create_subtask", CHANGE),
    ("complete_task", CHANGE),
    ("uncomplete_task", CHANGE_IDEMPOTENT),
    ("set_task_repetition", DESTRUCTIVE_IDEMPOTENT),
    ("update_task", DESTRUCTIVE_IDEMPOTENT),
    ("delete_task", DESTRUCTIVE_IDEMPOTENT),
    ("delete_tasks_batch", DESTRUCTIVE_IDEMPOTENT),
    ("move_task", CHANGE_IDEMPOTENT),
    ("move_tasks_batch", CHANGE_IDEMPOTENT),
    ("append_to_note", CHANGE),
    // tasks: notifications
    ("list_notifications", READ),
    ("add_notification", CHANGE),
    ("remove_notification", DESTRUCTIVE_IDEMPOTENT),
    // projects
    ("list_projects", READ),
    ("get_project_counts", READ),
    ("search_projects", READ),
    ("get_project", READ),
    ("create_project", CHANGE),
    ("complete_project", CHANGE),
    ("uncomplete_project", CHANGE_IDEMPOTENT),
    ("delete_project", DESTRUCTIVE),
    ("delete_projects_batch", DESTRUCTIVE),
    ("move_project", CHANGE_IDEMPOTENT),
    ("update_project", DESTRUCTIVE),
    ("set_project_status", CHANGE_IDEMPOTENT),
    // tags
    ("search_tags", READ),
    ("list_tags", READ),
    ("create_tag", CHANGE),
    ("update_tag", DESTRUCTIVE),
    ("delete_tag", DESTRUCTIVE),
    ("delete_tags_batch", DESTRUCTIVE),
    // folders
    ("list_folders", READ),
    ("create_folder", CHANGE),
    ("get_folder", READ),
    ("update_folder", DESTRUCTIVE),
    ("delete_folder", DESTRUCTIVE),
    ("delete_folders_batch", DESTRUCTIVE),
    // views
    ("get_forecast", READ),
    ("list_perspectives", READ),
];

/// The four hint keys, as they appear on the wire.
const HINT_KEYS: [&str; 4] = [
    "readOnlyHint",
    "destructiveHint",
    "idempotentHint",
    "openWorldHint",
];

// ---------------------------------------------------------------- MCP round trip

/// Writes one JSON-RPC message as a line.
async fn send<W: AsyncWriteExt + Unpin>(writer: &mut W, message: Value) {
    let mut line = serde_json::to_vec(&message).expect("message serializes");
    line.push(b'\n');
    writer.write_all(&line).await.expect("write to server");
}

/// Reads the next JSON-RPC message line, failing after `EXCHANGE_TIMEOUT`.
async fn receive<R: AsyncBufReadExt + Unpin>(lines: &mut tokio::io::Lines<R>) -> Value {
    let line = tokio::time::timeout(EXCHANGE_TIMEOUT, lines.next_line())
        .await
        .expect("server answers in time")
        .expect("read from server")
        .expect("server keeps the stream open");
    serde_json::from_str(&line).expect("server sends JSON")
}

/// Serves the real `OmniFocusServer` over an in-memory stream, performs the
/// MCP handshake, and returns the `tools` array of the `tools/list` result.
async fn list_tools_over_mcp() -> Vec<Value> {
    let (client_io, server_io) = tokio::io::duplex(256 * 1024);
    let serving = tokio::spawn(async move {
        let service = OmniFocusServer::new(NeverCalledRunner)
            .serve(tokio::io::split(server_io))
            .await
            .expect("server completes the handshake");
        service.waiting().await.expect("server shuts down cleanly");
    });

    let (client_read, mut client_write) = tokio::io::split(client_io);
    let mut lines = BufReader::new(client_read).lines();
    send(
        &mut client_write,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "0"}}}),
    )
    .await;
    let initialized = receive(&mut lines).await;
    assert_eq!(initialized["id"], 1, "initialize response: {initialized}");
    send(
        &mut client_write,
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    )
    .await;
    send(
        &mut client_write,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
    )
    .await;
    let response = receive(&mut lines).await;

    drop(client_write);
    drop(lines);
    tokio::time::timeout(EXCHANGE_TIMEOUT, serving)
        .await
        .expect("server stops once the client disconnects")
        .expect("server task does not panic");

    assert_eq!(response["id"], 2, "tools/list response: {response}");
    assert!(
        response["result"]["nextCursor"].is_null(),
        "tools/list is expected to fit on one page: {response}"
    );
    response["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("tools/list returns a tools array: {response}"))
        .clone()
}

fn tool_name(tool: &Value) -> &str {
    tool["name"].as_str().expect("every tool has a name")
}

// ---------------------------------------------------------------- tests

#[tokio::test]
async fn every_tool_states_all_four_hints() {
    let tools = list_tools_over_mcp().await;
    let mut incomplete = Vec::new();
    for tool in &tools {
        let annotations = &tool["annotations"];
        let missing: Vec<&str> = HINT_KEYS
            .into_iter()
            .filter(|key| !annotations[*key].is_boolean())
            .collect();
        if !missing.is_empty() {
            incomplete.push(format!("{} lacks {missing:?}", tool_name(tool)));
        }
    }
    assert!(
        incomplete.is_empty(),
        "tools falling back to the MCP defaults (destructive, open world):\n{}",
        incomplete.join("\n")
    );
}

#[tokio::test]
async fn the_table_names_exactly_the_registered_tools() {
    let tools = list_tools_over_mcp().await;
    let registered: BTreeSet<&str> = tools.iter().map(tool_name).collect();
    let specified: BTreeSet<&str> = EXPECTED.iter().map(|(name, _)| *name).collect();
    assert_eq!(specified.len(), EXPECTED.len(), "EXPECTED repeats a tool");
    assert_eq!(
        registered, specified,
        "registered tools and EXPECTED differ; classify every new tool"
    );
}

#[tokio::test]
async fn every_tool_advertises_its_specified_hints() {
    let tools = list_tools_over_mcp().await;
    let mut mismatches = Vec::new();
    for (name, hints) in &EXPECTED {
        let Some(tool) = tools.iter().find(|tool| tool_name(tool) == *name) else {
            mismatches.push(format!("{name}: not registered"));
            continue;
        };
        let expected = json!({
            "readOnlyHint": hints.read_only,
            "destructiveHint": hints.destructive,
            "idempotentHint": hints.idempotent,
            "openWorldHint": hints.open_world,
        });
        let actual: Value = HINT_KEYS
            .into_iter()
            .map(|key| (key.to_string(), tool["annotations"][key].clone()))
            .collect::<serde_json::Map<_, _>>()
            .into();
        if actual != expected {
            mismatches.push(format!("{name}: expected {expected}, got {actual}"));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn the_table_follows_the_naming_rules() {
    for (name, hints) in &EXPECTED {
        let is_read = ["list_", "get_", "search_"]
            .iter()
            .any(|prefix| name.starts_with(prefix));
        // Removes or overwrites: the delete, remove and update tools, and
        // `set_task_repetition`, the one `set_` tool that replaces content.
        let removes_or_overwrites = ["delete_", "remove_", "update_"]
            .iter()
            .any(|prefix| name.starts_with(prefix))
            || *name == "set_task_repetition";
        assert_eq!(hints.read_only, is_read, "{name}: read-only by its name");
        assert_eq!(
            hints.destructive, removes_or_overwrites,
            "{name}: destructive by its name"
        );
        assert!(!hints.open_world, "{name}: OmniFocus is a closed world");
    }
}
