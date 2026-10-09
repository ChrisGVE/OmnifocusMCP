//! Drives the real `OmniFocusServer` over an in-memory MCP stream, for tests
//! that must see what a client sees: whether a call is rejected by the
//! protocol layer, fails as a tool, or reaches OmniFocus at all.
//!
//! Test files include it with `#[path = "support/mcp_session.rs"] mod
//! mcp_session;`. The runner records every script instead of contacting
//! OmniFocus, so a test can assert that a rejected call ran nothing.

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use omnifocus_mcp::{jxa::JxaRunner, server::OmniFocusServer};
use rmcp::ServiceExt;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Upper bound on any single exchange with the in-process server.
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(10);

/// Records each script it is asked to run and answers with an empty object.
#[derive(Clone, Default)]
struct RecordingRunner {
    scripts: Arc<Mutex<Vec<String>>>,
}

impl JxaRunner for RecordingRunner {
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

/// What one `tools/call` produced: the JSON-RPC response and every script
/// the server ran while handling it.
pub struct ToolCall {
    pub response: Value,
    pub scripts: Vec<String>,
}

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

/// Serves the real server, completes the MCP handshake, calls `tool` with
/// `arguments` and returns the response with the scripts it ran.
pub async fn call_tool(tool: &str, arguments: Value) -> ToolCall {
    let runner = RecordingRunner::default();
    let scripts = Arc::clone(&runner.scripts);
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
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
               "params": {"name": tool, "arguments": arguments}}),
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
    let scripts = scripts.lock().expect("script lock").clone();
    ToolCall { response, scripts }
}

/// Asserts a JSON-RPC error response (the call was rejected before the tool
/// ran) and returns its message.
pub fn protocol_error_message(response: &Value) -> String {
    assert!(
        response.get("result").is_none(),
        "expected a protocol error, got a result: {response}"
    );
    response["error"]["message"]
        .as_str()
        .unwrap_or_else(|| panic!("expected a JSON-RPC error message: {response}"))
        .to_string()
}

/// Asserts a tool execution error (`isError: true`) and returns its text.
pub fn tool_error_text(response: &Value) -> String {
    assert!(
        response.get("error").is_none(),
        "a tool failure must not be a protocol error: {response}"
    );
    assert_eq!(
        response["result"]["isError"], true,
        "result must set isError: {response}"
    );
    response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("tool error has a text item: {response}"))
        .to_string()
}

/// Asserts a successful tool result.
pub fn assert_tool_success(response: &Value) {
    assert!(
        response.get("error").is_none(),
        "expected success, got a protocol error: {response}"
    );
    assert_ne!(
        response["result"]["isError"], true,
        "expected success, got a tool error: {response}"
    );
}
