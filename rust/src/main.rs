use std::{net::SocketAddr, process::ExitCode};

use clap::Parser;
use omnifocus_mcp::{
    http::{self, BearerToken},
    jxa::RealJxaRunner,
    server::OmniFocusServer,
    shutdown::{classify_initialize_error, classify_service_end, Shutdown},
};
use rmcp::{transport::stdio, ServiceExt};

#[derive(Parser, Debug)]
#[command(name = "omnifocus-mcp", version, about = "OmniFocus MCP server")]
struct Cli {
    /// Serve streamable HTTP on this address (e.g. 100.101.102.103:8765)
    /// instead of stdio. Requires the OMNIFOCUS_MCP_TOKEN environment
    /// variable; clients send it as `Authorization: Bearer <token>`.
    #[arg(long, value_name = "ADDR")]
    http: Option<SocketAddr>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let shutdown = match cli.http {
        Some(address) => serve_http_until_shutdown(address).await,
        None => serve_until_shutdown().await,
    };
    match shutdown {
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

/// Serves MCP over streamable HTTP on `address` until SIGINT or SIGTERM
/// (what launchd sends on stop). Refuses to start without a valid token.
async fn serve_http_until_shutdown(address: SocketAddr) -> Shutdown {
    let token = match BearerToken::from_env() {
        Ok(token) => token,
        Err(error) => return Shutdown::Failed(error.to_string()),
    };
    let listener = match tokio::net::TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(error) => return Shutdown::Failed(format!("cannot listen on {address}: {error}")),
    };
    eprintln!(
        "omnifocus-mcp: serving MCP on http://{address}{}",
        http::MCP_PATH
    );

    let cancel = tokio_util::sync::CancellationToken::new();
    let app = http::router(
        OmniFocusServer::new(RealJxaRunner::new()),
        token,
        cancel.clone(),
    );
    let result = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            termination_signal().await;
            cancel.cancel();
        })
        .await;
    match result {
        Ok(()) => Shutdown::Clean,
        Err(error) => Shutdown::Failed(format!("HTTP server failed: {error}")),
    }
}

/// Resolves on SIGINT or SIGTERM.
async fn termination_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut terminate = signal(SignalKind::terminate()).expect("SIGTERM handler installs");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}
