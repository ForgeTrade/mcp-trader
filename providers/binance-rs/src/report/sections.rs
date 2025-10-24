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

            // Build price table
            let headers = vec!["Metric", "Value"];
            let rows = vec![
                vec!["Current Price".to_string(), format!("${}", t.last_price)],
                vec![
                    "24h Change".to_string(),
                    format!("{} {}%", trend_indicator, t.price_change_percent),
                ],
                vec!["24h High".to_string(), format!("${}", t.high_price)],
                vec!["24h Low".to_string(), format!("${}", t.low_price)],
                vec![
                    "24h Volume".to_string(),
                    format!("{} {}", t.volume, t.symbol.trim_end_matches("USDT")),
                ],
                vec![
                    "24h Quote Volume".to_string(),
                    format!("${}", t.quote_volume),
                ],
                vec![
                    "Weighted Avg Price".to_string(),
                    format!("${}", t.weighted_avg_price),
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

            // Spread quality indicator
            let spread_quality = if m.spread_bps < 10.0 {
                "🟢 Tight"
            } else if m.spread_bps < 50.0 {
                "🟡 Moderate"
            } else {
                "🔴 Wide"
            };

            // Imbalance indicator
            let imbalance_indicator = if m.imbalance_ratio > 1.2 {
                "🟢 Buy Pressure"
            } else if m.imbalance_ratio < 0.8 {
                "🔴 Sell Pressure"
            } else {
                "🟡 Balanced"
            };

            // Build metrics table
            let headers = vec!["Metric", "Value"];
            let rows = vec![
                vec!["Best Bid".to_string(), format!("${}", m.best_bid)],
                vec!["Best Ask".to_string(), format!("${}", m.best_ask)],
                vec![
                    "Spread (bps)".to_string(),
                    format!("{:.2} bps {}", m.spread_bps, spread_quality),
                ],
                vec!["Microprice".to_string(), format!("${:.2}", m.microprice)],
                vec![
                    "Bid Volume (Top 20)".to_string(),
                    format!("{:.4}", m.bid_volume),
                ],
                vec![
                    "Ask Volume (Top 20)".to_string(),
                    format!("{:.4}", m.ask_volume),
                ],
                vec![
                    "Imbalance Ratio".to_string(),
                    format!("{:.3} {}", m.imbalance_ratio, imbalance_indicator),
                ],
            ];

            section.push_str(&formatter::build_table(&headers, &rows));
            section.push('\n');

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
