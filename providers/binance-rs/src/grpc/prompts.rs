use crate::binance::client::BinanceClient;
use crate::error::{ProviderError, Result};
use crate::pb::{Json, PromptMessage, PromptRequest, PromptResponse};

// Helper function to parse Json payload
fn parse_json(json_opt: &Option<Json>) -> Result<serde_json::Value> {
    match json_opt {
        Some(json) => {
            let json_str = std::str::from_utf8(&json.value)
                .map_err(|_| ProviderError::Validation("Invalid UTF-8 in arguments".to_string()))?;
            serde_json::from_str(json_str).map_err(|e| ProviderError::Json(e))
        }
        None => Ok(serde_json::json!({})),
    }
}

/// Handle prompt template request by routing based on prompt name
pub async fn handle_prompt(
    client: &BinanceClient,
    request: &PromptRequest,
) -> Result<PromptResponse> {
    tracing::debug!("Handling prompt: {}", request.prompt_name);

    let messages = match request.prompt_name.as_str() {
        "trading-analysis" => handle_trading_analysis_prompt(client, request).await?,
        "portfolio-risk" => handle_portfolio_risk_prompt(client, request).await?,
        _ => return Err(ProviderError::PromptNotFound(request.prompt_name.clone())),
    };

    Ok(PromptResponse {
        messages,
        error: String::new(),
    })
}

// ========== Prompt Handlers ==========

async fn handle_trading_analysis_prompt(
    client: &BinanceClient,
    request: &PromptRequest,
) -> Result<Vec<PromptMessage>> {
    let args = parse_json(&request.arguments)?;

    let symbol = args["symbol"].as_str().ok_or_else(|| {
        ProviderError::Validation("Missing required argument: symbol".to_string())
    })?;

    let timeframe = args["timeframe"].as_str().unwrap_or("1d");

    tracing::info!(
        "Generating trading analysis prompt for symbol: {}, timeframe: {}",
        symbol,
        timeframe
    );

    // Fetch real market data from Binance API
    let ticker = client
        .get_24hr_ticker(symbol)
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let orderbook = client
        .get_order_book(symbol, Some(10))
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let base_asset = symbol.strip_suffix("USDT").unwrap_or(symbol);

    let system_message = format!(
        r#"You are a professional cryptocurrency trading analyst. Analyze the market conditions for {} using the provided data and suggest trading strategies.

Focus on:
1. Price action and trend analysis
2. Support and resistance levels
3. Volume analysis and liquidity
4. Market sentiment indicators
5. Risk-reward ratio for potential trades

Provide actionable insights with clear entry/exit points and risk management suggestions.
Timeframe for analysis: {}"#,
        symbol, timeframe
    );

    // Calculate spread for order book analysis
    let (best_bid, best_ask, spread, spread_pct) = if let (Some(bid), Some(ask)) =
        (orderbook.bids.first(), orderbook.asks.first()) {
        let bid_price = bid.0.parse::<f64>().unwrap_or(0.0);
        let ask_price = ask.0.parse::<f64>().unwrap_or(0.0);
        let spread_val = ask_price - bid_price;
        let spread_pct_val = if bid_price > 0.0 {
            (spread_val / bid_price) * 100.0
        } else {
            0.0
        };
        (bid.0.clone(), ask.0.clone(), spread_val, spread_pct_val)
    } else {
        ("N/A".to_string(), "N/A".to_string(), 0.0, 0.0)
    };

    let context_message = format!(
        r#"## Market Data for {}

### Price Statistics (24h)
- Last Price: ${}
- 24h Change: ${} ({}%)
- 24h High: ${}
- 24h Low: ${}
- 24h Volume: {} {}
- Quote Volume: ${} USDT

### Order Book Snapshot
- Best Bid: ${}
- Best Ask: ${}
- Spread: ${:.2} ({:.4}%)

### Top Bids & Asks
Top 5 Bids: {}
Top 5 Asks: {}

Use this data to provide a comprehensive trading analysis."#,
        symbol,
        ticker.last_price,
        ticker.price_change,
        ticker.price_change_percent,
        ticker.high_price,
        ticker.low_price,
        ticker.volume,
        base_asset,
        ticker.quote_volume,
        best_bid,
        best_ask,
        spread,
        spread_pct,
        orderbook.bids.iter().take(5)
            .map(|(p, q)| format!("{}@{}", q, p))
            .collect::<Vec<_>>()
            .join(", "),
        orderbook.asks.iter().take(5)
            .map(|(p, q)| format!("{}@{}", q, p))
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(vec![
        PromptMessage {
            role: "system".to_string(),
            content: system_message,
        },
        PromptMessage {
            role: "user".to_string(),
            content: context_message,
        },
    ])
}

async fn handle_portfolio_risk_prompt(
    client: &BinanceClient,
    request: &PromptRequest,
) -> Result<Vec<PromptMessage>> {
    let args = parse_json(&request.arguments)?;

    let risk_tolerance = args["risk_tolerance"].as_str().unwrap_or("medium");

    tracing::info!(
        "Generating portfolio risk assessment prompt with risk tolerance: {}",
        risk_tolerance
    );

    // Fetch real account data from Binance API
    let account = client
        .get_account()
        .await
        .map_err(|e| ProviderError::BinanceApi(e.to_string()))?;

    let system_message = format!(
        r#"You are a professional cryptocurrency portfolio manager and risk analyst. Assess the current portfolio composition and provide risk management recommendations.

User's risk tolerance: {}

Focus on:
1. Portfolio diversification analysis
2. Asset allocation recommendations
3. Risk concentration assessment
4. Volatility and correlation analysis
5. Rebalancing strategies
6. Position sizing recommendations

Provide actionable portfolio management advice tailored to the user's risk tolerance."#,
        risk_tolerance
    );

    // Filter non-zero balances and build holdings table
    let holdings: Vec<_> = account
        .balances
        .iter()
        .filter(|b| {
            let free = b.free.parse::<f64>().unwrap_or(0.0);
            let locked = b.locked.parse::<f64>().unwrap_or(0.0);
            free + locked > 0.0
        })
        .collect();

    let holdings_table = if holdings.is_empty() {
        "| - | 0.00000000 | - | - | - |\n*No holdings found*".to_string()
    } else {
        holdings
            .iter()
            .map(|b| {
                let free = b.free.parse::<f64>().unwrap_or(0.0);
                let locked = b.locked.parse::<f64>().unwrap_or(0.0);
                let total = free + locked;
                format!(
                    "| {} | {:.8} | - | - | - |",
                    b.asset, total
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let num_assets = holdings.len();
    let largest_position = holdings
        .iter()
        .max_by(|a, b| {
            let a_total = a.free.parse::<f64>().unwrap_or(0.0) + a.locked.parse::<f64>().unwrap_or(0.0);
            let b_total = b.free.parse::<f64>().unwrap_or(0.0) + b.locked.parse::<f64>().unwrap_or(0.0);
            a_total.partial_cmp(&b_total).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|b| b.asset.as_str())
        .unwrap_or("N/A");

    let context_message = format!(
        r#"## Current Portfolio

### Holdings
| Asset | Quantity | USD Value | % of Portfolio | 24h Change |
|-------|----------|-----------|----------------|------------|
{}

### Portfolio Metrics
- **Number of Assets**: {}
- **Largest Position**: {}
- **Account Type**: {}
- **Can Trade**: {}

### Risk Metrics
*Note: Price data and advanced metrics require additional API calls. Current data shows account balances only.*
*For detailed USD valuations and % allocations, use the binance.get_account tool or market data resources.*

Use this account data to provide a comprehensive portfolio risk assessment and rebalancing recommendations."#,
        holdings_table,
        num_assets,
        largest_position,
        account.account_type,
        account.can_trade
    );

    Ok(vec![
        PromptMessage {
            role: "system".to_string(),
            content: system_message,
        },
        PromptMessage {
            role: "user".to_string(),
            content: context_message,
        },
    ])
}
