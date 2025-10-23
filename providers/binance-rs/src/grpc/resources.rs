use crate::binance::client::BinanceClient;
use crate::error::{ProviderError, Result};
use crate::pb::{ResourceRequest, ResourceResponse};

/// Handle resource read request by routing based on URI
pub async fn handle_resource(
    client: &BinanceClient,
    request: &ResourceRequest,
) -> Result<ResourceResponse> {
    tracing::debug!("Handling resource URI: {}", request.uri);

    // Parse URI and route to appropriate handler
    if let Some(symbol) = parse_market_uri(&request.uri) {
        handle_market_resource(client, &symbol).await
    } else {
        Err(ProviderError::ResourceNotFound(request.uri.clone()))
    }
}

// ========== URI Parsers ==========

/// Parse binance://market/{symbol} URI
fn parse_market_uri(uri: &str) -> Option<String> {
    if let Some(rest) = uri.strip_prefix("binance://market/") {
        if !rest.is_empty() {
            return Some(rest.to_uppercase());
        }
    }
    None
}

/// Parse binance://account/trades/{symbol} URI

// ========== Resource Handlers ==========

async fn handle_market_resource(client: &BinanceClient, symbol: &str) -> Result<ResourceResponse> {
    tracing::info!("Fetching market resource for symbol: {}", symbol);

    // Fetch real market data from Binance API
    let ticker = client
        .get_24hr_ticker(symbol)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let orderbook = client
        .get_order_book(symbol, Some(5))
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    // Format bid/ask data
    let top_bids = orderbook
        .bids
        .iter()
        .take(5)
        .map(|(price, qty)| format!("| ${} | {} |", price, qty))
        .collect::<Vec<_>>()
        .join("\n");

    let top_asks = orderbook
        .asks
        .iter()
        .take(5)
        .map(|(price, qty)| format!("| ${} | {} |", price, qty))
        .collect::<Vec<_>>()
        .join("\n");

    let base_asset = symbol.strip_suffix("USDT").unwrap_or(symbol);
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");

    let content = format!(
        r#"# Market Data Summary: {}

## 24h Price Statistics

| Metric | Value |
|--------|-------|
| Last Price | ${} |
| 24h Change | ${} ({}) |
| 24h High | ${} |
| 24h Low | ${} |
| 24h Volume | {} {} |
| Quote Volume | ${} USDT |

## Order Book Snapshot

### Top 5 Bids (Buy Orders)
| Price | Quantity |
|-------|----------|
{}

### Top 5 Asks (Sell Orders)
| Price | Quantity |
|-------|----------|
{}

### Spread
- Best Bid: ${}
- Best Ask: ${}
- Spread: ${:.2} ({:.4}%)

*Data fetched at: {}*
"#,
        symbol,
        ticker.last_price,
        ticker.price_change,
        ticker.price_change_percent,
        ticker.high_price,
        ticker.low_price,
        ticker.volume,
        base_asset,
        ticker.quote_volume,
        top_bids,
        top_asks,
        orderbook.bids.first().map(|(p, _)| p.as_str()).unwrap_or("N/A"),
        orderbook.asks.first().map(|(p, _)| p.as_str()).unwrap_or("N/A"),
        orderbook.asks.first().and_then(|ask| {
            orderbook.bids.first().map(|bid| {
                ask.0.parse::<f64>().unwrap_or(0.0) - bid.0.parse::<f64>().unwrap_or(0.0)
            })
        }).unwrap_or(0.0),
        orderbook.asks.first().and_then(|ask| {
            orderbook.bids.first().map(|bid| {
                let ask_price = ask.0.parse::<f64>().unwrap_or(0.0);
                let bid_price = bid.0.parse::<f64>().unwrap_or(0.0);
                if bid_price > 0.0 {
                    ((ask_price - bid_price) / bid_price) * 100.0
                } else {
                    0.0
                }
            })
        }).unwrap_or(0.0),
        timestamp
    );

    Ok(ResourceResponse {
        content: content.as_bytes().to_vec(),
        mime_type: "text/markdown".to_string(),
        error: String::new(),
    })
}



