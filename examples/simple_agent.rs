//! Simple agent example demonstrating basic Atlas functionality

use std::net::SocketAddr;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use tokio;

use atlas_agent::{Agent, AgentBuilder, Config, State};
use atlas_core::Metadata;
use atlas_mcp::{MCPServer, MCPTool, ServerBuilder, ServerConfig, ServerCapabilities};

/// Simple calculator tool
#[derive(Clone)]
struct CalculatorTool;

#[async_trait]
impl MCPTool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> &str {
        "A simple calculator tool"
    }

    async fn execute(&self, params: Metadata) -> Result<Metadata> {
        let operation = params.get::<String>("operation")
            .ok_or_else(|| anyhow::anyhow!("Operation is required"))?;
        
        let a = params.get::<f64>("a")
            .ok_or_else(|| anyhow::anyhow!("Parameter 'a' is required"))?;
        
        let b = params.get::<f64>("b")
            .ok_or_else(|| anyhow::anyhow!("Parameter 'b' is required"))?;

        let result = match operation.as_str() {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => {
                if b == 0.0 {
                    return Err(anyhow::anyhow!("Division by zero"));
                }
                a / b
            }
            _ => return Err(anyhow::anyhow!("Unknown operation")),
        };

        let mut response = Metadata::new();
        response.insert("result", result);
        Ok(response)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create MCP server
    let server_config = ServerConfig {
        name: "example_server".to_string(),
        version: "0.1.0".to_string(),
        description: Some("Example MCP server".to_string()),
        capabilities: ServerCapabilities {
            tools: vec!["calculator".to_string()],
            resources: vec![],
        },
    };

    let server = ServerBuilder::new()
        .config(server_config)
        .tool("calculator", CalculatorTool)
        .build()?;

    // Start server in background
    let server_handle = tokio::spawn(async move {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        server.serve(addr).await
    });

    // Create agent
    let agent_config = Config {
        name: "calculator_agent".to_string(),
        description: Some("Agent that performs calculations".to_string()),
        capabilities: vec!["calculator".to_string()],
        config: Metadata::new(),
    };

    let agent = AgentBuilder::new()
        .config(agent_config)
        .tool("calculator", CalculatorTool)
        .build()?;

    // Execute some calculations
    let calculations = vec![
        ("add", 5.0, 3.0),
        ("subtract", 10.0, 4.0),
        ("multiply", 6.0, 7.0),
        ("divide", 15.0, 3.0),
    ];

    for (operation, a, b) in calculations {
        let mut params = Metadata::new();
        params.insert("tool", "calculator");
        params.insert("operation", operation);
        params.insert("a", a);
        params.insert("b", b);

        let result = agent
            .execute_task(atlas_core::TaskId::new(), params)
            .await?;

        println!(
            "{} {} {} = {}",
            a,
            match operation {
                "add" => "+",
                "subtract" => "-",
                "multiply" => "*",
                "divide" => "/",
                _ => "?",
            },
            b,
            result.get::<f64>("result").unwrap()
        );
    }

    // Try an invalid operation
    let mut params = Metadata::new();
    params.insert("tool", "calculator");
    params.insert("operation", "power");
    params.insert("a", 2.0);
    params.insert("b", 3.0);

    match agent.execute_task(atlas_core::TaskId::new(), params).await {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected error: {}", e),
    }

    // Shutdown server
    drop(server_handle);

    Ok(())
}
