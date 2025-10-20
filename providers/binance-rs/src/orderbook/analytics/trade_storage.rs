// Trade persistence storage layer for Feature 008
// Handles RocksDB operations for storing and querying aggregate trade batches

use anyhow::{Context, Result};
use rocksdb::{DB, WriteBatch};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// RocksDB key prefix for trade batches
const TRADES_KEY_PREFIX: &str = "trades:";

/// Simplified aggregate trade for persistence (minimal fields needed for analytics)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggTrade {
    #[serde(rename = "p")]
    pub price: String,

    #[serde(rename = "q")]
    pub quantity: String,

    #[serde(rename = "T")]
    pub timestamp: i64,

    #[serde(rename = "a")]
    pub trade_id: i64,

    #[serde(rename = "m")]
    pub buyer_is_maker: bool,
}

impl From<&super::trade_stream::AggTrade> for AggTrade {
    fn from(trade: &super::trade_stream::AggTrade) -> Self {
        Self {
            price: trade.price.clone(),
            quantity: trade.quantity.clone(),
            timestamp: trade.trade_time,
            trade_id: trade.agg_trade_id as i64,
            buyer_is_maker: trade.is_buyer_maker,
        }
    }
}

/// Trade persistence storage
pub struct TradeStorage {
    db: Arc<DB>,
}

impl TradeStorage {
    pub fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    /// Store a batch of trades for a symbol at a specific timestamp
    ///
    /// Key format: `trades:{symbol}:{batch_timestamp_ms}`
    /// Value: MessagePack-serialized Vec<AggTrade>
    pub fn store_batch(
        &self,
        symbol: &str,
        batch_timestamp: i64,
        trades: Vec<AggTrade>,
    ) -> Result<()> {
        if trades.is_empty() {
            return Ok(());
        }

        let key = format!("{}{}:{}", TRADES_KEY_PREFIX, symbol, batch_timestamp);
        let value = rmp_serde::to_vec(&trades)
            .context("Failed to serialize trade batch with MessagePack")?;

        self.db.put(key.as_bytes(), value)
            .context("Failed to write trade batch to RocksDB")?;

        Ok(())
    }

    /// Query trades for a symbol within a time range
    ///
    /// Returns all trades where start_time <= timestamp <= end_time
    pub fn query_trades(
        &self,
        symbol: &str,
        start_time: i64,
        end_time: i64,
    ) -> Result<Vec<AggTrade>> {
        // Validate query parameters
        if end_time < start_time {
            anyhow::bail!("end_time must be >= start_time");
        }

        let window_hours = (end_time - start_time) / (3600 * 1000);
        if window_hours > 168 {
            anyhow::bail!("Query window exceeds maximum 7 days (168 hours)");
        }

        let prefix = format!("{}{}:", TRADES_KEY_PREFIX, symbol);
        tracing::info!(
            "Querying trades: symbol={} prefix='{}' start_time={} end_time={} window_hours={}",
            symbol, prefix, start_time, end_time, window_hours
        );
        let mut all_trades = Vec::new();

        let iter = self.db.prefix_iterator(prefix.as_bytes());
        let mut key_count = 0;

        for item in iter {
            let (key, value) = item.context("Failed to read from RocksDB iterator")?;
            key_count += 1;

            // Parse timestamp from key (format: "trades:SYMBOL:TIMESTAMP")
            if let Some(timestamp) = parse_timestamp_from_key(&key) {
                // Filter by time range
                if timestamp >= start_time && timestamp <= end_time {
                    // Deserialize batch
                    let batch: Vec<AggTrade> = rmp_serde::from_slice(&value)
                        .context("Failed to deserialize trade batch from MessagePack")?;
                    all_trades.extend(batch);
                }

                // Early termination if beyond end_time (keys are ordered chronologically)
                if timestamp > end_time {
                    break;
                }
            }
        }

        tracing::info!(
            "Query complete: found {} trades from {} keys for symbol={}",
            all_trades.len(), key_count, symbol
        );
        Ok(all_trades)
    }

    /// Delete trades older than the retention period (7 days)
    ///
    /// Should be called periodically (e.g., hourly) as a background cleanup task
    pub fn cleanup_old_trades(&self, cutoff_timestamp: i64) -> Result<usize> {
        let mut batch = WriteBatch::default();
        let mut deleted_count = 0;

        let iter = self.db.iterator(rocksdb::IteratorMode::Start);

        for item in iter {
            let (key, _value) = item.context("Failed to read from RocksDB iterator")?;

            // Only process trade keys
            if key.starts_with(TRADES_KEY_PREFIX.as_bytes()) {
                if let Some(timestamp) = parse_timestamp_from_key(&key) {
                    if timestamp < cutoff_timestamp {
                        batch.delete(&key);
                        deleted_count += 1;
                    }
                }
            }
        }

        if deleted_count > 0 {
            self.db.write(batch)
                .context("Failed to execute batch delete of old trades")?;
        }

        Ok(deleted_count)
    }
}

/// Parse timestamp from RocksDB key
///
/// Key format: `trades:{symbol}:{timestamp}`
/// Example: `trades:BTCUSDT:1760903627000` → Some(1760903627000)
fn parse_timestamp_from_key(key: &[u8]) -> Option<i64> {
    let key_str = std::str::from_utf8(key).ok()?;
    let parts: Vec<&str> = key_str.split(':').collect();

    if parts.len() == 3 && parts[0] == "trades" {
        parts[2].parse::<i64>().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parse_timestamp_from_key() {
        let key = b"trades:BTCUSDT:1760903627000";
        assert_eq!(parse_timestamp_from_key(key), Some(1760903627000));

        let invalid_key = b"invalid:key:format";
        assert_eq!(parse_timestamp_from_key(invalid_key), None);
    }

    #[test]
    fn test_store_and_query_trades() {
        let temp_dir = tempdir().unwrap();
        let db = Arc::new(DB::open_default(temp_dir.path()).unwrap());
        let storage = TradeStorage::new(db);

        // Create 100 test trades across 2 batches
        let mut trades_batch1 = Vec::new();
        let mut trades_batch2 = Vec::new();

        let base_timestamp = 1760903627000;

        for i in 0..50 {
            trades_batch1.push(AggTrade {
                price: format!("43250.{}", i),
                quantity: format!("0.{}", i + 10),
                timestamp: base_timestamp + i,
                trade_id: i,
                buyer_is_maker: i % 2 == 0,
            });
        }

        for i in 50..100 {
            trades_batch2.push(AggTrade {
                price: format!("43251.{}", i),
                quantity: format!("0.{}", i + 10),
                timestamp: base_timestamp + 1000 + i,
                trade_id: i,
                buyer_is_maker: i % 2 == 0,
            });
        }

        // Store batches
        storage.store_batch("BTCUSDT", base_timestamp, trades_batch1).unwrap();
        storage.store_batch("BTCUSDT", base_timestamp + 1000, trades_batch2).unwrap();

        // Query 1-hour window (should return all 100 trades)
        let queried = storage.query_trades(
            "BTCUSDT",
            base_timestamp,
            base_timestamp + 3600 * 1000,
        ).unwrap();

        assert_eq!(queried.len(), 100);
        assert_eq!(queried[0].price, "43250.0");
        assert_eq!(queried[99].price, "43251.99");
    }
}
