// Unit tests for report caching functionality

use binance_provider::report::{MarketReport, ReportCache};
use std::thread;
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_report(symbol: &str) -> MarketReport {
        MarketReport {
            markdown_content: format!("# Report for {}", symbol),
            symbol: symbol.to_string(),
            generated_at: 1729780000000,
            data_age_ms: 100,
            failed_sections: vec![],
            generation_time_ms: 245,
        }
    }

    #[test]
    fn test_cache_set_and_get() {
        let cache = ReportCache::new(60);
        let report = create_test_report("BTCUSDT");
        let cache_key = "BTCUSDT:sections:all;volume:24;levels:20".to_string();

        // Set report in cache
        cache.set(cache_key.clone(), report.clone());

        // Get report from cache
        let cached = cache.get(&cache_key).expect("Cache should contain report");
        assert_eq!(cached.symbol, "BTCUSDT");
        assert_eq!(cached.generation_time_ms, 245);
        assert!(cached.markdown_content.contains("BTCUSDT"));
    }

    #[test]
    fn test_cache_ttl_expiration() {
        // Create cache with 1-second TTL
        let cache = ReportCache::new(1);
        let report = create_test_report("ETHUSDT");
        let cache_key = "ETHUSDT:sections:all;volume:24;levels:20".to_string();

        // Set report
        cache.set(cache_key.clone(), report);

        // Should be available immediately
        assert!(
            cache.get(&cache_key).is_some(),
            "Cache should have report immediately"
        );

        // Wait for TTL to expire
        thread::sleep(Duration::from_millis(1100));

        // Should be None after expiration
        assert!(
            cache.get(&cache_key).is_none(),
            "Cache should expire after TTL"
        );
    }

    #[test]
    fn test_cache_invalidate() {
        let cache = ReportCache::new(60);

        // Add multiple reports for same symbol with different options
        let key1 = "BTCUSDT:sections:all;volume:24;levels:20".to_string();
        let key2 = "BTCUSDT:sections:price_overview;volume:48;levels:50".to_string();
        let key3 = "ETHUSDT:sections:all;volume:24;levels:20".to_string();

        cache.set(key1.clone(), create_test_report("BTCUSDT"));
        cache.set(key2.clone(), create_test_report("BTCUSDT"));
        cache.set(key3.clone(), create_test_report("ETHUSDT"));

        // Verify all are cached
        assert!(cache.get(&key1).is_some());
        assert!(cache.get(&key2).is_some());
        assert!(cache.get(&key3).is_some());

        // Invalidate BTCUSDT (all variants)
        cache.invalidate("BTCUSDT");

        // BTCUSDT reports should be gone
        assert!(
            cache.get(&key1).is_none(),
            "BTCUSDT key1 should be invalidated"
        );
        assert!(
            cache.get(&key2).is_none(),
            "BTCUSDT key2 should be invalidated"
        );

        // ETHUSDT should still be cached
        assert!(cache.get(&key3).is_some(), "ETHUSDT should remain cached");
    }

    #[test]
    fn test_cache_miss() {
        let cache = ReportCache::new(60);

        // Get non-existent key
        let result = cache.get("NONEXISTENT:sections:all;volume:24;levels:20");
        assert!(result.is_none(), "Cache miss should return None");
    }

    #[test]
    fn test_cache_concurrent_access() {
        use std::sync::Arc;

        let cache = Arc::new(ReportCache::new(60));
        let mut handles = vec![];

        // Spawn 10 threads that concurrently read/write cache
        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = thread::spawn(move || {
                let symbol = if i % 2 == 0 { "BTCUSDT" } else { "ETHUSDT" };
                let key = format!("{}:sections:all;volume:24;levels:20", symbol);

                // Write
                cache_clone.set(key.clone(), create_test_report(symbol));

                // Read
                thread::sleep(Duration::from_millis(10));
                cache_clone.get(&key)
            });
            handles.push(handle);
        }

        // Wait for all threads and verify no panics
        for handle in handles {
            let result = handle.join().expect("Thread should not panic");
            assert!(result.is_some(), "Concurrent access should work");
        }
    }

    #[test]
    fn test_cache_stale_data() {
        // Create cache with 2-second TTL
        let cache = ReportCache::new(2);
        let report = create_test_report("BTCUSDT");
        let cache_key = "BTCUSDT:sections:all;volume:24;levels:20".to_string();

        // Set initial report
        cache.set(cache_key.clone(), report.clone());

        // Wait 1 second (within TTL)
        thread::sleep(Duration::from_millis(1000));

        // Should still be cached
        let fresh = cache.get(&cache_key);
        assert!(fresh.is_some(), "Data should be fresh within TTL");

        // Wait another 1.5 seconds (exceeds TTL)
        thread::sleep(Duration::from_millis(1500));

        // Should be stale and removed
        let stale = cache.get(&cache_key);
        assert!(
            stale.is_none(),
            "Stale data should be automatically removed"
        );
    }

    #[test]
    fn test_cache_overwrite_existing() {
        let cache = ReportCache::new(60);
        let cache_key = "BTCUSDT:sections:all;volume:24;levels:20".to_string();

        // Set first report
        let report1 = MarketReport {
            markdown_content: "# Report v1".to_string(),
            symbol: "BTCUSDT".to_string(),
            generated_at: 1000,
            data_age_ms: 100,
            failed_sections: vec![],
            generation_time_ms: 200,
        };
        cache.set(cache_key.clone(), report1);

        // Overwrite with new report
        let report2 = MarketReport {
            markdown_content: "# Report v2".to_string(),
            symbol: "BTCUSDT".to_string(),
            generated_at: 2000,
            data_age_ms: 50,
            failed_sections: vec![],
            generation_time_ms: 150,
        };
        cache.set(cache_key.clone(), report2);

        // Should get the latest report
        let cached = cache.get(&cache_key).unwrap();
        assert_eq!(cached.generated_at, 2000);
        assert!(cached.markdown_content.contains("v2"));
        assert_eq!(cached.generation_time_ms, 150);
    }
}
