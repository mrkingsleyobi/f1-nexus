//! Model Context Protocol (MCP) implementation for F1 Nexus
//!
//! Provides stdio and SSE transports for AI agent integration

pub mod server;
pub mod tools;
pub mod stdio;
pub mod sse;
pub mod weather_api;

pub use server::*;
pub use tools::*;
pub use weather_api::*;

use serde::{Deserialize, Serialize};

/// MCP server configuration
#[derive(Debug, Clone)]
pub struct McpConfig {
    pub transport: McpTransport,
    pub enable_telemetry_tool: bool,
    pub enable_strategy_tool: bool,
    pub enable_simulation_tool: bool,
    pub enable_historical_tool: bool,
}

/// MCP transport type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpTransport {
    Stdio,
    Sse,
}

impl Default for McpConfig {
    fn default() -> Self {
        McpConfig {
            transport: McpTransport::Stdio,
            enable_telemetry_tool: true,
            enable_strategy_tool: true,
            enable_simulation_tool: true,
            enable_historical_tool: true,
        }
    }
}

/// MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP tools catalog
pub fn get_mcp_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "optimize_strategy".to_string(),
            description: "Optimize race strategy given current conditions".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "current_lap": {"type": "number"},
                    "tire_age": {"type": "number"},
                    "fuel_remaining": {"type": "number"},
                    "position": {"type": "number"}
                },
                "required": ["current_lap"]
            }),
        },
        McpTool {
            name: "predict_tire_life".to_string(),
            description: "Predict remaining tire life based on current conditions".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "compound": {"type": "string"},
                    "age_laps": {"type": "number"},
                    "track_temp": {"type": "number"}
                },
                "required": ["compound", "age_laps"]
            }),
        },
        McpTool {
            name: "simulate_race".to_string(),
            description: "Run Monte Carlo race simulation with given strategy".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "strategy": {"type": "object"},
                    "num_simulations": {"type": "number"},
                    "track_id": {"type": "string"}
                },
                "required": ["strategy"]
            }),
        },
        McpTool {
            name: "get_weather_forecast".to_string(),
            description: "Get real-time weather forecast for a race track".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "circuit": {"type": "string"},
                    "api_key": {"type": "string"}
                },
                "required": ["circuit"]
            }),
        },
        McpTool {
            name: "query_historical".to_string(),
            description: "Find similar historical races using vector similarity search".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "track_id": {"type": "string"},
                    "weather": {"type": "string"},
                    "top_k": {"type": "number"}
                },
                "required": ["track_id"]
            }),
        },
        McpTool {
            name: "get_agent_consensus".to_string(),
            description: "Get multi-agent consensus on a strategy decision".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "question": {"type": "string"},
                    "timeout_ms": {"type": "number"}
                },
                "required": ["question"]
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tools() {
        let tools = get_mcp_tools();
        assert!(tools.len() >= 5);
        assert!(tools.iter().any(|t| t.name == "optimize_strategy"));
    }
}
