//! Which ends of the server are a clean shutdown (audit CR-043).
//!
//! The binary exits 0 when the client simply goes away — its input reaches
//! end of file, before or after the handshake — and logs to stderr and exits
//! non-zero on anything else. The decision used to be made by searching the
//! error's text for "connection closed" or "initialized request"; rmcp's
//! handshake violation renders as "expect initialized request, but
//! received: …", which matched, so a client that skipped `initialize` made the
//! server exit 0 without a word. It is now made on the error's variant.
//!
//! The end-to-end cases drive rmcp's real handshake over an in-memory stream
//! and classify whatever it returns, so they follow rmcp rather than a
//! hand-built error.

use std::{future::Future, pin::Pin, time::Duration};

use omnifocus_mcp::{
    jxa::JxaRunner,
    server::OmniFocusServer,
    shutdown::{classify_initialize_error, classify_service_end, Shutdown},
};
use rmcp::{
    service::{QuitReason, ServerInitializeError},
    ServiceExt,
};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(10);

/// Never reached: no test here calls a tool.
struct UnusedRunner;

impl JxaRunner for UnusedRunner {
    fn run_omnijs<'a>(
        &'a self,
        _script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        Box::pin(async move { panic!("no script runs in these tests") })
    }
}

async fn send<W: AsyncWriteExt + Unpin>(writer: &mut W, message: Value) {
    let mut line = serde_json::to_vec(&message).expect("message serializes");
    line.push(b'\n');
    writer.write_all(&line).await.expect("write to server");
}

/// A `JoinError` from a task that panicked with `message`.
async fn panicked_task(message: &'static str) -> tokio::task::JoinError {
    tokio::spawn(async move { panic!("{message}") })
        .await
        .expect_err("the task panics")
}

fn assert_failed(outcome: Shutdown, expected_text: &str) {
    match outcome {
        Shutdown::Failed(message) => assert!(
            message.contains(expected_text),
            "message should mention {expected_text:?}: {message}"
        ),
        Shutdown::Clean => panic!("expected a failure mentioning {expected_text:?}, got Clean"),
    }
}

// ---------------------------------------------------------------- handshake

#[test]
fn a_request_before_initialize_is_a_failure() {
    let outcome =
        classify_initialize_error(&ServerInitializeError::ExpectedInitializeRequest(None));
    assert_failed(outcome, "expect initialized request");
}

#[test]
fn a_request_instead_of_the_initialized_notification_is_a_failure() {
    let outcome = classify_initialize_error(
        &ServerInitializeError::ExpectedInitializedNotification(None),
    );
    assert_failed(outcome, "expect initialized notification");
}

#[test]
fn end_of_input_during_the_handshake_is_clean() {
    // rmcp names the message it was waiting for; the text plays no part.
    for waiting_for in ["initialized request", "initialize notification", "anything"] {
        let error = ServerInitializeError::ConnectionClosed(waiting_for.to_string());
        assert_eq!(classify_initialize_error(&error), Shutdown::Clean);
    }
}

#[test]
fn cancellation_during_the_handshake_is_clean() {
    assert_eq!(
        classify_initialize_error(&ServerInitializeError::Cancelled),
        Shutdown::Clean
    );
}

#[test]
fn a_failed_initialize_is_a_failure_whatever_its_text() {
    let error = ServerInitializeError::InitializeFailed(rmcp::ErrorData::internal_error(
        "connection closed by policy",
        None,
    ));
    assert_failed(
        classify_initialize_error(&error),
        "connection closed by policy",
    );
}

#[tokio::test]
async fn a_client_that_skips_initialize_makes_the_server_fail() {
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    let serving = tokio::spawn(async move {
        OmniFocusServer::new(UnusedRunner)
            .serve(tokio::io::split(server_io))
            .await
            .map(|_| ())
    });
    let (_client_read, mut client_write) = tokio::io::split(client_io);
    send(
        &mut client_write,
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}),
    )
    .await;
    let error = tokio::time::timeout(EXCHANGE_TIMEOUT, serving)
        .await
        .expect("the handshake ends in time")
        .expect("the server task does not panic")
        .expect_err("a request before initialize fails the handshake");
    assert_failed(
        classify_initialize_error(&error),
        "expect initialized request",
    );
}

#[tokio::test]
async fn a_client_that_disconnects_before_initialize_is_clean() {
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    drop(client_io);
    let error = tokio::time::timeout(
        EXCHANGE_TIMEOUT,
        OmniFocusServer::new(UnusedRunner).serve(tokio::io::split(server_io)),
    )
    .await
    .expect("the handshake ends in time")
    .map(|_| ())
    .expect_err("no handshake without a client");
    assert_eq!(classify_initialize_error(&error), Shutdown::Clean);
}

// ---------------------------------------------------------------- after the handshake

#[tokio::test]
async fn a_client_that_disconnects_after_initialize_is_clean() {
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    let serving = tokio::spawn(async move {
        let service = OmniFocusServer::new(UnusedRunner)
            .serve(tokio::io::split(server_io))
            .await
            .expect("the handshake completes");
        classify_service_end(service.waiting().await)
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
    tokio::time::timeout(EXCHANGE_TIMEOUT, lines.next_line())
        .await
        .expect("server answers in time")
        .expect("read from server")
        .expect("initialize response");
    send(
        &mut client_write,
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    )
    .await;
    drop(client_write);
    drop(lines);
    let outcome = tokio::time::timeout(EXCHANGE_TIMEOUT, serving)
        .await
        .expect("server stops once the client disconnects")
        .expect("server task does not panic");
    assert_eq!(outcome, Shutdown::Clean);
}

#[test]
fn closed_and_cancelled_service_ends_are_clean() {
    assert_eq!(
        classify_service_end(Ok(QuitReason::Closed)),
        Shutdown::Clean
    );
    assert_eq!(
        classify_service_end(Ok(QuitReason::Cancelled)),
        Shutdown::Clean
    );
}

#[tokio::test]
async fn a_panicked_service_task_is_a_failure_whatever_its_message() {
    let error = panicked_task("connection closed").await;
    assert_failed(classify_service_end(Err(error)), "panicked");
}

#[tokio::test]
async fn a_join_error_inside_the_service_is_a_failure() {
    let error = panicked_task("send task failed").await;
    assert_failed(
        classify_service_end(Ok(QuitReason::JoinError(error))),
        "panicked",
    );
}
