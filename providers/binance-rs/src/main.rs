use binance_provider::grpc::BinanceProviderServer;
use binance_provider::pb::provider_server::ProviderServer;
use std::net::SocketAddr;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing/logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting Binance Provider...");

    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    let (mode, port) = parse_args(&args);

    match mode.as_str() {
        "grpc" => run_grpc_server(port).await?,
        "http" => run_http_server(port).await?,
        "stdio" => run_stdio_server().await?,
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
            _ => 50053,
        };
    }

    (mode, port)
}

/// Print usage information
fn print_usage() {
    println!("Binance Provider - MCP server for Binance cryptocurrency trading");
    println!();
    println!("USAGE:");
    println!("    binance-provider [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --mode <MODE>       Transport mode: grpc, http, or stdio (default: grpc)");
    println!("    --grpc              Run in gRPC mode (shortcut for --mode grpc)");
    println!("    --http              Run in HTTP mode (shortcut for --mode http)");
    println!("    --stdio             Run in stdio MCP mode (shortcut for --mode stdio)");
    println!("    --port <PORT>       Port to listen on (default: 50053 for gRPC, 3000 for HTTP)");
    println!("    --help, -h          Print this help message");
    println!();
    println!("ENVIRONMENT VARIABLES:");
    println!("    BINANCE_API_KEY       Binance API key (required for authenticated operations)");
    println!("    BINANCE_API_SECRET    Binance API secret (required for authenticated operations)");
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

    #[cfg(feature = "orderbook_analytics")]
    {
        tracing::info!("  - 21 tools (16 base + 5 analytics):");
        tracing::info!("    * Market data: ticker, orderbook, trades, klines, exchange_info, avg_price");
        tracing::info!("    * Account: get_account, get_my_trades");
        tracing::info!("    * Orders: place, cancel, get, get_open, get_all");
        tracing::info!("    * OrderBook: L1 metrics, L2 depth, health");
        tracing::info!("    * Analytics: order_flow, volume_profile, anomalies, health, liquidity_vacuums");
    }

    #[cfg(all(feature = "orderbook", not(feature = "orderbook_analytics")))]
    {
        tracing::info!("  - 16 tools (market data, account, orders, orderbook L1/L2)");
    }

    #[cfg(not(feature = "orderbook"))]
    {
        tracing::info!("  - 13 tools (market data, account, orders)");
    }

    tracing::info!("  - 4 resources (market, balances, trades, orders)");
    tracing::info!("  - 2 prompts (trading-analysis, portfolio-risk)");

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
        let _persistence_handle = binance_provider::orderbook::analytics::storage::spawn_snapshot_persistence_task(
            provider.analytics_storage.clone(),
            provider.orderbook_manager.clone(),
            &["BTCUSDT", "ETHUSDT"], // T020: Verify correct symbol parameters
            persistence_shutdown_rx, // T019: Pass shutdown_rx for graceful shutdown
        );

        tracing::info!("Snapshot persistence task spawned for BTCUSDT, ETHUSDT");
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
        )
        .await?;
    }

    #[cfg(not(feature = "orderbook"))]
    {
        let provider = BinanceProviderServer::new()?;
        binance_provider::transport::http::start_http_server(
            port,
            provider.binance_client,
        )
        .await?;
    }

    Ok(())
}

#[cfg(not(feature = "http_transport"))]
async fn run_http_server(_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("HTTP transport not available. Build with --features http_transport");
    std::process::exit(1);
}

/// Run the provider in stdio MCP mode
async fn run_stdio_server() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting in stdio MCP mode...");

    // TODO: Implement stdio MCP server using rmcp
    // This would use the rmcp library to run as a traditional stdio MCP server
    // For now, this is a placeholder for future implementation

    tracing::error!("stdio mode not yet implemented");
    tracing::info!("Please use --grpc mode instead");

    Err("stdio mode not implemented".into())
}
