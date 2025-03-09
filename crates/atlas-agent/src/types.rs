//! Common types for the agent system

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use atlas_core::Metadata;
use atlas_mcp::ToolInfo;

/// Agent context for task execution
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentContext {
    /// Task ID
    pub task_id: Uuid,
    
    /// Task configuration
    pub task_config: TaskConfig,
    
    /// Available tools
    pub tools: Vec<ToolInfo>,
    
    /// Agent state snapshot
    pub state: Metadata,
    
    /// Context metadata
    pub metadata: Metadata,
}

impl AgentContext {
    /// Create a new agent context
    pub fn new(
        task_id: Uuid,
        task_config: TaskConfig,
        tools: Vec<ToolInfo>,
        state: Metadata,
    ) -> Self {
        Self {
            task_id,
            task_config,
            tools,
            state,
            metadata: Metadata::new(),
        }
    }

    /// Add metadata to the context
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Task configuration
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TaskConfig {
    /// Task name
    pub name: String,
    
    /// Task description
    pub description: Option<String>,
    
    /// Task parameters
    pub parameters: HashMap<String, Value>,
    
    /// Task constraints
    pub constraints: TaskConstraints,
    
    /// Task timeout in seconds
    pub timeout: Option<u64>,
}

/// Task constraints
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TaskConstraints {
    /// Required tools
    pub required_tools: Vec<String>,
    
    /// Maximum number of steps
    pub max_steps: Option<u32>,
    
    /// Maximum memory usage
    pub max_memory: Option<u64>,
    
    /// Maximum execution time in seconds
    pub max_time: Option<u64>,
}

/// Agent response types
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentResponse {
    /// Task completed successfully
    Success {
        /// Task result
        result: Metadata,
        
        /// Response metadata
        metadata: Metadata,
    },
    
    /// Task failed
    Error {
        /// Error message
        message: String,
        
        /// Error details
        details: Option<Value>,
        
        /// Response metadata
        metadata: Metadata,
    },
    
    /// Task progress update
    Progress {
        /// Progress message
        message: String,
        
        /// Progress percentage (0-100)
        percentage: f32,
        
        /// Progress details
        details: Option<Value>,
        
        /// Response metadata
        metadata: Metadata,
    },
}

impl AgentResponse {
    /// Create a success response
    pub fn success(result: Metadata) -> Self {
        Self::Success {
            result,
            metadata: Metadata::new(),
        }
    }

    /// Create an error response
    pub fn error<S: Into<String>>(message: S) -> Self {
        Self::Error {
            message: message.into(),
            details: None,
            metadata: Metadata::new(),
        }
    }

    /// Create a progress response
    pub fn progress<S: Into<String>>(message: S, percentage: f32) -> Self {
        Self::Progress {
            message: message.into(),
            percentage,
            details: None,
            metadata: Metadata::new(),
        }
    }

    /// Add metadata to the response
    pub fn with_metadata(self, metadata: Metadata) -> Self {
        match self {
            Self::Success { result, .. } => Self::Success { result, metadata },
            Self::Error { message, details, .. } => Self::Error {
                message,
                details,
                metadata,
            },
            Self::Progress {
                message,
                percentage,
                details,
                ..
            } => Self::Progress {
                message,
                percentage,
                details,
                metadata,
            },
        }
    }

    /// Add details to the response
    pub fn with_details(self, details: Value) -> Self {
        match self {
            Self::Error {
                message, metadata, ..
            } => Self::Error {
                message,
                details: Some(details),
                metadata,
            },
            Self::Progress {
                message,
                percentage,
                metadata,
                ..
            } => Self::Progress {
                message,
                percentage,
                details: Some(details),
                metadata,
            },
            _ => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_agent_context() {
        let context = AgentContext::new(
            Uuid::new_v4(),
            TaskConfig::default(),
            vec![],
            Metadata::new(),
        );

        let mut metadata = Metadata::new();
        metadata.insert("test", "value");

        let context = context.with_metadata(metadata);
        assert_eq!(
            context.metadata.get::<String>("test").unwrap(),
            "value"
        );
    }

    #[test]
    fn test_agent_response() {
        let mut result = Metadata::new();
        result.insert("success", true);

        let response = AgentResponse::success(result)
            .with_metadata({
                let mut m = Metadata::new();
                m.insert("test", "value");
                m
            });

        match response {
            AgentResponse::Success { result, metadata } => {
                assert!(result.get::<bool>("success").unwrap());
                assert_eq!(metadata.get::<String>("test").unwrap(), "value");
            }
            _ => panic!("Wrong response type"),
        }
    }

    #[test]
    fn test_error_response() {
        let response = AgentResponse::error("test error")
            .with_details(json!({"code": 404}));

        match response {
            AgentResponse::Error {
                message,
                details,
                metadata: _,
            } => {
                assert_eq!(message, "test error");
                assert_eq!(details.unwrap()["code"], 404);
            }
            _ => panic!("Wrong response type"),
        }
    }
}
