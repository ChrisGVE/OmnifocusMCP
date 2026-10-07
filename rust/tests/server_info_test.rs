//! Server identity reported in the MCP `initialize` result.
//!
//! rmcp's default `Implementation` is built from rmcp's own build
//! environment, so a server that leaves `server_info` at its default
//! introduces itself to every client as "rmcp 0.17.0". These tests pin the
//! identity to this crate's package name and version.

use std::{future::Future, pin::Pin};

use omnifocus_mcp::{jxa::JxaRunner, server::OmniFocusServer};
use rmcp::ServerHandler;
use serde_json::Value;

/// `get_info` reads no OmniFocus data; any script run is a test failure.
struct NeverCalledRunner;

impl JxaRunner for NeverCalledRunner {
    fn run_omnijs<'a>(
        &'a self,
        _script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        Box::pin(async { panic!("get_info must not run omnijs") })
    }
}

#[test]
fn server_reports_its_own_name() {
    let info = OmniFocusServer::new(NeverCalledRunner).get_info();
    assert_eq!(info.server_info.name, "omnifocus-mcp");
}

#[test]
fn server_reports_the_crate_version() {
    let info = OmniFocusServer::new(NeverCalledRunner).get_info();
    assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
}
