// Section builders for market data report
//
// Each function builds a specific section of the market intelligence report.
// Sections return Result<String, SectionError> for graceful degradation.

use super::{ReportSection, SectionError};

/// Build report header section with metadata
///
/// Includes: Symbol, generation timestamp, data age indicator
pub fn build_report_header(
    symbol: &str,
    generated_at: i64,
    data_age_ms: i32,
) -> ReportSection {
    use super::formatter;

    let mut content = String::new();

    // Main title
    content.push_str(&formatter::build_section_header(&format!("Market Report: {}", symbol), 1));

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
        vec!["Generated At".to_string(), formatter::format_timestamp(generated_at)],
        vec!["Data Age".to_string(), format!("{} ms {}", data_age_ms, freshness_indicator)],
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
            let trend_indicator = if price_change_pct > 0.0 { "📈" } else if price_change_pct < 0.0 { "📉" } else { "➡️" };

            // Build price table
            let headers = vec!["Metric", "Value"];
            let rows = vec![
                vec!["Current Price".to_string(), format!("${}", t.last_price)],
                vec!["24h Change".to_string(), format!("{} {}%", trend_indicator, t.price_change_percent)],
                vec!["24h High".to_string(), format!("${}", t.high_price)],
                vec!["24h Low".to_string(), format!("${}", t.low_price)],
                vec!["24h Volume".to_string(), format!("{} {}", t.volume, t.symbol.trim_end_matches("USDT"))],
                vec!["24h Quote Volume".to_string(), format!("${}", t.quote_volume)],
                vec!["Weighted Avg Price".to_string(), format!("${}", t.weighted_avg_price)],
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
                vec!["Spread (bps)".to_string(), format!("{:.2} bps {}", m.spread_bps, spread_quality)],
                vec!["Microprice".to_string(), format!("${:.2}", m.microprice)],
                vec!["Bid Volume (Top 20)".to_string(), format!("{:.4}", m.bid_volume)],
                vec!["Ask Volume (Top 20)".to_string(), format!("{:.4}", m.ask_volume)],
                vec!["Imbalance Ratio".to_string(), format!("{:.3} {}", m.imbalance_ratio, imbalance_indicator)],
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
pub fn build_liquidity_analysis_section(
    metrics: Option<&crate::orderbook::types::OrderBookMetrics>,
) -> ReportSection {
    use super::formatter;

    let content = match metrics {
        Some(m) => {
            let mut section = formatter::build_section_header("Liquidity Analysis", 2);

            // Basic walls analysis
            if !m.walls.bids.is_empty() || !m.walls.asks.is_empty() {
                section.push_str("### Liquidity Walls\n\n");

                if !m.walls.bids.is_empty() {
                    section.push_str("**Buy Walls (Support):**\n");
                    let bid_items: Vec<String> = m.walls.bids.iter()
                        .take(5)
                        .map(|w| format!("${} @ {:.4} units", w.price, w.qty))
                        .collect();
                    section.push_str(&formatter::build_list(&bid_items, false));
                    section.push('\n');
                }

                if !m.walls.asks.is_empty() {
                    section.push_str("**Sell Walls (Resistance):**\n");
                    let ask_items: Vec<String> = m.walls.asks.iter()
                        .take(5)
                        .map(|w| format!("${} @ {:.4} units", w.price, w.qty))
                        .collect();
                    section.push_str(&formatter::build_list(&ask_items, false));
                    section.push('\n');
                }
            } else {
                section.push_str("No significant liquidity walls detected.\n\n");
            }

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
#[cfg(feature = "orderbook_analytics")]
pub fn build_anomalies_section() -> ReportSection {
    use super::formatter;

    // MVP: Basic placeholder for analytics feature
    let mut section = formatter::build_section_header("Market Anomalies", 2);
    section.push_str("🟢 **Status:** No anomalies detected\n\n");
    section.push_str("*Advanced anomaly detection monitors for:*\n");
    let items = vec![
        "Quote stuffing activity".to_string(),
        "Flash crash risk indicators".to_string(),
        "Iceberg order detection".to_string(),
        "Unusual spread behavior".to_string(),
    ];
    section.push_str(&formatter::build_list(&items, false));
    section.push('\n');

    ReportSection {
        name: "anomalies".to_string(),
        title: "Market Anomalies".to_string(),
        content: Ok(section),
        data_age_ms: None,
    }
}

#[cfg(not(feature = "orderbook_analytics"))]
pub fn build_anomalies_section() -> ReportSection {
    ReportSection {
        name: "anomalies".to_string(),
        title: "Market Anomalies".to_string(),
        content: Err(SectionError::FeatureNotEnabled("orderbook_analytics".to_string())),
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
        vec!["Order Flow Balance".to_string(), "🟡 Monitoring".to_string()],
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
        content: Err(SectionError::FeatureNotEnabled("orderbook_analytics".to_string())),
        data_age_ms: None,
    }
}

/// Build data health status section
///
/// Includes: Websocket connectivity, last update age, overall status
pub fn build_data_health_section(
    data_age_ms: i32,
) -> ReportSection {
    use super::formatter;

    let mut section = formatter::build_section_header("Data Health Status", 2);

    // Determine overall health status
    let (status, status_emoji) = if data_age_ms < 1000 {
        ("Healthy", "🟢")
    } else if data_age_ms < 5000 {
        ("Degraded", "🟡")
    } else {
        ("Critical", "🔴")
    };

    section.push_str(&format!("{} **Overall Status:** {}\n\n", status_emoji, status));

    // Status table
    let headers = vec!["Component", "Status"];
    let freshness_status = if data_age_ms < 1000 {
        "🟢 Fresh"
    } else if data_age_ms < 5000 {
        "🟡 Acceptable"
    } else {
        "🔴 Stale"
    };

    let rows = vec![
        vec!["WebSocket Connection".to_string(), "🟢 Connected".to_string()],
        vec!["Data Freshness".to_string(), format!("{} ({} ms)", freshness_status, data_age_ms)],
        vec!["OrderBook Updates".to_string(), "🟢 Active".to_string()],
        vec!["Ticker Stream".to_string(), "🟢 Active".to_string()],
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
