//! MCP server implementation

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    create_router, Error, MCPTool, MCPResource, ServerConfig, ServerState,
    ToolRegistry, ResourceRegistry,
};

/// MCP server builder
#[derive(Default)]
pub struct ServerBuilder {
    config: Option<ServerConfig>,
    tools: Vec<(String, Box<dyn MCPTool>)>,
    resources: Vec<(String, Box<dyn MCPResource>)>,
}

impl ServerBuilder {
    /// Create a new server builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the server configuration
    pub fn config(mut self, config: ServerConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Add a tool to the server
    pub fn tool<T>(mut self, name: impl Into<String>, tool: T) -> Self
    where
        T: MCPTool + 'static,
    {
        self.tools.push((name.into(), Box::new(tool)));
        self
    }

    /// Add a resource to the server
    pub fn resource<R>(mut self, name: impl Into<String>, resource: R) -> Self
    where
        R: MCPResource + 'static,
    {
        self.resources.push((name.into(), Box::new(resource)));
        self
    }

    /// Build the server
    pub fn build(self) -> Result<MCPServer> {
        let config = self.config.ok_or_else(|| {
            Error::ServerError("Server configuration is required".to_string())
        })?;

        let mut tool_registry = ToolRegistry::new();
        for (name, tool) in self.tools {
            tool_registry.register(name, tool);
        }

        let mut resource_registry = ResourceRegistry::new();
        for (name, resource) in self.resources {
            resource_registry.register(name, resource);
        }

        let state = ServerState {
            config,
            tools: Arc::new(RwLock::new(tool_registry)),
            resources: Arc::new(RwLock::new(resource_registry)),
        };

        Ok(MCPServer {
            state: Arc::new(state),
            router: create_router(state),
        })
    }
}

/// MCP server
pub struct MCPServer {
    state: Arc<ServerState>,
    router: Router,
}

impl MCPServer {
    /// Create a new server builder
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Get a reference to the server state
    pub fn state(&self) -> &Arc<ServerState> {
        &self.state
    }

    /// Start the server
    pub async fn serve(self, addr: SocketAddr) -> Result<()> {
        info!(
            "Starting MCP server '{}' on {}",
            self.state.config.name, addr
        );

        // Log available tools
        let tools = self.state.tools.read().await;
        let tool_count = tools.tools.len();
        if tool_count > 0 {
            info!("Registered {} tools:", tool_count);
            for (name, tool) in tools.tools.iter() {
                info!("  - {}: {}", name, tool.description());
            }
        } else {
            warn!("No tools registered");
        }

        // Log available resources
        let resources = self.state.resources.read().await;
        let resource_count = resources.resources.len();
        if resource_count > 0 {
            info!("Registered {} resources:", resource_count);
            for (name, resource) in resources.resources.iter() {
                info!(
                    "  - {} ({})",
                    name,
                    resource.resource_type()
                );
            }
        } else {
            warn!("No resources registered");
        }

        // Start the server
        axum::Server::bind(&addr)
            .serve(self.router.into_make_service())
            .await
            .map_err(|e| Error::ServerError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolInfo;
    use atlas_core::Metadata;

    #[derive(Clone)]
    struct TestTool;

    #[async_trait::async_trait]
    impl MCPTool for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }

        fn description(&self) -> &str {
            "A test tool"
        }

        async fn execute(&self, _params: Metadata) -> Result<Metadata> {
            Ok(Metadata::new())
        }
    }

    #[tokio::test]
    async fn test_server_builder() {
        let config = ServerConfig {
            name: "test_server".to_string(),
            version: "0.1.0".to_string(),
            description: None,
            capabilities: Default::default(),
        };

        let server = ServerBuilder::new()
            .config(config)
            .tool("test_tool", TestTool)
            .build()
            .unwrap();

        let tools = server.state.tools.read().await;
        assert_eq!(tools.tools.len(), 1);
        assert!(tools.tools.contains_key("test_tool"));
    }

    #[tokio::test]
    async fn test_tool_registration() {
        let config = ServerConfig {
            name: "test_server".to_string(),
            version: "0.1.0".to_string(),
            description: None,
            capabilities: Default::default(),
        };

        let server = ServerBuilder::new()
            .config(config)
            .tool("test_tool", TestTool)
            .build()
            .unwrap();

        let tools = server.state.tools.read().await;
        let tool = tools.get("test_tool").unwrap();
        assert_eq!(tool.name(), "test_tool");
        assert_eq!(tool.description(), "A test tool");
    }
}
