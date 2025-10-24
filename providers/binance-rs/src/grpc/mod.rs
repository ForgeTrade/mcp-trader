use crate::binance::client::BinanceClient;
use crate::error::Result;
use crate::pb::{provider_server::Provider, *};
#[cfg(feature = "orderbook")]
use crate::report::ReportGenerator;
use tonic::{Request, Response, Status};

#[cfg(feature = "orderbook")]
use crate::orderbook::OrderBookManager;
#[cfg(feature = "orderbook")]
use std::sync::Arc;

pub mod capabilities;
pub mod prompts;
pub mod resources;
pub mod tools;

use capabilities::CapabilityBuilder;

/// BinanceProviderServer implements the Provider gRPC service
#[derive(Clone)]
pub struct BinanceProviderServer {
    /// Binance API client
    pub binance_client: BinanceClient,

    /// Order book manager (optional, enabled with orderbook feature)
    #[cfg(feature = "orderbook")]
    pub orderbook_manager: Arc<OrderBookManager>,

    /// Analytics storage (optional, enabled with orderbook_analytics feature)
    #[cfg(feature = "orderbook_analytics")]
    pub analytics_storage: Arc<crate::orderbook::analytics::SnapshotStorage>,

    /// Trade persistence storage (optional, enabled with orderbook_analytics feature)
    #[cfg(feature = "orderbook_analytics")]
    pub trade_storage: Arc<crate::orderbook::analytics::TradeStorage>,

    /// Market data report generator
    #[cfg(feature = "orderbook")]
    pub report_generator: Arc<ReportGenerator>,
}

impl BinanceProviderServer {
    /// Create a new BinanceProviderServer with credentials from environment
    pub fn new() -> Result<Self> {
        let binance_client = BinanceClient::with_credentials();

        #[cfg(all(feature = "orderbook", feature = "orderbook_analytics"))]
        {
            tracing::info!("OrderBook feature enabled - initializing WebSocket manager");
            let orderbook_manager =
                Arc::new(OrderBookManager::new(Arc::new(binance_client.clone())));

            tracing::info!("Analytics feature enabled - initializing RocksDB storage");
            let data_path = std::env::var("ANALYTICS_DATA_PATH")
                .unwrap_or_else(|_| "./data/analytics".to_string());

            let analytics_storage = Arc::new(
                crate::orderbook::analytics::SnapshotStorage::new(&data_path).map_err(|e| {
                    crate::error::ProviderError::Initialization(format!(
                        "Failed to initialize analytics storage: {}",
                        e
                    ))
                })?,
            );

            tracing::info!("Analytics storage initialized at: {}", data_path);

            // Initialize TradeStorage (shares same RocksDB as SnapshotStorage)
            let trade_storage = Arc::new(crate::orderbook::analytics::TradeStorage::new(
                analytics_storage.db(),
            ));

            tracing::info!("Trade persistence storage initialized (shared RocksDB)");

            // Initialize ReportGenerator
            let report_generator = Arc::new(ReportGenerator::new(
                Arc::new(binance_client.clone()),
                orderbook_manager.clone(),
                60, // 60 second cache TTL
            ));

            tracing::info!("Market data report generator initialized");

            Ok(Self {
                binance_client,
                orderbook_manager,
                analytics_storage,
                trade_storage,
                report_generator,
            })
        }

        #[cfg(all(feature = "orderbook", not(feature = "orderbook_analytics")))]
        {
            tracing::info!("OrderBook feature enabled - initializing WebSocket manager");
            let orderbook_manager =
                Arc::new(OrderBookManager::new(Arc::new(binance_client.clone())));

            // Initialize ReportGenerator
            let report_generator = Arc::new(ReportGenerator::new(
                Arc::new(binance_client.clone()),
                orderbook_manager.clone(),
                60, // 60 second cache TTL
            ));

            tracing::info!("Market data report generator initialized");

            Ok(Self {
                binance_client,
                orderbook_manager,
                report_generator,
            })
        }

        #[cfg(not(feature = "orderbook"))]
        {
            Ok(Self { binance_client })
        }
    }
}

#[tonic::async_trait]
impl Provider for BinanceProviderServer {
    type StreamStream = futures::stream::Empty<std::result::Result<CloudEvent, Status>>;

    async fn list_capabilities(
        &self,
        _request: Request<()>,
    ) -> std::result::Result<Response<Capabilities>, Status> {
        tracing::info!("ListCapabilities RPC called");

        let builder = CapabilityBuilder::new();
        let capabilities = builder.build()?;

        Ok(Response::new(capabilities))
    }

    async fn invoke(
        &self,
        request: Request<InvokeRequest>,
    ) -> std::result::Result<Response<InvokeResponse>, Status> {
        let req = request.into_inner();
        tracing::info!(
            "Invoke RPC called: tool_name={}, correlation_id={}",
            req.tool_name,
            req.correlation_id
        );

        // Route to tool handler
        #[cfg(all(feature = "orderbook", feature = "orderbook_analytics"))]
        let response = tools::route_tool(
            &self.binance_client,
            Some(self.orderbook_manager.clone()),
            Some(self.analytics_storage.clone()),
            Some(self.trade_storage.clone()),
            Some(self.report_generator.clone()),
            &req,
        )
        .await?;

        #[cfg(all(feature = "orderbook", not(feature = "orderbook_analytics")))]
        let response = tools::route_tool(
            &self.binance_client,
            Some(self.orderbook_manager.clone()),
            None,
            None,
            Some(self.report_generator.clone()),
            &req,
        )
        .await?;

        #[cfg(not(feature = "orderbook"))]
        let response =
            tools::route_tool(&self.binance_client, None, None, None, None, &req).await?;

        Ok(Response::new(response))
    }

    async fn read_resource(
        &self,
        request: Request<ResourceRequest>,
    ) -> std::result::Result<Response<ResourceResponse>, Status> {
        let req = request.into_inner();
        tracing::info!(
            "ReadResource RPC called: uri={}, correlation_id={}",
            req.uri,
            req.correlation_id
        );

        let response = resources::handle_resource(&self.binance_client, &req).await?;

        Ok(Response::new(response))
    }

    async fn get_prompt(
        &self,
        request: Request<PromptRequest>,
    ) -> std::result::Result<Response<PromptResponse>, Status> {
        let req = request.into_inner();
        tracing::info!(
            "GetPrompt RPC called: prompt_name={}, correlation_id={}",
            req.prompt_name,
            req.correlation_id
        );

        let response = prompts::handle_prompt(&self.binance_client, &req).await?;

        Ok(Response::new(response))
    }

    async fn stream(
        &self,
        _request: Request<StreamRequest>,
    ) -> std::result::Result<Response<Self::StreamStream>, Status> {
        // Not implemented for binance-rs provider
        Err(Status::unimplemented(
            "Streaming not supported by binance-rs provider",
        ))
    }
}

impl Default for BinanceProviderServer {
    fn default() -> Self {
        Self::new().expect("Failed to create BinanceProviderServer")
    }
}
