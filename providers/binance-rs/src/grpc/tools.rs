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
    #[cfg(feature = "orderbook")] orderbook_manager: Option<Arc<OrderBookManager>>,
    #[cfg(not(feature = "orderbook"))] _orderbook_manager: Option<()>,
    #[cfg(feature = "orderbook_analytics")] analytics_storage: Option<
        Arc<crate::orderbook::analytics::SnapshotStorage>,
    >,
    #[cfg(not(feature = "orderbook_analytics"))] _analytics_storage: Option<()>,
    #[cfg(feature = "orderbook_analytics")] trade_storage: Option<
        Arc<crate::orderbook::analytics::TradeStorage>,
    >,
    #[cfg(not(feature = "orderbook_analytics"))] _trade_storage: Option<()>,
    #[cfg(feature = "orderbook")] report_generator: Option<Arc<crate::report::ReportGenerator>>,
    #[cfg(not(feature = "orderbook"))] _report_generator: Option<()>,
    request: &InvokeRequest,
) -> Result<InvokeResponse> {
    tracing::debug!("Routing tool: {}", request.tool_name);

    let result = match request.tool_name.as_str() {
        // Unified market data report - THE ONLY PUBLIC TOOL (per FR-002)
        #[cfg(feature = "orderbook")]
        "binance.generate_market_report" => {
            handle_generate_market_report(report_generator.as_ref(), request).await?
        }

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

// ========== OrderBook Analysis Tool Handlers (Feature-gated) ==========

#[cfg(feature = "orderbook")]
async fn handle_orderbook_l1(
    manager: Option<&Arc<OrderBookManager>>,
    request: &InvokeRequest,
) -> Result<Json> {
    use crate::orderbook::tools::{get_orderbook_metrics, GetOrderBookMetricsParams};

    // Check if manager is available
    let manager = manager.ok_or_else(|| {
        ProviderError::Validation("OrderBook manager not initialized".to_string())
    })?;

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
    let manager = manager.ok_or_else(|| {
        ProviderError::Validation("OrderBook manager not initialized".to_string())
    })?;

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
    let manager = manager.ok_or_else(|| {
        ProviderError::Validation("OrderBook manager not initialized".to_string())
    })?;

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

// ========== Advanced Analytics Tool Handlers (Feature-gated) ==========

#[cfg(feature = "orderbook_analytics")]
async fn handle_get_order_flow(
    storage: Option<&Arc<crate::orderbook::analytics::SnapshotStorage>>,
    request: &InvokeRequest,
) -> Result<Json> {
    use crate::orderbook::analytics::tools::{get_order_flow, GetOrderFlowParams};

    // Check if storage is available
    let storage = storage.ok_or_else(|| {
        ProviderError::Validation("Analytics storage not initialized".to_string())
    })?;

    // Parse parameters
    let args = parse_json(&request.payload)?;
    let params: GetOrderFlowParams = serde_json::from_value(args)
        .map_err(|e| ProviderError::Validation(format!("Invalid parameters: {}", e)))?;

    tracing::info!(
        "Getting order flow analysis for symbol: {} (window: {}s)",
        params.symbol,
        params.window_duration_secs
    );

    // Call analytics tool
    let snapshot = get_order_flow(storage.clone(), params)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&snapshot)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

#[cfg(feature = "orderbook_analytics")]
async fn handle_get_volume_profile(
    trade_storage: Option<&Arc<crate::orderbook::analytics::TradeStorage>>,
    request: &InvokeRequest,
) -> Result<Json> {
    use crate::orderbook::analytics::tools::{get_volume_profile, GetVolumeProfileParams};

    let trade_storage = trade_storage
        .ok_or_else(|| ProviderError::Validation("Trade storage not initialized".to_string()))?;

    // Parse parameters
    let args = parse_json(&request.payload)?;
    let params: GetVolumeProfileParams = serde_json::from_value(args)
        .map_err(|e| ProviderError::Validation(format!("Invalid parameters: {}", e)))?;

    tracing::info!(
        "Getting volume profile for symbol: {} (duration: {}h)",
        params.symbol,
        params.duration_hours
    );

    // Query trades from TradeStorage for the specified time window
    let end_time = chrono::Utc::now().timestamp_millis();
    let start_time = end_time - (params.duration_hours as i64 * 3600 * 1000);

    let trades = trade_storage
        .query_trades(&params.symbol, start_time, end_time)
        .map_err(|e| ProviderError::BinanceApi(format!("Failed to query trades: {}", e)))?;

    tracing::debug!(
        "Queried {} trades for {} over {}h window",
        trades.len(),
        params.symbol,
        params.duration_hours
    );

    // Convert trade_storage::AggTrade to trade_stream::AggTrade
    let trades_for_profile: Vec<crate::orderbook::analytics::trade_stream::AggTrade> = trades
        .into_iter()
        .map(|t| crate::orderbook::analytics::trade_stream::AggTrade {
            event_type: "aggTrade".to_string(),
            event_time: t.timestamp,
            symbol: params.symbol.clone(),
            agg_trade_id: t.trade_id as u64,
            price: t.price.clone(),
            quantity: t.quantity.clone(),
            first_trade_id: t.trade_id as u64,
            last_trade_id: t.trade_id as u64,
            trade_time: t.timestamp,
            is_buyer_maker: t.buyer_is_maker,
            is_best_match: true,
        })
        .collect();

    // Call analytics tool
    let profile = get_volume_profile(trades_for_profile, params)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&profile)?;

    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

#[cfg(feature = "orderbook_analytics")]
async fn handle_detect_market_anomalies(
    storage: Option<&Arc<crate::orderbook::analytics::SnapshotStorage>>,
    request: &InvokeRequest,
) -> Result<Json> {
    use crate::orderbook::analytics::tools::detect_market_anomalies;

    let storage = storage.ok_or_else(|| {
        ProviderError::Validation("Analytics storage not initialized".to_string())
    })?;

    let args = parse_json(&request.payload)?;
    let symbol = args["symbol"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing symbol".to_string()))?;

    let anomalies = detect_market_anomalies(storage.clone(), symbol)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&anomalies)?;
    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

#[cfg(feature = "orderbook_analytics")]
async fn handle_get_microstructure_health(
    storage: Option<&Arc<crate::orderbook::analytics::SnapshotStorage>>,
    request: &InvokeRequest,
) -> Result<Json> {
    use crate::orderbook::analytics::tools::get_microstructure_health;

    let storage = storage.ok_or_else(|| {
        ProviderError::Validation("Analytics storage not initialized".to_string())
    })?;

    let args = parse_json(&request.payload)?;
    let symbol = args["symbol"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing symbol".to_string()))?;

    let health = get_microstructure_health(storage.clone(), symbol)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&health)?;
    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

#[cfg(feature = "orderbook_analytics")]
async fn handle_get_liquidity_vacuums(
    storage: Option<&Arc<crate::orderbook::analytics::SnapshotStorage>>,
    request: &InvokeRequest,
) -> Result<Json> {
    use crate::orderbook::analytics::tools::{get_liquidity_vacuums, GetLiquidityVacuumsParams};

    let storage = storage.ok_or_else(|| {
        ProviderError::Validation("Analytics storage not initialized".to_string())
    })?;

    let args = parse_json(&request.payload)?;
    let params: GetLiquidityVacuumsParams = serde_json::from_value(args)
        .map_err(|e| ProviderError::Validation(format!("Invalid parameters: {}", e)))?;

    let vacuums = get_liquidity_vacuums(storage.clone(), params)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let result = serde_json::to_value(&vacuums)?;
    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}

// ========== Market Data Report Handler ==========

#[cfg(feature = "orderbook")]
async fn handle_generate_market_report(
    report_generator: Option<&Arc<crate::report::ReportGenerator>>,
    request: &InvokeRequest,
) -> Result<Json> {
    let generator = report_generator
        .ok_or_else(|| ProviderError::Validation("Report generator not initialized".to_string()))?;

    let args = parse_json(&request.payload)?;
    let symbol = args["symbol"]
        .as_str()
        .ok_or_else(|| ProviderError::Validation("Missing required field: symbol".to_string()))?;

    tracing::info!("Generating market report for symbol: {}", symbol);

    // Parse options if provided
    let options = if let Some(opts) = args.get("options") {
        serde_json::from_value(opts.clone())
            .map_err(|e| ProviderError::Validation(format!("Invalid options: {}", e)))?
    } else {
        crate::report::ReportOptions::default()
    };

    // Generate report
    let report = generator
        .generate_report(symbol, options)
        .await
        .map_err(|e| ProviderError::BinanceApi(e))?;

    let result = serde_json::to_value(&report)?;
    Ok(Json {
        value: serde_json::to_string(&result)?.as_bytes().to_vec(),
    })
}
