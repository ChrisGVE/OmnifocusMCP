//! Serving MCP over streamable HTTP, behind a bearer token.
//!
//! Over stdio only the process that launched the server can talk to it. Over
//! HTTP anything that can open the port can, and the tools can delete
//! OmniFocus data, so every request must carry
//! `Authorization: Bearer <token>`. The check runs in front of the MCP
//! service: a request without the exact token is answered 401 and never
//! reaches a tool. rmcp's streamable-HTTP service does no authentication of
//! its own.
//!
//! The token is a shared secret the operator generates (for example
//! `openssl rand -base64 32`) and hands to both this server, through the
//! `OMNIFOCUS_MCP_TOKEN` environment variable, and the client. It is compared
//! in constant time and never printed.
//!
//! The bearer check does not replace network scoping: bind to an address
//! only trusted machines can reach (a Tailscale address, or loopback).

use std::{fmt, sync::Arc};

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderValue, Response, StatusCode},
    middleware::{self, Next},
    Router,
};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use subtle::ConstantTimeEq;
use tokio_util::sync::CancellationToken;

use crate::{jxa::JxaRunner, server::OmniFocusServer};

/// The path the MCP endpoint is served on.
pub const MCP_PATH: &str = "/mcp";

/// The environment variable the token is read from.
pub const TOKEN_ENV: &str = "OMNIFOCUS_MCP_TOKEN";

/// The shortest token accepted. 32 characters of base64 carry 192 bits;
/// `openssl rand -base64 32` produces 44.
pub const MIN_TOKEN_LEN: usize = 32;

/// Why a token was refused at startup.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TokenError {
    #[error("{TOKEN_ENV} is not set; --http requires a bearer token")]
    Missing,
    #[error("{TOKEN_ENV} is {length} characters; at least {MIN_TOKEN_LEN} are required")]
    TooShort { length: usize },
    #[error("{TOKEN_ENV} must contain only visible ASCII characters (no spaces or newlines)")]
    NotVisibleAscii,
}

/// The shared secret every HTTP request must present.
#[derive(Clone)]
pub struct BearerToken(Arc<[u8]>);

impl BearerToken {
    /// Accepts a token of at least `MIN_TOKEN_LEN` visible ASCII characters.
    ///
    /// Whitespace is refused rather than trimmed: a trailing newline from a
    /// secrets file would otherwise make the server and the client disagree
    /// on the secret in a way neither reports.
    pub fn parse(raw: &str) -> Result<Self, TokenError> {
        if raw.len() < MIN_TOKEN_LEN {
            return Err(TokenError::TooShort { length: raw.len() });
        }
        if !raw.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(TokenError::NotVisibleAscii);
        }
        Ok(Self(Arc::from(raw.as_bytes())))
    }

    /// Reads the token from `OMNIFOCUS_MCP_TOKEN`.
    pub fn from_env() -> Result<Self, TokenError> {
        match std::env::var(TOKEN_ENV) {
            Ok(raw) => Self::parse(&raw),
            Err(_) => Err(TokenError::Missing),
        }
    }

    /// True when `authorization` is `Bearer <this token>`. The scheme is
    /// case-insensitive (RFC 9110 §11.1); the token is compared in constant
    /// time.
    pub fn authorizes(&self, authorization: Option<&HeaderValue>) -> bool {
        let Some(value) = authorization else {
            return false;
        };
        let value = value.as_bytes();
        let scheme = b"bearer ";
        if value.len() <= scheme.len() || !value[..scheme.len()].eq_ignore_ascii_case(scheme) {
            return false;
        }
        bool::from(value[scheme.len()..].ct_eq(&self.0))
    }
}

/// Never prints the secret.
impl fmt::Debug for BearerToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BearerToken(<redacted>)")
    }
}

/// The HTTP application: the MCP service at `MCP_PATH`, with the bearer
/// check in front of every path. Each MCP session gets its own clone of
/// `server`; OmniFocus calls stay one at a time across sessions (see
/// `crate::jxa`). Cancelling `cancel` ends every open session.
pub fn router<R>(
    server: OmniFocusServer<R>,
    token: BearerToken,
    cancel: CancellationToken,
) -> Router
where
    R: JxaRunner + Clone + Send + Sync + 'static,
{
    let config = StreamableHttpServerConfig {
        cancellation_token: cancel,
        ..Default::default()
    };
    let mcp = StreamableHttpService::new(
        move || Ok(server.clone()),
        Arc::new(LocalSessionManager::default()),
        config,
    );
    Router::new()
        .nest_service(MCP_PATH, mcp)
        .layer(middleware::from_fn_with_state(token, require_bearer))
}

async fn require_bearer(
    State(token): State<BearerToken>,
    request: Request,
    next: Next,
) -> Response<Body> {
    if token.authorizes(request.headers().get(header::AUTHORIZATION)) {
        return next.run(request).await;
    }
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::WWW_AUTHENTICATE, "Bearer")
        .body(Body::empty())
        .expect("static 401 response is valid")
}
