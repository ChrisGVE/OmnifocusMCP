//! How the end of the server is turned into an exit status.
//!
//! The server runs until its client goes away. When the client's input reaches
//! end of file — before, during or after the MCP handshake — that is a normal
//! shutdown and the process exits 0. Anything else (a handshake the client got
//! wrong, a task that panicked) is a failure: `main` logs it to stderr and
//! exits non-zero, so a misbehaving client or a crash is never silent.
//!
//! The decision is made on rmcp's error and quit-reason variants, never on
//! their text: rmcp's handshake-violation message ("expect initialized
//! request, but received: …") contains the same words as its end-of-input
//! message ("connection closed: initialized request").

use rmcp::service::{QuitReason, ServerInitializeError};
use tokio::task::JoinError;

/// How the server ended.
#[derive(Debug, PartialEq, Eq)]
pub enum Shutdown {
    /// The client went away, or the server was cancelled: exit 0.
    Clean,
    /// Anything else, with the reason to log: exit non-zero.
    Failed(String),
}

/// Classifies a failed MCP handshake (`serve` returning an error).
///
/// End of input while waiting for the client is clean, and so is
/// cancellation, which only our own cancellation token can cause. Every
/// other variant means the handshake itself went wrong.
pub fn classify_initialize_error(error: &ServerInitializeError) -> Shutdown {
    match error {
        ServerInitializeError::ConnectionClosed(_) | ServerInitializeError::Cancelled => {
            Shutdown::Clean
        }
        other => Shutdown::Failed(format!("MCP handshake failed: {other}")),
    }
}

/// Classifies the end of a running service (the result of `waiting`).
///
/// The input stream closing and cancellation are clean. A join error — the
/// service task itself panicking, or rmcp quitting because one of its send
/// tasks did — is a failure.
pub fn classify_service_end(end: Result<QuitReason, JoinError>) -> Shutdown {
    match end {
        Ok(QuitReason::Closed | QuitReason::Cancelled) => Shutdown::Clean,
        Ok(QuitReason::JoinError(error)) => {
            Shutdown::Failed(format!("MCP service task failed: {error}"))
        }
        Err(error) => Shutdown::Failed(format!("MCP service stopped unexpectedly: {error}")),
    }
}
