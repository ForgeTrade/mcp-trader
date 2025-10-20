//! MCP Tool and Prompt Parameter Types
//!
//! This module defines parameter types for MCP tools and prompts with JsonSchema support.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Common parameter for symbol-based tools
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SymbolParam {
    /// Trading pair symbol (e.g., BTCUSDT, ETHUSDT)
    #[schemars(description = "Trading pair symbol in uppercase (e.g., BTCUSDT, ETHUSDT)")]
    pub symbol: String,
}

/// Parameters for orderbook depth query
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderbookParam {
    /// Trading pair symbol
    #[schemars(description = "Trading pair symbol (e.g., BTCUSDT)")]
    pub symbol: String,

    /// Depth limit (5, 10, 20, 50, 100, 500, 1000, 5000)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Order book depth limit (default: 20)")]
    pub limit: Option<u32>,
}

/// Arguments for trading_analysis prompt
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradingAnalysisArgs {
    /// Trading pair symbol (e.g., BTCUSDT, ETHUSDT)
    #[schemars(description = "Trading pair symbol (e.g., BTCUSDT, ETHUSDT)")]
    pub symbol: String,

    /// Optional trading strategy preference
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Trading strategy: aggressive, balanced, or conservative")]
    pub strategy: Option<TradingStrategy>,

    /// Optional risk tolerance level
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Risk tolerance: low, medium, or high")]
    pub risk_tolerance: Option<RiskTolerance>,
}

/// Trading strategy preference
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum TradingStrategy {
    /// High-frequency, short-term trades
    Aggressive,
    /// Mixed approach balancing risk and reward
    Balanced,
    /// Low-risk, long-term holds
    Conservative,
}

/// Risk tolerance level
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum RiskTolerance {
    /// Risk-averse, prefer stable assets
    Low,
    /// Moderate risk acceptable
    Medium,
    /// High-risk, high-reward tolerance
    High,
}

/// Arguments for portfolio_risk prompt
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PortfolioRiskArgs {
    // Empty struct - no parameters required
    // Account info is derived from API credentials
}

/// Arguments for market_microstructure_analysis prompt
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[cfg(feature = "orderbook_analytics")]
pub struct MarketMicrostructureArgs {
    /// Trading pair symbol (e.g., BTCUSDT, ETHUSDT)
    #[schemars(description = "Trading pair symbol (e.g., BTCUSDT, ETHUSDT)")]
    pub symbol: String,

    /// Analysis depth: quick (5min), standard (1h), deep (24h)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Analysis depth: quick, standard, or deep")]
    pub analysis_depth: Option<AnalysisDepth>,
}

/// Analysis depth level for advanced market analysis
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Copy)]
#[serde(rename_all = "lowercase")]
#[cfg(feature = "orderbook_analytics")]
pub enum AnalysisDepth {
    /// Quick analysis (5 minutes)
    Quick,
    /// Standard analysis (1 hour) - default
    Standard,
    /// Deep analysis (24 hours)
    Deep,
}
