# f1-nexus-wasm

[![npm](https://img.shields.io/npm/v/@f1-nexus/wasm.svg)](https://www.npmjs.com/package/@f1-nexus/wasm)
[![Crates.io](https://img.shields.io/crates/v/f1-nexus-wasm.svg)](https://crates.io/crates/f1-nexus-wasm)
[![License](https://img.shields.io/crates/l/f1-nexus-wasm.svg)](https://github.com/mrkingsleyobi/f1-nexus)

F1 Nexus WebAssembly bindings for browser-based Formula 1 race strategy optimization.

## Features

- **🌐 Browser-Native**: Run F1 strategy optimization directly in the browser
- **⚡ High Performance**: Near-native speed using WebAssembly
- **📦 Zero Server Cost**: All computation happens client-side
- **🔒 Data Privacy**: No data leaves the user's browser
- **📱 Cross-Platform**: Works on desktop, mobile, and tablets
- **🎯 TypeScript Support**: Full type definitions included

## Installation

### npm

```bash
npm install @f1-nexus/wasm
```

### yarn

```bash
yarn add @f1-nexus/wasm
```

### pnpm

```bash
pnpm add @f1-nexus/wasm
```

## Quick Start

```javascript
import init, { F1Nexus } from '@f1-nexus/wasm';

// Initialize WASM module
await init();

// Create F1 Nexus instance
const f1 = new F1Nexus();

// Optimize pit strategy
const strategy = f1.optimizeStrategy({
  track: 'monaco',
  totalLaps: 78,
  currentLap: 1,
  currentCompound: 'C3',
  availableCompounds: ['C1', 'C2', 'C3'],
  fuelRemaining: 110.0,
  position: 3
});

console.log('Optimal Strategy:');
console.log(`Starting compound: ${strategy.startingCompound}`);
strategy.pitStops.forEach((stop, i) => {
  console.log(`Stop ${i + 1}: Lap ${stop.lap} → ${stop.compound}`);
});
console.log(`Predicted race time: ${strategy.predictedRaceTime}s`);
```

## React Integration

```tsx
import { useEffect, useState } from 'react';
import init, { F1Nexus } from '@f1-nexus/wasm';

function StrategyOptimizer() {
  const [f1, setF1] = useState<F1Nexus | null>(null);
  const [strategy, setStrategy] = useState(null);

  useEffect(() => {
    init().then(() => {
      setF1(new F1Nexus());
    });
  }, []);

  const optimize = () => {
    if (!f1) return;
    const result = f1.optimizeStrategy({
      track: 'monaco',
      totalLaps: 78,
      availableCompounds: ['C1', 'C2', 'C3']
    });
    setStrategy(result);
  };

  return (
    <div>
      <button onClick={optimize}>Optimize Strategy</button>
      {strategy && <StrategyDisplay strategy={strategy} />}
    </div>
  );
}
```

## API Reference

### `optimizeStrategy(params)`
Find optimal pit stop strategy using dynamic programming.

**Parameters**:
- `track: string` - Circuit name (e.g., 'monaco', 'silverstone')
- `totalLaps: number` - Total race laps
- `availableCompounds: string[]` - Tire compounds available
- `currentLap?: number` - Current lap (default: 1)
- `fuelRemaining?: number` - Fuel in kg (default: 110.0)

**Returns**: `{ strategyId, startingCompound, pitStops[], predictedRaceTime, confidence }`

### `simulateRace(params)`
Run Monte Carlo simulation to validate strategy.

**Parameters**: Strategy + simulation config
**Returns**: Distribution of finish times, confidence intervals

### `predictTireLife(params)`
Predict tire degradation and optimal pit window.

**Parameters**: Tire data + track conditions
**Returns**: Remaining laps, degradation rate

### `getCircuits()`
Get list of supported F1 circuits.

**Returns**: `string[]` - Circuit IDs

### `getTireCompounds()`
Get list of tire compound types.

**Returns**: `string[]` - Compound IDs (C0-C5, Intermediate, Wet)

### `version()`
Get package version.

**Returns**: `string` - Version number

## Supported Browsers

- ✅ Chrome 90+
- ✅ Firefox 89+
- ✅ Safari 15+
- ✅ Edge 90+

## Bundle Size

- **WASM**: ~280 KB (uncompressed)
- **JS Glue**: ~15 KB
- **Gzipped Total**: ~95 KB

## Performance

- **Optimization**: <50ms for typical race (70 laps, 3 compounds)
- **Simulation**: 1,000 iterations in ~200ms
- **Memory**: <10MB peak usage

## Use Cases

- **F1 Fantasy Apps**: Real-time strategy recommendations
- **Educational Tools**: Interactive F1 strategy learning
- **Esports**: Browser-based F1 game strategy tools
- **Data Visualization**: Live strategy analysis dashboards
- **Mobile Apps**: PWAs with offline strategy optimization

## Examples

See [docs/EXAMPLES.md](https://github.com/mrkingsleyobi/f1-nexus/blob/main/docs/EXAMPLES.md) for:
- Vanilla JavaScript
- React hooks
- Vue composition API
- Real-time updates
- Web workers

## Documentation

- [API Reference](https://github.com/mrkingsleyobi/f1-nexus/blob/main/docs/API.md)
- [Examples](https://github.com/mrkingsleyobi/f1-nexus/blob/main/docs/EXAMPLES.md)
- [Crate Docs](https://docs.rs/f1-nexus-wasm)

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
