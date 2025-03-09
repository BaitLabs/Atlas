//! HTTP handlers for the MCP server endpoints

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ServerState, MCPTool, MCPResource};
use atlas_core::Metadata;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthCheck {
    status: String,
    version: String,
}

/// Tool information response
#[derive(Debug, Serialize)]
pub struct ToolInfo {
    name: String,
    description: String,
}

/// Resource information response
#[derive(Debug, Serialize)]
pub struct ResourceInfo {
    name: String,
    resource_type: String,
}

/// Tool execution request
#[derive(Debug, Deserialize)]
pub struct ExecuteToolRequest {
    params: Value,
}

/// Tool execution response
#[derive(Debug, Serialize)]
pub struct ExecuteToolResponse {
    success: bool,
    result: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Health check handler
pub async fn health_check(
    State(state): State<Arc<ServerState>>,
) -> Json<HealthCheck> {
    Json(HealthCheck {
        status: "ok".to_string(),
        version: state.config.version.clone(),
    })
}

/// List available tools
pub async fn list_tools(
    State(state): State<Arc<ServerState>>,
) -> Json<Vec<ToolInfo>> {
    let tools = state.tools.read().await;
    let mut tool_list = Vec::new();

    for (name, tool) in tools.tools.iter() {
        tool_list.push(ToolInfo {
            name: name.clone(),
            description: tool.description().to_string(),
        });
    }

    Json(tool_list)
}

/// Execute a tool
pub async fn execute_tool(
    State(state): State<Arc<ServerState>>,
    Path(tool_name): Path<String>,
    Json(request): Json<ExecuteToolRequest>,
) -> Result<Json<ExecuteToolResponse>, StatusCode> {
    let tools = state.tools.read().await;
    
    let tool = tools
        .get(&tool_name)
        .ok_or(StatusCode::NOT_FOUND)?;

    let params = Metadata::from(request.params);
    
    match tool.execute(params).await {
        Ok(result) => Ok(Json(ExecuteToolResponse {
            success: true,
            result: serde_json::to_value(result).unwrap(),
            error: None,
        })),
        Err(err) => Ok(Json(ExecuteToolResponse {
            success: false,
            result: Value::Null,
            error: Some(err.to_string()),
        })),
    }
}

/// List available resources
pub async fn list_resources(
    State(state): State<Arc<ServerState>>,
) -> Json<Vec<ResourceInfo>> {
    let resources = state.resources.read().await;
    let mut resource_list = Vec::new();

    for (name, resource) in resources.resources.iter() {
        resource_list.push(ResourceInfo {
            name: name.clone(),
            resource_type: resource.resource_type().to_string(),
        });
    }

    Json(resource_list)
}

/// Access a resource
pub async fn access_resource(
    State(state): State<Arc<ServerState>>,
    Path(resource_name): Path<String>,
    Json(params): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let resources = state.resources.read().await;
    
    let resource = resources
        .get(&resource_name)
        .ok_or(StatusCode::NOT_FOUND)?;

    let params = Metadata::from(params);
    
    match resource.access(params).await {
        Ok(result) => Ok(Json(serde_json::to_value(result).unwrap())),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ServerConfig, ServerCapabilities};
    use anyhow::Result;
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

        async fn execute(&self, _params: Metadata) -> Result<Metadata> {
            let mut result = Metadata::new();
            result.insert("success", true);
            Ok(result)
        }
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = ServerConfig {
            name: "test".to_string(),
            version: "0.1.0".to_string(),
            description: None,
            capabilities: ServerCapabilities::default(),
        };
        let state = Arc::new(ServerState::new(config));
        
        let response = health_check(State(state.clone())).await;
        assert_eq!(response.0.status, "ok");
        assert_eq!(response.0.version, "0.1.0");
    }

    #[tokio::test]
    async fn test_list_tools() {
        let config = ServerConfig {
            name: "test".to_string(),
            version: "0.1.0".to_string(),
            description: None,
            capabilities: ServerCapabilities::default(),
        };
        let state = Arc::new(ServerState::new(config));
        
        state.tools.write().await.register("test_tool".to_string(), TestTool);
        
        let response = list_tools(State(state.clone())).await;
        assert_eq!(response.0.len(), 1);
        assert_eq!(response.0[0].name, "test_tool");
        assert_eq!(response.0[0].description, "A test tool");
    }
}
