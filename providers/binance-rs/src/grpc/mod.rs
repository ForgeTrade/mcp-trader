use crate::binance::client::BinanceClient;
use crate::error::Result;
use crate::pb::{provider_server::Provider, *};
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
    pub(crate) binance_client: BinanceClient,

    /// Order book manager (optional, enabled with orderbook feature)
    #[cfg(feature = "orderbook")]
    pub(crate) orderbook_manager: Arc<OrderBookManager>,
}

impl BinanceProviderServer {
    /// Create a new BinanceProviderServer with credentials from environment
    pub fn new() -> Result<Self> {
        let binance_client = BinanceClient::with_credentials();

        #[cfg(feature = "orderbook")]
        {
            tracing::info!("OrderBook feature enabled - initializing WebSocket manager");
            let orderbook_manager = Arc::new(OrderBookManager::new(Arc::new(binance_client.clone())));

            Ok(Self {
                binance_client,
                orderbook_manager,
            })
        }

        #[cfg(not(feature = "orderbook"))]
        {
            Ok(Self {
                binance_client,
            })
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
        #[cfg(feature = "orderbook")]
        let response = tools::route_tool(&self.binance_client, Some(self.orderbook_manager.clone()), &req).await?;

        #[cfg(not(feature = "orderbook"))]
        let response = tools::route_tool(&self.binance_client, None, &req).await?;

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
