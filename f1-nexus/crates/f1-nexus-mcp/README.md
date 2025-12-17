# f1-nexus-mcp

[![Crates.io](https://img.shields.io/crates/v/f1-nexus-mcp.svg)](https://crates.io/crates/f1-nexus-mcp)
[![Documentation](https://docs.rs/f1-nexus-mcp/badge.svg)](https://docs.rs/f1-nexus-mcp)
[![License](https://img.shields.io/crates/l/f1-nexus-mcp.svg)](https://github.com/mrkingsleyobi/f1-nexus)

Model Context Protocol (MCP) server for F1 Nexus - Enable AI agents to optimize F1 race strategies through standardized tools.

## Features

- **🤖 AI Agent Integration**: Connect Claude, GPT-4, and other LLMs to F1 strategy optimization
- **🔧 MCP Tools**: Standardized tools for strategy optimization, simulation, and analysis
- **🌐 Dual Transport**: Support for both stdio and Server-Sent Events (SSE)
- **🌦️ Weather API Integration**: Real-time weather data from OpenWeatherMap
- **📊 Vector Search**: Query historical race data using semantic search
- **🏁 Multi-Agent Consensus**: Combine strategies from multiple AI agents

## Installation

```toml
[dependencies]
f1-nexus-mcp = "1.0.0-alpha.2"
```

## MCP Tools

### `optimize_strategy`
Optimize pit stop strategy for current race conditions.

**Input**:
```json
{
  "track_id": "monaco",
  "total_laps": 78,
  "current_lap": 15,
  "tire_age": 14,
  "fuel_remaining": 95.5,
  "position": 3,
  "available_compounds": ["C1", "C2", "C3"]
}
```

**Output**: Optimal pit stops, tire compounds, predicted race time

### `simulate_race`
Run Monte Carlo simulations for strategy validation.

**Input**: Strategy + weather + variance parameters
**Output**: Distribution of finish times, DNF probability, confidence intervals

### `predict_tire_life`
ML-based tire degradation prediction.

**Input**: Current tire data + track conditions
**Output**: Remaining laps, degradation rate, optimal pit window

### `get_weather_forecast`
Get real-time weather data for F1 circuits.

**Input**: Circuit name or coordinates
**Output**: Temperature, humidity, precipitation, wind, track conditions

### `query_historical`
Semantic search over historical race data.

**Input**: Natural language query
**Output**: Relevant historical strategies and outcomes

### `get_agent_consensus`
Multi-agent strategy voting and consensus.

**Input**: Multiple strategy proposals
**Output**: Consensus strategy with confidence scores

## Quick Start

### Stdio Transport (for Claude Desktop, etc.)

```rust
use f1_nexus_mcp::*;

#[tokio::main]
async fn main() -> Result<()> {
    let server = MCPServer::new(Transport::Stdio);
    server.run().await?;
    Ok(())
}
```

### SSE Transport (for web clients)

```rust
use f1_nexus_mcp::*;
use axum::{Router, routing::get};

#[tokio::main]
async fn main() -> Result<()> {
    let app = Router::new()
        .route("/mcp/sse", get(sse_handler))
        .route("/mcp/messages", post(message_handler));

    axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
```

## Claude Desktop Integration

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "f1-nexus": {
      "command": "f1-nexus-mcp",
      "args": ["--stdio"],
      "env": {
        "OPENWEATHERMAP_API_KEY": "your-api-key-here"
      }
    }
  }
}
```

Then ask Claude: "Optimize pit strategy for Monaco GP, starting P3 with 78 laps"

## Supported Circuits

Real-time weather data for all 24 F1 circuits including Monaco, Silverstone, Spa, Monza, Suzuka, Singapore, and more.

## Use Cases

- **AI Race Engineer**: Let AI agents make real-time strategy decisions
- **Strategy Analysis**: Use LLMs to analyze and compare different strategies
- **What-if Scenarios**: Ask AI to explore edge cases and alternative strategies
- **Educational Chatbots**: Build F1 strategy learning assistants
- **Voice Assistants**: "Alexa, optimize my pit strategy for wet conditions"

## Documentation

- Full API docs: [docs.rs/f1-nexus-mcp](https://docs.rs/f1-nexus-mcp)
- MCP Specification: [modelcontextprotocol.io](https://modelcontextprotocol.io)

## Related Crates

- [`f1-nexus-strategy`](https://crates.io/crates/f1-nexus-strategy) - Strategy optimization engine
- [`f1-nexus-core`](https://crates.io/crates/f1-nexus-core) - Core F1 types
- [`f1-nexus-cli`](https://crates.io/crates/f1-nexus-cli) - Command-line interface

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
