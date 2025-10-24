//! HTTP transport for MCP using Axum
//!
//! Provides streamable HTTP transport with JSON-RPC 2.0 protocol.
//! Session management with 30-minute timeout and 50 concurrent session limit.

pub mod error;
pub mod handler;
pub mod jsonrpc;
pub mod session;

use axum::{routing::post, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use handler::{handle_jsonrpc, AppState};
use session::SessionStore;

/// Start HTTP server with MCP JSON-RPC endpoint
///
/// # Arguments
/// * `port` - Port to listen on (default: 3000)
/// * `binance_client` - Binance API client
/// * `orderbook_manager` - Optional orderbook manager
/// * `analytics_storage` - Optional analytics storage
/// * `trade_storage` - Optional trade storage
/// * `report_generator` - Optional market report generator
///
/// # Endpoints
/// - POST /mcp: JSON-RPC 2.0 endpoint
///   - initialize: Create session
///   - tools/list: List available tools
///   - tools/call: Execute tool
///
/// # CORS
/// Configured to allow all origins (*) for development.
/// In production, should be restricted to specific origins.
pub async fn start_http_server(
    port: u16,
    binance_client: crate::binance::client::BinanceClient,
    #[cfg(feature = "orderbook")] orderbook_manager: Option<
        Arc<crate::orderbook::OrderBookManager>,
    >,
    #[cfg(feature = "orderbook_analytics")] analytics_storage: Option<
        Arc<crate::orderbook::analytics::SnapshotStorage>,
    >,
    #[cfg(feature = "orderbook_analytics")] trade_storage: Option<
        Arc<crate::orderbook::analytics::TradeStorage>,
    >,
    #[cfg(feature = "orderbook")] report_generator: Option<Arc<crate::report::ReportGenerator>>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Initializing HTTP MCP server...");

    // Create session store (max 50 concurrent sessions)
    let sessions = SessionStore::new(50);

    // Build application state
    let state = AppState {
        sessions,
        binance_client,
        #[cfg(feature = "orderbook")]
        orderbook_manager,
        #[cfg(feature = "orderbook_analytics")]
        analytics_storage,
        #[cfg(feature = "orderbook_analytics")]
        trade_storage,
        #[cfg(feature = "orderbook")]
        report_generator,
    };

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        .route("/mcp", post(handle_jsonrpc))
        .layer(cors)
        .with_state(state);

    // Bind to address
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;

    tracing::info!("HTTP MCP server listening on {}", addr);
    tracing::info!("Endpoint: POST http://{}:{}/mcp", addr.ip(), addr.port());

    #[cfg(feature = "orderbook_analytics")]
    {
        tracing::info!("  - 21 tools (16 base + 5 analytics)");
    }

    #[cfg(all(feature = "orderbook", not(feature = "orderbook_analytics")))]
    {
        tracing::info!("  - 16 tools (market data, account, orders, orderbook L1/L2)");
    }

    #[cfg(not(feature = "orderbook"))]
    {
        tracing::info!("  - 13 tools (market data, account, orders)");
    }

    tracing::info!("Session management:");
    tracing::info!("  - Max concurrent sessions: 50");
    tracing::info!("  - Session timeout: 30 minutes");
    tracing::info!("  - Header: Mcp-Session-Id (UUID)");

    // Start server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Create graceful shutdown handler
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Spawn shutdown signal handler
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                tracing::info!("Received shutdown signal (Ctrl+C)");
                let _ = shutdown_tx.send(());
            }
            Err(err) => {
                tracing::error!("Failed to listen for shutdown signal: {}", err);
            }
        }
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            shutdown_rx.await.ok();
            tracing::info!("Shutting down HTTP server...");
        })
        .await?;

    tracing::info!("Server stopped");
    Ok(())
}
