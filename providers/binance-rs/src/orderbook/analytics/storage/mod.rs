//! Time-series storage for orderbook snapshots using RocksDB
//!
//! Storage design:
//! - **Key format**: `{symbol}:{unix_timestamp_sec}` (e.g., "BTCUSDT:1737158400")
//! - **Value format**: MessagePack-serialized OrderBookSnapshot
//! - **Retention**: 7 days (background cleanup task deletes keys older than 7 days)
//! - **Compression**: Zstd for ~500MB-1GB storage (12M snapshots for 20 pairs)
//! - **Query pattern**: Prefix scan for time-range queries (<200ms target)

pub mod query;
pub mod snapshot;

use anyhow::{Context, Result};
use rocksdb::{DB, Options, WriteBatch};
use std::path::Path;
use std::sync::Arc;

/// RocksDB storage handle for orderbook snapshots
#[derive(Clone)]
pub struct SnapshotStorage {
    db: Arc<DB>,
}

impl SnapshotStorage {
    /// Initialize RocksDB with optimized settings for time-series workload
    ///
    /// Configuration:
    /// - LSM-tree optimized for write-heavy workload (1 snapshot/sec × 20 pairs)
    /// - Zstd compression for storage efficiency
    /// - Prefix bloom filter for fast time-range scans
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);

        // Write optimization (20 writes/sec is light, but prepare for bursts)
        opts.set_write_buffer_size(64 * 1024 * 1024); // 64MB write buffer
        opts.set_max_write_buffer_number(3);

        // Compression (Zstd for best ratio)
        opts.set_compression_type(rocksdb::DBCompressionType::Zstd);

        // Prefix bloom filter for symbol-based scans
        opts.set_prefix_extractor(rocksdb::SliceTransform::create_fixed_prefix(10));

        let db = DB::open(&opts, path).context("Failed to open RocksDB for snapshot storage")?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Store a snapshot with key format `{symbol}:{unix_timestamp_sec}`
    pub async fn put(&self, symbol: &str, timestamp_sec: i64, value: &[u8]) -> Result<()> {
        let key = format!("{}:{}", symbol, timestamp_sec);
        let db = self.db.clone();
        let value_owned = value.to_vec(); // Convert to owned Vec<u8> for 'static

        // Spawn blocking to avoid blocking async runtime
        tokio::task::spawn_blocking(move || {
            db.put(key.as_bytes(), &value_owned)
                .context("Failed to write snapshot to RocksDB")
        })
        .await??;

        Ok(())
    }

    /// Retrieve a snapshot by exact key
    pub async fn get(&self, symbol: &str, timestamp_sec: i64) -> Result<Option<Vec<u8>>> {
        let key = format!("{}:{}", symbol, timestamp_sec);
        let db = self.db.clone();

        tokio::task::spawn_blocking(move || {
            db.get(key.as_bytes())
                .context("Failed to read snapshot from RocksDB")
        })
        .await?
    }

    /// Delete keys older than retention period (7 days)
    ///
    /// Called by background cleanup task (hourly).
    /// Expected cleanup: ~1.7M keys/day (86,400 snapshots/day/pair × 20 pairs)
    pub async fn cleanup_old_snapshots(&self, retention_secs: i64) -> Result<usize> {
        let cutoff_timestamp = chrono::Utc::now().timestamp() - retention_secs;
        let db = self.db.clone();

        tokio::task::spawn_blocking(move || {
            let mut batch = WriteBatch::default();
            let mut deleted_count = 0;

            // Iterate all keys (no prefix filter - global cleanup)
            let iter = db.iterator(rocksdb::IteratorMode::Start);

            for item in iter {
                let (key, _) = item?;
                let key_str = String::from_utf8_lossy(&key);

                // Parse timestamp from key format "{symbol}:{timestamp}"
                if let Some(timestamp_str) = key_str.split(':').nth(1) {
                    if let Ok(timestamp) = timestamp_str.parse::<i64>() {
                        if timestamp < cutoff_timestamp {
                            batch.delete(&key);
                            deleted_count += 1;
                        }
                    }
                }
            }

            if deleted_count > 0 {
                db.write(batch).context("Failed to delete old snapshots")?;
            }

            Ok(deleted_count)
        })
        .await?
    }

    /// Get database handle for advanced queries (prefix scans)
    pub(crate) fn db(&self) -> &Arc<DB> {
        &self.db
    }
}

/// Spawn background task for periodic snapshot persistence
///
/// Captures orderbook snapshots every 1 second and persists to RocksDB.
/// Gracefully shuts down when shutdown signal is received.
///
/// # Arguments
/// * `storage` - RocksDB snapshot storage handle
/// * `manager` - OrderBook manager for accessing current orderbook state
/// * `symbols` - List of symbols to persist (e.g., ["BTCUSDT", "ETHUSDT"])
/// * `shutdown_rx` - Broadcast receiver for shutdown signal
///
/// # Returns
/// JoinHandle for the spawned task (allows awaiting on shutdown)
pub fn spawn_snapshot_persistence_task(
    storage: Arc<SnapshotStorage>,
    manager: Arc<crate::orderbook::OrderBookManager>,
    symbols: &[&str],
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> tokio::task::JoinHandle<()> {
    let symbols_owned: Vec<String> = symbols.iter().map(|s| s.to_string()).collect();

    tokio::spawn(async move {
        use snapshot::OrderBookSnapshot;

        // T008: 1-second interval loop
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        tracing::info!("Snapshot persistence task started for {} symbols", symbols_owned.len());

        loop {
            tokio::select! {
                // T009: Graceful shutdown handling
                _ = shutdown_rx.recv() => {
                    tracing::info!("Snapshot persistence task shutting down");
                    break;
                }

                // T008: 1-second tick
                _ = interval.tick() => {
                    for symbol in &symbols_owned {
                        // T010: Capture snapshot from OrderBookManager
                        let orderbook = match manager.get_order_book(symbol).await {
                            Ok(ob) => ob,
                            Err(e) => {
                                // T014: ERROR-level logging for failures
                                tracing::error!(
                                    symbol = %symbol,
                                    error = %e,
                                    "Failed to get orderbook for snapshot"
                                );
                                continue;
                            }
                        };

                        // Skip empty orderbooks
                        if orderbook.bids.is_empty() && orderbook.asks.is_empty() {
                            tracing::warn!(
                                symbol = %symbol,
                                "Skipping snapshot: empty orderbook"
                            );
                            continue;
                        }

                        // T011: Serialize to MessagePack
                        let snapshot = OrderBookSnapshot::from_orderbook(&orderbook);

                        // T021: DEBUG-level logging for snapshot capture details
                        tracing::debug!(
                            symbol = %symbol,
                            timestamp = %snapshot.timestamp,
                            update_id = %snapshot.update_id,
                            bid_levels = %snapshot.bids.len(),
                            ask_levels = %snapshot.asks.len(),
                            "Captured orderbook snapshot"
                        );

                        let bytes = match snapshot.to_bytes() {
                            Ok(b) => b,
                            Err(e) => {
                                // T014: ERROR-level logging for serialization failures
                                tracing::error!(
                                    symbol = %symbol,
                                    timestamp = %snapshot.timestamp,
                                    error = %e,
                                    "Failed to serialize snapshot to MessagePack"
                                );
                                continue;
                            }
                        };

                        // T012: Store in RocksDB
                        if let Err(e) = storage.put(symbol, snapshot.timestamp, &bytes).await {
                            // T014: ERROR-level logging for storage failures
                            tracing::error!(
                                symbol = %symbol,
                                timestamp = %snapshot.timestamp,
                                error = %e,
                                "Failed to persist snapshot to RocksDB"
                            );
                        } else {
                            // T013: INFO-level logging for successful persistence
                            tracing::info!(
                                symbol = %symbol,
                                timestamp = %snapshot.timestamp,
                                "Stored snapshot"
                            );
                        }
                    }
                }
            }
        }

        tracing::info!("Snapshot persistence task stopped");
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_put_and_get() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = SnapshotStorage::new(temp_dir.path())?;

        let test_data = b"test_snapshot_data";
        storage.put("BTCUSDT", 1737158400, test_data).await?;

        let retrieved = storage.get("BTCUSDT", 1737158400).await?;
        assert_eq!(retrieved.as_deref(), Some(test_data.as_ref()));

        Ok(())
    }

    #[tokio::test]
    async fn test_cleanup_old_snapshots() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = SnapshotStorage::new(temp_dir.path())?;

        // Insert old snapshot (8 days ago)
        let old_timestamp = chrono::Utc::now().timestamp() - (8 * 24 * 3600);
        storage.put("BTCUSDT", old_timestamp, b"old_data").await?;

        // Insert recent snapshot (1 day ago)
        let recent_timestamp = chrono::Utc::now().timestamp() - (1 * 24 * 3600);
        storage
            .put("ETHUSDT", recent_timestamp, b"recent_data")
            .await?;

        // Cleanup with 7-day retention
        let deleted = storage.cleanup_old_snapshots(7 * 24 * 3600).await?;
        assert_eq!(deleted, 1); // Only old snapshot deleted

        // Verify old snapshot removed, recent remains
        assert!(storage.get("BTCUSDT", old_timestamp).await?.is_none());
        assert!(storage.get("ETHUSDT", recent_timestamp).await?.is_some());

        Ok(())
    }
}
