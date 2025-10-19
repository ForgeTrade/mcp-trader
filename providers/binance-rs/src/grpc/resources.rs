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
    } else if request.uri == "binance://account/balances" {
        handle_account_balances_resource(client).await
    } else if let Some(symbol) = parse_trades_uri(&request.uri) {
        handle_trades_resource(client, &symbol).await
    } else if let Some(status) = parse_orders_uri(&request.uri) {
        handle_orders_resource(client, &status).await
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
fn parse_trades_uri(uri: &str) -> Option<String> {
    if let Some(rest) = uri.strip_prefix("binance://account/trades/") {
        if !rest.is_empty() {
            return Some(rest.to_uppercase());
        }
    }
    None
}

/// Parse binance://orders/{status} URI
fn parse_orders_uri(uri: &str) -> Option<String> {
    if let Some(rest) = uri.strip_prefix("binance://orders/") {
        if !rest.is_empty() && matches!(rest, "open" | "filled" | "canceled") {
            return Some(rest.to_string());
        }
    }
    None
}

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

async fn handle_account_balances_resource(client: &BinanceClient) -> Result<ResourceResponse> {
    tracing::info!("Fetching account balances resource");

    // Fetch real account data from Binance API
    let account = client
        .get_account()
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");

    // Filter balances with non-zero total
    let balances: Vec<String> = account
        .balances
        .iter()
        .filter(|b| {
            let free = b.free.parse::<f64>().unwrap_or(0.0);
            let locked = b.locked.parse::<f64>().unwrap_or(0.0);
            free + locked > 0.0
        })
        .map(|b| {
            let free = b.free.parse::<f64>().unwrap_or(0.0);
            let locked = b.locked.parse::<f64>().unwrap_or(0.0);
            let total = free + locked;
            format!(
                "| {} | {:.8} | {:.8} | {:.8} |",
                b.asset, free, locked, total
            )
        })
        .collect();

    let balance_rows = if balances.is_empty() {
        "| - | 0.00000000 | 0.00000000 | 0.00000000 |".to_string()
    } else {
        balances.join("\n")
    };

    let content = format!(
        r#"# Account Balances

## Spot Balances

| Asset | Free | Locked | Total |
|-------|------|--------|-------|
{}

## Account Information

| Property | Value |
|----------|-------|
| Account Type | {} |
| Can Trade | {} |
| Can Withdraw | {} |
| Can Deposit | {} |

*Data fetched at: {}*
*Note: Requires valid API credentials.*
"#,
        balance_rows,
        account.account_type,
        account.can_trade,
        account.can_withdraw,
        account.can_deposit,
        timestamp
    );

    Ok(ResourceResponse {
        content: content.as_bytes().to_vec(),
        mime_type: "text/markdown".to_string(),
        error: String::new(),
    })
}

async fn handle_trades_resource(client: &BinanceClient, symbol: &str) -> Result<ResourceResponse> {
    tracing::info!("Fetching trades resource for symbol: {}", symbol);

    // Fetch real trade history from Binance API
    let trades = client
        .get_my_trades(symbol, Some(10))
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");

    let trade_rows: Vec<String> = trades
        .iter()
        .map(|t| {
            let time = chrono::DateTime::from_timestamp_millis(t.time)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let side = if t.is_buyer { "BUY" } else { "SELL" };
            format!(
                "| {} | {} | {} | ${} | {} | ${} |",
                time, t.id, side, t.price, t.qty, t.quote_qty
            )
        })
        .collect();

    let trade_table = if trade_rows.is_empty() {
        "| - | - | - | - | - | - |\n*No trades found for this symbol*".to_string()
    } else {
        trade_rows.join("\n")
    };

    let content = format!(
        r#"# Trade History: {}

## Recent Trades (Last 10)

| Time | Trade ID | Side | Price | Quantity | Quote Qty |
|------|----------|------|-------|----------|-----------|
{}

*Data fetched at: {}*
*Note: Requires valid API credentials.*
"#,
        symbol, trade_table, timestamp
    );

    Ok(ResourceResponse {
        content: content.as_bytes().to_vec(),
        mime_type: "text/markdown".to_string(),
        error: String::new(),
    })
}

async fn handle_orders_resource(client: &BinanceClient, status: &str) -> Result<ResourceResponse> {
    tracing::info!("Fetching orders resource with status: {}", status);

    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");

    // Fetch orders based on status
    let content = if status == "open" {
        // Fetch open orders (all symbols)
        let orders = client
            .get_open_orders(None)
            .await
            .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

        let order_rows: Vec<String> = orders
            .iter()
            .map(|o| {
                let time = chrono::DateTime::from_timestamp_millis(o.transact_time)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "N/A".to_string());
                format!(
                    "| {} | {} | {} | {} | {} | ${} | {} | {} |",
                    o.order_id, time, o.symbol, o.order_type, o.side, o.price, o.orig_qty, o.status
                )
            })
            .collect();

        let order_table = if order_rows.is_empty() {
            "| - | - | - | - | - | - | - | - |\n*No open orders*".to_string()
        } else {
            order_rows.join("\n")
        };

        format!(
            r#"# Open Orders

## Order List

| Order ID | Time | Symbol | Type | Side | Price | Quantity | Status |
|----------|------|--------|------|------|-------|----------|--------|
{}

*Data fetched at: {}*
*Note: Requires valid API credentials.*
"#,
            order_table, timestamp
        )
    } else {
        // For filled/canceled, return a note
        format!(
            r#"# {} Orders

*Note: Filtering by order status ({}) requires querying specific symbols.*
*Use the `binance.get_all_orders` tool with a symbol to see {} orders.*

*Data fetched at: {}*
*Note: Requires valid API credentials.*
"#,
            status.to_uppercase(),
            status,
            status,
            timestamp
        )
    };

    Ok(ResourceResponse {
        content: content.as_bytes().to_vec(),
        mime_type: "text/markdown".to_string(),
        error: String::new(),
    })
}
