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
    let mut port = 50053u16;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--grpc" => mode = "grpc".to_string(),
            "--stdio" => mode = "stdio".to_string(),
            "--port" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().unwrap_or(50053);
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
    println!("    --grpc              Run in gRPC mode (default)");
    println!("    --stdio             Run in stdio MCP mode");
    println!("    --port <PORT>       gRPC port to listen on (default: 50053)");
    println!("    --help, -h          Print this help message");
    println!();
    println!("ENVIRONMENT VARIABLES:");
    println!("    BINANCE_API_KEY     Binance API key (required for authenticated operations)");
    println!("    BINANCE_API_SECRET  Binance API secret (required for authenticated operations)");
    println!("    BINANCE_BASE_URL    Binance API base URL (default: https://api.binance.com)");
    println!("    RUST_LOG            Logging level (default: info)");
    println!();
    println!("EXAMPLES:");
    println!("    # Start gRPC server on default port (50053)");
    println!("    binance-provider --grpc");
    println!();
    println!("    # Start gRPC server on custom port");
    println!("    binance-provider --grpc --port 8080");
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
    tracing::info!("  - 16 tools (market data, account, orders, orderbook)");
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

    // Start the gRPC server with graceful shutdown
    Server::builder()
        .add_service(ProviderServer::new(provider))
        .serve_with_shutdown(addr, async {
            shutdown_rx.await.ok();
            tracing::info!("Shutting down gRPC server...");
        })
        .await?;

    tracing::info!("Server stopped");
    Ok(())
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
