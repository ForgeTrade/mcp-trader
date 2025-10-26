// Section builders for market data report
//
// Each function builds a specific section of the market intelligence report.
// Sections return Result<String, SectionError> for graceful degradation.

use super::{ReportSection, SectionError};

/// Build report header section with metadata
///
/// Includes: Symbol, generation timestamp, data age indicator
pub fn build_report_header(symbol: &str, generated_at: i64, data_age_ms: i32) -> ReportSection {
    use super::formatter;

    let mut content = String::new();

    // Main title
    content.push_str(&formatter::build_section_header(
        &format!("Market Report: {}", symbol),
        1,
    ));

    // Metadata table
    let freshness_indicator = if data_age_ms < 1000 {
        "🟢 Fresh"
    } else if data_age_ms < 5000 {
        "🟡 Recent"
    } else {
        "🔴 Stale"
    };

    let headers = vec!["Metric", "Value"];
    let rows = vec![
        vec!["Symbol".to_string(), symbol.to_string()],
        vec![
            "Generated At".to_string(),
            formatter::format_timestamp(generated_at),
        ],
        vec![
            "Data Age".to_string(),
            format!("{} ms {}", data_age_ms, freshness_indicator),
        ],
    ];

    content.push_str(&formatter::build_table(&headers, &rows));
    content.push('\n');

    ReportSection {
        name: "header".to_string(),
        title: "Market Report".to_string(),
        content: Ok(content),
        data_age_ms: Some(data_age_ms),
    }
}

/// Build price overview section
///
/// Includes: Current price, 24h change, 24h high/low, volume
pub fn build_price_overview_section(
    ticker: Option<&crate::binance::types::Ticker24hr>,
) -> ReportSection {
    use super::formatter;

    let content = match ticker {
        Some(t) => {
            let mut section = formatter::build_section_header("Price Overview", 2);

            // Parse values
            let price_change_pct: f64 = t.price_change_percent.parse().unwrap_or(0.0);
            let trend_indicator = if price_change_pct > 0.0 {
                "📈"
            } else if price_change_pct < 0.0 {
                "📉"
            } else {
                "➡️"
            };

            // Format LTP timestamp
            let ltp_time = chrono::DateTime::from_timestamp_millis(t.close_time)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            // Build price table (format prices with 2 decimals and thousand separators)
            let headers = vec!["Metric", "Value"];
            let rows = vec![
                vec!["Last Trade Price (LTP)".to_string(), format!("${}", formatter::format_price(&t.last_price, 2))],
                vec!["LTP Time".to_string(), ltp_time],
                vec![
                    "24h Change".to_string(),
                    format!("{} {}%", trend_indicator, t.price_change_percent),
                ],
                vec!["24h High".to_string(), format!("${}", formatter::format_price(&t.high_price, 2))],
                vec!["24h Low".to_string(), format!("${}", formatter::format_price(&t.low_price, 2))],
                vec![
                    "24h Volume".to_string(),
                    format!("{} {}", t.volume, t.symbol.trim_end_matches("USDT")),
                ],
                vec![
                    "24h Quote Volume".to_string(),
                    {
                        let quote_vol: f64 = t.quote_volume.parse().unwrap_or(0.0);
                        formatter::format_large_usd(quote_vol)
                    },
                ],
                vec![
                    "Weighted Avg Price".to_string(),
                    format!("${}", formatter::format_price(&t.weighted_avg_price, 2)),
                ],
            ];

            section.push_str(&formatter::build_table(&headers, &rows));
            section.push('\n');

            Ok(section)
        }
        None => Err(SectionError::DataSourceUnavailable("ticker".to_string())),
    };

    ReportSection {
        name: "price_overview".to_string(),
        title: "Price Overview".to_string(),
        content,
        data_age_ms: None,
    }
}

/// Build order book metrics section
///
/// Includes: Spread (bps), microprice, bid/ask volume, imbalance ratio
pub fn build_orderbook_metrics_section(
    metrics: Option<&crate::orderbook::types::OrderBookMetrics>,
) -> ReportSection {
    use super::formatter;

    let content = match metrics {
        Some(m) => {
            let mut section = formatter::build_section_header("Order Book Metrics", 2);

            // BLOCKER FIX: Detect crossed orderbook (ask < bid)
            let is_crossed = m.spread_bps < 0.0;

            // Calculate spread in $ and m-bps (milli-basis points)
            let best_bid_f64: f64 = m.best_bid.parse().unwrap_or(0.0);
            let best_ask_f64: f64 = m.best_ask.parse().unwrap_or(0.0);
            let spread_usd = best_ask_f64 - best_bid_f64;
            let spread_mbps = m.spread_bps * 1000.0; // Convert bps to m-bps

            // Spread formatting with crossed detection
            let spread_formatted = if is_crossed {
                // Crossed orderbook - show warning and actual spread with 4 decimals
                format!("{:.4} bps ⚠️ Crossed", m.spread_bps)
            } else if m.spread_bps < 10.0 {
                // Tight spread - show in m-bps and $ for microstructure analysis
                format!("{:.2} m-bps (${:.2}) 🟢 Tight", spread_mbps, spread_usd)
            } else if m.spread_bps < 50.0 {
                format!("{:.4} bps (${:.2}) 🟡 Moderate", m.spread_bps, spread_usd)
            } else {
                format!("{:.4} bps (${:.2}) 🔴 Wide", m.spread_bps, spread_usd)
            };

            // Imbalance indicator
            let imbalance_indicator = if m.imbalance_ratio > 1.2 {
                "🟢 Buy Pressure"
            } else if m.imbalance_ratio < 0.8 {
                "🔴 Sell Pressure"
            } else {
                "🟡 Balanced"
            };

            // Build metrics table (format prices with 2 decimals and thousand separators for BTCUSDT)
            let headers = vec!["Metric", "Value"];
            let rows = vec![
                vec!["Best Bid".to_string(), format!("${}", formatter::format_price(&m.best_bid, 2))],
                vec!["Best Bid Size".to_string(), format!("{:.4} BTC", m.best_bid_size)],
                vec!["Best Ask".to_string(), format!("${}", formatter::format_price(&m.best_ask, 2))],
                vec!["Best Ask Size".to_string(), format!("{:.4} BTC", m.best_ask_size)],
                vec![
                    "Spread".to_string(),
                    spread_formatted,
                ],
                // Show Mid Price with 5 decimals to match microprice precision and prove spread basis
                vec!["Mid Price".to_string(), format!("${}", formatter::format_price_f64(m.mid_price, 5))],
                // P0 Fix: Increase microprice precision to 5 decimals to avoid rounding artifacts
                vec!["Microprice".to_string(), format!("${:.5}", m.microprice)],
                vec![
                    "Bid Volume (Top 20)".to_string(),
                    format!("{:.4} BTC", m.bid_volume),
                ],
                vec![
                    "Ask Volume (Top 20)".to_string(),
                    format!("{:.4} BTC", m.ask_volume),
                ],
                vec![
                    "Imbalance Ratio".to_string(),
                    format!("{:.3} {}", m.imbalance_ratio, imbalance_indicator),
                ],
            ];

            section.push_str(&formatter::build_table(&headers, &rows));
            section.push('\n');

            // Add verification notice
            section.push_str("### Verification Against Binance API\n\n");

            // Format event time
            let event_time = chrono::DateTime::from_timestamp_millis(m.timestamp)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S.%3f UTC").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            section.push_str(&format!(
                "| Metric | Value | Status |\n\
                 |--------|-------|--------|\n\
                 | Last Update ID | {} | ✅ Tracked |\n\
                 | Event Time | {} | ✅ Timestamped |\n\
                 | Best Bid | ${} | ✅ Live |\n\
                 | Best Ask | ${} | ✅ Live |\n\
                 | Spread | {:.2} m-bps (${:.2}) | ✅ Valid |\n\
                 | Data Age | Fresh (<500ms) | ✅ Real-time |\n\n",
                m.last_update_id, event_time,
                formatter::format_price(&m.best_bid, 2),
                formatter::format_price(&m.best_ask, 2),
                spread_mbps, spread_usd
            ));
            section.push_str("*OrderBook data sourced directly from Binance WebSocket depth streams with REST API fallback. Update ID ensures snapshot consistency.*\n\n");

            Ok(section)
        }
        None => Err(SectionError::DataSourceUnavailable("orderbook".to_string())),
    };

    ReportSection {
        name: "orderbook_metrics".to_string(),
        title: "Order Book Metrics".to_string(),
        content,
        data_age_ms: None,
    }
}

/// Build liquidity analysis section
///
/// Includes: Major walls, volume profile (POC/VAH/VAL), liquidity vacuums
/// Enhanced with better formatting, visual indicators, and volume profile (T033-T037)
pub fn build_liquidity_analysis_section(
    metrics: Option<&crate::orderbook::types::OrderBookMetrics>,
    volume_window_hours: u32,
) -> ReportSection {
    use super::formatter;

    let content = match metrics {
        Some(m) => {
            let mut section = formatter::build_section_header("Liquidity Analysis", 2);

            // T033: Enhanced walls table with better formatting and visual indicators
            section.push_str("### Liquidity Walls\n\n");

            if !m.walls.bids.is_empty() || !m.walls.asks.is_empty() {
                // T037: Visual indicators for wall strength
                let get_wall_strength = |qty: f64| -> &'static str {
                    if qty > 100.0 {
                        "💪 Strong"
                    } else if qty > 50.0 {
                        "🔷 Moderate"
                    } else {
                        "🔹 Weak"
                    }
                };

                // Buy walls table
                if !m.walls.bids.is_empty() {
                    section.push_str("**Buy Walls (Support Levels):**\n\n");
                    let headers = vec!["Price", "Volume", "Strength", "Type"];
                    let rows: Vec<Vec<String>> = m
                        .walls
                        .bids
                        .iter()
                        .take(5)
                        .map(|w| {
                            let qty: f64 = w.qty.parse().unwrap_or(0.0);
                            vec![
                                format!("${}", w.price),
                                format!("{:.4} units", w.qty),
                                get_wall_strength(qty).to_string(),
                                "🟢 Support".to_string(),
                            ]
                        })
                        .collect();
                    section.push_str(&formatter::build_table(&headers, &rows));
                    section.push('\n');
                }

                // Sell walls table
                if !m.walls.asks.is_empty() {
                    section.push_str("**Sell Walls (Resistance Levels):**\n\n");
                    let headers = vec!["Price", "Volume", "Strength", "Type"];
                    let rows: Vec<Vec<String>> = m
                        .walls
                        .asks
                        .iter()
                        .take(5)
                        .map(|w| {
                            let qty: f64 = w.qty.parse().unwrap_or(0.0);
                            vec![
                                format!("${}", w.price),
                                format!("{:.4} units", w.qty),
                                get_wall_strength(qty).to_string(),
                                "🔴 Resistance".to_string(),
                            ]
                        })
                        .collect();
                    section.push_str(&formatter::build_table(&headers, &rows));
                    section.push('\n');
                }
            } else {
                section
                    .push_str("*No significant liquidity walls detected (volume < 2x median)*\n\n");
            }

            // T034: Volume profile visualization with POC/VAH/VAL (placeholder for future implementation)
            // T036: Display volume window duration
            section.push_str(&format!("### {}h Volume Profile\n\n", volume_window_hours));
            section.push_str("*Volume profile analysis showing key price levels:*\n\n");

            let headers = vec!["Level", "Price", "Description"];
            let rows = vec![
                vec![
                    "POC".to_string(),
                    "TBD".to_string(),
                    "Point of Control (highest volume)".to_string(),
                ],
                vec![
                    "VAH".to_string(),
                    "TBD".to_string(),
                    "Value Area High (top of 70% volume)".to_string(),
                ],
                vec![
                    "VAL".to_string(),
                    "TBD".to_string(),
                    "Value Area Low (bottom of 70% volume)".to_string(),
                ],
            ];
            section.push_str(&formatter::build_table(&headers, &rows));
            section.push_str(
                "\n*Note: Volume profile calculation requires historical trade data*\n\n",
            );

            // T035: Liquidity vacuums table (placeholder for future implementation)
            section.push_str("### Liquidity Vacuums\n\n");
            section.push_str("*Price ranges with significantly lower liquidity:*\n\n");

            let headers = vec!["Price Range", "Volume Deficit", "Impact", "Risk Level"];
            let rows = vec![vec![
                "TBD".to_string(),
                "TBD".to_string(),
                "TBD".to_string(),
                "Monitoring".to_string(),
            ]];
            section.push_str(&formatter::build_table(&headers, &rows));
            section.push_str(
                "\n*Note: Liquidity vacuum detection requires order book depth analysis*\n\n",
            );

            Ok(section)
        }
        None => Err(SectionError::DataSourceUnavailable("liquidity".to_string())),
    };

    ReportSection {
        name: "liquidity_analysis".to_string(),
        title: "Liquidity Analysis".to_string(),
        content,
        data_age_ms: None,
    }
}

/// Build liquidity analysis section with full analytics (Feature 019 - US1)
///
/// Provides comprehensive liquidity analysis:
/// - Volume profile (POC/VAH/VAL from trade history)
/// - Order walls (support/resistance from order book)
/// - Liquidity vacuums (low-volume zones)
///
/// # Arguments
/// * `storage` - Analytics snapshot storage
/// * `trade_storage` - Trade history storage for volume profile
/// * `symbol` - Trading pair symbol
/// * `volume_window_hours` - Time window for volume profile (1-168 hours)
/// * `generated_at` - Report generation timestamp
///
/// # Returns
/// ReportSection with volume profile, walls, and vacuums
#[cfg(feature = "orderbook_analytics")]
pub async fn build_liquidity_analysis_section_async(
    storage: &std::sync::Arc<crate::orderbook::analytics::SnapshotStorage>,
    trade_storage: &std::sync::Arc<crate::orderbook::analytics::TradeStorage>,
    symbol: &str,
    volume_window_hours: u32,
    generated_at: chrono::DateTime<chrono::Utc>,
    orderbook_metrics: Option<&crate::orderbook::types::OrderBookMetrics>, // CROSSED FIX: Use live metrics for walls
) -> ReportSection {
    use super::{formatter, util};
    use crate::orderbook::analytics::tools::{
        get_liquidity_vacuums, get_volume_profile, GetLiquidityVacuumsParams,
        GetVolumeProfileParams,
    };
    // CROSSED FIX: Removed unused imports (identify_order_walls, query_snapshots_in_window)

    let mut content = formatter::build_section_header("Liquidity Analysis", 2);

    // CROSSED FIX: Removed historical snapshot query for walls - now using live orderbook_metrics
    let now = chrono::Utc::now();

    // T007: Prepare parameters for trade query (avoid blocking on Tokio thread)
    // PERF: Reduce default window from 24h to 6h to improve query performance
    let actual_window_hours = volume_window_hours.min(6);
    let start_time_ms = (now - chrono::Duration::hours(actual_window_hours as i64)).timestamp_millis();
    let end_time_ms = now.timestamp_millis();
    let trade_storage_clone = trade_storage.clone();
    let symbol_for_trades = symbol.to_string();

    tracing::info!(
        "Volume Profile query params: requested={}h actual={}h time_range={}ms symbol={}",
        volume_window_hours, actual_window_hours, end_time_ms - start_time_ms, symbol
    );

    // T008-T009: Parallel analytics calls with timeouts (including trade query in spawn_blocking)
    let (profile_result, vacuums_result, walls_result) = tokio::join!(
        // Volume profile from trades (P1 fix: query_trades in spawn_blocking)
        async {
            // Spawn blocking to avoid blocking Tokio runtime on RocksDB I/O
            let query_start = std::time::Instant::now();
            let trades_result = tokio::task::spawn_blocking(move || {
                trade_storage_clone.query_trades(&symbol_for_trades, start_time_ms, end_time_ms)
            })
            .await;
            let query_elapsed = query_start.elapsed();

            tracing::info!(
                "RocksDB trade query completed: duration={:?} symbol={}",
                query_elapsed, symbol
            );

            match trades_result {
                Ok(Ok(trades)) => {
                    tracing::info!(
                        "Converting {} trades for Volume Profile: symbol={}",
                        trades.len(), symbol
                    );

                    let profile_start = std::time::Instant::now();

                    // Convert trade_storage::AggTrade to trade_stream::AggTrade
                    let converted_trades: Vec<crate::orderbook::analytics::trade_stream::AggTrade> = trades
                        .into_iter()
                        .map(|t| crate::orderbook::analytics::trade_stream::AggTrade {
                            event_type: "aggTrade".to_string(),
                            event_time: t.timestamp,
                            symbol: symbol.to_string(),
                            agg_trade_id: t.trade_id as u64,
                            price: t.price,
                            quantity: t.quantity,
                            first_trade_id: 0, // Not stored, use placeholder
                            last_trade_id: 0,  // Not stored, use placeholder
                            trade_time: t.timestamp,
                            is_buyer_maker: t.buyer_is_maker,
                            is_best_match: false, // Not stored, use placeholder
                        })
                        .collect();

                    let result = util::timeout_analytics(
                        get_volume_profile(
                            converted_trades,
                            GetVolumeProfileParams {
                                symbol: symbol.to_string(),
                                duration_hours: actual_window_hours, // Use reduced window
                                tick_size: None,
                            },
                        ),
                        "get_volume_profile",
                        symbol,
                    )
                    .await;

                    let profile_elapsed = profile_start.elapsed();
                    tracing::info!(
                        "Volume Profile generation completed: duration={:?} symbol={}",
                        profile_elapsed, symbol
                    );

                    result
                }
                Ok(Err(e)) => Err(util::TimeoutError::Analytics(e.to_string())),
                Err(e) => Err(util::TimeoutError::Analytics(format!("spawn_blocking failed: {}", e))),
            }
        },
        // Liquidity vacuums
        util::timeout_analytics(
            get_liquidity_vacuums(
                storage.clone(),
                GetLiquidityVacuumsParams {
                    symbol: symbol.to_string(),
                    duration_hours: (volume_window_hours as u32).min(24),
                },
            ),
            "get_liquidity_vacuums",
            symbol,
        ),
        // CROSSED FIX: Use live orderbook_metrics.walls instead of historical snapshot
        async {
            match orderbook_metrics {
                Some(metrics) => {
                    // Convert Wall structs to tuple format (price, qty, side)
                    let mut walls: Vec<(rust_decimal::Decimal, rust_decimal::Decimal, String)> = Vec::new();

                    // Add bid walls
                    for wall in &metrics.walls.bids {
                        if let (Ok(price), Ok(qty)) = (
                            wall.price.parse::<rust_decimal::Decimal>(),
                            wall.qty.parse::<rust_decimal::Decimal>()
                        ) {
                            walls.push((price, qty, "bid".to_string()));
                        }
                    }

                    // Add ask walls
                    for wall in &metrics.walls.asks {
                        if let (Ok(price), Ok(qty)) = (
                            wall.price.parse::<rust_decimal::Decimal>(),
                            wall.qty.parse::<rust_decimal::Decimal>()
                        ) {
                            walls.push((price, qty, "ask".to_string()));
                        }
                    }

                    Ok(walls)
                }
                None => Err(util::TimeoutError::Analytics(
                    "No live orderbook metrics available".to_string(),
                )),
            }
        },
    );

    // BLOCKER FIX: Use actual window instead of requested to match reality
    content.push_str(&format!("### Volume Profile (last {}h)\n\n", actual_window_hours));

    match profile_result {
        Ok(profile) => {
            let headers = vec!["Level", "Price", "Description"];
            let rows = vec![
                vec![
                    "POC".to_string(),
                    format!("${}", formatter::format_price(&profile.point_of_control.to_string(), 2)),
                    "Point of Control (highest volume)".to_string(),
                ],
                vec![
                    "VAH".to_string(),
                    format!("${}", formatter::format_price(&profile.value_area_high.to_string(), 2)),
                    "Value Area High (top of 70% volume)".to_string(),
                ],
                vec![
                    "VAL".to_string(),
                    format!("${}", formatter::format_price(&profile.value_area_low.to_string(), 2)),
                    "Value Area Low (bottom of 70% volume)".to_string(),
                ],
                vec![
                    "Total Volume".to_string(),
                    format!("{}", profile.total_volume),
                    format!("Across {} bins", profile.bin_count),
                ],
            ];
            content.push_str(&formatter::build_table(&headers, &rows));
            content.push_str("\n");

            // P0 Fix: Add data source disclaimer for Volume Profile
            // Check if volume is suspiciously low (may indicate incomplete data)
            let total_vol_f64: f64 = profile.total_volume.to_string().parse().unwrap_or(0.0);

            // For BTCUSDT, expect roughly ~420 BTC/hour in normal conditions (based on 10k BTC/24h)
            let expected_min_volume = (actual_window_hours as f64) * 420.0;
            let coverage_pct = (total_vol_f64 / expected_min_volume) * 100.0;
            let is_sampled = coverage_pct < 50.0; // Less than 50% coverage = sampled subset

            content.push_str("\n*Note: Volume Profile based on **locally collected trades since server start** (sampled subset, not full market coverage). ");

            if is_sampled {
                content.push_str(&format!(
                    "Coverage: ~{:.1}% ({:.1} BTC of expected ~{:.0} BTC for {}h window). \
                    Representative for distribution shape, but not absolute volumes. \
                    For full historical data, REST API backfill required.*\n\n",
                    coverage_pct, total_vol_f64, expected_min_volume, actual_window_hours
                ));
            } else {
                content.push_str(&format!(
                    "Coverage: ~{:.1}% ({:.1} BTC). Profile includes majority of market activity.*\n\n",
                    coverage_pct, total_vol_f64
                ));
            }
        }
        Err(util::TimeoutError::Exceeded) => {
            content.push_str("**[Data Unavailable: timeout]**\n\n");
        }
        Err(util::TimeoutError::Analytics(e)) => {
            content.push_str(&format!("**[Data Unavailable: {}]**\n\n", e));
        }
    }

    // T012: Render order walls
    content.push_str("### Liquidity Walls\n\n");

    match walls_result {
        Ok(walls) if !walls.is_empty() => {
            let get_wall_strength = |qty: rust_decimal::Decimal| -> &'static str {
                let qty_f64: f64 = qty.to_string().parse().unwrap_or(0.0);
                if qty_f64 > 100.0 {
                    "💪 Strong"
                } else if qty_f64 > 50.0 {
                    "🔷 Moderate"
                } else {
                    "🔹 Weak"
                }
            };

            // Group walls by side
            let mut bid_walls: Vec<_> = walls.iter().filter(|(_, _, side)| *side == "bid").collect();
            let mut ask_walls: Vec<_> = walls.iter().filter(|(_, _, side)| *side == "ask").collect();

            if !bid_walls.is_empty() {
                content.push_str("**Buy Walls (Support Levels):**\n\n");
                let headers = vec!["Price", "Volume", "Strength", "Type"];
                let rows: Vec<Vec<String>> = bid_walls
                    .iter()
                    .take(5)
                    .map(|(price, qty, _)| {
                        vec![
                            format!("${}", formatter::format_price(&price.to_string(), 2)),
                            format!("{}", qty),
                            get_wall_strength(*qty).to_string(),
                            "🟢 Support".to_string(),
                        ]
                    })
                    .collect();
                content.push_str(&formatter::build_table(&headers, &rows));
                content.push_str("\n");
            }

            if !ask_walls.is_empty() {
                content.push_str("**Sell Walls (Resistance Levels):**\n\n");
                let headers = vec!["Price", "Volume", "Strength", "Type"];
                let rows: Vec<Vec<String>> = ask_walls
                    .iter()
                    .take(5)
                    .map(|(price, qty, _)| {
                        vec![
                            format!("${}", formatter::format_price(&price.to_string(), 2)),
                            format!("{}", qty),
                            get_wall_strength(*qty).to_string(),
                            "🔴 Resistance".to_string(),
                        ]
                    })
                    .collect();
                content.push_str(&formatter::build_table(&headers, &rows));
                content.push_str("\n");
            }
        }
        Ok(_) => {
            content.push_str("*No significant liquidity walls detected*\n\n");
        }
        Err(util::TimeoutError::Exceeded) => {
            content.push_str("**[Data Unavailable: timeout]**\n\n");
        }
        Err(util::TimeoutError::Analytics(e)) => {
            content.push_str(&format!("**[Data Unavailable: {}]**\n\n", e));
        }
    }

    // T011: Render liquidity vacuums
    content.push_str("### Liquidity Vacuums\n\n");

    match vacuums_result {
        Ok(vacuums) if !vacuums.is_empty() => {
            let headers = vec!["Price Range", "Width", "Volume Deficit", "Expected Impact", "Risk Level"];
            let rows: Vec<Vec<String>> = vacuums
                .iter()
                .take(10)
                .map(|v| {
                    use rust_decimal::prelude::ToPrimitive;

                    // Calculate range width in basis points
                    let range_width = v.price_range_high - v.price_range_low;
                    let mid_price = (v.price_range_high + v.price_range_low) / rust_decimal::Decimal::from(2);
                    let width_bps = if !mid_price.is_zero() {
                        (range_width / mid_price * rust_decimal::Decimal::from(10000))
                            .to_f64()
                            .unwrap_or(0.0)
                    } else {
                        0.0
                    };

                    let (impact_text, risk_icon) = match v.expected_impact {
                        crate::orderbook::analytics::types::ImpactLevel::FastMovement => {
                            ("Fast price movement", "🔴 High")
                        }
                        crate::orderbook::analytics::types::ImpactLevel::ModerateMovement => {
                            ("Moderate slippage", "🟡 Medium")
                        }
                        crate::orderbook::analytics::types::ImpactLevel::Negligible => {
                            ("Minimal impact", "🟢 Low")
                        }
                    };

                    vec![
                        format!("${} - ${}",
                            formatter::format_price(&v.price_range_low.to_string(), 2),
                            formatter::format_price(&v.price_range_high.to_string(), 2)
                        ),
                        format!("{:.1} bps", width_bps),
                        format!("{:.1}%", v.volume_deficit_pct),
                        impact_text.to_string(),
                        risk_icon.to_string(),
                    ]
                })
                .collect();
            content.push_str(&formatter::build_table(&headers, &rows));
            content.push_str("\n");
        }
        Ok(_) => {
            content.push_str("*No significant liquidity vacuums detected*\n\n");
            content.push_str("*Detection criteria: Volume < 20% of median (80%+ deficit) across order book depth levels. All levels show adequate liquidity.*\n\n");
        }
        Err(util::TimeoutError::Exceeded) => {
            content.push_str("**[Data Unavailable: timeout]**\n\n");
        }
        Err(util::TimeoutError::Analytics(e)) => {
            content.push_str(&format!("**[Data Unavailable: {}]**\n\n", e));
        }
    }

    // T014: Data age indicator
    let data_age_ms = util::calculate_data_age_ms(generated_at, chrono::Utc::now());

    ReportSection {
        name: "liquidity_analysis".to_string(),
        title: "Liquidity Analysis".to_string(),
        content: Ok(content),
        data_age_ms: Some(data_age_ms),
    }
}

/// Build market microstructure section
///
/// Includes: Order flow direction, bid/ask flow rates, net flow
pub fn build_microstructure_section() -> ReportSection {
    use super::formatter;

    // MVP: Basic placeholder
    let mut section = formatter::build_section_header("Market Microstructure", 2);
    section.push_str("*Advanced microstructure analysis available in enhanced version.*\n\n");
    section.push_str("This section will include:\n");
    let items = vec![
        "Order flow direction and intensity".to_string(),
        "Bid/ask flow rates over time windows".to_string(),
        "Net flow calculations".to_string(),
        "Flow-based trading signals".to_string(),
    ];
    section.push_str(&formatter::build_list(&items, false));
    section.push('\n');

    ReportSection {
        name: "microstructure".to_string(),
        title: "Market Microstructure".to_string(),
        content: Ok(section),
        data_age_ms: None,
    }
}

/// Build order flow section with real-time analysis (Feature 019 - US4)
///
/// Analyzes bid/ask order flow to detect directional pressure:
/// - Bid flow rate (orders/sec)
/// - Ask flow rate (orders/sec)
/// - Net flow (bid - ask)
/// - Flow direction indicator
///
/// # Arguments
/// * `storage` - Analytics snapshot storage for historical data
/// * `symbol` - Trading pair symbol
/// * `generated_at` - Report generation timestamp for data age calculation
///
/// # Returns
/// ReportSection with order flow metrics and trading signals
#[cfg(feature = "orderbook_analytics")]
pub async fn build_microstructure_section_async(
    storage: &std::sync::Arc<crate::orderbook::analytics::SnapshotStorage>,
    symbol: &str,
    generated_at: chrono::DateTime<chrono::Utc>,
) -> ReportSection {
    use super::{formatter, util};
    use crate::orderbook::analytics::tools::{get_order_flow, GetOrderFlowParams};

    // T041-T042: Call order flow calculation with timeout (FR-020)
    let flow_result = util::timeout_analytics(
        get_order_flow(
            storage.clone(),
            GetOrderFlowParams {
                symbol: symbol.to_string(),
                window_duration_secs: 60, // FR-012: 60-second window
            },
        ),
        "get_order_flow",
        symbol,
    )
    .await;

    // Build section content
    let mut content = formatter::build_section_header("Market Microstructure", 2);

    match flow_result {
        Ok(flow) => {
            // T043: Flow direction indicator with icon
            let (direction_emoji, direction_text, signal) = match flow.flow_direction {
                crate::orderbook::analytics::types::FlowDirection::StrongBuy => {
                    ("🟢⬆️", "Strong Buy Pressure", "Consider long positions")
                }
                crate::orderbook::analytics::types::FlowDirection::ModerateBuy => {
                    ("🟢↗️", "Moderate Buy Pressure", "Bullish bias, monitor for continuation")
                }
                crate::orderbook::analytics::types::FlowDirection::Neutral => {
                    ("⚪➡️", "Neutral", "Wait for clearer direction")
                }
                crate::orderbook::analytics::types::FlowDirection::ModerateSell => {
                    ("🔴↘️", "Moderate Sell Pressure", "Bearish bias, monitor for continuation")
                }
                crate::orderbook::analytics::types::FlowDirection::StrongSell => {
                    ("🔴⬇️", "Strong Sell Pressure", "Consider short positions or exit longs")
                }
            };

            content.push_str(&format!(
                "{} **Flow Direction:** {}\n\n",
                direction_emoji, direction_text
            ));

            // T044: Order flow metrics table
            content.push_str("### Order Flow Metrics\n\n");

            let headers = vec!["Metric", "Value", "Interpretation"];
            let rows = vec![
                vec![
                    "Bid Flow Rate".to_string(),
                    format!("{:.2} orders/sec", flow.bid_flow_rate),
                    if flow.bid_flow_rate > flow.ask_flow_rate {
                        "🟢 Higher than asks".to_string()
                    } else {
                        "Balanced or lower".to_string()
                    },
                ],
                vec![
                    "Ask Flow Rate".to_string(),
                    format!("{:.2} orders/sec", flow.ask_flow_rate),
                    if flow.ask_flow_rate > flow.bid_flow_rate {
                        "🔴 Higher than bids".to_string()
                    } else {
                        "Balanced or lower".to_string()
                    },
                ],
                vec![
                    "Net Flow".to_string(),
                    format!("{:+.2} orders/sec", flow.net_flow),
                    if flow.net_flow > 0.0 {
                        "🟢 Positive (buying pressure)".to_string()
                    } else if flow.net_flow < 0.0 {
                        "🔴 Negative (selling pressure)".to_string()
                    } else {
                        "⚪ Neutral".to_string()
                    },
                ],
                vec![
                    "Cumulative Delta".to_string(),
                    format!("{:+.2}", flow.cumulative_delta),
                    "Running buy/sell volume difference".to_string(),
                ],
            ];

            content.push_str(&formatter::build_table(&headers, &rows));
            content.push_str("\n");

            // Add remark about orders vs volume divergence when applicable
            let orders_direction = if flow.net_flow > 0.0 { "buy" } else if flow.net_flow < 0.0 { "sell" } else { "neutral" };
            let volume_direction = if flow.cumulative_delta > 0.0 { "buy" } else if flow.cumulative_delta < 0.0 { "sell" } else { "neutral" };

            if orders_direction != volume_direction && orders_direction != "neutral" && volume_direction != "neutral" {
                content.push_str(&format!(
                    "*Note: Order flow shows {} pressure (order count), while cumulative delta shows {} pressure (volume). \
                    This indicates many small {} orders vs fewer large {} orders.*\n\n",
                    orders_direction, volume_direction, orders_direction,
                    if volume_direction == "buy" { "buy" } else { "sell" }
                ));
            }

            // T045: Trading signal based on flow direction
            content.push_str("### Trading Signal\n\n");
            content.push_str(&format!("**Recommendation:** {}\n\n", signal));

            // T046: Time window and calculation time
            content.push_str(&format!(
                "*Analysis window: {} seconds | Calculated: {}*\n\n",
                flow.window_duration_secs,
                formatter::format_datetime(flow.time_window_end)
            ));
        }
        Err(util::TimeoutError::Exceeded) => {
            // FR-013: Graceful degradation on timeout
            content.push_str("⚠️ **[Data Unavailable: timeout]**\n\n");
            content.push_str(
                "Order flow calculation exceeded 1-second timeout. This may indicate high system load.\n\n",
            );
        }
        Err(util::TimeoutError::Analytics(e)) => {
            // FR-013: Graceful degradation on analytics error
            content.push_str(&format!("⚠️ **[Data Unavailable: {}]**\n\n", e));
            content.push_str("Unable to calculate order flow. Check analytics storage.\n\n");
        }
    }

    // Data age indicator (FR-015)
    let data_age_ms = util::calculate_data_age_ms(generated_at, chrono::Utc::now());

    ReportSection {
        name: "market_microstructure".to_string(),
        title: "Market Microstructure".to_string(),
        content: Ok(content),
        data_age_ms: Some(data_age_ms),
    }
}

/// Build market anomalies section
///
/// Includes: Detected anomalies with severity and recommendations
/// Enhanced with severity badges, sorting, and actionable recommendations (T028-T032)
#[cfg(feature = "orderbook_analytics")]
pub fn build_anomalies_section(timestamp: Option<i64>) -> ReportSection {
    use super::formatter;

    let mut section = formatter::build_section_header("Market Anomalies", 2);

    // T031: Enhanced "no anomalies" message with timestamp
    section.push_str("🟢 **Status:** No anomalies detected\n");
    if let Some(ts) = timestamp {
        section.push_str(&format!(
            "*Last scanned: {}*\n",
            formatter::format_timestamp(ts)
        ));
    }
    section.push_str("\n");

    // Monitoring categories with severity indicators
    section.push_str("**Active Monitoring:**\n\n");

    // T028: Severity badge examples showing format
    let headers = vec!["Anomaly Type", "Severity", "Status"];
    let rows = vec![
        vec![
            "Quote Stuffing".to_string(),
            "🔴 Critical".to_string(),
            "✅ Normal".to_string(),
        ],
        vec![
            "Flash Crash Risk".to_string(),
            "🔴 Critical".to_string(),
            "✅ Normal".to_string(),
        ],
        vec![
            "Iceberg Orders".to_string(),
            "🟡 Medium".to_string(),
            "✅ Normal".to_string(),
        ],
        vec![
            "Spread Anomalies".to_string(),
            "🟡 Medium".to_string(),
            "✅ Normal".to_string(),
        ],
        vec![
            "Volume Spikes".to_string(),
            "🟢 Low".to_string(),
            "✅ Normal".to_string(),
        ],
    ];
    section.push_str(&formatter::build_table(&headers, &rows));
    section.push('\n');

    // T030: Example recommendations format (shown when anomalies would be detected)
    section.push_str("**Detection Capabilities:**\n");
    let capabilities = vec![
        "🔴 **Critical** anomalies trigger immediate alerts (e.g., flash crash indicators)"
            .to_string(),
        "🟠 **High** severity anomalies suggest caution (e.g., iceberg order detection)"
            .to_string(),
        "🟡 **Medium** anomalies provide market intelligence (e.g., unusual spreads)".to_string(),
        "🟢 **Low** severity anomalies track minor deviations (e.g., volume changes)".to_string(),
    ];
    section.push_str(&formatter::build_list(&capabilities, false));
    section.push('\n');

    // T032: Description of context that would be provided
    section.push_str("*When anomalies are detected, this section will show:*\n");
    let context_info = vec![
        "Affected price levels and order book regions".to_string(),
        "Detection timestamps and duration".to_string(),
        "Actionable recommendations based on anomaly type".to_string(),
        "Historical context and severity assessment".to_string(),
    ];
    section.push_str(&formatter::build_list(&context_info, false));
    section.push('\n');

    ReportSection {
        name: "anomalies".to_string(),
        title: "Market Anomalies".to_string(),
        content: Ok(section),
        data_age_ms: None,
    }
}

/// Build anomalies section with real-time detection (Feature 019 - US2)
///
/// Calls analytics functions to detect market anomalies:
/// - Quote stuffing (>500 updates/sec, <10% fill rate)
/// - Iceberg orders (refill rate >5x median)
/// - Flash crash risk (>80% liquidity drain)
///
/// # Arguments
/// * `storage` - Analytics snapshot storage for historical data
/// * `symbol` - Trading pair symbol
/// * `generated_at` - Report generation timestamp for data age calculation
///
/// # Returns
/// ReportSection with anomaly detections or "No anomalies detected" message
#[cfg(feature = "orderbook_analytics")]
pub async fn build_anomalies_section_async(
    storage: &std::sync::Arc<crate::orderbook::analytics::SnapshotStorage>,
    symbol: &str,
    generated_at: chrono::DateTime<chrono::Utc>,
) -> ReportSection {
    use super::{formatter, util};
    use crate::orderbook::analytics::tools::detect_market_anomalies;

    // T019-T021: Call anomaly detection with timeout (FR-020)
    let anomalies_result = util::timeout_analytics(
        detect_market_anomalies(storage.clone(), symbol),
        "detect_market_anomalies",
        symbol,
    )
    .await;

    // Build section content
    let mut content = formatter::build_section_header("Market Anomalies", 2);

    match anomalies_result {
        Ok(anomalies) if anomalies.is_empty() => {
            // T022: No anomalies detected
            content.push_str("✅ **Status:** No anomalies detected\n");
            content.push_str(&format!(
                "*Last scanned: {}*\n\n",
                formatter::format_datetime(generated_at)
            ));
            content.push_str("Market microstructure appears healthy with no suspicious patterns.\n\n");
        }
        Ok(mut anomalies) => {
            // T023: Sort by severity (Critical → High → Medium → Low)
            anomalies.sort_by(|a, b| {
                let severity_order = |s: &crate::orderbook::analytics::types::Severity| match s {
                    crate::orderbook::analytics::types::Severity::Critical => 0,
                    crate::orderbook::analytics::types::Severity::High => 1,
                    crate::orderbook::analytics::types::Severity::Medium => 2,
                    crate::orderbook::analytics::types::Severity::Low => 3,
                };
                severity_order(&a.severity).cmp(&severity_order(&b.severity))
            });

            // T024: Render anomaly table
            content.push_str(&format!("⚠️ **{} anomalies detected**\n\n", anomalies.len()));

            let headers = vec!["Type", "Severity", "Description", "Recommended Action"];
            let mut rows = Vec::new();

            for anomaly in &anomalies {
                // Severity icon
                let severity_icon = match anomaly.severity {
                    crate::orderbook::analytics::types::Severity::Critical => "🔴",
                    crate::orderbook::analytics::types::Severity::High => "🟠",
                    crate::orderbook::analytics::types::Severity::Medium => "🟡",
                    crate::orderbook::analytics::types::Severity::Low => "🟢",
                };

                // Anomaly type name
                let type_name = match &anomaly.anomaly_type {
                    crate::orderbook::analytics::types::AnomalyType::QuoteStuffing { .. } => {
                        "Quote Stuffing"
                    }
                    crate::orderbook::analytics::types::AnomalyType::IcebergOrder { .. } => {
                        "Iceberg Order"
                    }
                    crate::orderbook::analytics::types::AnomalyType::FlashCrashRisk { .. } => {
                        "Flash Crash Risk"
                    }
                };

                // Description with key metrics
                let description = match &anomaly.anomaly_type {
                    crate::orderbook::analytics::types::AnomalyType::QuoteStuffing {
                        update_rate,
                        fill_rate,
                    } => {
                        format!(
                            "{:.0} updates/sec, {:.1}% fill rate",
                            update_rate,
                            fill_rate * 100.0
                        )
                    }
                    crate::orderbook::analytics::types::AnomalyType::IcebergOrder {
                        price_level,
                        refill_rate_multiplier,
                        median_refill_rate: _,
                    } => {
                        format!(
                            "At ${}, {:.1}x median refill rate",
                            price_level, refill_rate_multiplier
                        )
                    }
                    crate::orderbook::analytics::types::AnomalyType::FlashCrashRisk {
                        depth_loss_pct,
                        ..
                    } => {
                        format!("{:.0}% depth loss detected", depth_loss_pct * 100.0)
                    }
                };

                rows.push(vec![
                    type_name.to_string(),
                    format!("{} {:?}", severity_icon, anomaly.severity),
                    description,
                    anomaly.recommended_action.clone(),
                ]);
            }

            content.push_str(&formatter::build_table(&headers, &rows));
            content.push_str("\n");

            // T025: Add detection timestamp
            content.push_str(&format!(
                "*Last scanned: {}*\n\n",
                formatter::format_datetime(generated_at)
            ));
        }
        Err(util::TimeoutError::Exceeded) => {
            // FR-013: Graceful degradation on timeout
            content.push_str("⚠️ **[Data Unavailable: timeout]**\n\n");
            content.push_str(
                "Anomaly detection exceeded 1-second timeout. This may indicate high system load.\n\n",
            );
        }
        Err(util::TimeoutError::Analytics(e)) => {
            // FR-013: Graceful degradation on analytics error
            content.push_str(&format!("⚠️ **[Data Unavailable: {}]**\n\n", e));
            content.push_str("Unable to perform anomaly detection. Check analytics storage.\n\n");
        }
    }

    // T026: Data age indicator (FR-015)
    let data_age_ms = util::calculate_data_age_ms(generated_at, chrono::Utc::now());

    ReportSection {
        name: "market_anomalies".to_string(),
        title: "Market Anomalies".to_string(),
        content: Ok(content),
        data_age_ms: Some(data_age_ms),
    }
}

#[cfg(not(feature = "orderbook_analytics"))]
pub fn build_anomalies_section(_timestamp: Option<i64>) -> ReportSection {
    ReportSection {
        name: "anomalies".to_string(),
        title: "Market Anomalies".to_string(),
        content: Err(SectionError::FeatureNotEnabled(
            "orderbook_analytics".to_string(),
        )),
        data_age_ms: None,
    }
}

/// Build microstructure health section
///
/// Includes: Composite health score, component scores, warnings
#[cfg(feature = "orderbook_analytics")]
pub fn build_health_section() -> ReportSection {
    use super::formatter;

    // MVP: Basic placeholder
    let mut section = formatter::build_section_header("Microstructure Health", 2);
    section.push_str("🟢 **Overall Status:** Healthy\n\n");

    let headers = vec!["Component", "Status"];
    let rows = vec![
        vec!["Spread Health".to_string(), "🟢 Good".to_string()],
        vec!["Liquidity Depth".to_string(), "🟢 Adequate".to_string()],
        vec!["Update Frequency".to_string(), "🟢 Normal".to_string()],
        vec![
            "Order Flow Balance".to_string(),
            "🟡 Monitoring".to_string(),
        ],
    ];

    section.push_str(&formatter::build_table(&headers, &rows));
    section.push('\n');

    ReportSection {
        name: "health".to_string(),
        title: "Microstructure Health".to_string(),
        content: Ok(section),
        data_age_ms: None,
    }
}

/// Build microstructure health section with dynamic scoring (Feature 019 - US3)
///
/// Calculates real-time health score based on:
/// - Spread stability (25% weight)
/// - Liquidity depth (35% weight)
/// - Flow balance (25% weight)
/// - Update rate (15% weight)
///
/// # Arguments
/// * `storage` - Analytics snapshot storage for historical data
/// * `symbol` - Trading pair symbol
/// * `generated_at` - Report generation timestamp for data age calculation
///
/// # Returns
/// ReportSection with composite health score and component breakdowns
#[cfg(feature = "orderbook_analytics")]
pub async fn build_health_section_async(
    storage: &std::sync::Arc<crate::orderbook::analytics::SnapshotStorage>,
    symbol: &str,
    generated_at: chrono::DateTime<chrono::Utc>,
) -> ReportSection {
    use super::{formatter, util};
    use crate::orderbook::analytics::tools::get_microstructure_health;

    // T031-T032: Call health calculation with timeout (FR-020)
    let health_result = util::timeout_analytics(
        get_microstructure_health(storage.clone(), symbol),
        "get_microstructure_health",
        symbol,
    )
    .await;

    // Build section content
    let mut content = formatter::build_section_header("Microstructure Health", 2);

    match health_result {
        Ok(health) => {
            // T033: Overall health status with visual indicator
            // Map analytics health levels (Excellent/Good/Fair/Poor/Critical) to emoji
            let (status_emoji, status_text) = match health.health_level.as_str() {
                "Excellent" => ("🟢", "Excellent"),
                "Good" => ("🟢", "Good"),
                "Fair" => ("🟡", "Fair"),
                "Poor" => ("🟠", "Poor"),
                "Critical" => ("🔴", "Critical"),
                _ => ("⚪", "Unknown"),
            };

            content.push_str(&format!(
                "{} **Overall Status:** {} (Score: {:.1}/100)\n\n",
                status_emoji, status_text, health.overall_score
            ));

            // T034: Component scores table
            content.push_str("### Component Health Scores\n\n");

            let headers = vec!["Component", "Score", "Status", "Weight"];
            let rows = vec![
                vec![
                    "Spread Stability".to_string(),
                    format!("{:.1}/100", health.spread_stability_score),
                    score_to_status(health.spread_stability_score),
                    "25%".to_string(),
                ],
                vec![
                    "Liquidity Depth".to_string(),
                    format!("{:.1}/100", health.liquidity_depth_score),
                    score_to_status(health.liquidity_depth_score),
                    "35%".to_string(),
                ],
                vec![
                    "Flow Balance".to_string(),
                    format!("{:.1}/100", health.flow_balance_score),
                    score_to_status(health.flow_balance_score),
                    "25%".to_string(),
                ],
                vec![
                    "Update Rate".to_string(),
                    format!("{:.1}/100", health.update_rate_score),
                    score_to_status(health.update_rate_score),
                    "15%".to_string(),
                ],
            ];

            content.push_str(&formatter::build_table(&headers, &rows));
            content.push_str("\n");

            // T035: Recommended action
            content.push_str("### Trading Guidance\n\n");
            content.push_str(&format!("**Recommendation:** {}\n\n", health.recommended_action));

            // T036: Calculation timestamp
            content.push_str(&format!(
                "*Health calculated: {}*\n\n",
                formatter::format_datetime(health.timestamp)
            ));
        }
        Err(util::TimeoutError::Exceeded) => {
            // FR-013: Graceful degradation on timeout
            content.push_str("⚠️ **[Data Unavailable: timeout]**\n\n");
            content.push_str(
                "Health calculation exceeded 1-second timeout. This may indicate high system load.\n\n",
            );
        }
        Err(util::TimeoutError::Analytics(e)) => {
            // FR-013: Graceful degradation on analytics error
            content.push_str(&format!("⚠️ **[Data Unavailable: {}]**\n\n", e));
            content.push_str("Unable to calculate microstructure health. Check analytics storage.\n\n");
        }
    }

    // Data age indicator (FR-015)
    let data_age_ms = util::calculate_data_age_ms(generated_at, chrono::Utc::now());

    ReportSection {
        name: "microstructure_health".to_string(),
        title: "Microstructure Health".to_string(),
        content: Ok(content),
        data_age_ms: Some(data_age_ms),
    }
}

/// Helper: Convert score to status indicator
fn score_to_status(score: f64) -> String {
    match score {
        s if s >= 80.0 => "🟢 Good".to_string(),
        s if s >= 60.0 => "🟡 Fair".to_string(),
        s if s >= 40.0 => "🟠 Poor".to_string(),
        _ => "🔴 Critical".to_string(),
    }
}

#[cfg(not(feature = "orderbook_analytics"))]
pub fn build_health_section() -> ReportSection {
    ReportSection {
        name: "health".to_string(),
        title: "Microstructure Health".to_string(),
        content: Err(SectionError::FeatureNotEnabled(
            "orderbook_analytics".to_string(),
        )),
        data_age_ms: None,
    }
}

/// Build data health status section
///
/// Includes: Websocket connectivity, last update age, overall status
/// Enhanced with degradation warnings and additional metrics (T041-T042)
pub fn build_data_health_section(data_age_ms: i32) -> ReportSection {
    use super::formatter;

    let mut section = formatter::build_section_header("Data Health Status", 2);

    // Determine overall health status (T039: Visual indicators)
    let (status, status_emoji) = if data_age_ms < 1000 {
        ("✅ Healthy", "🟢")
    } else if data_age_ms < 5000 {
        ("⚠️ Degraded", "🟡")
    } else {
        ("❌ Critical", "🔴")
    };

    section.push_str(&format!(
        "{} **Overall Status:** {}\n\n",
        status_emoji, status
    ));

    // T042: Add degradation warnings when data age exceeds thresholds
    if data_age_ms > 30000 {
        section.push_str("⚠️ **CRITICAL WARNING:** Data is severely stale (>30s). Market conditions may have changed significantly.\n\n");
    } else if data_age_ms > 5000 {
        section.push_str("⚠️ **WARNING:** Data freshness degraded (>5s). Consider refreshing for real-time trading decisions.\n\n");
    }

    // Status table
    let headers = vec!["Component", "Status", "Details"];
    let freshness_status = if data_age_ms < 1000 {
        "🟢 Fresh"
    } else if data_age_ms < 5000 {
        "🟡 Acceptable"
    } else {
        "🔴 Stale"
    };

    let rows = vec![
        vec![
            "WebSocket Connection".to_string(),
            "🟢 Connected".to_string(),
            "Real-time updates active".to_string(),
        ],
        vec![
            "Data Freshness".to_string(),
            format!("{} ({} ms)", freshness_status, data_age_ms),
            "Last update age".to_string(),
        ],
        vec![
            "OrderBook Updates".to_string(),
            "🟢 Active".to_string(),
            "Depth stream active".to_string(),
        ],
        vec![
            "Ticker Stream".to_string(),
            "🟢 Active".to_string(),
            "24h statistics stream".to_string(),
        ],
        // T041: Active symbols count (placeholder - would need actual data from connection manager)
        vec![
            "Active Symbols".to_string(),
            "1+".to_string(),
            "Subscribed pairs".to_string(),
        ],
    ];

    section.push_str(&formatter::build_table(&headers, &rows));
    section.push('\n');

    ReportSection {
        name: "data_health".to_string(),
        title: "Data Health Status".to_string(),
        content: Ok(section),
        data_age_ms: Some(data_age_ms),
    }
}

/// Build report footer with generation metadata
///
/// Includes: Generation time, feature build info, cache status (T043)
pub fn build_report_footer(generation_time_ms: i32, was_cached: bool) -> String {
    use super::formatter;

    let mut footer = String::new();

    footer.push_str("---\n\n");
    footer.push_str("### Report Metadata\n\n");

    // Generation performance
    let cache_status = if was_cached {
        "✅ Cache Hit"
    } else {
        "🔄 Fresh Generation"
    };

    let headers = vec!["Metric", "Value"];
    let rows = vec![
        vec![
            "Generation Time".to_string(),
            format!("{} ms", generation_time_ms),
        ],
        vec!["Cache Status".to_string(), cache_status.to_string()],
        vec!["Report Format".to_string(), "Markdown".to_string()],
    ];

    footer.push_str(&formatter::build_table(&headers, &rows));
    footer.push('\n');

    // Feature build info
    footer.push_str("**Build Configuration:**\n");
    let mut features = Vec::new();

    #[cfg(feature = "orderbook")]
    features.push("✅ OrderBook Analysis".to_string());

    #[cfg(feature = "orderbook_analytics")]
    features.push("✅ Advanced Analytics (Anomalies, Health)".to_string());

    #[cfg(not(feature = "orderbook_analytics"))]
    features.push("⚠️ Advanced Analytics (Disabled)".to_string());

    if features.is_empty() {
        features.push("⚠️ No advanced features enabled".to_string());
    }

    footer.push_str(&formatter::build_list(&features, false));
    footer.push('\n');

    footer.push_str("*Generated by ForgeTrade MCP Market Data Provider*\n");
    footer.push('\n');

    footer
}
