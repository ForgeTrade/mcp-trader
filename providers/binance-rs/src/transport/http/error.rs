//! HTTP transport error handling
//!
//! Converts internal errors to JSON-RPC error responses with appropriate
//! HTTP status codes and error details.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use super::jsonrpc::{JsonRpcError, JsonRpcResponse};
use super::session::SessionError;

/// HTTP transport errors
#[derive(Debug, thiserror::Error)]
pub enum HttpTransportError {
    #[error("Session error: {0}")]
    Session(#[from] SessionError),

    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Invalid JSON-RPC request: {0}")]
    InvalidRequest(String),

    #[error("Method not found: {0}")]
    MethodNotFound(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Provider error: {0}")]
    Provider(#[from] crate::error::ProviderError),
}

impl HttpTransportError {
    /// Convert to JSON-RPC error code
    pub fn to_jsonrpc_error(&self) -> JsonRpcError {
        match self {
            HttpTransportError::Session(SessionError::SessionNotFound(_)) => {
                JsonRpcError::session_missing()
            }
            HttpTransportError::Session(SessionError::SessionExpired(_)) => {
                JsonRpcError::session_invalid()
            }
            HttpTransportError::Session(SessionError::SessionLimitExceeded(max)) => {
                JsonRpcError::session_limit_exceeded(*max)
            }
            HttpTransportError::Session(SessionError::InvalidSessionId) => {
                JsonRpcError::session_missing()
            }
            HttpTransportError::JsonParse(_) => JsonRpcError::parse_error(),
            HttpTransportError::InvalidRequest(msg) => {
                JsonRpcError::new(-32600, format!("Invalid Request: {}", msg))
            }
            HttpTransportError::MethodNotFound(method) => {
                JsonRpcError::new(-32601, format!("Method not found: {}", method))
            }
            HttpTransportError::InvalidParams(msg) => {
                JsonRpcError::new(-32602, format!("Invalid params: {}", msg))
            }
            HttpTransportError::Internal(msg) => {
                JsonRpcError::new(-32603, format!("Internal error: {}", msg))
            }
            HttpTransportError::Provider(err) => {
                JsonRpcError::new(-32603, format!("Provider error: {}", err))
            }
        }
    }

    /// Get HTTP status code for error
    pub fn status_code(&self) -> StatusCode {
        match self {
            HttpTransportError::Session(SessionError::SessionLimitExceeded(_)) => {
                StatusCode::TOO_MANY_REQUESTS
            }
            HttpTransportError::Session(_) => StatusCode::UNAUTHORIZED,
            HttpTransportError::JsonParse(_) => StatusCode::BAD_REQUEST,
            HttpTransportError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            HttpTransportError::MethodNotFound(_) => StatusCode::NOT_FOUND,
            HttpTransportError::InvalidParams(_) => StatusCode::BAD_REQUEST,
            HttpTransportError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            HttpTransportError::Provider(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for HttpTransportError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let jsonrpc_error = self.to_jsonrpc_error();

        // Create JSON-RPC error response with null id (since we don't have request context)
        let response = JsonRpcResponse::error(jsonrpc_error, serde_json::json!(null));

        (status, Json(response)).into_response()
    }
}

/// Result type for HTTP transport operations
pub type Result<T> = std::result::Result<T, HttpTransportError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_error_conversion() {
        let err = HttpTransportError::Session(SessionError::SessionNotFound(
            uuid::Uuid::new_v4(),
        ));

        let jsonrpc_err = err.to_jsonrpc_error();
        assert_eq!(jsonrpc_err.code, -32002);
        assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_invalid_request_conversion() {
        let err = HttpTransportError::InvalidRequest("Missing field 'method'".to_string());

        let jsonrpc_err = err.to_jsonrpc_error();
        assert_eq!(jsonrpc_err.code, -32600);
        assert!(jsonrpc_err.message.contains("Invalid Request"));
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_method_not_found_conversion() {
        let err = HttpTransportError::MethodNotFound("unknown/method".to_string());

        let jsonrpc_err = err.to_jsonrpc_error();
        assert_eq!(jsonrpc_err.code, -32601);
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_session_limit_conversion() {
        let err = HttpTransportError::Session(SessionError::SessionLimitExceeded(50));

        let jsonrpc_err = err.to_jsonrpc_error();
        assert_eq!(jsonrpc_err.code, -32000);
        assert!(jsonrpc_err.data.is_some());
        assert_eq!(err.status_code(), StatusCode::TOO_MANY_REQUESTS);
    }
}
