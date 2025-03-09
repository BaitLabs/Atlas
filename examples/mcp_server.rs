//! Example demonstrating how to create and run an MCP server with custom tools and resources

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use tokio;

use atlas_core::Metadata;
use atlas_mcp::{MCPTool, MCPResource, ServerBuilder, ServerConfig, ServerCapabilities};

/// Weather tool that provides weather information
#[derive(Clone)]
struct WeatherTool {
    api_key: String,
}

impl WeatherTool {
    fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl MCPTool for WeatherTool {
    fn name(&self) -> &str {
        "weather"
    }

    fn description(&self) -> &str {
        "Get weather information for a location"
    }

    async fn execute(&self, params: Metadata) -> Result<Metadata> {
        let location = params.get::<String>("location")
            .ok_or_else(|| anyhow::anyhow!("Location is required"))?;

        // In a real implementation, this would make an API call
        // For this example, we'll return mock data
        let mut response = Metadata::new();
        response.insert("location", location);
        response.insert("temperature", 22.5);
        response.insert("conditions", "sunny");
        response.insert("humidity", 45);

        Ok(response)
    }
}

/// News resource that provides access to news articles
#[derive(Clone)]
struct NewsResource {
    articles: Arc<Vec<Article>>,
}

#[derive(Clone, serde::Serialize)]
struct Article {
    id: String,
    title: String,
    content: String,
    published: chrono::DateTime<chrono::Utc>,
}

impl NewsResource {
    fn new() -> Self {
        // Mock articles for demonstration
        let articles = vec![
            Article {
                id: "1".to_string(),
                title: "Atlas Framework Released".to_string(),
                content: "The Atlas Framework has been released...".to_string(),
                published: chrono::Utc::now(),
            },
            Article {
                id: "2".to_string(),
                title: "MCP Protocol Gains Adoption".to_string(),
                content: "The Model Context Protocol is seeing increased adoption...".to_string(),
                published: chrono::Utc::now(),
            },
        ];

        Self {
            articles: Arc::new(articles),
        }
    }
}

#[async_trait]
impl MCPResource for NewsResource {
    fn name(&self) -> &str {
        "news"
    }

    fn resource_type(&self) -> &str {
        "articles"
    }

    async fn access(&self, params: Metadata) -> Result<Metadata> {
        let article_id = params.get::<String>("id");

        let articles = if let Some(id) = article_id {
            // Return specific article
            self.articles
                .iter()
                .filter(|a| a.id == id)
                .cloned()
                .collect::<Vec<_>>()
        } else {
            // Return all articles
            self.articles.to_vec()
        };

        let mut response = Metadata::new();
        response.insert("articles", json!(articles));
        Ok(response)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create server configuration
    let server_config = ServerConfig {
        name: "example_server".to_string(),
        version: "0.1.0".to_string(),
        description: Some("Example MCP server with custom tools and resources".to_string()),
        capabilities: ServerCapabilities {
            tools: vec!["weather".to_string()],
            resources: vec!["news".to_string()],
        },
    };

    // Create and configure server
    let server = ServerBuilder::new()
        .config(server_config)
        .tool(
            "weather",
            WeatherTool::new("mock-api-key".to_string()),
        )
        .resource("news", NewsResource::new())
        .build()?;

    println!("Starting MCP server...");

    // Start the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    server.serve(addr).await?;

    Ok(())
}

// Example client code (not executed, just for demonstration)
async fn example_client_usage() -> Result<()> {
    use atlas_agent::{Agent, AgentBuilder, Config};

    // Create agent configuration
    let agent_config = Config {
        name: "weather_agent".to_string(),
        description: Some("Agent that uses weather tool".to_string()),
        capabilities: vec!["weather".to_string()],
        config: Metadata::new(),
    };

    // Create agent
    let agent = AgentBuilder::new()
        .config(agent_config)
        .build()?;

    // Execute weather tool
    let mut params = Metadata::new();
    params.insert("tool", "weather");
    params.insert("location", "San Francisco");

    let result = agent
        .execute_task(atlas_core::TaskId::new(), params)
        .await?;

    println!(
        "Weather in {}: {}°C, {}",
        result.get::<String>("location").unwrap(),
        result.get::<f64>("temperature").unwrap(),
        result.get::<String>("conditions").unwrap()
    );

    Ok(())
}

// Example resource access (not executed, just for demonstration)
async fn example_resource_access() -> Result<()> {
    use atlas_mcp::{MCPRequest, MCPResponse};
    use reqwest::Client;

    // Create HTTP client
    let client = Client::new();

    // Access news resource
    let response = client
        .get("http://localhost:3000/resources/news")
        .send()
        .await?
        .json::<MCPResponse>()
        .await?;

    match response {
        MCPResponse::ResourceContent { contents } => {
            for content in contents {
                println!("Article: {}", content.text);
            }
        }
        _ => println!("Unexpected response type"),
    }

    Ok(())
}
