//! Tool management for agents

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use atlas_core::Metadata;
use atlas_mcp::MCPTool;

use crate::error::Error;

/// Tool configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolConfig {
    /// Tool name
    pub name: String,
    
    /// Tool description
    pub description: String,
    
    /// Tool configuration
    pub config: Metadata,
    
    /// Tool input schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>,
}

/// Tool execution context
#[derive(Clone, Debug)]
pub struct ToolContext {
    /// Tool configuration
    pub config: ToolConfig,
    
    /// Execution parameters
    pub params: Metadata,
}

impl ToolContext {
    /// Create a new tool context
    pub fn new(config: ToolConfig, params: Metadata) -> Self {
        Self { config, params }
    }

    /// Validate the parameters against the input schema
    pub fn validate(&self) -> Result<()> {
        if let Some(schema) = &self.config.input_schema {
            // TODO: Implement JSON Schema validation
            Ok(())
        } else {
            Ok(())
        }
    }
}

/// Tool registry for managing agent tools
#[derive(Debug, Default)]
pub struct ToolManager {
    /// Registered tools
    pub(crate) tools: HashMap<String, Arc<dyn MCPTool>>,
    
    /// Tool configurations
    configs: HashMap<String, ToolConfig>,
}

impl ToolManager {
    /// Create a new tool manager
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register<T>(&mut self, name: String, tool: T)
    where
        T: MCPTool + 'static,
    {
        let config = ToolConfig {
            name: name.clone(),
            description: tool.description().to_string(),
            config: Metadata::new(),
            input_schema: None,
        };
        
        self.configs.insert(name.clone(), config);
        self.tools.insert(name, Arc::new(tool));
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn MCPTool>> {
        self.tools.get(name).cloned()
    }

    /// Get a tool's configuration
    pub fn get_config(&self, name: &str) -> Option<&ToolConfig> {
        self.configs.get(name)
    }

    /// Update a tool's configuration
    pub fn update_config(&mut self, name: &str, config: ToolConfig) -> Result<()> {
        if !self.tools.contains_key(name) {
            return Err(Error::ToolNotFound(name.to_string()).into());
        }
        self.configs.insert(name.to_string(), config);
        Ok(())
    }

    /// List all registered tools
    pub fn list_tools(&self) -> Vec<&ToolConfig> {
        self.configs.values().collect()
    }

    /// Create a tool execution context
    pub fn create_context(&self, name: &str, params: Metadata) -> Result<ToolContext> {
        let config = self.get_config(name)
            .ok_or_else(|| Error::ToolNotFound(name.to_string()))?
            .clone();
            
        let context = ToolContext::new(config, params);
        context.validate()?;
        
        Ok(context)
    }
}

/// Tool execution middleware
pub trait ToolMiddleware: Send + Sync {
    /// Process the tool execution
    fn process<'a>(
        &'a self,
        context: &'a ToolContext,
        next: Box<dyn FnOnce(&'a ToolContext) -> Result<Metadata> + 'a>,
    ) -> Result<Metadata>;
}

/// Tool execution pipeline
pub struct ToolPipeline {
    /// Tool manager
    manager: ToolManager,
    
    /// Middleware chain
    middleware: Vec<Box<dyn ToolMiddleware>>,
}

impl ToolPipeline {
    /// Create a new tool pipeline
    pub fn new(manager: ToolManager) -> Self {
        Self {
            manager,
            middleware: Vec::new(),
        }
    }

    /// Add middleware to the pipeline
    pub fn with_middleware<M>(mut self, middleware: M) -> Self
    where
        M: ToolMiddleware + 'static,
    {
        self.middleware.push(Box::new(middleware));
        self
    }

    /// Execute a tool with the middleware chain
    pub async fn execute(&self, name: &str, params: Metadata) -> Result<Metadata> {
        let context = self.manager.create_context(name, params)?;
        
        let tool = self.manager
            .get(name)
            .ok_or_else(|| Error::ToolNotFound(name.to_string()))?;

        let mut middleware_chain = self.middleware.iter();
        
        fn create_next<'a>(
            mut middleware_chain: std::slice::Iter<'a, Box<dyn ToolMiddleware>>,
            tool: &'a Arc<dyn MCPTool>,
            context: &'a ToolContext,
        ) -> Box<dyn FnOnce(&'a ToolContext) -> Result<Metadata> + 'a> {
            if let Some(middleware) = middleware_chain.next() {
                Box::new(move |ctx| {
                    middleware.process(
                        ctx,
                        create_next(middleware_chain, tool, context),
                    )
                })
            } else {
                Box::new(move |ctx| tool.execute(ctx.params.clone()))
            }
        }

        let next = create_next(middleware_chain, &tool, &context);
        next(&context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

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

    struct LoggingMiddleware;

    impl ToolMiddleware for LoggingMiddleware {
        fn process<'a>(
            &'a self,
            context: &'a ToolContext,
            next: Box<dyn FnOnce(&'a ToolContext) -> Result<Metadata> + 'a>,
        ) -> Result<Metadata> {
            println!("Executing tool: {}", context.config.name);
            let result = next(context);
            println!("Tool execution completed");
            result
        }
    }

    #[test]
    fn test_tool_registration() {
        let mut manager = ToolManager::new();
        manager.register("test_tool".to_string(), TestTool);

        let tool = manager.get("test_tool").unwrap();
        assert_eq!(tool.name(), "test_tool");
        assert_eq!(tool.description(), "A test tool");
    }

    #[test]
    fn test_tool_config() {
        let mut manager = ToolManager::new();
        manager.register("test_tool".to_string(), TestTool);

        let config = manager.get_config("test_tool").unwrap();
        assert_eq!(config.name, "test_tool");
        assert_eq!(config.description, "A test tool");
    }

    #[tokio::test]
    async fn test_tool_pipeline() {
        let mut manager = ToolManager::new();
        manager.register("test_tool".to_string(), TestTool);

        let pipeline = ToolPipeline::new(manager)
            .with_middleware(LoggingMiddleware);

        let result = pipeline
            .execute("test_tool", Metadata::new())
            .await
            .unwrap();

        assert_eq!(result.get::<bool>("success"), Some(true));
    }
}
