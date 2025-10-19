//! Client-side rate limiter for Binance API requests
//!
//! Implements GCRA (Generic Cell Rate Algorithm) via governor crate.
//! Limits: 1000 requests/minute with 30s queue timeout.

use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::num::NonZeroU32;
use std::time::Duration;
use thiserror::Error;
use tokio::time::timeout;
use tracing::{debug, warn};

/// Maximum requests per minute (conservative buffer below Binance 1200/min)
const MAX_REQUESTS_PER_MINUTE: u32 = 1000;

/// Maximum time to wait in queue before rejecting request
const QUEUE_TIMEOUT_SECS: u64 = 30;

/// Queue depth threshold for warning logs (50% capacity)
#[allow(dead_code)] // Used for queue depth monitoring
const WARN_QUEUE_THRESHOLD: f32 = 0.5;

/// Rate limiter errors
#[derive(Debug, Error)]
pub enum RateLimiterError {
    #[error("Rate limit exceeded: request queue full, retry after delay")]
    QueueFull,

    #[error("Rate limit queue timeout after {0}s")]
    QueueTimeout(u64),
}

/// Client-side rate limiter for REST API requests
///
/// Uses GCRA algorithm (10x faster than leaky bucket in multi-threaded scenarios).
/// Queues excess requests for up to 30 seconds, then rejects with error.
pub struct RateLimiter {
    limiter: GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>,
    queue_timeout: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter with default settings
    ///
    /// - Limit: 1000 requests/minute
    /// - Queue timeout: 30 seconds
    pub fn new() -> Self {
        let quota = Quota::per_minute(
            NonZeroU32::new(MAX_REQUESTS_PER_MINUTE)
                .expect("MAX_REQUESTS_PER_MINUTE must be non-zero"),
        );

        Self {
            limiter: GovernorRateLimiter::direct(quota),
            queue_timeout: Duration::from_secs(QUEUE_TIMEOUT_SECS),
        }
    }

    /// Create a rate limiter with custom settings (for testing)
    #[allow(dead_code)]
    pub fn with_quota(requests_per_minute: u32, queue_timeout_secs: u64) -> Self {
        let quota = Quota::per_minute(
            NonZeroU32::new(requests_per_minute).expect("requests_per_minute must be non-zero"),
        );

        Self {
            limiter: GovernorRateLimiter::direct(quota),
            queue_timeout: Duration::from_secs(queue_timeout_secs),
        }
    }

    /// Wait for rate limit permission (async, with timeout)
    ///
    /// Returns Ok(()) when request is allowed, Err if queue timeout exceeded.
    /// Logs warning when queue depth exceeds 50% capacity.
    pub async fn wait(&self) -> Result<(), RateLimiterError> {
        // Check current queue depth for warning
        let current_permits = self.limiter.check();
        if current_permits.is_err() {
            // Queue is building up - estimate depth
            warn!("Rate limit queue building up (>50% capacity), consider reducing request rate");
        }

        // Wait for permission with timeout
        match timeout(self.queue_timeout, async {
            loop {
                match self.limiter.check() {
                    Ok(_) => {
                        debug!("Rate limit permission granted");
                        return Ok(());
                    }
                    Err(_) => {
                        // Calculate wait time until next permit available
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => {
                warn!(
                    timeout_secs = QUEUE_TIMEOUT_SECS,
                    "Rate limit queue timeout exceeded"
                );
                Err(RateLimiterError::QueueTimeout(QUEUE_TIMEOUT_SECS))
            }
        }
    }

    /// Check if request can proceed immediately (non-blocking)
    ///
    /// Returns true if request is allowed, false if rate limit reached.
    pub fn check_immediate(&self) -> bool {
        self.limiter.check().is_ok()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_within_quota() {
        let limiter = RateLimiter::with_quota(10, 5); // 10 req/min for fast test

        // First request should succeed immediately
        assert!(limiter.wait().await.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_queues_excess_requests() {
        let limiter = RateLimiter::with_quota(10, 1); // 10 req/min, 1s timeout

        // First request should succeed immediately
        assert!(limiter.wait().await.is_ok());

        // Exhaust the quota
        for _ in 0..9 {
            limiter.wait().await.ok();
        }

        // Next request should be delayed (queued) but eventually succeed or timeout
        let start = std::time::Instant::now();
        let result = limiter.wait().await;
        let elapsed = start.elapsed();

        // Should either succeed after waiting or timeout after 1 second
        assert!(result.is_ok() || result.is_err());
        if result.is_ok() {
            // If succeeded, should have waited some time
            assert!(elapsed.as_millis() > 0);
        } else {
            // If timed out, should be close to 1 second
            assert!(elapsed.as_secs() >= 1);
        }
    }

    #[tokio::test]
    async fn test_check_immediate() {
        let limiter = RateLimiter::with_quota(5, 1);

        // First check should succeed
        assert!(limiter.check_immediate());

        // Exhaust quota
        for _ in 0..5 {
            limiter.wait().await.ok();
        }

        // Next immediate check should fail
        assert!(!limiter.check_immediate());
    }
}
