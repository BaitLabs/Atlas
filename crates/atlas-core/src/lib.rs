//! Atlas Core - Fundamental components for building AI agents
//! 
//! This crate provides the core traits and types used throughout the Atlas framework.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

pub mod agent;
pub mod error;
pub mod event;
pub mod state;
pub mod types;

// Re-exports
pub use agent::{Agent, AgentConfig, AgentState};
pub use error::{Error, ErrorKind};
pub use event::{Event, EventBus, EventHandler};
pub use state::{State, StateManager};
pub use types::{Metadata, Resource, TaskId, Tool};

/// Core error types for the Atlas framework
#[derive(Debug, Error)]
pub enum Error {
    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("State error: {0}")]
    State(String),

    #[error("Tool error: {0}")]
    Tool(String),

    #[error("Event error: {0}")]
    Event(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Common metadata type used throughout the framework
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Metadata(HashMap<String, serde_json::Value>);

impl Metadata {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert<K, V>(&mut self, key: K, value: V) -> Option<serde_json::Value>
    where
        K: Into<String>,
        V: Serialize,
    {
        self.0.insert(
            key.into(),
            serde_json::to_value(value).expect("Failed to serialize value"),
        )
    }

    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.0
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

/// Base trait for all agents in the framework
#[async_trait]
pub trait Agent: Send + Sync {
    /// The configuration type for this agent
    type Config: AgentConfig;
    
    /// The state type for this agent
    type State: AgentState;

    /// Initialize a new agent instance
    async fn new(config: Self::Config) -> Result<Self>
    where
        Self: Sized;

    /// Get the agent's current state
    async fn state(&self) -> Result<Arc<RwLock<Self::State>>>;

    /// Process an incoming event
    async fn handle_event(&self, event: Event) -> Result<()>;

    /// Execute a task with the given parameters
    async fn execute_task(&self, task_id: TaskId, params: Metadata) -> Result<Metadata>;
}

/// Configuration trait for agents
pub trait AgentConfig: Clone + fmt::Debug + Send + Sync {
    /// Validate the configuration
    fn validate(&self) -> Result<()>;
}

/// State management trait for agents
pub trait AgentState: Clone + fmt::Debug + Send + Sync {
    /// Update the state with new data
    fn update(&mut self, data: Metadata) -> Result<()>;
    
    /// Get a snapshot of the current state
    fn snapshot(&self) -> Result<Metadata>;
}

/// Event system for inter-agent communication
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Event {
    /// Unique identifier for this event
    pub id: Uuid,
    
    /// The type of event
    pub event_type: String,
    
    /// Event payload
    pub payload: Metadata,
    
    /// Event metadata
    pub metadata: Metadata,
}

impl Event {
    pub fn new<T: Into<String>>(event_type: T, payload: Metadata) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: event_type.into(),
            payload,
            metadata: Metadata::new(),
        }
    }
}

/// Task identifier type
#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct TaskId(Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource type for managing agent capabilities
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Resource {
    /// Unique identifier for this resource
    pub id: Uuid,
    
    /// Resource name
    pub name: String,
    
    /// Resource type
    pub resource_type: String,
    
    /// Resource configuration
    pub config: Metadata,
}

impl Resource {
    pub fn new<T: Into<String>>(name: T, resource_type: T, config: Metadata) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            resource_type: resource_type.into(),
            config,
        }
    }
}

/// Tool type for agent actions
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tool {
    /// Unique identifier for this tool
    pub id: Uuid,
    
    /// Tool name
    pub name: String,
    
    /// Tool description
    pub description: String,
    
    /// Tool configuration
    pub config: Metadata,
}

impl Tool {
    pub fn new<T: Into<String>>(name: T, description: T, config: Metadata) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: description.into(),
            config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let mut metadata = Metadata::new();
        metadata.insert("key", "value");
        
        assert_eq!(metadata.get::<String>("key"), Some("value".to_string()));
        assert_eq!(metadata.get::<String>("nonexistent"), None);
    }

    #[test]
    fn test_event() {
        let mut payload = Metadata::new();
        payload.insert("data", "test");
        
        let event = Event::new("test_event", payload);
        
        assert_eq!(event.event_type, "test_event");
        assert_eq!(
            event.payload.get::<String>("data"),
            Some("test".to_string())
        );
    }

    #[test]
    fn test_task_id() {
        let task_id = TaskId::new();
        let task_id2 = TaskId::new();
        
        assert_ne!(task_id, task_id2);
    }
}
