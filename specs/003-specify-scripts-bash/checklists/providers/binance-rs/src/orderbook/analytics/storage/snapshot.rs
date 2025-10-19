//! Snapshot capture logic for orderbook data
//!
//! Captures orderbook state at 1-second intervals with MessagePack serialization
//! and automatic 7-day retention cleanup.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc, Duration};
use rocksdb::DB;
use serde::{Deserialize, Serialize};

use super::{Storage, encode_key};

const RETENTION_DAYS: i64 = 7; // 7-day retention from FR-013

/// Orderbook snapshot for time-series storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookSnapshot {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub bids: Vec<(f64, f64)>,  // (price, quantity)
    pub asks: Vec<(f64, f64)>,  // (price, quantity)
    pub update_count: u64,      // Number of updates in this second
}

impl Storage {
    /// Capture and store an orderbook snapshot
    ///
    /// Uses MessagePack serialization for 70% size reduction (from research.md)
    pub fn capture_snapshot(&self, snapshot: &OrderbookSnapshot) -> Result<()> {
        // Check storage limit before writing (from clarifications Q5)
        self.check_size_limit()?;

        // Encode key: symbol + timestamp (14 bytes binary)
        let timestamp = snapshot.timestamp.timestamp();
        let key = encode_key(&snapshot.symbol, timestamp);

        // Serialize with MessagePack
        let value = rmp_serde::to_vec(snapshot)
            .context("Failed to serialize snapshot with MessagePack")?;

        // Write to RocksDB
        self.db().put(&key, &value)
            .context("Failed to write snapshot to RocksDB")?;

        Ok(())
    }

    /// Clean up snapshots older than retention period
    ///
    /// Removes snapshots older than 7 days to prevent unbounded growth
    pub fn cleanup_old_snapshots(&self, symbol: &str) -> Result<usize> {
        let cutoff = Utc::now() - Duration::days(RETENTION_DAYS);
        let cutoff_ts = cutoff.timestamp();

        let mut deleted_count = 0;
        let prefix = encode_key(symbol, 0);

        // Iterate over all snapshots for this symbol
        let iter = self.db().prefix_iterator(&prefix[..6]);
        
        for item in iter {
            let (key, _value) = item?;
            
            // Decode key to check timestamp
            if let Ok((snap_symbol, snap_ts)) = super::decode_key(&key) {
                if snap_symbol == symbol && snap_ts < cutoff_ts {
                    self.db().delete(&key)
                        .context("Failed to delete old snapshot")?;
                    deleted_count += 1;
                }
            }
        }

        Ok(deleted_count)
    }

    /// Get current storage statistics
    pub fn get_snapshot_count(&self, symbol: &str) -> Result<usize> {
        let prefix = encode_key(symbol, 0);
        let iter = self.db().prefix_iterator(&prefix[..6]);
        
        let count = iter
            .filter_map(|item| item.ok())
            .filter(|(key, _)| {
                super::decode_key(key)
                    .map(|(s, _)| s == symbol)
                    .unwrap_or(false)
            })
            .count();
        
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_snapshot_capture() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::new(Some(temp_dir.path())).unwrap();

        let snapshot = OrderbookSnapshot {
            symbol: "BTCUSDT".to_string(),
            timestamp: Utc::now(),
            bids: vec![(100.0, 1.5), (99.5, 2.0)],
            asks: vec![(100.5, 1.0), (101.0, 3.0)],
            update_count: 10,
        };

        storage.capture_snapshot(&snapshot).unwrap();
        
        let count = storage.get_snapshot_count("BTCUSDT").unwrap();
        assert_eq!(count, 1);
    }
}
