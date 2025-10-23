use crate::error::Result;
use crate::pb::*;

/// Builder for constructing the Capabilities response
pub struct CapabilityBuilder {
    tools: Vec<Tool>,
    resources: Vec<Resource>,
    prompts: Vec<Prompt>,
}

impl CapabilityBuilder {
    /// Create a new CapabilityBuilder with all binance-rs capabilities
    pub fn new() -> Self {
        let mut builder = Self {
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
        };

        // Add ONLY the unified market data report tool (per FR-002)
        builder.add_unified_report_tool();

        // Add resources
        builder.add_resources();

        // Add prompts
        builder.add_prompts();

        builder
    }

    /// Build the final Capabilities message
    pub fn build(self) -> Result<Capabilities> {
        Ok(Capabilities {
            tools: self.tools,
            resources: self.resources,
            prompts: self.prompts,
            provider_version: "0.1.0".to_string(),
        })
    }

    // Helper to create Json from string
    fn json_schema(schema: &str) -> Option<Json> {
        Some(Json {
            value: schema.as_bytes().to_vec(),
        })
    }

    // ========== Unified Market Data Report Tool (THE ONLY PUBLIC TOOL) ==========
    // Per FR-002: All market data methods consolidated into single unified method

    fn add_unified_report_tool(&mut self) {
        #[cfg(feature = "orderbook")]
        self.tools.push(Tool {
            name: "binance.generate_market_report".to_string(),
            description: "Generate comprehensive market intelligence report combining price, orderbook, volume, and analytics (THE ONLY PUBLIC TOOL - per FR-002)".to_string(),
            input_schema: Self::json_schema(
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {
      "type": "string",
      "description": "Trading pair symbol (e.g., BTCUSDT)",
      "pattern": "^[A-Z0-9]{6,12}$"
    },
    "options": {
      "type": "object",
      "description": "Report generation options",
      "properties": {
        "include_sections": {
          "type": "array",
          "description": "Section names to include (omit for all sections)",
          "items": {"type": "string"}
        },
        "volume_window_hours": {
          "type": "integer",
          "description": "Time window for volume profile (hours)",
          "minimum": 1,
          "maximum": 168,
          "default": 24
        },
        "orderbook_levels": {
          "type": "integer",
          "description": "Number of orderbook levels for depth analysis",
          "minimum": 1,
          "maximum": 100,
          "default": 20
        }
      },
      "additionalProperties": false
    }
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
            output_schema: None,
        });
    }

    // ========== DEPRECATED: Individual Market Data Tools (Removed per FR-002) ==========
    // These methods are commented out as they've been consolidated into generate_market_report

    #[allow(dead_code)]
    fn add_market_data_tools(&mut self) {
        // Tool 1: Get 24h ticker
        self.tools.push(Tool {
            name: "binance.get_ticker".to_string(),
            description: "Get 24-hour rolling window price change statistics for a symbol"
                .to_string(),
            input_schema: Self::json_schema(
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {
      "type": "string",
      "description": "Trading pair symbol (e.g., BTCUSDT)",
      "pattern": "^[A-Z0-9]{6,12}$"
    }
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
            output_schema: None,
        });

        // Tool 2: Get orderbook snapshot
        self.tools.push(Tool {
            name: "binance.get_orderbook".to_string(),
            description: "Get current order book (bids and asks) for a symbol".to_string(),
            input_schema: Self::json_schema(
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {
      "type": "string",
      "description": "Trading pair symbol (e.g., BTCUSDT)",
      "pattern": "^[A-Z0-9]{6,12}$"
    },
    "limit": {
      "type": "integer",
      "description": "Number of price levels to return (5, 10, 20, 50, 100, 500, 1000, 5000)",
      "enum": [5, 10, 20, 50, 100, 500, 1000, 5000],
      "default": 100
    }
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
            output_schema: None,
        });

        // Tool 3: Get recent trades
        self.tools.push(Tool {
            name: "binance.get_recent_trades".to_string(),
            description: "Get recent trades for a symbol (up to last 1000 trades)".to_string(),
            input_schema: Self::json_schema(
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {
      "type": "string",
      "description": "Trading pair symbol (e.g., BTCUSDT)",
      "pattern": "^[A-Z0-9]{6,12}$"
    },
    "limit": {
      "type": "integer",
      "description": "Number of trades to return",
      "minimum": 1,
      "maximum": 1000,
      "default": 500
    }
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
            output_schema: None,
        });

        // Tool 4: Get candlestick data (klines)
        self.tools.push(Tool {
            name: "binance.get_klines".to_string(),
            description: "Get candlestick/kline data for a symbol (OHLCV)".to_string(),
            input_schema: Self::json_schema(r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {
      "type": "string",
      "description": "Trading pair symbol (e.g., BTCUSDT)",
      "pattern": "^[A-Z0-9]{6,12}$"
    },
    "interval": {
      "type": "string",
      "description": "Candlestick interval",
      "enum": ["1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "6h", "8h", "12h", "1d", "3d", "1w", "1M"]
    },
    "limit": {
      "type": "integer",
      "description": "Number of candlesticks to return",
      "minimum": 1,
      "maximum": 1000,
      "default": 500
    },
    "start_time": {
      "type": "integer",
      "description": "Start time in milliseconds (optional)"
    },
    "end_time": {
      "type": "integer",
      "description": "End time in milliseconds (optional)"
    }
  },
  "required": ["symbol", "interval"],
  "additionalProperties": false
}"#),
            output_schema: None,
        });

        // Tool 5: Get exchange info
        self.tools.push(Tool {
            name: "binance.get_exchange_info".to_string(),
            description: "Get exchange trading rules and symbol information".to_string(),
            input_schema: Self::json_schema(
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {
      "type": "string",
      "description": "Trading pair symbol (optional, omit for all symbols)",
      "pattern": "^[A-Z0-9]{6,12}$"
    }
  },
  "additionalProperties": false
}"#,
            ),
            output_schema: None,
        });

        // Tool 6: Get average price
        self.tools.push(Tool {
            name: "binance.get_avg_price".to_string(),
            description: "Get current average price for a symbol".to_string(),
            input_schema: Self::json_schema(
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {
      "type": "string",
      "description": "Trading pair symbol (e.g., BTCUSDT)",
      "pattern": "^[A-Z0-9]{6,12}$"
    }
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
            output_schema: None,
        });

        // Tool 7: Generate unified market report (requires orderbook feature)
        #[cfg(feature = "orderbook")]
        self.tools.push(Tool {
            name: "binance.generate_market_report".to_string(),
            description: "Generate comprehensive market report combining price, orderbook, volume, and analytics".to_string(),
            input_schema: Self::json_schema(
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {
      "type": "string",
      "description": "Trading pair symbol (e.g., BTCUSDT)",
      "pattern": "^[A-Z0-9]{6,12}$"
    },
    "options": {
      "type": "object",
      "description": "Report generation options",
      "properties": {
        "include_sections": {
          "type": "array",
          "description": "Section names to include (omit for all sections)",
          "items": {"type": "string"}
        },
        "volume_window_hours": {
          "type": "integer",
          "description": "Time window for volume profile (hours)",
          "minimum": 1,
          "maximum": 168,
          "default": 24
        },
        "orderbook_levels": {
          "type": "integer",
          "description": "Number of orderbook levels for depth analysis",
          "minimum": 1,
          "maximum": 100,
          "default": 20
        }
      },
      "additionalProperties": false
    }
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
            output_schema: None,
        });
    }



    // ========== OrderBook Analysis Tools (Feature-gated) ==========

    #[cfg(feature = "orderbook")]
    fn add_orderbook_tools(&mut self) {
        let orderbook_tools = vec![
            (
                "binance.orderbook_l1",
                "Get Level 1 orderbook metrics (best bid/ask, spread)",
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {"type": "string", "pattern": "^[A-Z0-9]{6,12}$"}
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
            (
                "binance.orderbook_l2",
                "Get Level 2 orderbook metrics (depth, liquidity)",
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {"type": "string", "pattern": "^[A-Z0-9]{6,12}$"},
    "depth_levels": {"type": "integer", "minimum": 5, "maximum": 100, "default": 20}
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
            (
                "binance.orderbook_health",
                "Get orderbook health metrics",
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {"type": "string", "pattern": "^[A-Z0-9]{6,12}$"}
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
        ];

        for (name, desc, schema) in orderbook_tools {
            self.tools.push(Tool {
                name: name.to_string(),
                description: desc.to_string(),
                input_schema: Self::json_schema(schema),
                output_schema: None,
            });
        }
    }

    // ========== Analytics Tools (Feature-gated) ==========

    #[cfg(feature = "orderbook_analytics")]
    fn add_analytics_tools(&mut self) {
        let analytics_tools = vec![
            (
                "binance.get_order_flow",
                "Analyze bid/ask order flow dynamics over a time window",
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {"type": "string", "pattern": "^[A-Z]+$", "description": "Trading pair (e.g., BTCUSDT)"},
    "window_duration_secs": {"type": "integer", "minimum": 10, "maximum": 300, "default": 60, "description": "Analysis window in seconds"}
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
            (
                "binance.get_volume_profile",
                "Generate volume profile histogram with POC and value area",
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {"type": "string", "pattern": "^[A-Z]+$", "description": "Trading pair"},
    "duration_hours": {"type": "integer", "minimum": 1, "maximum": 168, "default": 24, "description": "Time period in hours"},
    "tick_size": {"type": "number", "description": "Optional bin size"}
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
            (
                "binance.detect_market_anomalies",
                "Detect quote stuffing, iceberg orders, and flash crash risks",
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {"type": "string", "pattern": "^[A-Z]+$", "description": "Trading pair"}
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
            (
                "binance.get_microstructure_health",
                "Get composite market health score (0-100) with component breakdowns",
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {"type": "string", "pattern": "^[A-Z]+$", "description": "Trading pair"}
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
            (
                "binance.get_liquidity_vacuums",
                "Identify price zones with low volume for stop-loss placement",
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {"type": "string", "pattern": "^[A-Z]+$", "description": "Trading pair"},
    "duration_hours": {"type": "integer", "minimum": 1, "maximum": 168, "default": 24, "description": "Lookback period in hours"}
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
        ];

        for (name, desc, schema) in analytics_tools {
            self.tools.push(Tool {
                name: name.to_string(),
                description: desc.to_string(),
                input_schema: Self::json_schema(schema),
                output_schema: None,
            });
        }
    }

    // ========== Resources ==========

    fn add_resources(&mut self) {
        self.resources.push(Resource {
            uri_scheme: "binance".to_string(),
            description:
                "Binance cryptocurrency trading resources (market data, balances, trades, orders)"
                    .to_string(),
            mime_type: "text/markdown".to_string(),
        });
    }

    // ========== Prompts ==========

    fn add_prompts(&mut self) {
        self.prompts.push(Prompt {
            name: "trading-analysis".to_string(),
            description: "Analyze market conditions and suggest trading strategies for a symbol"
                .to_string(),
            args_schema: Self::json_schema(
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {"type": "string", "description": "Trading pair symbol"},
    "timeframe": {"type": "string", "description": "Analysis timeframe (1h, 4h, 1d)"}
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
        });

    }
}

impl Default for CapabilityBuilder {
    fn default() -> Self {
        Self::new()
    }
}
