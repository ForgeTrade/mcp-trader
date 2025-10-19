use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Prompt not found: {0}")]
    PromptNotFound(String),

    #[error("Invalid URI: {0}")]
    InvalidUri(String),

    #[error("Binance API error: {0}")]
    BinanceApi(String),

    #[error("Authentication required: {0}")]
    AuthRequired(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Initialization error: {0}")]
    Initialization(String),

    #[error("MCP error: {0}")]
    Mcp(#[from] McpError),
}

/// Main error type for MCP Binance Server (from mcp-binance-rs)
#[derive(Error, Debug)]
pub enum McpError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Server not ready: {0}")]
    NotReady(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl McpError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            McpError::ConnectionError(_) | McpError::RateLimitError(_)
        )
    }

    pub fn error_type(&self) -> &'static str {
        match self {
            McpError::ConnectionError(_) => "connection_error",
            McpError::RateLimitError(_) => "rate_limit",
            McpError::ParseError(_) => "parse_error",
            McpError::InvalidRequest(_) => "invalid_request",
            McpError::NotReady(_) => "not_ready",
            McpError::InternalError(_) => "internal_error",
        }
    }
}

impl From<reqwest::Error> for McpError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            McpError::ConnectionError(
                "Request timeout. Please check your internet connection.".to_string(),
            )
        } else if err.is_connect() {
            McpError::ConnectionError(
                "Failed to connect to Binance API. Please check your internet connection."
                    .to_string(),
            )
        } else if let Some(status) = err.status() {
            match status.as_u16() {
                429 => McpError::RateLimitError(
                    "Too many requests to Binance API. Retry after 60 seconds.".to_string(),
                ),
                418 => McpError::ConnectionError(
                    "IP address banned by Binance. Please contact support.".to_string(),
                ),
                403 => McpError::ConnectionError(
                    "WAF limit violated. Please reduce request frequency.".to_string(),
                ),
                500..=599 => McpError::ConnectionError(format!(
                    "Binance server error (HTTP {}). Please try again later.",
                    status.as_u16()
                )),
                _ => McpError::InternalError(format!("HTTP error: {}", status)),
            }
        } else {
            McpError::InternalError(err.to_string())
        }
    }
}

impl From<serde_json::Error> for McpError {
    fn from(err: serde_json::Error) -> Self {
        McpError::ParseError(format!("JSON parsing failed: {}", err))
    }
}

pub type Result<T> = std::result::Result<T, ProviderError>;

impl From<ProviderError> for tonic::Status {
    fn from(err: ProviderError) -> Self {
        match err {
            ProviderError::ToolNotFound(msg) => tonic::Status::not_found(msg),
            ProviderError::ResourceNotFound(msg) => tonic::Status::not_found(msg),
            ProviderError::PromptNotFound(msg) => tonic::Status::not_found(msg),
            ProviderError::AuthRequired(msg) => tonic::Status::unauthenticated(msg),
            ProviderError::Validation(msg) => tonic::Status::invalid_argument(msg),
            _ => tonic::Status::internal(err.to_string()),
        }
    }
}
