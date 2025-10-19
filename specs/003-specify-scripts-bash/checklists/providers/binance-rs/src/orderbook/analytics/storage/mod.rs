//! Time-series storage for orderbook snapshots using RocksDB
//!
//! Provides 1-second interval snapshot storage with 7-day retention,
//! binary key encoding, and Zstd compression.

use anyhow::{Context, Result};
use rocksdb::{DB, Options, SliceTransform, DBCompressionType};
use std::path::Path;
use std::sync::Arc;

pub mod snapshot;
pub mod query;

const STORAGE_PATH: &str = "./data/orderbook_snapshots";
const STORAGE_LIMIT_GB: u64 = 1; // 1GB hard limit from clarifications

/// RocksDB-backed time-series storage for orderbook snapshots
pub struct Storage {
    db: Arc<DB>,
}

impl Storage {
    /// Initialize RocksDB storage with optimized configuration
    pub fn new<P: AsRef<Path>>(path: Option<P>) -> Result<Self> {
        let path = path
            .map(|p| p.as_ref().to_path_buf())
            .unwrap_or_else(|| Path::new(STORAGE_PATH).to_path_buf());

        // Create data directory if it doesn't exist
        std::fs::create_dir_all(&path)
            .context("Failed to create storage directory")?;

        let mut opts = Options::default();
        opts.create_if_missing(true);
        
        // Zstd compression (from research.md decision #1)
        opts.set_compression_type(DBCompressionType::Zstd);
        
        // Write buffer configuration (from research.md)
        opts.set_write_buffer_size(64 * 1024 * 1024); // 64MB
        opts.set_max_write_buffer_number(3);
        opts.set_target_file_size_base(64 * 1024 * 1024);
        
        // Prefix extractor for symbol-based queries (6-byte symbol prefix)
        opts.set_prefix_extractor(SliceTransform::create_fixed_prefix(6));

        let db = DB::open(&opts, path)
            .context("Failed to open RocksDB")?;

        Ok(Self {
            db: Arc::new(db),
        })
    }

    /// Get reference to underlying DB
    pub fn db(&self) -> &DB {
        &self.db
    }

    /// Check if storage size exceeds 1GB limit (from clarifications)
    pub fn check_size_limit(&self) -> Result<()> {
        let path = self.db.path();
        let size = Self::dir_size(path)?;
        let limit = STORAGE_LIMIT_GB * 1024 * 1024 * 1024;

        if size > limit {
            anyhow::bail!(
                "storage_limit_exceeded: Storage size ({} bytes) exceeds 1GB hard limit. \
                Oldest data must be purged.",
                size
            );
        }

        Ok(())
    }

    /// Calculate directory size recursively
    fn dir_size(path: &Path) -> Result<u64> {
        let mut total = 0;
        
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            
            if metadata.is_dir() {
                total += Self::dir_size(&entry.path())?;
            } else {
                total += metadata.len();
            }
        }
        
        Ok(total)
    }
}

/// Encode binary key: 6-byte symbol + 8-byte big-endian unix timestamp
///
/// Format from research.md decision #1:
/// - Symbol: 6-byte fixed-width ASCII (left-padded with spaces)
/// - Timestamp: 8-byte big-endian u64 (unix seconds)
/// - Total: 14 bytes
pub fn encode_key(symbol: &str, timestamp: i64) -> [u8; 14] {
    let mut key = [0u8; 14];
    
    // Symbol: left-pad with spaces to 6 bytes
    let symbol_bytes = symbol.as_bytes();
    let len = symbol_bytes.len().min(6);
    key[6 - len..6].copy_from_slice(&symbol_bytes[..len]);
    
    // Timestamp: big-endian for lexicographic ordering
    key[6..14].copy_from_slice(&(timestamp as u64).to_be_bytes());
    
    key
}

/// Decode binary key to (symbol, timestamp)
pub fn decode_key(key: &[u8]) -> Result<(String, i64)> {
    if key.len() != 14 {
        anyhow::bail!("Invalid key length: expected 14, got {}", key.len());
    }
    
    // Extract symbol (trim trailing spaces)
    let symbol = String::from_utf8_lossy(&key[..6])
        .trim_end()
        .to_string();
    
    // Extract timestamp (big-endian u64)
    let timestamp_bytes: [u8; 8] = key[6..14].try_into()?;
    let timestamp = u64::from_be_bytes(timestamp_bytes) as i64;
    
    Ok((symbol, timestamp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_encoding() {
        let key = encode_key("BTCUSDT", 1729350000);
        assert_eq!(key.len(), 14);
        
        let (symbol, timestamp) = decode_key(&key).unwrap();
        assert_eq!(symbol, "BTCUSDT");
        assert_eq!(timestamp, 1729350000);
    }

    #[test]
    fn test_key_encoding_short_symbol() {
        let key = encode_key("BTC", 1729350000);
        let (symbol, timestamp) = decode_key(&key).unwrap();
        assert_eq!(symbol, "BTC");
        assert_eq!(timestamp, 1729350000);
    }
}
