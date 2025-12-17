# f1-nexus-core

[![Crates.io](https://img.shields.io/crates/v/f1-nexus-core.svg)](https://crates.io/crates/f1-nexus-core)
[![Documentation](https://docs.rs/f1-nexus-core/badge.svg)](https://docs.rs/f1-nexus-core)
[![License](https://img.shields.io/crates/l/f1-nexus-core.svg)](https://github.com/mrkingsleyobi/f1-nexus)

Core domain types and fundamental racing logic for Formula 1 strategy optimization.

## Features

- **Comprehensive F1 Domain Models**: Telemetry data, race state, tire compounds, fuel management
- **Track Definitions**: 24 official F1 circuits with accurate characteristics
- **Tire Physics**: Advanced degradation models for all compound types (C0-C5, Intermediate, Wet)
- **Fuel Consumption**: Realistic fuel burn rates and lap time impacts
- **Weather Integration**: Weather conditions and their effects on strategy
- **ERS Management**: Energy recovery system modeling and deployment strategies
- **FIA Regulations**: Rule compliance checking and penalty calculations

## Installation

```toml
[dependencies]
f1-nexus-core = "1.0.0-alpha.2"
```

## Quick Start

```rust
use f1_nexus_core::*;

// Create a circuit
let monaco = Circuit {
    id: "monaco".to_string(),
    name: "Circuit de Monaco".to_string(),
    country: "Monaco".to_string(),
    typical_race_laps: 78,
    lap_distance_km: 3.337,
    track_characteristics: TrackCharacteristics {
        avg_speed_kph: 160.0,
        top_speed_kph: 290.0,
        num_corners: 19,
        elevation_change_m: 42.0,
        overtaking_difficulty: 0.95,
        tire_wear_factor: 0.7,
        fuel_consumption_factor: 0.85,
    },
};

// Define tire compounds
let tire = TireCompound::C3;
println!("Using tire compound: {:?}", tire);

// Create race strategy
let strategy = RaceStrategy {
    id: "strategy-1".to_string(),
    starting_compound: TireCompound::C3,
    pit_stops: vec![],
    fuel_strategy: FuelStrategy::default(),
    ers_plan: ErsDeploymentPlan::default(),
    expected_lap_times: BTreeMap::new(),
    predicted_race_time: 0.0,
    confidence: 0.85,
    metadata: StrategyMetadata::default(),
};
```

## Use Cases

- **Race Strategy Development**: Build custom pit stop and tire strategies
- **Telemetry Analysis**: Process and analyze real-time F1 telemetry data
- **Race Simulation**: Create race scenarios with different strategies
- **Track Analysis**: Study circuit characteristics and their impact on strategy
- **Educational Tools**: Learn about F1 race engineering and strategy

## Supported Circuits

Monaco, Silverstone, Spa-Francorchamps, Monza, Suzuka, Singapore, Abu Dhabi, and 17 more official F1 circuits with accurate track data.

## Documentation

Full API documentation available at [docs.rs/f1-nexus-core](https://docs.rs/f1-nexus-core)

## Related Crates

- [`f1-nexus-strategy`](https://crates.io/crates/f1-nexus-strategy) - Pit stop optimization algorithms
- [`f1-nexus-telemetry`](https://crates.io/crates/f1-nexus-telemetry) - Real-time telemetry processing
- [`f1-nexus-cli`](https://crates.io/crates/f1-nexus-cli) - Command-line interface

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
