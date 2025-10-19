//! Order book data structures and types
//!
//! Core entities for order book depth analysis with progressive disclosure strategy.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Order book state for a single trading symbol
///
/// Maintains sorted bid/ask levels using BTreeMap for efficient range queries.
/// Updated via WebSocket delta streams or REST API snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    /// Trading pair symbol (uppercase, e.g., "BTCUSDT")
    pub symbol: String,

    /// Bid levels: price → quantity (sorted descending, best bid first)
    pub bids: BTreeMap<Decimal, Decimal>,

    /// Ask levels: price → quantity (sorted ascending, best ask first)
    pub asks: BTreeMap<Decimal, Decimal>,

    /// Last update ID from Binance (for delta processing)
    pub last_update_id: i64,

    /// Timestamp of last update (milliseconds since Unix epoch)
    pub timestamp: i64,
}

impl OrderBook {
    /// Create a new empty order book for the given symbol
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_update_id: 0,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Get best bid price (highest bid)
    pub fn best_bid(&self) -> Option<&Decimal> {
        self.bids.keys().next_back() // BTreeMap is ascending, so last key is highest
    }

    /// Get best ask price (lowest ask)
    pub fn best_ask(&self) -> Option<&Decimal> {
        self.asks.keys().next() // BTreeMap is ascending, so first key is lowest
    }

    /// Update a bid level (remove if quantity is zero)
    pub fn update_bid(&mut self, price: Decimal, quantity: Decimal) {
        if quantity.is_zero() {
            self.bids.remove(&price);
        } else {
            self.bids.insert(price, quantity);
        }
    }

    /// Update an ask level (remove if quantity is zero)
    pub fn update_ask(&mut self, price: Decimal, quantity: Decimal) {
        if quantity.is_zero() {
            self.asks.remove(&price);
        } else {
            self.asks.insert(price, quantity);
        }
    }
}

/// L1 aggregated metrics for quick spread and liquidity assessment
///
/// Provides lightweight analysis without full depth data (15% token cost vs L2-full).
/// Calculated on-demand from current OrderBook state.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OrderBookMetrics {
    /// Trading pair symbol (uppercased)
    pub symbol: String,

    /// Metrics calculation time (milliseconds since Unix epoch)
    pub timestamp: i64,

    /// Spread in basis points: ((best_ask - best_bid) / best_bid) * 10000
    pub spread_bps: f64,

    /// Volume-weighted fair price: (best_bid * ask_vol + best_ask * bid_vol) / (bid_vol + ask_vol)
    pub microprice: f64,

    /// Sum of top 20 bid level quantities (buy-side liquidity depth)
    pub bid_volume: f64,

    /// Sum of top 20 ask level quantities (sell-side liquidity depth)
    pub ask_volume: f64,

    /// Bid/ask volume ratio (bid_volume / ask_volume). >1 = more buy pressure
    pub imbalance_ratio: f64,

    /// Highest bid price (string for decimal precision)
    pub best_bid: String,

    /// Lowest ask price (string for decimal precision)
    pub best_ask: String,

    /// Significant price levels (qty > 2x median of top 20 levels)
    pub walls: Walls,

    /// VWAP-based slippage estimates for standard target amounts
    pub slippage_estimates: SlippageEstimates,
}

/// Container for bid and ask walls (support/resistance zones)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Walls {
    /// Buy walls (large support levels)
    pub bids: Vec<Wall>,

    /// Sell walls (large resistance levels)
    pub asks: Vec<Wall>,
}

/// Significant price level with large quantity (wall detection)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Wall {
    /// Price level (string for decimal precision)
    pub price: String,

    /// Quantity at this level (string for decimal precision)
    pub qty: String,

    /// Side of the order book
    pub side: WallSide,
}

/// Side of a wall (bid = support, ask = resistance)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum WallSide {
    /// Buy wall (support level)
    Bid,

    /// Sell wall (resistance level)
    Ask,
}

/// Container for slippage estimates across buy/sell directions and target amounts
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SlippageEstimates {
    /// Slippage for buying $10,000 worth
    pub buy_10k_usd: Option<SlippageEstimate>,

    /// Slippage for buying $25,000 worth
    pub buy_25k_usd: Option<SlippageEstimate>,

    /// Slippage for buying $50,000 worth
    pub buy_50k_usd: Option<SlippageEstimate>,

    /// Slippage for selling $10,000 worth
    pub sell_10k_usd: Option<SlippageEstimate>,

    /// Slippage for selling $25,000 worth
    pub sell_25k_usd: Option<SlippageEstimate>,

    /// Slippage for selling $50,000 worth
    pub sell_50k_usd: Option<SlippageEstimate>,
}

/// VWAP-based slippage calculation for a target USD amount
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SlippageEstimate {
    /// Target amount in USD (10000, 25000, or 50000)
    pub target_usd: f64,

    /// Volume-weighted average price for this fill
    pub avg_price: f64,

    /// Slippage in basis points: ((avg_price - best_price) / best_price) * 10000
    pub slippage_bps: f64,

    /// Actual quantity filled (may be less than target if liquidity insufficient)
    pub filled_qty: f64,

    /// Actual USD amount filled (may be less than target)
    pub filled_usd: f64,
}

/// L2 depth data with compact integer encoding for token efficiency
///
/// Reduces JSON size by ~40% using scaled integers instead of full decimal strings.
/// Supports L2-lite (20 levels) and L2-full (100 levels).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OrderBookDepth {
    /// Trading pair symbol (uppercased)
    pub symbol: String,

    /// Snapshot time (milliseconds since Unix epoch)
    pub timestamp: i64,

    /// Scaling factor for prices (fixed at 100)
    /// Actual price = scaled_price / 100
    pub price_scale: i32,

    /// Scaling factor for quantities (fixed at 100000)
    /// Actual qty = scaled_qty / 100000
    pub qty_scale: i32,

    /// Bid levels as [scaled_price, scaled_qty] tuples (sorted descending by price)
    pub bids: Vec<[i64; 2]>,

    /// Ask levels as [scaled_price, scaled_qty] tuples (sorted ascending by price)
    pub asks: Vec<[i64; 2]>,
}

/// Service health status for order book tracking
///
/// Provides operational visibility into WebSocket connections and data freshness.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OrderBookHealth {
    /// Overall health status
    pub status: HealthStatus,

    /// Number of symbols with active WebSocket connections (0-20)
    pub orderbook_symbols_active: usize,

    /// Milliseconds since last successful depth update across all symbols
    /// <5000 is healthy, >5000 indicates staleness
    pub last_update_age_ms: i64,

    /// Overall WebSocket health (true if ≥1 connection active)
    pub websocket_connected: bool,

    /// Health check time (milliseconds since Unix epoch)
    pub timestamp: i64,

    /// Human-readable error message if status != 'ok'
    pub reason: Option<String>,
}

/// Health status levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All systems operational
    Ok,

    /// Partial functionality (some connections down or data stale)
    Degraded,

    /// Critical failure (all connections down)
    Error,
}
