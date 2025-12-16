# F1 Nexus API Documentation

Complete API reference for F1 Nexus WASM (browser) and Node.js (NAPI) bindings.

---

## Table of Contents

- [WASM Browser API](#wasm-browser-api)
- [Node.js NAPI API](#nodejs-napi-api)
- [MCP Tools API](#mcp-tools-api)
- [Common Types](#common-types)
- [Error Handling](#error-handling)

---

## WASM Browser API

Use F1 Nexus directly in the browser with WebAssembly for client-side privacy and performance.

### Installation

```bash
npm install @f1-nexus/wasm
# or
yarn add @f1-nexus/wasm
```

### Initialization

```javascript
import init, { F1Nexus } from '@f1-nexus/wasm';

// Initialize WASM module
await init();

// Create F1 Nexus instance
const nexus = new F1Nexus();
```

### Methods

#### `version(): string`

Returns the F1 Nexus version.

```javascript
const version = nexus.version();
console.log(version); // "1.0.0-alpha.1"
```

#### `optimizeStrategy(params: object): object`

Optimizes pit stop strategy using dynamic programming.

**Parameters:**
```typescript
{
  track: string;              // Circuit name: "monaco", "spa", "silverstone", etc.
  total_laps?: number;        // Race length (defaults to circuit default)
  starting_fuel?: number;     // Starting fuel in kg (default: 110.0)
  position?: number;          // Current position (default: 5)
  available_compounds?: string[]; // Tire compounds (default: ["C1", "C2", "C3"])
}
```

**Returns:**
```typescript
{
  strategy_id: string;
  starting_compound: string;
  pit_stops: Array<{
    lap: number;
    compound: string;
    pit_loss: number;        // Time loss in seconds
    reason: string;          // "Mandatory", "Undercut", etc.
    confidence: number;      // 0.0-1.0
  }>;
  predicted_race_time: number; // Total race time in seconds
  confidence: number;          // Overall confidence 0.0-1.0
}
```

**Example:**
```javascript
const strategy = nexus.optimizeStrategy({
  track: "monaco",
  total_laps: 78,
  starting_fuel: 110.0,
  position: 5,
  available_compounds: ["C1", "C2", "C3"]
});

console.log(`Pit stops: ${strategy.pit_stops.length}`);
console.log(`Predicted time: ${strategy.predicted_race_time}s`);
```

#### `simulateRace(params: object): object`

Runs Monte Carlo race simulation with a given strategy.

**Parameters:**
```typescript
{
  track: string;              // Circuit name
  num_simulations?: number;   // Number of Monte Carlo runs (default: 100)
  starting_compound?: string; // Starting tire (default: "C3")
  pit_stops: Array<{
    lap: number;
    compound: string;
  }>;
}
```

**Returns:**
```typescript
{
  num_simulations: number;
  mean_race_time: number;    // Average race time in seconds
  min_race_time: number;     // Best case scenario
  max_race_time: number;     // Worst case scenario
  total_laps: number;
  pit_stops: number;
  final_fuel_kg: number;
  fastest_lap: number;       // Fastest lap time in seconds
}
```

**Example:**
```javascript
const simulation = nexus.simulateRace({
  track: "spa",
  num_simulations: 1000,
  starting_compound: "C3",
  pit_stops: [
    { lap: 22, compound: "C2" }
  ]
});

console.log(`Mean time: ${simulation.mean_race_time}s`);
console.log(`Fastest lap: ${simulation.fastest_lap}s`);
```

#### `predictTireLife(params: object): object`

Predicts tire degradation and remaining life.

**Parameters:**
```typescript
{
  compound: string;           // "C0"-"C5", "Intermediate", "Wet"
  age_laps: number;          // Current tire age in laps
  track_temp?: number;       // Track temperature in °C (default: 100.0)
  track_severity?: number;   // Track severity multiplier (default: 1.0)
}
```

**Returns:**
```typescript
{
  compound: string;
  current_age_laps: number;
  current_wear_percent: number;      // 0-100%
  typical_life_laps: number;
  estimated_remaining_laps: number;
  grip_multiplier: number;           // Based on temperature
  recommended_pit_soon: boolean;     // True if >70% worn or <5 laps left
}
```

**Example:**
```javascript
const tireLife = nexus.predictTireLife({
  compound: "C3",
  age_laps: 20,
  track_temp: 35.0,
  track_severity: 1.2
});

if (tireLife.recommended_pit_soon) {
  console.log(`Pit recommended! Only ${tireLife.estimated_remaining_laps} laps left`);
}
```

#### `getCircuits(): string[]`

Returns list of supported F1 circuits.

```javascript
const circuits = nexus.getCircuits();
console.log(circuits);
// ["monaco", "spa", "silverstone", "monza", "suzuka", ...]
```

#### `getTireCompounds(): string[]`

Returns list of available tire compounds.

```javascript
const compounds = nexus.getTireCompounds();
console.log(compounds);
// ["C0", "C1", "C2", "C3", "C4", "C5", "Intermediate", "Wet"]
```

---

## Node.js NAPI API

High-performance native Node.js bindings using NAPI-RS.

### Installation

```bash
npm install @f1-nexus/node
# or
yarn add @f1-nexus/node
```

### Usage

```javascript
const f1nexus = require('@f1-nexus/node');

// All functions use JSON strings for parameters
const result = f1nexus.optimizeStrategy(JSON.stringify({
  track: "monaco",
  total_laps: 78
}));

const strategy = JSON.parse(result);
console.log(strategy);
```

### Functions

All NAPI functions accept JSON strings and return JSON strings for maximum compatibility.

#### `version(): string`

Returns version string.

```javascript
const version = f1nexus.version();
console.log(version); // "1.0.0-alpha.1"
```

#### `optimizeStrategy(paramsJson: string): string`

Same as WASM API but uses JSON strings.

```javascript
const params = JSON.stringify({
  track: "monaco",
  total_laps: 78,
  starting_fuel: 110.0,
  position: 5,
  available_compounds: ["C1", "C2", "C3"]
});

const resultJson = f1nexus.optimizeStrategy(params);
const strategy = JSON.parse(resultJson);
```

#### `simulateRace(paramsJson: string): string`

Same as WASM API but uses JSON strings.

```javascript
const params = JSON.stringify({
  track: "spa",
  num_simulations: 1000,
  starting_compound: "C3",
  pit_stops: [{ lap: 22, compound: "C2" }]
});

const resultJson = f1nexus.simulateRace(params);
const simulation = JSON.parse(resultJson);
```

#### `predictTireLife(paramsJson: string): string`

Same as WASM API but uses JSON strings.

```javascript
const params = JSON.stringify({
  compound: "C3",
  age_laps: 20,
  track_temp: 35.0
});

const resultJson = f1nexus.predictTireLife(params);
const tireLife = JSON.parse(resultJson);
```

#### `getCircuits(): string[]`

Returns array of circuit names.

```javascript
const circuits = f1nexus.getCircuits();
console.log(circuits);
```

#### `getTireCompounds(): string[]`

Returns array of tire compound names.

```javascript
const compounds = f1nexus.getTireCompounds();
console.log(compounds);
```

---

## MCP Tools API

F1 Nexus provides MCP (Model Context Protocol) tools for AI agent integration.

### Available Tools

#### `optimize_strategy`

Optimizes pit stop strategy for current race conditions.

**Input Schema:**
```json
{
  "current_lap": 25,
  "tire_age": 15,
  "fuel_remaining": 80.0,
  "position": 3,
  "track_id": "monaco"
}
```

**Output:**
```json
{
  "success": true,
  "strategy": {
    "id": "uuid-here",
    "starting_compound": "C3",
    "pit_stops": [
      {
        "lap": 40,
        "compound": "C2",
        "pit_loss": 22.0,
        "reason": "Mandatory",
        "confidence": 0.85
      }
    ],
    "predicted_race_time": 5643.2,
    "confidence": 0.82
  }
}
```

#### `predict_tire_life`

Predicts tire degradation and remaining life.

**Input Schema:**
```json
{
  "compound": "C3",
  "age_laps": 15,
  "track_temp": 32.0,
  "track_severity": 1.2
}
```

**Output:**
```json
{
  "success": true,
  "prediction": {
    "compound": "C3",
    "current_age_laps": 15,
    "current_wear_percent": 45.5,
    "typical_life_laps": 25,
    "estimated_remaining_laps": 8.3,
    "grip_multiplier": 0.95,
    "recommended_pit_soon": false
  }
}
```

#### `simulate_race`

Runs Monte Carlo race simulation.

**Input Schema:**
```json
{
  "track_id": "spa",
  "num_simulations": 100,
  "pit_stops": [
    {"lap": 22, "compound": "C2"}
  ]
}
```

**Output:**
```json
{
  "success": true,
  "simulation": {
    "num_simulations": 100,
    "mean_race_time_seconds": 4835.2,
    "min_race_time_seconds": 4812.1,
    "max_race_time_seconds": 4861.8,
    "mean_race_time_formatted": "80:35.200",
    "sample_breakdown": {
      "total_laps": 44,
      "pit_stops": 1,
      "final_fuel_kg": 3.2,
      "fastest_lap_seconds": 107.5
    }
  }
}
```

#### `get_weather_forecast`

Fetches real-time weather data from OpenWeatherMap.

**Input Schema:**
```json
{
  "circuit": "monaco",
  "api_key": "your-openweathermap-key"  // Optional if OPENWEATHERMAP_API_KEY env var set
}
```

**Output:**
```json
{
  "success": true,
  "circuit": "monaco",
  "forecast": {
    "overall_condition": "Dry",
    "air_temperature_celsius": 24.5,
    "track_temperature_celsius": 34.5,
    "humidity": 0.65,
    "wind_speed_kmh": 12.5,
    "rain_probability": 0.15,
    "recommended_tire": "Dry",
    "sector_conditions": [...],
    "predictions": [...]
  }
}
```

#### `query_historical`

Queries historical race data (placeholder - returns mock data).

**Input Schema:**
```json
{
  "track_id": "monaco",
  "weather": "dry",
  "top_k": 5
}
```

#### `get_agent_consensus`

Gets multi-agent consensus on strategy decisions (placeholder - returns single agent response).

**Input Schema:**
```json
{
  "question": "Should we pit now or wait 5 more laps?",
  "timeout_ms": 5000
}
```

---

## Common Types

### Circuit Names

Supported circuits:
- `"monaco"` - Circuit de Monaco (78 laps)
- `"spa"` - Circuit de Spa-Francorchamps (44 laps)
- `"silverstone"` - Silverstone Circuit (52 laps)
- `"monza"` - Autodromo Nazionale di Monza
- `"suzuka"` - Suzuka International Racing Course
- `"interlagos"` - Autódromo José Carlos Pace
- `"austin"` - Circuit of The Americas (COTA)
- `"barcelona"` - Circuit de Barcelona-Catalunya
- `"austria"` - Red Bull Ring
- `"hungary"` - Hungaroring

### Tire Compounds

Dry compounds (hardest to softest):
- `"C0"` - Hardest (rarely used)
- `"C1"` - Hard
- `"C2"` - Medium
- `"C3"` - Medium
- `"C4"` - Soft
- `"C5"` - Softest

Wet compounds:
- `"Intermediate"` - Light rain
- `"Wet"` - Heavy rain

### Pit Stop Reasons

- `"Mandatory"` - Required by regulations
- `"Undercut"` - Strategic undercut attempt
- `"Overcut"` - Strategic overcut attempt
- `"TireDegradation"` - Tires worn out
- `"Opportunistic"` - Safety car/VSC opportunity
- `"Damage"` - Car damage repair

---

## Error Handling

### WASM Errors

WASM functions throw JavaScript errors:

```javascript
try {
  const strategy = nexus.optimizeStrategy({
    track: "invalid-track"
  });
} catch (error) {
  console.error("Optimization failed:", error.message);
}
```

### NAPI Errors

NAPI functions throw Node.js errors:

```javascript
try {
  const result = f1nexus.optimizeStrategy(JSON.stringify({
    track: "invalid-track"
  }));
} catch (error) {
  console.error("Optimization failed:", error.message);
}
```

### Common Errors

- **"Invalid tire compound"** - Unknown compound name
- **"Optimization failed"** - No valid strategy found
- **"Missing required parameter"** - Required field not provided
- **"Invalid input JSON"** - Malformed JSON input (NAPI only)
- **"Serialization error"** - Output serialization failed

---

## Performance Tips

### WASM

1. **Initialize once**: Call `init()` once at app startup
2. **Reuse instance**: Create one `F1Nexus` instance and reuse it
3. **Batch operations**: Group related calculations together
4. **Use Web Workers**: Run simulations in background threads

```javascript
// Good: Initialize once
await init();
const nexus = new F1Nexus();

// Bad: Initializing multiple times
for (let i = 0; i < 10; i++) {
  await init(); // Slow!
  const nexus = new F1Nexus();
}
```

### Node.js

1. **Avoid JSON parsing overhead**: Cache parsed objects when possible
2. **Use async/await**: NAPI functions are synchronous but CPU-intensive
3. **Worker threads**: Use Node.js worker threads for parallel simulations
4. **Memory**: Native bindings are memory-efficient, no need to worry

```javascript
// Good: Cache circuit list
const circuits = f1nexus.getCircuits();

// Bad: Parse JSON repeatedly
for (let i = 0; i < 1000; i++) {
  const result = JSON.parse(f1nexus.optimizeStrategy(...)); // Slow!
}
```

---

## Version Compatibility

- **WASM**: Requires browser with WebAssembly support (all modern browsers)
- **Node.js**: Requires Node.js 14+ (NAPI-RS compatibility)
- **Rust**: Built with Rust 1.75+

---

## Support & Resources

- **GitHub**: https://github.com/mrkingsleyobi/f1-nexus
- **Issues**: https://github.com/mrkingsleyobi/f1-nexus/issues
- **Discord**: [Join our community](https://discord.gg/f1-nexus)
- **Examples**: See [EXAMPLES.md](./EXAMPLES.md)

---

## License

MIT OR Apache-2.0
