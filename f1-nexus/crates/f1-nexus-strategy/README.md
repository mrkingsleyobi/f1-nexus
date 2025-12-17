# f1-nexus-strategy

[![Crates.io](https://img.shields.io/crates/v/f1-nexus-strategy.svg)](https://crates.io/crates/f1-nexus-strategy)
[![Documentation](https://docs.rs/f1-nexus-strategy/badge.svg)](https://docs.rs/f1-nexus-strategy)
[![License](https://img.shields.io/crates/l/f1-nexus-strategy.svg)](https://github.com/mrkingsleyobi/f1-nexus)

Advanced pit stop optimization and race strategy algorithms for Formula 1 using dynamic programming and Monte Carlo simulation.

## Features

- **🏎️ Dynamic Programming Optimization**: Find optimal pit stop windows using advanced algorithms
- **🎲 Monte Carlo Race Simulation**: Run 10,000+ race simulations with realistic variance
- **🏁 Tire Strategy Analysis**: Optimize compound selection and stint lengths
- **⛽ Fuel Management**: Calculate optimal fuel loads and fuel-saving strategies
- **🔋 ERS Deployment**: Plan energy recovery system usage for maximum performance
- **🌦️ Weather Adaptation**: Adjust strategies based on weather forecasts
- **📊 Statistical Analysis**: Confidence intervals and probability distributions

## Installation

```toml
[dependencies]
f1-nexus-strategy = "1.0.0-alpha.2"
```

## Quick Start

```rust
use f1_nexus_strategy::*;
use f1_nexus_core::*;

// Create optimization config
let config = OptimizationConfig {
    total_laps: 70,
    circuit: create_monaco_circuit(),
    available_compounds: vec![TireCompound::C1, TireCompound::C2, TireCompound::C3],
    pit_lane_time_loss: 22.0,
    tire_change_time: 2.5,
    current_position: 1,
    competitors_ahead: vec![],
    degradation_factors: DegradationFactors::default(),
    fuel_model: FuelConsumptionModel::default(),
    starting_fuel: 110.0,
};

// Optimize pit strategy
let strategy = optimize_pit_strategy(&config)?;

println!("Optimal strategy:");
println!("Starting compound: {:?}", strategy.starting_compound);
for (i, stop) in strategy.pit_stops.iter().enumerate() {
    println!("Stop {}: Lap {} -> {:?}", i + 1, stop.lap.0, stop.compound);
}
println!("Predicted race time: {:.2}s", strategy.predicted_race_time);
println!("Confidence: {:.1}%", strategy.confidence * 100.0);
```

## Race Simulation

```rust
use f1_nexus_strategy::simulation::*;

let sim_config = SimulationConfig {
    num_iterations: 10000,
    circuit: create_spa_circuit(),
    weather_forecast: WeatherForecast::default(),
    degradation_variance: 0.05,
    lap_time_variance: 0.02,
};

let results = simulate_race(&strategy, &sim_config)?;

println!("Mean finish time: {:.2}s", results.mean_finish_time);
println!("P10 time: {:.2}s", results.percentile_10);
println!("P90 time: {:.2}s", results.percentile_90);
println!("DNF probability: {:.1}%", results.dnf_probability * 100.0);
```

## Algorithms

### Dynamic Programming Optimization
- **Time Complexity**: O(n² × m) where n = laps, m = compounds
- **Space Complexity**: O(n × m)
- **Accuracy**: Finds globally optimal solution

### Monte Carlo Simulation
- **Iterations**: 1,000 - 100,000 (configurable)
- **Variance Modeling**: Tire degradation, lap times, incidents
- **Output**: Mean, median, P10/P90, full distribution

## Use Cases

- **Race Engineers**: Optimize real-time strategy during races
- **Strategy Analysts**: Pre-race strategy planning and what-if analysis
- **Esports**: AI-powered strategy for F1 games
- **Education**: Learn about race strategy optimization algorithms
- **Research**: Study optimization techniques in motorsport

## Performance

- **Optimization Speed**: <100ms for typical race (70 laps, 3 compounds)
- **Simulation Speed**: 10,000 iterations in ~2 seconds
- **Memory Usage**: <50MB for full race simulation

## Documentation

Full API documentation and examples at [docs.rs/f1-nexus-strategy](https://docs.rs/f1-nexus-strategy)

## Related Crates

- [`f1-nexus-core`](https://crates.io/crates/f1-nexus-core) - Core F1 domain types
- [`f1-nexus-telemetry`](https://crates.io/crates/f1-nexus-telemetry) - Real-time telemetry
- [`f1-nexus-mcp`](https://crates.io/crates/f1-nexus-mcp) - Model Context Protocol server

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
