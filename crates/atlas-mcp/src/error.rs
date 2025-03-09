//! Error types for the MCP server

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// MCP server error types
#[derive(Debug, Error)]
pub enum Error {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),

    #[error("Resource access failed: {0}")]
    ResourceAccessFailed(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Error response for the API
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code
    pub code: ErrorCode,
    
    /// Error message
    pub message: String,
    
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Error codes for the API
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Tool not found
    ToolNotFound,
    
    /// Resource not found
    ResourceNotFound,
    
    /// Invalid request
    InvalidRequest,
    
    /// Tool execution failed
    ToolExecutionFailed,
    
    /// Resource access failed
    ResourceAccessFailed,
    
    /// Server error
    ServerError,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::ToolNotFound => write!(f, "tool_not_found"),
            ErrorCode::ResourceNotFound => write!(f, "resource_not_found"),
            ErrorCode::InvalidRequest => write!(f, "invalid_request"),
            ErrorCode::ToolExecutionFailed => write!(f, "tool_execution_failed"),
            ErrorCode::ResourceAccessFailed => write!(f, "resource_access_failed"),
            ErrorCode::ServerError => write!(f, "server_error"),
        }
    }
}

impl From<Error> for ErrorResponse {
    fn from(err: Error) -> Self {
        match err {
            Error::ToolNotFound(msg) => Self {
                code: ErrorCode::ToolNotFound,
                message: msg,
                details: None,
            },
            Error::ResourceNotFound(msg) => Self {
                code: ErrorCode::ResourceNotFound,
                message: msg,
                details: None,
            },
            Error::InvalidRequest(msg) => Self {
                code: ErrorCode::InvalidRequest,
                message: msg,
                details: None,
            },
            Error::ToolExecutionFailed(msg) => Self {
                code: ErrorCode::ToolExecutionFailed,
                message: msg,
                details: None,
            },
            Error::ResourceAccessFailed(msg) => Self {
                code: ErrorCode::ResourceAccessFailed,
                message: msg,
                details: None,
            },
            Error::ServerError(msg) => Self {
                code: ErrorCode::ServerError,
                message: msg,
                details: None,
            },
            Error::Other(err) => Self {
                code: ErrorCode::ServerError,
                message: err.to_string(),
                details: None,
            },
        }
    }
}

impl From<Error> for axum::http::StatusCode {
    fn from(err: Error) -> Self {
        match err {
            Error::ToolNotFound(_) | Error::ResourceNotFound(_) => {
                axum::http::StatusCode::NOT_FOUND
            }
            Error::InvalidRequest(_) => axum::http::StatusCode::BAD_REQUEST,
            Error::ToolExecutionFailed(_) | Error::ResourceAccessFailed(_) => {
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            }
            Error::ServerError(_) | Error::Other(_) => {
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

/// Result type for MCP operations
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response_conversion() {
        let err = Error::ToolNotFound("test_tool".to_string());
        let response: ErrorResponse = err.into();
        
        assert_eq!(response.code, ErrorCode::ToolNotFound);
        assert_eq!(response.message, "test_tool");
        assert!(response.details.is_none());
    }

    #[test]
    fn test_status_code_conversion() {
        let err = Error::ToolNotFound("test_tool".to_string());
        let status: axum::http::StatusCode = err.into();
        
        assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_error_display() {
        let err = Error::InvalidRequest("bad request".to_string());
        assert_eq!(err.to_string(), "Invalid request: bad request");
    }
}
