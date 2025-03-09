//! Core types for the MCP protocol

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use atlas_core::Metadata;

/// MCP request types
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MCPRequest {
    /// Execute a tool
    ExecuteTool {
        /// Tool name
        tool_name: String,
        
        /// Tool arguments
        arguments: Value,
    },
    
    /// Access a resource
    AccessResource {
        /// Resource URI
        uri: String,
    },
    
    /// List available tools
    ListTools,
    
    /// List available resources
    ListResources,
    
    /// List available resource templates
    ListResourceTemplates,
}

/// MCP response types
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MCPResponse {
    /// Tool execution response
    ToolResult {
        /// Whether the execution was successful
        success: bool,
        
        /// Result data
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Value>,
        
        /// Error message if execution failed
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    
    /// Resource access response
    ResourceContent {
        /// Resource contents
        contents: Vec<ResourceContent>,
    },
    
    /// Tool listing response
    Tools {
        /// Available tools
        tools: Vec<ToolInfo>,
    },
    
    /// Resource listing response
    Resources {
        /// Available resources
        resources: Vec<ResourceInfo>,
    },
    
    /// Resource template listing response
    ResourceTemplates {
        /// Available resource templates
        resource_templates: Vec<ResourceTemplate>,
    },
}

/// Tool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Tool name
    pub name: String,
    
    /// Tool description
    pub description: String,
    
    /// Tool input schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>,
}

/// Resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    /// Resource URI
    pub uri: String,
    
    /// Resource name
    pub name: String,
    
    /// Resource MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    
    /// Resource description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Resource template information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTemplate {
    /// Template URI pattern
    pub uri_template: String,
    
    /// Template name
    pub name: String,
    
    /// Resource MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    
    /// Template description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Resource content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    /// Resource URI
    pub uri: String,
    
    /// Content MIME type
    pub mime_type: String,
    
    /// Content text
    pub text: String,
}

/// JSON schema helper functions
pub mod schema {
    use serde_json::{json, Value};

    /// Create a string property schema
    pub fn string_property(description: &str) -> Value {
        json!({
            "type": "string",
            "description": description
        })
    }

    /// Create a number property schema
    pub fn number_property(description: &str) -> Value {
        json!({
            "type": "number",
            "description": description
        })
    }

    /// Create a boolean property schema
    pub fn boolean_property(description: &str) -> Value {
        json!({
            "type": "boolean",
            "description": description
        })
    }

    /// Create an array property schema
    pub fn array_property(items: Value, description: &str) -> Value {
        json!({
            "type": "array",
            "items": items,
            "description": description
        })
    }

    /// Create an object property schema
    pub fn object_property(
        properties: HashMap<String, Value>,
        required: Vec<String>,
        description: &str,
    ) -> Value {
        json!({
            "type": "object",
            "properties": properties,
            "required": required,
            "description": description
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_serialization() {
        let request = MCPRequest::ExecuteTool {
            tool_name: "test_tool".to_string(),
            arguments: json!({
                "param": "value"
            }),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: MCPRequest = serde_json::from_str(&json).unwrap();

        match parsed {
            MCPRequest::ExecuteTool { tool_name, arguments } => {
                assert_eq!(tool_name, "test_tool");
                assert_eq!(arguments["param"], "value");
            }
            _ => panic!("Wrong request type"),
        }
    }

    #[test]
    fn test_response_serialization() {
        let response = MCPResponse::ToolResult {
            success: true,
            data: Some(json!({
                "result": "success"
            })),
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("tool_result"));
        assert!(json.contains("success"));
        assert!(json.contains("result"));
    }

    #[test]
    fn test_schema_helpers() {
        let props = {
            let mut map = HashMap::new();
            map.insert(
                "name".to_string(),
                schema::string_property("The name field"),
            );
            map.insert(
                "count".to_string(),
                schema::number_property("The count field"),
            );
            map
        };

        let schema = schema::object_property(
            props,
            vec!["name".to_string()],
            "A test object",
        );

        assert_eq!(schema["type"], "object");
        assert!(schema["required"].as_array().unwrap().contains(&json!("name")));
    }
}
