use std::process::ExitCode;

use clap::Parser;
use omnifocus_mcp::{
    jxa::RealJxaRunner,
    server::OmniFocusServer,
    shutdown::{classify_initialize_error, classify_service_end, Shutdown},
};
use rmcp::{transport::stdio, ServiceExt};

#[derive(Parser, Debug)]
#[command(name = "omnifocus-mcp", version, about = "OmniFocus MCP server")]
struct Cli {}

#[tokio::main]
async fn main() -> ExitCode {
    let _cli = Cli::parse();

    match serve_until_shutdown().await {
        Shutdown::Clean => ExitCode::SUCCESS,
        Shutdown::Failed(reason) => {
            eprintln!("omnifocus-mcp: {reason}");
            ExitCode::FAILURE
        }
    }
}

/// Serves MCP over stdio until the client goes away, the service fails, or
/// Ctrl-C cancels it, and says which (see `omnifocus_mcp::shutdown`).
async fn serve_until_shutdown() -> Shutdown {
    let server = OmniFocusServer::new(RealJxaRunner::new());
    let service = match server.serve(stdio()).await {
        Ok(service) => service,
        Err(error) => return classify_initialize_error(&error),
    };
    let cancel_token = service.cancellation_token();
    let waiting = service.waiting();
    tokio::pin!(waiting);

    tokio::select! {
        end = &mut waiting => classify_service_end(end),
        _ = tokio::signal::ctrl_c() => {
            cancel_token.cancel();
            classify_service_end(waiting.await)
        }
    }
}
