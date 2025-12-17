# f1-nexus-cli

[![Crates.io](https://img.shields.io/crates/v/f1-nexus-cli.svg)](https://crates.io/crates/f1-nexus-cli)
[![Documentation](https://docs.rs/f1-nexus-cli/badge.svg)](https://docs.rs/f1-nexus-cli)
[![License](https://img.shields.io/crates/l/f1-nexus-cli.svg)](https://github.com/mrkingsleyobi/f1-nexus)

Command-line interface for F1 Nexus - Optimize Formula 1 race strategies from your terminal.

## Features

- **🏎️ Strategy Optimization**: Find optimal pit stops with a single command
- **🎲 Race Simulation**: Run Monte Carlo simulations for strategy validation
- **📊 Beautiful Output**: Colored, formatted results with progress indicators
- **⚡ Fast**: Optimization in <100ms, simulation in ~2 seconds
- **🔧 Flexible**: Configure every aspect of race strategy
- **📈 Statistics**: Confidence intervals, percentiles, distributions

## Installation

### cargo

```bash
cargo install f1-nexus-cli
```

### From source

```bash
git clone https://github.com/mrkingsleyobi/f1-nexus
cd f1-nexus
cargo install --path crates/f1-nexus-cli
```

## Quick Start

### Optimize pit strategy

```bash
f1-nexus optimize --track monaco --laps 78
```

Output:
```
🏁 F1 Nexus Strategy Optimizer
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Track: Circuit de Monaco
Total Laps: 78
Available Compounds: C1, C2, C3

Optimal Strategy:
  Starting Compound: C3 (Soft)

  Pit Stops:
  1. Lap 28 → C2 (Medium)
  2. Lap 52 → C1 (Hard)

  Predicted Race Time: 6847.2s (1:54:07.2)
  Confidence: 87.5%

✓ Strategy optimized in 67ms
```

### Simulate race

```bash
f1-nexus simulate --track spa --laps 44 --iterations 10000
```

Output:
```
🎲 Running 10,000 race simulations...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
[████████████████████████████████] 10000/10000

Results:
  Mean Finish Time: 4852.3s
  Median:           4849.1s
  Std Dev:          ±23.7s

  Percentiles:
  P10:  4812.5s
  P25:  4834.2s
  P50:  4849.1s
  P75:  4868.9s
  P90:  4891.7s

  DNF Probability: 2.3%

✓ Simulation complete in 2.1s
```

## Commands

### `optimize`
Find optimal pit stop strategy using dynamic programming.

```bash
f1-nexus optimize [OPTIONS]
```

**Options**:
- `--track <TRACK>` - Circuit name (monaco, silverstone, spa, etc.)
- `--laps <N>` - Total race laps
- `--compounds <C1,C2,C3>` - Available tire compounds
- `--position <N>` - Starting position (default: 1)
- `--fuel <KG>` - Starting fuel in kg (default: 110.0)
- `--json` - Output as JSON

**Example**:
```bash
f1-nexus optimize --track silverstone --laps 52 --compounds C1,C2,C3 --position 5
```

### `simulate`
Run Monte Carlo race simulation.

```bash
f1-nexus simulate [OPTIONS]
```

**Options**:
- `--track <TRACK>` - Circuit name
- `--laps <N>` - Total race laps
- `--iterations <N>` - Number of simulations (default: 10000)
- `--strategy <FILE>` - Load strategy from JSON file
- `--json` - Output as JSON

**Example**:
```bash
f1-nexus simulate --track monaco --laps 78 --iterations 50000 --json > results.json
```

### `circuits`
List all supported F1 circuits.

```bash
f1-nexus circuits
```

### `compounds`
List all tire compound types.

```bash
f1-nexus compounds
```

## Supported Circuits

- 🇲🇨 Monaco (Circuit de Monaco)
- 🇬🇧 Silverstone (Silverstone Circuit)
- 🇧🇪 Spa (Spa-Francorchamps)
- 🇮🇹 Monza (Autodromo Nazionale di Monza)
- 🇯🇵 Suzuka (Suzuka Circuit)
- 🇸🇬 Singapore (Marina Bay Street Circuit)
- 🇦🇪 Abu Dhabi (Yas Marina Circuit)
- ...and 17 more official F1 circuits

## JSON Output

Use `--json` flag for machine-readable output:

```bash
f1-nexus optimize --track monaco --laps 78 --json | jq
```

```json
{
  "strategyId": "opt-monaco-1734389234",
  "track": "monaco",
  "totalLaps": 78,
  "startingCompound": "C3",
  "pitStops": [
    {
      "lap": 28,
      "compound": "C2",
      "pitLoss": 22.0,
      "reason": "Optimal",
      "confidence": 0.89
    }
  ],
  "predictedRaceTime": 6847.2,
  "confidence": 0.875
}
```

## Integration Examples

### Use in shell scripts

```bash
#!/bin/bash

# Optimize for all circuits
for circuit in monaco silverstone spa monza; do
  echo "Optimizing for $circuit..."
  f1-nexus optimize --track $circuit --laps 70 --json > "strategy-$circuit.json"
done
```

### Pipe to other tools

```bash
# Export to CSV
f1-nexus simulate --track spa --iterations 10000 --json | \
  jq -r '.simulations[] | [.finishTime, .position] | @csv' > results.csv

# Compare strategies
f1-nexus optimize --track monaco --json | jq '.predictedRaceTime'
```

## Performance

- **Optimization**: 45-100ms for typical race
- **Simulation**: 1.8-2.5s for 10,000 iterations
- **Memory**: <50MB peak usage
- **CPU**: Utilizes all cores for simulation

## Use Cases

- **Race Engineers**: Quick strategy calculations during practice/qualifying
- **Strategy Analysts**: Batch analysis of different scenarios
- **Data Scientists**: Generate training data for ML models
- **Developers**: Test F1 Nexus libraries
- **Students**: Learn about F1 strategy optimization

## Configuration

Set environment variables:

```bash
export F1_NEXUS_LOG=debug        # Enable debug logging
export F1_NEXUS_THREADS=8        # Set thread count
export F1_NEXUS_CACHE=/tmp/f1    # Set cache directory
```

## Documentation

- [User Guide](https://github.com/mrkingsleyobi/f1-nexus/blob/main/docs/CLI.md)
- [API Docs](https://docs.rs/f1-nexus-cli)
- [Examples](https://github.com/mrkingsleyobi/f1-nexus/blob/main/docs/EXAMPLES.md)

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
