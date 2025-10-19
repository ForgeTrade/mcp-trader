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

        // Add all tools
        builder.add_market_data_tools();
        builder.add_account_tools();
        builder.add_order_tools();

        #[cfg(feature = "orderbook")]
        builder.add_orderbook_tools();

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

    // ========== Market Data Tools (Public, No Auth Required) ==========

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
    }

    // ========== Account Tools (Authenticated) ==========

    fn add_account_tools(&mut self) {
        // Tool 7: Get account information
        self.tools.push(Tool {
            name: "binance.get_account".to_string(),
            description: "Get current account information including balances (requires API key)"
                .to_string(),
            input_schema: Self::json_schema(
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {},
  "additionalProperties": false
}"#,
            ),
            output_schema: None,
        });

        // Tool 8: Get my trades
        self.tools.push(Tool {
            name: "binance.get_my_trades".to_string(),
            description: "Get account trade history for a symbol (requires API key)".to_string(),
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
    },
    "from_id": {
      "type": "integer",
      "description": "Trade ID to fetch from (optional)"
    }
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
            output_schema: None,
        });
    }

    // ========== Order Management Tools (Authenticated) ==========

    fn add_order_tools(&mut self) {
        // Tool 9: Place order
        self.tools.push(Tool {
            name: "binance.place_order".to_string(),
            description: "Create a new order (requires API key)".to_string(),
            input_schema: Self::json_schema(r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {
      "type": "string",
      "description": "Trading pair symbol (e.g., BTCUSDT)",
      "pattern": "^[A-Z0-9]{6,12}$"
    },
    "side": {
      "type": "string",
      "description": "Order side",
      "enum": ["BUY", "SELL"]
    },
    "order_type": {
      "type": "string",
      "description": "Order type",
      "enum": ["LIMIT", "MARKET", "STOP_LOSS", "STOP_LOSS_LIMIT", "TAKE_PROFIT", "TAKE_PROFIT_LIMIT", "LIMIT_MAKER"]
    },
    "quantity": {
      "type": "string",
      "description": "Order quantity",
      "pattern": "^[0-9]+(\\.[0-9]+)?$"
    },
    "price": {
      "type": "string",
      "description": "Order price (required for LIMIT orders)",
      "pattern": "^[0-9]+(\\.[0-9]+)?$"
    },
    "time_in_force": {
      "type": "string",
      "description": "Time in force (required for LIMIT orders)",
      "enum": ["GTC", "IOC", "FOK"]
    },
    "stop_price": {
      "type": "string",
      "description": "Stop price (required for STOP orders)",
      "pattern": "^[0-9]+(\\.[0-9]+)?$"
    }
  },
  "required": ["symbol", "side", "order_type", "quantity"],
  "additionalProperties": false
}"#),
            output_schema: None,
        });

        // Remaining order tools...
        let order_tools = vec![
            (
                "binance.cancel_order",
                "Cancel an active order (requires API key)",
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {"type": "string", "pattern": "^[A-Z0-9]{6,12}$"},
    "order_id": {"type": "integer"},
    "orig_client_order_id": {"type": "string"}
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
            (
                "binance.get_order",
                "Query order status (requires API key)",
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {"type": "string", "pattern": "^[A-Z0-9]{6,12}$"},
    "order_id": {"type": "integer"},
    "orig_client_order_id": {"type": "string"}
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
            (
                "binance.get_open_orders",
                "Get all open orders (requires API key)",
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {"type": "string", "pattern": "^[A-Z0-9]{6,12}$"}
  },
  "additionalProperties": false
}"#,
            ),
            (
                "binance.get_all_orders",
                "Get all orders for a symbol (requires API key)",
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "symbol": {"type": "string", "pattern": "^[A-Z0-9]{6,12}$"},
    "limit": {"type": "integer", "minimum": 1, "maximum": 1000, "default": 500}
  },
  "required": ["symbol"],
  "additionalProperties": false
}"#,
            ),
        ];

        for (name, desc, schema) in order_tools {
            self.tools.push(Tool {
                name: name.to_string(),
                description: desc.to_string(),
                input_schema: Self::json_schema(schema),
                output_schema: None,
            });
        }
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

        self.prompts.push(Prompt {
            name: "portfolio-risk".to_string(),
            description: "Assess portfolio risk and suggest rebalancing strategies".to_string(),
            args_schema: Self::json_schema(
                r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "risk_tolerance": {"type": "string", "enum": ["low", "medium", "high"]}
  },
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
