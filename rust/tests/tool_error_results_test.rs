//! How a failing tool call reaches the MCP client (audit CR-010).
//!
//! The MCP specification (2025-11-25, Tools / Error Handling) separates two
//! kinds of failure:
//!
//! - **protocol errors** — a JSON-RPC `error` response — for requests that
//!   are malformed at the protocol level: an unknown tool, arguments that do
//!   not match the tool's schema. rmcp produces these itself before our
//!   handler runs, and they stay as they are.
//! - **tool execution errors** — a normal `result` with `isError: true` — for
//!   everything that goes wrong while the tool runs: an invalid argument
//!   value, an object not found, OmniFocus not running, Automation refused, a
//!   timeout, an error thrown by the script. Clients pass these on to the
//!   model, so it can see what went wrong and correct itself.
//!
//! The server used to turn every tool failure into a protocol error. These
//! tests drive the real `OmniFocusServer` over an in-memory MCP stream and pin
//! which failures surface which way. Prompts and resources have no `isError`
//! result in the specification, so their failures stay protocol errors.

use std::{future::Future, pin::Pin, time::Duration};

use omnifocus_mcp::{
    error::OmniFocusError,
    jxa::{friendly_jxa_error, unwrap_omnijs_envelope, JxaRunner},
    server::OmniFocusServer,
};
use rmcp::ServiceExt;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Upper bound on any single exchange with the in-process server.
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(10);

/// JSON-RPC's "invalid params" code, which rmcp uses for an unknown tool and
/// for arguments that do not deserialize.
const INVALID_PARAMS: i64 = -32602;

/// Answers every script with what `reply` returns, as OmniFocus would.
#[derive(Clone)]
struct ScriptedRunner {
    reply: fn() -> omnifocus_mcp::error::Result<Value>,
}

impl JxaRunner for ScriptedRunner {
    fn run_omnijs<'a>(
        &'a self,
        _script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        Box::pin(async move { (self.reply)() })
    }
}

fn runner(reply: fn() -> omnifocus_mcp::error::Result<Value>) -> ScriptedRunner {
    ScriptedRunner { reply }
}

/// A runner for calls that must fail before any script runs.
fn unreachable_runner() -> ScriptedRunner {
    runner(|| panic!("this call must fail before OmniFocus is contacted"))
}

// ---------------------------------------------------------------- MCP round trip

async fn send<W: AsyncWriteExt + Unpin>(writer: &mut W, message: Value) {
    let mut line = serde_json::to_vec(&message).expect("message serializes");
    line.push(b'\n');
    writer.write_all(&line).await.expect("write to server");
}

async fn receive<R: AsyncBufReadExt + Unpin>(lines: &mut tokio::io::Lines<R>) -> Value {
    let line = tokio::time::timeout(EXCHANGE_TIMEOUT, lines.next_line())
        .await
        .expect("server answers in time")
        .expect("read from server")
        .expect("server keeps the stream open");
    serde_json::from_str(&line).expect("server sends JSON")
}

/// Serves the real server over an in-memory stream, completes the MCP
/// handshake, sends one request (`method` with `params`, id 2) and returns
/// the response.
async fn request(runner: ScriptedRunner, method: &str, params: Value) -> Value {
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    let serving = tokio::spawn(async move {
        let service = OmniFocusServer::new(runner)
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
        json!({"jsonrpc": "2.0", "id": 2, "method": method, "params": params}),
    )
    .await;
    let response = receive(&mut lines).await;
    assert_eq!(response["id"], 2, "response: {response}");

    drop(client_write);
    drop(lines);
    tokio::time::timeout(EXCHANGE_TIMEOUT, serving)
        .await
        .expect("server stops once the client disconnects")
        .expect("server task does not panic");
    response
}

async fn call_tool(runner: ScriptedRunner, tool: &str, arguments: Value) -> Value {
    request(
        runner,
        "tools/call",
        json!({"name": tool, "arguments": arguments}),
    )
    .await
}

/// Asserts a tool execution error and returns its text.
fn tool_error_text(response: &Value) -> String {
    assert!(
        response.get("error").is_none(),
        "a tool failure must not be a protocol error: {response}"
    );
    let result = &response["result"];
    assert_eq!(
        result["isError"], true,
        "result must set isError: {response}"
    );
    let content = result["content"].as_array().expect("result has content");
    assert_eq!(content.len(), 1, "one text item: {response}");
    assert_eq!(content[0]["type"], "text", "text content: {response}");
    content[0]["text"]
        .as_str()
        .expect("text is a string")
        .to_string()
}

/// Asserts a JSON-RPC error response and returns it.
fn protocol_error(response: &Value) -> &Value {
    assert!(
        response.get("result").is_none(),
        "expected a protocol error, got a result: {response}"
    );
    let error = &response["error"];
    assert!(error.is_object(), "expected a JSON-RPC error: {response}");
    error
}

// ---------------------------------------------------------------- tool execution errors

#[tokio::test]
async fn object_not_found_is_a_tool_error_with_the_message() {
    let response = call_tool(
        runner(|| Err(OmniFocusError::OmniFocus("Task not found: abc".to_string()))),
        "get_task",
        json!({"task_id": "abc"}),
    )
    .await;
    assert_eq!(tool_error_text(&response), "Task not found: abc");
}

#[tokio::test]
async fn invalid_argument_value_is_a_tool_error_with_the_message() {
    let response = call_tool(unreachable_runner(), "list_tasks", json!({"limit": 0})).await;
    assert_eq!(tool_error_text(&response), "limit must be greater than 0.");
}

#[tokio::test]
async fn script_error_from_the_envelope_is_a_tool_error_with_the_message() {
    let response = call_tool(
        runner(|| {
            unwrap_omnijs_envelope(
                json!({"ok": false, "error": "Cannot move a task under itself."}),
            )
        }),
        "move_task",
        json!({"task_id": "a", "parent_task_id": "a"}),
    )
    .await;
    assert_eq!(
        tool_error_text(&response),
        "Cannot move a task under itself."
    );
}

#[tokio::test]
async fn self_move_in_a_batch_surfaces_like_a_single_self_move() {
    // move_tasks_batch rejects the self-move in Rust, move_task in the script;
    // both must reach the client the same way.
    let response = call_tool(
        unreachable_runner(),
        "move_tasks_batch",
        json!({"task_ids": ["a"], "parent_task_id": "a"}),
    )
    .await;
    assert_eq!(
        tool_error_text(&response),
        "parent_task_id must not be included in task_ids (cannot move a task under itself)."
    );
}

#[tokio::test]
async fn automation_refused_is_a_tool_error_with_the_message() {
    let response = call_tool(
        runner(|| {
            Err(OmniFocusError::JxaExecution(friendly_jxa_error(
                "execution error: Not authorized to send Apple events to OmniFocus. (-1743)",
            )))
        }),
        "get_inbox",
        json!({}),
    )
    .await;
    assert_eq!(
        tool_error_text(&response),
        "JXA execution failed: macOS blocked Automation access to OmniFocus. Grant permission in System Settings > Privacy & Security > Automation."
    );
}

#[tokio::test]
async fn omnifocus_not_running_is_a_tool_error_with_the_message() {
    let response = call_tool(
        runner(|| {
            Err(OmniFocusError::JxaExecution(friendly_jxa_error(
                "execution error: OmniFocus got an error: Application isn't running. (-600)",
            )))
        }),
        "get_inbox",
        json!({}),
    )
    .await;
    assert_eq!(
        tool_error_text(&response),
        "JXA execution failed: OmniFocus is not running. Please open OmniFocus and try again."
    );
}

#[tokio::test]
async fn timeout_is_a_tool_error_with_the_message() {
    let response = call_tool(
        runner(|| {
            Err(OmniFocusError::Timeout {
                after: Duration::from_secs(30),
            })
        }),
        "list_tags",
        json!({}),
    )
    .await;
    assert_eq!(
        tool_error_text(&response),
        "JXA command timed out after 30s."
    );
}

#[tokio::test]
async fn a_successful_call_is_not_marked_as_an_error() {
    let response = call_tool(runner(|| Ok(json!([]))), "get_inbox", json!({})).await;
    assert!(response.get("error").is_none(), "{response}");
    assert_eq!(response["result"]["isError"], false, "{response}");
}

// ---------------------------------------------------------------- protocol errors kept

#[tokio::test]
async fn unknown_tool_stays_a_protocol_error() {
    let response = call_tool(unreachable_runner(), "no_such_tool", json!({})).await;
    let error = protocol_error(&response);
    assert_eq!(error["code"], INVALID_PARAMS);
    assert_eq!(error["message"], "tool not found");
}

#[tokio::test]
async fn undeclared_argument_key_stays_a_protocol_error() {
    let response = call_tool(unreachable_runner(), "get_task", json!({"taskId": "abc"})).await;
    let error = protocol_error(&response);
    assert_eq!(error["code"], INVALID_PARAMS);
    let message = error["message"].as_str().expect("error has a message");
    assert!(message.contains("unknown field `taskId`"), "{response}");
}

#[tokio::test]
async fn missing_required_argument_stays_a_protocol_error() {
    let response = call_tool(unreachable_runner(), "get_task", json!({})).await;
    assert_eq!(protocol_error(&response)["code"], INVALID_PARAMS);
}

#[tokio::test]
async fn failing_prompt_stays_a_protocol_error() {
    let response = request(
        runner(|| {
            Err(OmniFocusError::Timeout {
                after: Duration::from_secs(30),
            })
        }),
        "prompts/get",
        json!({"name": "daily_review"}),
    )
    .await;
    let error = protocol_error(&response);
    assert_eq!(error["message"], "JXA command timed out after 30s.");
}

#[tokio::test]
async fn failing_resource_stays_a_protocol_error() {
    let response = request(
        runner(|| {
            Err(OmniFocusError::Timeout {
                after: Duration::from_secs(30),
            })
        }),
        "resources/read",
        json!({"uri": "omnifocus://inbox"}),
    )
    .await;
    let error = protocol_error(&response);
    assert_eq!(error["message"], "JXA command timed out after 30s.");
}
