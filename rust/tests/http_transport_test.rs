//! The streamable-HTTP transport and the bearer token that guards it.
//!
//! Over HTTP the server is reachable by anything that can open the port, and
//! its tools can delete OmniFocus data. These tests pin the guard: a request
//! without the exact token is refused with 401 before it reaches the MCP
//! service, so no script runs, and a request with it gets a normal MCP
//! answer. The token itself must be long enough to be a secret.

use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use axum::{
    body::Body,
    http::{header, HeaderValue, Request, StatusCode},
};
use http_body_util::BodyExt;
use omnifocus_mcp::{
    http::{router, BearerToken, TokenError, MCP_PATH, MIN_TOKEN_LEN},
    jxa::JxaRunner,
    server::OmniFocusServer,
};
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

const TOKEN: &str = "k3Jq9vXr2LmT8sWz4NbY6cHd1FgA5ePu0QiOoRtVwXy";

/// Counts the scripts it is asked to run; `initialize` must run none.
#[derive(Clone, Default)]
struct CountingRunner {
    calls: Arc<AtomicUsize>,
}

impl JxaRunner for CountingRunner {
    fn run_omnijs<'a>(
        &'a self,
        _script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(json!({})) })
    }
}

fn token() -> BearerToken {
    BearerToken::parse(TOKEN).expect("test token is valid")
}

fn initialize_request(authorization: Option<&str>) -> Request<Body> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {"name": "http-test", "version": "0"}
        }
    });
    let mut builder = Request::post(MCP_PATH)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json, text/event-stream");
    if let Some(value) = authorization {
        builder = builder.header(header::AUTHORIZATION, value);
    }
    builder
        .body(Body::from(body.to_string()))
        .expect("request builds")
}

/// Reads the response body until the initialize result has arrived. A
/// stateful SSE stream may stay open after it, so this does not wait for EOF.
async fn read_until_result(body: Body) -> String {
    let mut body = body;
    let mut text = String::new();
    let read = async {
        while let Some(frame) = body.frame().await {
            let frame = frame.expect("body frame");
            if let Some(chunk) = frame.data_ref() {
                text.push_str(&String::from_utf8_lossy(chunk));
                if text.contains("\"result\"") {
                    break;
                }
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(10), read)
        .await
        .expect("initialize result within 10 s");
    text
}

// --- the token -----------------------------------------------------------

#[test]
fn a_token_of_the_minimum_length_is_accepted() {
    let raw = "a".repeat(MIN_TOKEN_LEN);
    assert!(BearerToken::parse(&raw).is_ok());
}

#[test]
fn a_token_shorter_than_the_minimum_is_refused() {
    let raw = "a".repeat(MIN_TOKEN_LEN - 1);
    assert_eq!(
        BearerToken::parse(&raw).err(),
        Some(TokenError::TooShort {
            length: MIN_TOKEN_LEN - 1
        })
    );
}

#[test]
fn an_empty_token_is_refused() {
    assert_eq!(
        BearerToken::parse("").err(),
        Some(TokenError::TooShort { length: 0 })
    );
}

#[test]
fn a_token_with_whitespace_is_refused() {
    let raw = format!("{TOKEN}\n");
    assert_eq!(
        BearerToken::parse(&raw).err(),
        Some(TokenError::NotVisibleAscii)
    );
}

#[test]
fn the_token_is_not_printed_by_debug() {
    assert!(!format!("{:?}", token()).contains(TOKEN));
}

// --- the Authorization header --------------------------------------------

#[test]
fn a_missing_header_is_not_authorized() {
    assert!(!token().authorizes(None));
}

#[test]
fn the_exact_bearer_token_is_authorized() {
    let value = HeaderValue::from_str(&format!("Bearer {TOKEN}")).unwrap();
    assert!(token().authorizes(Some(&value)));
}

#[test]
fn the_bearer_scheme_is_case_insensitive() {
    let value = HeaderValue::from_str(&format!("bearer {TOKEN}")).unwrap();
    assert!(token().authorizes(Some(&value)));
}

#[test]
fn a_wrong_token_is_not_authorized() {
    let wrong = TOKEN.replace('k', "j");
    let value = HeaderValue::from_str(&format!("Bearer {wrong}")).unwrap();
    assert!(!token().authorizes(Some(&value)));
}

#[test]
fn a_prefix_of_the_token_is_not_authorized() {
    let value = HeaderValue::from_str(&format!("Bearer {}", &TOKEN[..20])).unwrap();
    assert!(!token().authorizes(Some(&value)));
}

#[test]
fn another_scheme_with_the_token_is_not_authorized() {
    let value = HeaderValue::from_str(&format!("Basic {TOKEN}")).unwrap();
    assert!(!token().authorizes(Some(&value)));
}

// --- the router ----------------------------------------------------------

#[tokio::test]
async fn a_request_without_a_token_gets_401_and_runs_nothing() {
    let runner = CountingRunner::default();
    let app = router(
        OmniFocusServer::new(runner.clone()),
        token(),
        CancellationToken::new(),
    );

    let response = app.oneshot(initialize_request(None)).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers().get(header::WWW_AUTHENTICATE).unwrap(),
        "Bearer"
    );
    assert_eq!(runner.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_request_with_a_wrong_token_gets_401() {
    let app = router(
        OmniFocusServer::new(CountingRunner::default()),
        token(),
        CancellationToken::new(),
    );
    let wrong = format!("Bearer {}", TOKEN.replace('k', "j"));

    let response = app.oneshot(initialize_request(Some(&wrong))).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_request_with_the_token_gets_the_initialize_result() {
    let app = router(
        OmniFocusServer::new(CountingRunner::default()),
        token(),
        CancellationToken::new(),
    );
    let authorization = format!("Bearer {TOKEN}");

    let response = app
        .oneshot(initialize_request(Some(&authorization)))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_until_result(response.into_body()).await;
    assert!(
        body.contains("\"omnifocus-mcp\""),
        "initialize result should name the server, got: {body}"
    );
}

#[tokio::test]
async fn any_other_path_is_also_guarded() {
    let app = router(
        OmniFocusServer::new(CountingRunner::default()),
        token(),
        CancellationToken::new(),
    );
    let request = Request::get("/").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
