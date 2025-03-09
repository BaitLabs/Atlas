//! Atlas Agent - Agent system for the Atlas framework
//! 
//! This crate provides the agent implementation for the Atlas framework.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use atlas_core::{Agent as CoreAgent, AgentConfig, AgentState, Metadata, Tool};
use atlas_mcp::{MCPTool, ToolInfo};

pub mod error;
pub mod state;
pub mod tool;
pub mod types;

// Re-exports
pub use error::Error;
pub use state::AgentStateManager;
pub use tool::ToolManager;
pub use types::{AgentContext, AgentResponse, TaskConfig};

/// Agent configuration
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    /// Agent name
    pub name: String,
    
    /// Agent description
    pub description: Option<String>,
    
    /// Agent capabilities
    pub capabilities: Vec<String>,
    
    /// Agent configuration
    pub config: Metadata,
}

impl AgentConfig for Config {
    fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(Error::InvalidConfig("Agent name is required".to_string()).into());
        }
        Ok(())
    }
}

/// Agent state
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct State {
    /// Agent memory
    pub memory: HashMap<String, serde_json::Value>,
    
    /// Active tasks
    pub tasks: HashMap<Uuid, TaskState>,
}

impl AgentState for State {
    fn update(&mut self, data: Metadata) -> Result<()> {
        // Update memory with new data
        for (key, value) in data.into_iter() {
            self.memory.insert(key, value);
        }
        Ok(())
    }

    fn snapshot(&self) -> Result<Metadata> {
        Ok(Metadata::from(self.memory.clone()))
    }
}

/// Task state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskState {
    /// Task ID
    pub id: Uuid,
    
    /// Task status
    pub status: TaskStatus,
    
    /// Task result
    pub result: Option<Metadata>,
    
    /// Task error
    pub error: Option<String>,
}

/// Task status
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is pending
    Pending,
    
    /// Task is running
    Running,
    
    /// Task completed successfully
    Completed,
    
    /// Task failed
    Failed,
}

/// Atlas agent builder
#[derive(Default)]
pub struct AgentBuilder {
    config: Option<Config>,
    tools: Vec<(String, Box<dyn MCPTool>)>,
    state: Option<State>,
}

impl AgentBuilder {
    /// Create a new agent builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the agent configuration
    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    /// Add a tool to the agent
    pub fn tool<T>(mut self, name: impl Into<String>, tool: T) -> Self
    where
        T: MCPTool + 'static,
    {
        self.tools.push((name.into(), Box::new(tool)));
        self
    }

    /// Set the initial agent state
    pub fn state(mut self, state: State) -> Self {
        self.state = Some(state);
        self
    }

    /// Build the agent
    pub fn build(self) -> Result<Agent> {
        let config = self.config.ok_or_else(|| {
            Error::InvalidConfig("Agent configuration is required".to_string())
        })?;

        let mut tool_manager = ToolManager::new();
        for (name, tool) in self.tools {
            tool_manager.register(name, tool);
        }

        let state = self.state.unwrap_or_default();

        Ok(Agent {
            config,
            state: Arc::new(RwLock::new(state)),
            tools: Arc::new(RwLock::new(tool_manager)),
        })
    }
}

/// Atlas agent
pub struct Agent {
    config: Config,
    state: Arc<RwLock<State>>,
    tools: Arc<RwLock<ToolManager>>,
}

#[async_trait]
impl CoreAgent for Agent {
    type Config = Config;
    type State = State;

    async fn new(config: Self::Config) -> Result<Self> {
        AgentBuilder::new()
            .config(config)
            .build()
    }

    async fn state(&self) -> Result<Arc<RwLock<Self::State>>> {
        Ok(self.state.clone())
    }

    async fn handle_event(&self, event: atlas_core::Event) -> Result<()> {
        let mut state = self.state.write().await;
        state.update(event.payload)?;
        Ok(())
    }

    async fn execute_task(&self, task_id: atlas_core::TaskId, params: Metadata) -> Result<Metadata> {
        let mut state = self.state.write().await;
        
        // Create task state
        let task_state = TaskState {
            id: *task_id,
            status: TaskStatus::Running,
            result: None,
            error: None,
        };
        state.tasks.insert(*task_id, task_state);

        // Execute task
        match self.execute_with_tools(params).await {
            Ok(result) => {
                state.tasks.get_mut(task_id).unwrap().status = TaskStatus::Completed;
                state.tasks.get_mut(task_id).unwrap().result = Some(result.clone());
                Ok(result)
            }
            Err(e) => {
                state.tasks.get_mut(task_id).unwrap().status = TaskStatus::Failed;
                state.tasks.get_mut(task_id).unwrap().error = Some(e.to_string());
                Err(e)
            }
        }
    }
}

impl Agent {
    /// Create a new agent builder
    pub fn builder() -> AgentBuilder {
        AgentBuilder::new()
    }

    /// Get the agent's configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get a list of available tools
    pub async fn list_tools(&self) -> Result<Vec<ToolInfo>> {
        let tools = self.tools.read().await;
        let mut tool_list = Vec::new();

        for (name, tool) in tools.tools.iter() {
            tool_list.push(ToolInfo {
                name: name.clone(),
                description: tool.description().to_string(),
                input_schema: None,
            });
        }

        Ok(tool_list)
    }

    /// Execute a task using available tools
    async fn execute_with_tools(&self, params: Metadata) -> Result<Metadata> {
        let tools = self.tools.read().await;
        
        // Get tool name from params
        let tool_name = params
            .get("tool")
            .ok_or_else(|| Error::InvalidRequest("Tool name is required".to_string()))?;

        // Get tool
        let tool = tools
            .get(&tool_name)
            .ok_or_else(|| Error::ToolNotFound(tool_name))?;

        // Execute tool
        tool.execute(params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        async fn execute(&self, _params: Metadata) -> Result<Metadata> {
            let mut result = Metadata::new();
            result.insert("success", true);
            Ok(result)
        }
    }

    #[tokio::test]
    async fn test_agent_builder() {
        let config = Config {
            name: "test_agent".to_string(),
            description: None,
            capabilities: vec![],
            config: Metadata::new(),
        };

        let agent = AgentBuilder::new()
            .config(config)
            .tool("test_tool", TestTool)
            .build()
            .unwrap();

        let tools = agent.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");
    }

    #[tokio::test]
    async fn test_agent_execution() {
        let config = Config {
            name: "test_agent".to_string(),
            description: None,
            capabilities: vec![],
            config: Metadata::new(),
        };

        let agent = AgentBuilder::new()
            .config(config)
            .tool("test_tool", TestTool)
            .build()
            .unwrap();

        let mut params = Metadata::new();
        params.insert("tool", "test_tool");

        let result = agent.execute_task(atlas_core::TaskId::new(), params).await.unwrap();
        assert_eq!(result.get::<bool>("success"), Some(true));
    }
}
