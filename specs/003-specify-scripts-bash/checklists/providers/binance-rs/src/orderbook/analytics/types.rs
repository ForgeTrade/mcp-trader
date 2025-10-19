//! Analytics type definitions
//!
//! Defines enums and data structures for order flow analysis, anomaly detection,
//! and liquidity mapping.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Flow direction indicator based on bid/ask ratio thresholds (FR-003)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FlowDirection {
    /// Bid flow >2x ask flow
    StrongBuy,
    /// Bid/ask ratio 1.2-2x
    ModerateBuy,
    /// Bid/ask ratio 0.8-1.2
    Neutral,
    /// Bid/ask ratio 0.5-0.8
    ModerateSell,
    /// Bid/ask ratio <0.5
    StrongSell,
}

/// Severity levels for anomalies and liquidity vacuums
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Expected impact level for liquidity vacuums
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImpactLevel {
    /// Expect rapid price discovery
    FastMovement,
    /// Moderate speed movement
    ModerateMovement,
    /// Minimal impact
    Negligible,
}

/// Direction of absorption events (accumulation vs distribution)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Direction {
    /// Buying (accumulation)
    Accumulation,
    /// Selling (distribution)
    Distribution,
}

/// Order flow snapshot capturing bid/ask pressure over a time window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFlowSnapshot {
    pub symbol: String,
    pub time_window_start: DateTime<Utc>,
    pub time_window_end: DateTime<Utc>,
    pub window_duration_secs: u32,
    pub bid_flow_rate: f64,
    pub ask_flow_rate: f64,
    pub net_flow: f64,
    pub flow_direction: FlowDirection,
    pub cumulative_delta: f64,
}

impl OrderFlowSnapshot {
    /// Validate that window_duration matches the time range
    pub fn validate(&self) -> Result<(), String> {
        let actual_duration = (self.time_window_end - self.time_window_start)
            .num_seconds()
            .abs() as u32;
        
        if (actual_duration as i32 - self.window_duration_secs as i32).abs() > 1 {
            return Err(format!(
                "Window duration mismatch: expected {}, got {}",
                self.window_duration_secs, actual_duration
            ));
        }
        
        if self.bid_flow_rate < 0.0 || self.ask_flow_rate < 0.0 {
            return Err("Flow rates must be non-negative".to_string());
        }
        
        Ok(())
    }
}
