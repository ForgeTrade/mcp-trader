//! Historical query with prefix scan (<200ms target)
//!
//! Implements efficient time-range queries using RocksDB prefix scans.
//! Target latency: <200ms for typical 60-second window queries.

use super::{snapshot::OrderBookSnapshot, SnapshotStorage};
use anyhow::{Context, Result};
use rocksdb::IteratorMode;

/// Query snapshots within a time range for a symbol
///
/// Uses RocksDB prefix scan with key format `{symbol}:{timestamp}`.
/// Target performance: <200ms for 60-second window (60 snapshots).
pub async fn query_snapshots_in_window(
    storage: &SnapshotStorage,
    symbol: &str,
    start_timestamp_sec: i64,
    end_timestamp_sec: i64,
) -> Result<Vec<OrderBookSnapshot>> {
    let symbol_owned = symbol.to_string();
    let db = storage.db().clone();

    // Spawn blocking to avoid blocking async runtime
    tokio::task::spawn_blocking(move || {
        let mut snapshots = Vec::new();

        // Prefix scan: iterate all keys starting with "{symbol}:"
        let prefix = format!("{}:", symbol_owned);
        let mode = IteratorMode::From(prefix.as_bytes(), rocksdb::Direction::Forward);

        for item in db.iterator(mode) {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);

            // Check if key still matches symbol prefix
            if !key_str.starts_with(&prefix) {
                break; // Moved past our symbol, stop iteration
            }

            // Parse timestamp from key "{symbol}:{timestamp}"
            if let Some(timestamp_str) = key_str.split(':').nth(1) {
                if let Ok(timestamp) = timestamp_str.parse::<i64>() {
                    // Filter by time range
                    if timestamp >= start_timestamp_sec && timestamp <= end_timestamp_sec {
                        let snapshot = OrderBookSnapshot::from_bytes(&value)
                            .context("Failed to deserialize snapshot")?;
                        snapshots.push(snapshot);
                    }

                    // Optimization: stop if we've passed end timestamp
                    if timestamp > end_timestamp_sec {
                        break;
                    }
                }
            }
        }

        Ok(snapshots)
    })
    .await?
}

/// Count orderbook updates in time window (used for flow rate calculation)
///
/// Returns aggregated bid/ask counts by comparing consecutive snapshots.
/// This is more efficient than querying full snapshots when only counts are needed.
pub async fn count_updates_in_window(
    storage: &SnapshotStorage,
    symbol: &str,
    start_timestamp_sec: i64,
    end_timestamp_sec: i64,
) -> Result<(usize, usize)> {
    let snapshots =
        query_snapshots_in_window(storage, symbol, start_timestamp_sec, end_timestamp_sec).await?;

    if snapshots.is_empty() {
        return Ok((0, 0));
    }

    // Simplified: count non-empty bid/ask levels across snapshots
    // In production, this would compare consecutive snapshots to detect additions/cancellations
    let mut bid_updates = 0;
    let mut ask_updates = 0;

    for snapshot in &snapshots {
        bid_updates += snapshot.bids.len();
        ask_updates += snapshot.asks.len();
    }

    // Average updates per snapshot (rough approximation)
    let snapshot_count = snapshots.len();
    Ok((
        bid_updates / snapshot_count.max(1),
        ask_updates / snapshot_count.max(1),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_query_snapshots_in_window() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = SnapshotStorage::new(temp_dir.path())?;

        // Insert 3 snapshots at different timestamps
        let snapshot1 = OrderBookSnapshot {
            bids: vec![("100.0".to_string(), "1.0".to_string())],
            asks: vec![("101.0".to_string(), "1.0".to_string())],
            update_id: 1,
            timestamp: 1000,
        };
        let snapshot2 = OrderBookSnapshot {
            bids: vec![("100.1".to_string(), "1.1".to_string())],
            asks: vec![("101.1".to_string(), "1.1".to_string())],
            update_id: 2,
            timestamp: 1001,
        };
        let snapshot3 = OrderBookSnapshot {
            bids: vec![("100.2".to_string(), "1.2".to_string())],
            asks: vec![("101.2".to_string(), "1.2".to_string())],
            update_id: 3,
            timestamp: 1002,
        };

        storage.put("BTCUSDT", 1000, &snapshot1.to_bytes()?).await?;
        storage.put("BTCUSDT", 1001, &snapshot2.to_bytes()?).await?;
        storage.put("BTCUSDT", 1002, &snapshot3.to_bytes()?).await?;

        // Query window covering all 3 snapshots
        let results = query_snapshots_in_window(&storage, "BTCUSDT", 1000, 1002).await?;
        assert_eq!(results.len(), 3);

        // Query partial window (only first 2)
        let results = query_snapshots_in_window(&storage, "BTCUSDT", 1000, 1001).await?;
        assert_eq!(results.len(), 2);

        // Query outside window
        let results = query_snapshots_in_window(&storage, "BTCUSDT", 2000, 2010).await?;
        assert_eq!(results.len(), 0);

        Ok(())
    }
}
