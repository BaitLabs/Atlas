//! Atlas MCP - Model Context Protocol implementation
//! 
//! This crate provides the MCP server implementation for the Atlas framework.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use atlas_core::{Metadata, Resource, Tool};

pub mod error;
pub mod handler;
pub mod server;
pub mod types;

// Re-exports
pub use error::Error;
pub use server::MCPServer;
pub use types::{MCPRequest, MCPResponse, MCPTool, MCPResource};

/// MCP server configuration
#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    /// Server name
    pub name: String,
    
    /// Server version
    pub version: String,
    
    /// Server description
    pub description: Option<String>,
    
    /// Server capabilities
    pub capabilities: ServerCapabilities,
}

/// Server capabilities configuration
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ServerCapabilities {
    /// Available tools
    pub tools: Vec<String>,
    
    /// Available resources
    pub resources: Vec<String>,
}

/// MCP tool registry
#[derive(Debug, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn MCPTool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register<T>(&mut self, name: String, tool: T)
    where
        T: MCPTool + 'static,
    {
        self.tools.insert(name, Arc::new(tool));
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn MCPTool>> {
        self.tools.get(name).cloned()
    }
}

/// MCP resource registry
#[derive(Debug, Default)]
pub struct ResourceRegistry {
    resources: HashMap<String, Arc<dyn MCPResource>>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    pub fn register<R>(&mut self, name: String, resource: R)
    where
        R: MCPResource + 'static,
    {
        self.resources.insert(name, Arc::new(resource));
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn MCPResource>> {
        self.resources.get(name).cloned()
    }
}

/// MCP tool trait
#[async_trait]
pub trait MCPTool: Send + Sync {
    /// Get the tool's name
    fn name(&self) -> &str;
    
    /// Get the tool's description
    fn description(&self) -> &str;
    
    /// Execute the tool with the given parameters
    async fn execute(&self, params: Metadata) -> Result<Metadata>;
}

/// MCP resource trait
#[async_trait]
pub trait MCPResource: Send + Sync {
    /// Get the resource's name
    fn name(&self) -> &str;
    
    /// Get the resource's type
    fn resource_type(&self) -> &str;
    
    /// Access the resource with the given parameters
    async fn access(&self, params: Metadata) -> Result<Metadata>;
}

/// MCP server state
#[derive(Debug)]
pub struct ServerState {
    /// Server configuration
    pub config: ServerConfig,
    
    /// Tool registry
    pub tools: Arc<RwLock<ToolRegistry>>,
    
    /// Resource registry
    pub resources: Arc<RwLock<ResourceRegistry>>,
}

impl ServerState {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            tools: Arc::new(RwLock::new(ToolRegistry::new())),
            resources: Arc::new(RwLock::new(ResourceRegistry::new())),
        }
    }
}

/// Create the Axum router for the MCP server
pub fn create_router(state: ServerState) -> Router {
    Router::new()
        .route("/", get(handler::health_check))
        .route("/tools", get(handler::list_tools))
        .route("/tools/:name", post(handler::execute_tool))
        .route("/resources", get(handler::list_resources))
        .route("/resources/:name", get(handler::access_resource))
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[derive(Clone)]
    struct TestTool;

    #[async_trait]
    impl MCPTool for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }

        fn description(&self) -> &str {
            "A test tool"
        }

        async fn execute(&self, params: Metadata) -> Result<Metadata> {
            let mut result = Metadata::new();
            result.insert("success", true);
            Ok(result)
        }
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        registry.register("test_tool".to_string(), TestTool);

        let tool = registry.get("test_tool").unwrap();
        assert_eq!(tool.name(), "test_tool");
        assert_eq!(tool.description(), "A test tool");

        let result = tool.execute(Metadata::new()).await.unwrap();
        assert_eq!(result.get::<bool>("success"), Some(true));
    }
}
