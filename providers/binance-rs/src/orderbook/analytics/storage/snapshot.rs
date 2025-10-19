//! Snapshot capture logic - 1-second interval orderbook snapshots
//!
//! Captures top 20 bid/ask levels every second for time-series analysis.
//! Snapshots are serialized with MessagePack and stored in RocksDB.

use super::SnapshotStorage;
use crate::orderbook::types::OrderBook;
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Simplified orderbook snapshot for storage (top 20 levels per side)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    /// Top 20 bid levels (price, quantity)
    pub bids: Vec<(String, String)>, // Decimal as strings for MessagePack
    /// Top 20 ask levels (price, quantity)
    pub asks: Vec<(String, String)>,
    /// Binance update ID for ordering
    pub update_id: u64,
    /// Capture timestamp (Unix seconds)
    pub timestamp: i64,
}

impl OrderBookSnapshot {
    /// Create snapshot from full OrderBook (take top 20 levels)
    pub fn from_orderbook(orderbook: &OrderBook) -> Self {
        let timestamp = Utc::now().timestamp();

        // Convert top 20 levels to string tuples for MessagePack
        let bids: Vec<(String, String)> = orderbook
            .bids
            .iter()
            .take(20)
            .map(|(price, qty)| (price.to_string(), qty.to_string()))
            .collect();

        let asks: Vec<(String, String)> = orderbook
            .asks
            .iter()
            .take(20)
            .map(|(price, qty)| (price.to_string(), qty.to_string()))
            .collect();

        Self {
            bids,
            asks,
            update_id: orderbook.last_update_id as u64,
            timestamp,
        }
    }

    /// Serialize to MessagePack bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        rmp_serde::to_vec(self).context("Failed to serialize snapshot to MessagePack")
    }

    /// Deserialize from MessagePack bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        rmp_serde::from_slice(data).context("Failed to deserialize snapshot from MessagePack")
    }
}

/// Snapshot capture task - runs every 1 second per symbol
///
/// Should be spawned as background tokio task for each monitored symbol.
pub async fn capture_snapshot_task(
    storage: SnapshotStorage,
    symbol: String,
    mut orderbook_rx: tokio::sync::watch::Receiver<OrderBook>,
) -> Result<()> {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

    loop {
        interval.tick().await;

        // Get latest orderbook from watch channel
        let orderbook = orderbook_rx.borrow_and_update().clone();

        // Skip if orderbook is stale (no recent updates)
        if orderbook.bids.is_empty() && orderbook.asks.is_empty() {
            tracing::warn!("Skipping snapshot for {}: empty orderbook", symbol);
            continue;
        }

        // Create and store snapshot
        let snapshot = OrderBookSnapshot::from_orderbook(&orderbook);
        let timestamp_sec = snapshot.timestamp;
        let bytes = snapshot.to_bytes()?;

        storage
            .put(&symbol, timestamp_sec, &bytes)
            .await
            .context("Failed to store snapshot")?;

        tracing::debug!(
            "Captured snapshot for {} at timestamp {}",
            symbol,
            timestamp_sec
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_serialization() -> Result<()> {
        let snapshot = OrderBookSnapshot {
            bids: vec![("100.50".to_string(), "1.5".to_string())],
            asks: vec![("100.60".to_string(), "2.0".to_string())],
            update_id: 12345,
            timestamp: 1737158400,
        };

        let bytes = snapshot.to_bytes()?;
        let deserialized = OrderBookSnapshot::from_bytes(&bytes)?;

        assert_eq!(snapshot.update_id, deserialized.update_id);
        assert_eq!(snapshot.timestamp, deserialized.timestamp);
        assert_eq!(snapshot.bids, deserialized.bids);

        Ok(())
    }
}
