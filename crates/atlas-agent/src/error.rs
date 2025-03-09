//! Error types for the agent system

use thiserror::Error;

/// Agent error types
#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),

    #[error("State error: {0}")]
    StateError(String),

    #[error("Task error: {0}")]
    TaskError(String),

    #[error("Memory error: {0}")]
    MemoryError(String),

    #[error(transparent)]
    Core(#[from] atlas_core::Error),

    #[error(transparent)]
    MCP(#[from] atlas_mcp::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type for agent operations
pub type Result<T> = std::result::Result<T, Error>;

impl From<Error> for atlas_core::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::InvalidConfig(msg) => atlas_core::Error::Config(msg),
            Error::InvalidRequest(msg) => atlas_core::Error::Agent(msg),
            Error::ToolNotFound(msg) => atlas_core::Error::Tool(msg),
            Error::ToolExecutionFailed(msg) => atlas_core::Error::Tool(msg),
            Error::StateError(msg) => atlas_core::Error::State(msg),
            Error::TaskError(msg) => atlas_core::Error::Agent(msg),
            Error::MemoryError(msg) => atlas_core::Error::State(msg),
            Error::Core(e) => e,
            Error::MCP(e) => atlas_core::Error::Other(e.into()),
            Error::Other(e) => atlas_core::Error::Other(e),
        }
    }
}

impl From<Error> for atlas_mcp::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::InvalidConfig(msg) => atlas_mcp::Error::InvalidRequest(msg),
            Error::InvalidRequest(msg) => atlas_mcp::Error::InvalidRequest(msg),
            Error::ToolNotFound(msg) => atlas_mcp::Error::ToolNotFound(msg),
            Error::ToolExecutionFailed(msg) => atlas_mcp::Error::ToolExecutionFailed(msg),
            Error::StateError(msg) => atlas_mcp::Error::ServerError(msg),
            Error::TaskError(msg) => atlas_mcp::Error::ServerError(msg),
            Error::MemoryError(msg) => atlas_mcp::Error::ServerError(msg),
            Error::Core(e) => atlas_mcp::Error::Other(e.into()),
            Error::MCP(e) => e,
            Error::Other(e) => atlas_mcp::Error::Other(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let err = Error::InvalidConfig("test error".to_string());
        let core_err: atlas_core::Error = err.clone().into();
        let mcp_err: atlas_mcp::Error = err.into();

        match core_err {
            atlas_core::Error::Config(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Wrong error type"),
        }

        match mcp_err {
            atlas_mcp::Error::InvalidRequest(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_error_display() {
        let err = Error::ToolNotFound("test_tool".to_string());
        assert_eq!(err.to_string(), "Tool not found: test_tool");
    }

    #[test]
    fn test_error_conversion_chain() {
        let err = Error::ToolExecutionFailed("test error".to_string());
        let core_err: atlas_core::Error = err.clone().into();
        let mcp_err: atlas_mcp::Error = core_err.into();

        match mcp_err {
            atlas_mcp::Error::Other(_) => (),
            _ => panic!("Wrong error type"),
        }
    }
}
