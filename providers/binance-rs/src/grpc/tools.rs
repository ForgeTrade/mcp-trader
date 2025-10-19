use crate::binance::client::BinanceClient;
use crate::error::{ProviderError, Result};
use crate::pb::{InvokeRequest, InvokeResponse, Json};

#[cfg(feature = "orderbook")]
use crate::orderbook::OrderBookManager;
#[cfg(feature = "orderbook")]
use std::sync::Arc;

// Helper functions for working with Json type
fn parse_json(json_opt: &Option<Json>) -> Result<serde_json::Value> {
    let json = json_opt
        .as_ref()
        .ok_or_else(|| ProviderError::Validation("Missing payload".to_string()))?;

    let json_str = std::str::from_utf8(&json.value)
        .map_err(|_| ProviderError::Validation("Invalid UTF-8 in payload".to_string()))?;

    serde_json::from_str(json_str).map_err(|e| ProviderError::Json(e))
}

/// Route tool invocation to appropriate handler
pub async fn route_tool(
    client: &BinanceClient,
    #[cfg(feature = "orderbook")]
    orderbook_manager: Option<Arc<OrderBookManager>>,
    #[cfg(not(feature = "orderbook"))]
    _orderbook_manager: Option<()>,
    request: &InvokeRequest,
) -> Result<InvokeResponse> {
    tracing::debug!("Routing tool: {}", request.tool_name);

    let result = match request.tool_name.as_str() {
        // Market data tools (public, no auth)
        "binance.get_ticker" => handle_get_ticker(client, request).await?,
        "binance.get_orderbook" => handle_get_orderbook(client, request).await?,
        "binance.get_recent_trades" => handle_get_recent_trades(client, request).await?,
        "binance.get_klines" => handle_get_klines(client, request).await?,
        "binance.get_exchange_info" => handle_get_exchange_info(client, request).await?,
        "binance.get_avg_price" => handle_get_avg_price(client, request).await?,

        // Account tools (authenticated)
        "binance.get_account" => handle_get_account(client, request).await?,
        "binance.get_my_trades" => handle_get_my_trades(client, request).await?,

        // Order management tools (authenticated)
        "binance.place_order" => handle_place_order(client, request).await?,
        "binance.cancel_order" => handle_cancel_order(client, request).await?,
        "binance.get_order" => handle_get_order(client, request).await?,
        "binance.get_open_orders" => handle_get_open_orders(client, request).await?,
        "binance.get_all_orders" => handle_get_all_orders(client, request).await?,

        // OrderBook analysis tools (feature-gated)
        #[cfg(feature = "orderbook")]
        "binance.orderbook_l1" => handle_orderbook_l1(orderbook_manager.as_ref(), request).await?,
        #[cfg(feature = "orderbook")]
        "binance.orderbook_l2" => handle_orderbook_l2(orderbook_manager.as_ref(), request).await?,
        #[cfg(feature = "orderbook")]
        "binance.orderbook_health" => handle_orderbook_health(orderbook_manager.as_ref(), request).await?,

        // Unknown tool
        _ => return Err(ProviderError::ToolNotFound(request.tool_name.clone())),
    };

    Ok(InvokeResponse {
        result: Some(result),
        error: String::new(),
    })
}

// ========== Market Data Tool Handlers ==========

async fn handle_get_ticker(client: &BinanceClient, request: &InvokeRequest) -> Result<Json> {
    let args = parse_json(&request.payload)?;
    let symbol = args["symbol"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing required field: symbol".to_string()))?;

    tracing::info!("Getting ticker for symbol: {}", symbol);

    // Call actual Binance API
    let ticker = client
        .get_24hr_ticker(symbol)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    // Serialize the response
    let result = serde_json::to_value(&ticker)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

async fn handle_get_orderbook(client: &BinanceClient, request: &InvokeRequest) -> Result<Json> {
    let args = parse_json(&request.payload)?;
    let symbol = args["symbol"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing required field: symbol".to_string()))?;
    let limit = args["limit"].as_u64().map(|l| l as u32);

    tracing::info!(
        "Getting orderbook for symbol: {}, limit: {:?}",
        symbol,
        limit
    );

    // Call actual Binance API
    let orderbook = client
        .get_order_book(symbol, limit)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&orderbook)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

async fn handle_get_recent_trades(client: &BinanceClient, request: &InvokeRequest) -> Result<Json> {
    let args = parse_json(&request.payload)?;
    let symbol = args["symbol"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing required field: symbol".to_string()))?;
    let limit = args["limit"].as_u64().map(|l| l as u32);

    tracing::info!(
        "Getting recent trades for symbol: {}, limit: {:?}",
        symbol,
        limit
    );

    // Call actual Binance API
    let trades = client
        .get_recent_trades(symbol, limit)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&trades)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

async fn handle_get_klines(client: &BinanceClient, request: &InvokeRequest) -> Result<Json> {
    let args = parse_json(&request.payload)?;
    let symbol = args["symbol"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing required field: symbol".to_string()))?;
    let interval = args["interval"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing required field: interval".to_string()))?;
    let limit = args["limit"].as_u64().map(|l| l as u32);

    tracing::info!(
        "Getting klines for symbol: {}, interval: {}, limit: {:?}",
        symbol,
        interval,
        limit
    );

    // Call actual Binance API
    let klines = client
        .get_klines(symbol, interval, limit)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&klines)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

async fn handle_get_exchange_info(
    _client: &BinanceClient,
    _request: &InvokeRequest,
) -> Result<Json> {
    tracing::info!("Getting exchange info");

    // TODO: Implement exchange_info endpoint in BinanceClient
    // For now, return a placeholder
    let result = serde_json::json!({
        "timezone": "UTC",
        "serverTime": chrono::Utc::now().timestamp_millis(),
        "note": "Exchange info endpoint not yet implemented in BinanceClient"
    });

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

async fn handle_get_avg_price(client: &BinanceClient, request: &InvokeRequest) -> Result<Json> {
    let args = parse_json(&request.payload)?;
    let symbol = args["symbol"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing required field: symbol".to_string()))?;

    tracing::info!("Getting average price for symbol: {}", symbol);

    // Use ticker_price as approximation since avg_price isn't in the client
    let ticker = client
        .get_ticker_price(symbol)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::json!({
        "symbol": ticker.symbol,
        "price": ticker.price
    });

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

// ========== Account Tool Handlers ==========

async fn handle_get_account(client: &BinanceClient, _request: &InvokeRequest) -> Result<Json> {
    tracing::info!("Getting account information");

    // Call actual Binance API
    let account = client
        .get_account()
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&account)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

async fn handle_get_my_trades(client: &BinanceClient, request: &InvokeRequest) -> Result<Json> {
    let args = parse_json(&request.payload)?;
    let symbol = args["symbol"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing required field: symbol".to_string()))?;
    let limit = args["limit"].as_u64().map(|l| l as u32);

    tracing::info!(
        "Getting my trades for symbol: {}, limit: {:?}",
        symbol,
        limit
    );

    // Call actual Binance API
    let trades = client
        .get_my_trades(symbol, limit)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&trades)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

// ========== Order Management Tool Handlers ==========

async fn handle_place_order(client: &BinanceClient, request: &InvokeRequest) -> Result<Json> {
    let args = parse_json(&request.payload)?;
    let symbol = args["symbol"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing required field: symbol".to_string()))?;
    let side = args["side"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing required field: side".to_string()))?;
    let order_type = args["order_type"].as_str().ok_or_else(|| {
        ProviderError::Validation("Missing required field: order_type".to_string())
    })?;
    let quantity = args["quantity"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing required field: quantity".to_string()))?;

    let price = args["price"].as_str();

    tracing::info!(
        "Placing order: symbol={}, side={}, type={}",
        symbol,
        side,
        order_type
    );

    // Call actual Binance API
    let order = client
        .create_order(symbol, side, order_type, quantity, price)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&order)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

async fn handle_cancel_order(client: &BinanceClient, request: &InvokeRequest) -> Result<Json> {
    let args = parse_json(&request.payload)?;
    let symbol = args["symbol"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing required field: symbol".to_string()))?;
    let order_id = args["order_id"]
        .as_i64()
        .ok_or_else(|| ProviderError::Validation("Missing required field: order_id".to_string()))?;

    tracing::info!("Canceling order: symbol={}, order_id={}", symbol, order_id);

    // Call actual Binance API
    let order = client
        .cancel_order(symbol, order_id)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&order)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

async fn handle_get_order(client: &BinanceClient, request: &InvokeRequest) -> Result<Json> {
    let args = parse_json(&request.payload)?;
    let symbol = args["symbol"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing required field: symbol".to_string()))?;
    let order_id = args["order_id"]
        .as_i64()
        .ok_or_else(|| ProviderError::Validation("Missing required field: order_id".to_string()))?;

    tracing::info!(
        "Getting order status: symbol={}, order_id={}",
        symbol,
        order_id
    );

    // Call actual Binance API
    let order = client
        .query_order(symbol, order_id)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&order)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

async fn handle_get_open_orders(client: &BinanceClient, request: &InvokeRequest) -> Result<Json> {
    let args = parse_json(&request.payload)?;
    let symbol = args["symbol"].as_str();

    tracing::info!("Getting open orders for symbol: {:?}", symbol);

    // Call actual Binance API
    let orders = client
        .get_open_orders(symbol)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&orders)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

async fn handle_get_all_orders(client: &BinanceClient, request: &InvokeRequest) -> Result<Json> {
    let args = parse_json(&request.payload)?;
    let symbol = args["symbol"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing required field: symbol".to_string()))?;
    let limit = args["limit"].as_u64().map(|l| l as u32);

    tracing::info!(
        "Getting all orders for symbol: {}, limit: {:?}",
        symbol,
        limit
    );

    // Call actual Binance API
    let orders = client
        .get_all_orders(symbol, limit)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&orders)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

// ========== OrderBook Analysis Tool Handlers (Feature-gated) ==========

#[cfg(feature = "orderbook")]
async fn handle_orderbook_l1(
    manager: Option<&Arc<OrderBookManager>>,
    request: &InvokeRequest,
) -> Result<Json> {
    use crate::orderbook::tools::{get_orderbook_metrics, GetOrderBookMetricsParams};

    // Check if manager is available
    let manager = manager
        .ok_or_else(|| ProviderError::Validation("OrderBook manager not initialized".to_string()))?;

    // Parse parameters
    let args = parse_json(&request.payload)?;
    let params: GetOrderBookMetricsParams = serde_json::from_value(args)
        .map_err(|e| ProviderError::Validation(format!("Invalid parameters: {}", e)))?;

    tracing::info!("Getting orderbook L1 metrics for symbol: {}", params.symbol);

    // Call orderbook tool
    let metrics = get_orderbook_metrics(manager.clone(), params)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&metrics)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

#[cfg(feature = "orderbook")]
async fn handle_orderbook_l2(
    manager: Option<&Arc<OrderBookManager>>,
    request: &InvokeRequest,
) -> Result<Json> {
    use crate::orderbook::tools::{get_orderbook_depth, GetOrderBookDepthParams};

    // Check if manager is available
    let manager = manager
        .ok_or_else(|| ProviderError::Validation("OrderBook manager not initialized".to_string()))?;

    // Parse parameters
    let args = parse_json(&request.payload)?;
    let params: GetOrderBookDepthParams = serde_json::from_value(args)
        .map_err(|e| ProviderError::Validation(format!("Invalid parameters: {}", e)))?;

    tracing::info!("Getting orderbook L2 depth for symbol: {}", params.symbol);

    // Call orderbook tool
    let depth = get_orderbook_depth(manager.clone(), params)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&depth)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

#[cfg(feature = "orderbook")]
async fn handle_orderbook_health(
    manager: Option<&Arc<OrderBookManager>>,
    _request: &InvokeRequest,
) -> Result<Json> {
    use crate::orderbook::tools::get_orderbook_health;

    // Check if manager is available
    let manager = manager
        .ok_or_else(|| ProviderError::Validation("OrderBook manager not initialized".to_string()))?;

    tracing::info!("Getting orderbook health status");

    // Call orderbook tool
    let health = get_orderbook_health(manager.clone())
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&health)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}
