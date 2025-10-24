use binance_provider::grpc::BinanceProviderServer;
use binance_provider::pb::provider_server::ProviderServer;
use std::net::SocketAddr;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments first to determine mode
    let args: Vec<String> = std::env::args().collect();
    let (mode, port) = parse_args(&args);

    // Initialize tracing/logging
    // For stdio mode, output to stderr (stdout is reserved for MCP protocol)
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .with_writer(std::io::stderr) // Always write to stderr for MCP compatibility
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting Binance Provider in {} mode...", mode);

    match mode.as_str() {
        "grpc" => run_grpc_server(port).await?,
        "http" => run_http_server(port).await?,
        "stdio" => run_stdio_server().await?,
        "sse" => run_sse_server(port).await?,
        _ => {
            eprintln!("Invalid mode: {}", mode);
            print_usage();
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Parse command-line arguments
fn parse_args(args: &[String]) -> (String, u16) {
    let mut mode = "grpc".to_string();
    let mut port = 0u16; // 0 means use default based on mode
    let mut port_set_explicitly = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                if i + 1 < args.len() {
                    mode = args[i + 1].clone();
                    i += 1;
                }
            }
            "--grpc" => mode = "grpc".to_string(),
            "--http" => mode = "http".to_string(),
            "--stdio" => mode = "stdio".to_string(),
            "--sse" => mode = "sse".to_string(),
            "--port" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().unwrap_or(0);
                    port_set_explicitly = true;
                    i += 1;
                }
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    // Set default port based on mode if not explicitly set
    if !port_set_explicitly || port == 0 {
        port = match mode.as_str() {
            "http" => 3000,
            "grpc" => 50053,
            "sse" => 8000,
            _ => 50053,
        };
    }

    (mode, port)
}

/// Print usage information
fn print_usage() {
    println!("Binance Provider - MCP server for Binance cryptocurrency market data analysis");
    println!();
    println!("USAGE:");
    println!("    binance-provider [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --mode <MODE>       Transport mode: grpc, http, stdio, or sse (default: grpc)");
    println!("    --grpc              Run in gRPC mode (shortcut for --mode grpc)");
    println!("    --http              Run in HTTP mode (shortcut for --mode http)");
    println!("    --stdio             Run in stdio MCP mode (shortcut for --mode stdio)");
    println!("    --sse               Run in SSE mode (shortcut for --mode sse)");
    println!("    --port <PORT>       Port to listen on (default: 50053 for gRPC, 3000 for HTTP, 8000 for SSE)");
    println!("    --help, -h          Print this help message");
    println!();
    println!("ENVIRONMENT VARIABLES:");
    println!("    BINANCE_API_KEY       Binance API key (optional, preserved for future use)");
    println!("    BINANCE_API_SECRET    Binance API secret (optional, preserved for future use)");
    println!("    BINANCE_BASE_URL      Binance API base URL (default: https://api.binance.com)");
    println!("    ANALYTICS_DATA_PATH   Analytics storage path (default: ./data/analytics)");
    println!("    RUST_LOG              Logging level (default: info)");
    println!();
    println!("EXAMPLES:");
    println!("    # Start gRPC server on default port (50053)");
    println!("    binance-provider --grpc");
    println!();
    println!("    # Start HTTP server on default port (3000)");
    println!("    binance-provider --http");
    println!();
    println!("    # Start HTTP server on custom port");
    println!("    binance-provider --mode http --port 8080");
    println!();
    println!("    # Start gRPC server with analytics features");
    println!("    cargo run --features orderbook,orderbook_analytics -- --grpc --port 50053");
    println!();
    println!("    # Start in stdio mode");
    println!("    binance-provider --stdio");
}

/// Run the provider in gRPC mode
async fn run_grpc_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;

    tracing::info!("Initializing Binance Provider Server...");
    let provider = BinanceProviderServer::new()?;

    tracing::info!("Starting gRPC server on {}", addr);
    tracing::info!("Provider capabilities:");

    #[cfg(feature = "orderbook")]
    {
        tracing::info!("  - 1 tool (unified market data report):");
        tracing::info!("    * generate_market_report - Comprehensive market intelligence");
        tracing::info!("      Consolidates: price, orderbook, liquidity, volume profile,");
        tracing::info!("      order flow, anomalies, and market health into single report");
    }

    #[cfg(not(feature = "orderbook"))]
    {
        tracing::info!("  - 0 tools (orderbook feature required for generate_market_report)");
    }

    tracing::info!("  - 1 resource (market data)");
    tracing::info!("  - 1 prompt (trading-analysis)");

    // Check for API credentials
    match (
        std::env::var("BINANCE_API_KEY"),
        std::env::var("BINANCE_API_SECRET"),
    ) {
        (Ok(_), Ok(_)) => {
            tracing::info!("API credentials found - authenticated operations enabled");
        }
        _ => {
            tracing::warn!("API credentials not found - only public market data tools will work");
            tracing::warn!("Set BINANCE_API_KEY and BINANCE_API_SECRET for full functionality");
        }
    }

    // Create graceful shutdown handler with broadcast channel (supports multiple receivers)
    let (shutdown_tx, _shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);
    let mut server_shutdown_rx = shutdown_tx.subscribe();

    // Spawn shutdown signal handler
    let signal_tx = shutdown_tx.clone();
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                tracing::info!("Received shutdown signal (Ctrl+C)");
                let _ = signal_tx.send(());
            }
            Err(err) => {
                tracing::error!("Failed to listen for shutdown signal: {}", err);
            }
        }
    });

    // Pre-subscribe to symbols and spawn snapshot persistence task (T015-T020)
    #[cfg(feature = "orderbook_analytics")]
    {
        // T015: Pre-subscribe to BTCUSDT WebSocket
        if let Err(e) = provider.orderbook_manager.subscribe("BTCUSDT").await {
            tracing::error!("Failed to pre-subscribe to BTCUSDT: {}", e);
        } else {
            // T017: INFO logging for pre-subscription
            tracing::info!("Pre-subscribed to BTCUSDT for snapshot persistence");
        }

        // T016: Pre-subscribe to ETHUSDT WebSocket
        if let Err(e) = provider.orderbook_manager.subscribe("ETHUSDT").await {
            tracing::error!("Failed to pre-subscribe to ETHUSDT: {}", e);
        } else {
            // T017: INFO logging for pre-subscription
            tracing::info!("Pre-subscribed to ETHUSDT for snapshot persistence");
        }

        // T018: Spawn snapshot persistence task
        let persistence_shutdown_rx = shutdown_tx.subscribe();
        let _persistence_handle =
            binance_provider::orderbook::analytics::storage::spawn_snapshot_persistence_task(
                provider.analytics_storage.clone(),
                provider.orderbook_manager.clone(),
                &["BTCUSDT", "ETHUSDT"], // T020: Verify correct symbol parameters
                persistence_shutdown_rx, // T019: Pass shutdown_rx for graceful shutdown
            );

        tracing::info!("Snapshot persistence task spawned for BTCUSDT, ETHUSDT");

        // Feature 008: Spawn trade stream persistence task
        let trade_shutdown_rx = shutdown_tx.subscribe();
        let trade_storage_handle = provider.trade_storage.clone();

        tokio::spawn(async move {
            use binance_provider::orderbook::analytics::trade_storage::AggTrade as PersistAggTrade;
            use binance_provider::orderbook::analytics::trade_stream::TradeStreamHandler;
            use std::time::Duration;
            use tokio::time::interval;

            // Create unbounded channels for BTC and ETH trade streams
            let (btc_tx, mut btc_rx) = tokio::sync::mpsc::unbounded_channel();
            let (eth_tx, mut eth_rx) = tokio::sync::mpsc::unbounded_channel();

            // Spawn WebSocket handlers
            let mut btc_handler = TradeStreamHandler::new("BTCUSDT");
            let mut eth_handler = TradeStreamHandler::new("ETHUSDT");

            tokio::spawn(async move {
                if let Err(e) = btc_handler.connect_with_backoff(btc_tx).await {
                    tracing::error!("BTCUSDT trade stream failed: {}", e);
                }
            });

            tokio::spawn(async move {
                if let Err(e) = eth_handler.connect_with_backoff(eth_tx).await {
                    tracing::error!("ETHUSDT trade stream failed: {}", e);
                }
            });

            tracing::info!("Starting trade stream collection for BTCUSDT");
            tracing::info!("Starting trade stream collection for ETHUSDT");

            // Buffers for 1-second batching
            let mut btc_buffer: Vec<PersistAggTrade> = Vec::new();
            let mut eth_buffer: Vec<PersistAggTrade> = Vec::new();

            let mut flush_interval = interval(Duration::from_secs(1));
            let mut shutdown_rx = trade_shutdown_rx;

            loop {
                tokio::select! {
                    Some(trade) = btc_rx.recv() => {
                        btc_buffer.push((&trade).into());
                    }
                    Some(trade) = eth_rx.recv() => {
                        eth_buffer.push((&trade).into());
                    }
                    _ = flush_interval.tick() => {
                        let now_ms = chrono::Utc::now().timestamp_millis();

                        if !btc_buffer.is_empty() {
                            let count = btc_buffer.len();
                            if let Err(e) = trade_storage_handle.store_batch("BTCUSDT", now_ms, btc_buffer.drain(..).collect()) {
                                tracing::error!("Failed to store BTCUSDT trades: {}", e);
                            } else {
                                tracing::info!("Stored {} trades for BTCUSDT at timestamp {}", count, now_ms);
                            }
                        }

                        if !eth_buffer.is_empty() {
                            let count = eth_buffer.len();
                            if let Err(e) = trade_storage_handle.store_batch("ETHUSDT", now_ms, eth_buffer.drain(..).collect()) {
                                tracing::error!("Failed to store ETHUSDT trades: {}", e);
                            } else {
                                tracing::info!("Stored {} trades for ETHUSDT at timestamp {}", count, now_ms);
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Shutting down trade persistence task...");
                        break;
                    }
                }
            }
        });

        tracing::info!("Trade persistence task spawned for BTCUSDT, ETHUSDT");
    }

    // Start the gRPC server with graceful shutdown
    Server::builder()
        .add_service(ProviderServer::new(provider))
        .serve_with_shutdown(addr, async move {
            server_shutdown_rx.recv().await.ok();
            tracing::info!("Shutting down gRPC server...");
        })
        .await?;

    tracing::info!("Server stopped");
    Ok(())
}

/// Run the provider in HTTP mode
#[cfg(feature = "http_transport")]
async fn run_http_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Initializing Binance Provider (HTTP mode)...");

    #[cfg(all(feature = "orderbook", feature = "orderbook_analytics"))]
    {
        let provider = BinanceProviderServer::new()?;
        binance_provider::transport::http::start_http_server(
            port,
            provider.binance_client,
            Some(provider.orderbook_manager),
            Some(provider.analytics_storage),
            Some(provider.trade_storage),
            Some(provider.report_generator),
        )
        .await?;
    }

    #[cfg(all(feature = "orderbook", not(feature = "orderbook_analytics")))]
    {
        let provider = BinanceProviderServer::new()?;
        binance_provider::transport::http::start_http_server(
            port,
            provider.binance_client,
            Some(provider.orderbook_manager),
            Some(provider.report_generator),
        )
        .await?;
    }

    #[cfg(not(feature = "orderbook"))]
    {
        let provider = BinanceProviderServer::new()?;
        binance_provider::transport::http::start_http_server(port, provider.binance_client).await?;
    }

    Ok(())
}

#[cfg(not(feature = "http_transport"))]
async fn run_http_server(_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("HTTP transport not available. Build with --features http_transport");
    std::process::exit(1);
}

/// Run the provider in stdio MCP mode
#[cfg(feature = "mcp_server")]
async fn run_stdio_server() -> Result<(), Box<dyn std::error::Error>> {
    use binance_provider::transport::stdio::run_stdio_server;
    run_stdio_server().await
}

#[cfg(not(feature = "mcp_server"))]
async fn run_stdio_server() -> Result<(), Box<dyn std::error::Error>> {
    tracing::error!("stdio mode not available - compile with 'mcp_server' feature");
    Err("stdio mode not available".into())
}

/// Run the provider in SSE mode (Server-Sent Events)
#[cfg(feature = "mcp_server")]
async fn run_sse_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    use binance_provider::mcp::BinanceServer;
    use binance_provider::transport::sse::{CancellationToken, SseServer, SseServerConfig};

    let addr = format!("0.0.0.0:{}", port).parse()?;
    tracing::info!("Starting SSE server on {}", addr);

    // Create SSE server configuration
    let config = SseServerConfig {
        bind: addr,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: None,
    };

    // Start SSE server
    let sse_server = SseServer::serve_with_config(config).await?;
    tracing::info!("SSE server ready on {}", addr);
    tracing::info!("  SSE endpoint: http://{}/sse", addr);
    tracing::info!("  POST endpoint: http://{}/message", addr);

    // Attach MCP service
    let shutdown_ct = sse_server.with_service(|| BinanceServer::new());

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    tracing::info!("Received shutdown signal (Ctrl+C)");
    shutdown_ct.cancel();

    Ok(())
}

#[cfg(not(feature = "mcp_server"))]
async fn run_sse_server(_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    tracing::error!("SSE mode not available - compile with 'mcp_server' feature");
    Err("SSE mode not available".into())
}
