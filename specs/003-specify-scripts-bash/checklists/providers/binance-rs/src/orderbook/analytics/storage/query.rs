//! Historical query operations with prefix scans
//!
//! Provides efficient time-range queries using RocksDB prefix scans,
//! targeting <200ms latency for 60-300 second windows.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::time::Duration;

use super::{Storage, encode_key, snapshot::OrderbookSnapshot};

impl Storage {
    /// Query snapshots within a time range
    ///
    /// Uses prefix scan for efficient retrieval. Target: <200ms for 60-300s windows
    /// (from FR-013, research.md)
    pub async fn query_snapshots(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        timeout: Option<Duration>,
    ) -> Result<Vec<OrderbookSnapshot>> {
        let timeout = timeout.unwrap_or(Duration::from_millis(200));
        
        // Use tokio::task::spawn_blocking for potentially slow RocksDB operation
        let db = self.db.clone();
        let symbol = symbol.to_string();
        let start_ts = start.timestamp();
        let end_ts = end.timestamp();

        let result = tokio::time::timeout(timeout, tokio::task::spawn_blocking(move || {
            Self::query_snapshots_blocking(&db, &symbol, start_ts, end_ts)
        }))
        .await
        .context("Query timeout exceeded (>200ms)")?
        .context("Query task failed")??;

        Ok(result)
    }

    /// Blocking version of snapshot query for spawn_blocking
    fn query_snapshots_blocking(
        db: &rocksdb::DB,
        symbol: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<OrderbookSnapshot>> {
        let mut snapshots = Vec::new();
        
        // Create prefix from symbol (6 bytes)
        let prefix = encode_key(symbol, 0);
        let iter = db.prefix_iterator(&prefix[..6]);

        for item in iter {
            let (key, value) = item?;
            
            // Decode key to check if it's in our time range
            if let Ok((snap_symbol, snap_ts)) = super::decode_key(&key) {
                if snap_symbol != symbol {
                    continue; // Different symbol, skip
                }

                if snap_ts < start_ts {
                    continue; // Before range, skip
                }

                if snap_ts > end_ts {
                    break; // Past range, we're done (keys are ordered)
                }

                // Deserialize MessagePack data
                let snapshot: OrderbookSnapshot = rmp_serde::from_slice(&value)
                    .context("Failed to deserialize snapshot")?;

                snapshots.push(snapshot);
            }
        }

        Ok(snapshots)
    }

    /// Count snapshots in a time range (faster than full query)
    pub async fn count_snapshots(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<usize> {
        let db = self.db.clone();
        let symbol = symbol.to_string();
        let start_ts = start.timestamp();
        let end_ts = end.timestamp();

        let count = tokio::task::spawn_blocking(move || {
            let prefix = encode_key(&symbol, 0);
            let iter = db.prefix_iterator(&prefix[..6]);

            let count = iter
                .filter_map(|item| item.ok())
                .filter(|(key, _)| {
                    if let Ok((snap_symbol, snap_ts)) = super::decode_key(key) {
                        snap_symbol == symbol && snap_ts >= start_ts && snap_ts <= end_ts
                    } else {
                        false
                    }
                })
                .count();

            count
        })
        .await?;

        Ok(count)
    }

    /// Get the latest snapshot for a symbol
    pub async fn get_latest_snapshot(&self, symbol: &str) -> Result<Option<OrderbookSnapshot>> {
        let db = self.db.clone();
        let symbol = symbol.to_string();

        let snapshot = tokio::task::spawn_blocking(move || {
            let prefix = encode_key(&symbol, 0);
            let mut iter = db.prefix_iterator(&prefix[..6]);

            // RocksDB iteration is in ascending order, so we need to find the last one
            let mut latest: Option<(Vec<u8>, Vec<u8>)> = None;

            while let Some(Ok((key, value))) = iter.next() {
                if let Ok((snap_symbol, _)) = super::decode_key(&key) {
                    if snap_symbol == symbol {
                        latest = Some((key.to_vec(), value.to_vec()));
                    } else {
                        break; // Moved to different symbol prefix
                    }
                }
            }

            if let Some((_key, value)) = latest {
                let snapshot: OrderbookSnapshot = rmp_serde::from_slice(&value)?;
                Ok(Some(snapshot))
            } else {
                Ok(None)
            }
        })
        .await??;

        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::snapshot::OrderbookSnapshot;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_query_snapshots() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::new(Some(temp_dir.path())).unwrap();

        // Insert 3 snapshots
        let now = Utc::now();
        for i in 0..3 {
            let snapshot = OrderbookSnapshot {
                symbol: "BTCUSDT".to_string(),
                timestamp: now + chrono::Duration::seconds(i),
                bids: vec![(100.0 + i as f64, 1.0)],
                asks: vec![(101.0 + i as f64, 1.0)],
                update_count: 1,
            };
            storage.capture_snapshot(&snapshot).unwrap();
        }

        // Query all 3
        let snapshots = storage
            .query_snapshots("BTCUSDT", now, now + chrono::Duration::seconds(5), None)
            .await
            .unwrap();

        assert_eq!(snapshots.len(), 3);
    }

    #[tokio::test]
    async fn test_count_snapshots() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::new(Some(temp_dir.path())).unwrap();

        let now = Utc::now();
        for i in 0..5 {
            let snapshot = OrderbookSnapshot {
                symbol: "ETHUSDT".to_string(),
                timestamp: now + chrono::Duration::seconds(i),
                bids: vec![],
                asks: vec![],
                update_count: 1,
            };
            storage.capture_snapshot(&snapshot).unwrap();
        }

        let count = storage
            .count_snapshots("ETHUSDT", now, now + chrono::Duration::seconds(10))
            .await
            .unwrap();

        assert_eq!(count, 5);
    }
}
