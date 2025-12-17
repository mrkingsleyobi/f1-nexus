# F1 Nexus Usage Examples

Practical examples for using F1 Nexus in real-world scenarios.

---

## Table of Contents

- [Browser Examples (WASM)](#browser-examples-wasm)
- [Node.js Examples (NAPI)](#nodejs-examples-napi)
- [React Integration](#react-integration)
- [Vue Integration](#vue-integration)
- [Express.js API](#expressjs-api)
- [CLI Usage](#cli-usage)
- [Advanced Scenarios](#advanced-scenarios)

---

## Browser Examples (WASM)

### Basic Strategy Optimization

```html
<!DOCTYPE html>
<html>
<head>
  <title>F1 Nexus Strategy Optimizer</title>
</head>
<body>
  <h1>F1 Strategy Optimizer</h1>

  <div>
    <label>Circuit:</label>
    <select id="circuit">
      <option>monaco</option>
      <option>spa</option>
      <option>silverstone</option>
    </select>
  </div>

  <button onclick="optimizeStrategy()">Optimize</button>

  <div id="result"></div>

  <script type="module">
    import init, { F1Nexus } from './node_modules/@f1-nexus/wasm/f1_nexus_wasm.js';

    let nexus;

    async function initWasm() {
      await init();
      nexus = new F1Nexus();
      console.log(`F1 Nexus ${nexus.version()} loaded`);
    }

    window.optimizeStrategy = function() {
      const circuit = document.getElementById('circuit').value;

      const strategy = nexus.optimizeStrategy({
        track: circuit,
        available_compounds: ["C1", "C2", "C3"]
      });

      let html = `<h2>Strategy for ${circuit}</h2>`;
      html += `<p>Starting: ${strategy.starting_compound}</p>`;
      html += `<p>Predicted time: ${strategy.predicted_race_time.toFixed(1)}s</p>`;
      html += `<h3>Pit Stops:</h3><ul>`;

      strategy.pit_stops.forEach((stop, i) => {
        html += `<li>Lap ${stop.lap}: ${stop.compound} (${stop.pit_loss.toFixed(1)}s loss)</li>`;
      });

      html += `</ul>`;
      document.getElementById('result').innerHTML = html;
    };

    initWasm();
  </script>
</body>
</html>
```

### Real-Time Tire Monitor

```javascript
import init, { F1Nexus } from '@f1-nexus/wasm';

await init();
const nexus = new F1Nexus();

class TireMonitor {
  constructor() {
    this.currentCompound = "C3";
    this.lapsSincePit = 0;
  }

  onLapComplete(lapNumber) {
    this.lapsSincePit++;

    const tireLife = nexus.predictTireLife({
      compound: this.currentCompound,
      age_laps: this.lapsSincePit,
      track_temp: 32.0,
      track_severity: 1.1
    });

    console.log(`Lap ${lapNumber}:`);
    console.log(`  Tire wear: ${tireLife.current_wear_percent.toFixed(1)}%`);
    console.log(`  Remaining laps: ${tireLife.estimated_remaining_laps.toFixed(1)}`);
    console.log(`  Grip: ${(tireLife.grip_multiplier * 100).toFixed(1)}%`);

    if (tireLife.recommended_pit_soon) {
      console.log(`  ⚠️  PIT RECOMMENDED!`);
      return true;
    }

    return false;
  }

  pit(newCompound) {
    console.log(`Pitting: ${this.currentCompound} → ${newCompound}`);
    this.currentCompound = newCompound;
    this.lapsSincePit = 0;
  }
}

// Usage in a race simulation
const monitor = new TireMonitor();

for (let lap = 1; lap <= 44; lap++) {
  const shouldPit = monitor.onLapComplete(lap);

  if (shouldPit && lap < 40) {
    monitor.pit("C2");
  }
}
```

### Monte Carlo Simulation Dashboard

```javascript
import init, { F1Nexus } from '@f1-nexus/wasm';

await init();
const nexus = new F1Nexus();

async function runSimulations(track, strategies, numSims = 1000) {
  const results = [];

  for (const strategy of strategies) {
    console.log(`Simulating ${strategy.name}...`);

    const simulation = nexus.simulateRace({
      track: track,
      num_simulations: numSims,
      starting_compound: strategy.starting,
      pit_stops: strategy.stops
    });

    results.push({
      name: strategy.name,
      mean: simulation.mean_race_time,
      min: simulation.min_race_time,
      max: simulation.max_race_time,
      range: simulation.max_race_time - simulation.min_race_time
    });
  }

  // Sort by mean time
  results.sort((a, b) => a.mean - b.mean);

  console.log('\nSimulation Results:');
  results.forEach((r, i) => {
    console.log(`${i + 1}. ${r.name}`);
    console.log(`   Mean: ${r.mean.toFixed(1)}s`);
    console.log(`   Range: ${r.min.toFixed(1)}s - ${r.max.toFixed(1)}s`);
    console.log(`   Variance: ${r.range.toFixed(1)}s`);
  });

  return results;
}

// Compare strategies
const strategies = [
  {
    name: "1-stop (C3→C2)",
    starting: "C3",
    stops: [{ lap: 22, compound: "C2" }]
  },
  {
    name: "2-stop (C4→C3→C2)",
    starting: "C4",
    stops: [
      { lap: 15, compound: "C3" },
      { lap: 30, compound: "C2" }
    ]
  }
];

runSimulations("spa", strategies, 5000);
```

---

## Node.js Examples (NAPI)

### CLI Strategy Optimizer

```javascript
#!/usr/bin/env node

const f1nexus = require('@f1-nexus/node');

function optimizeForCircuit(circuit) {
  const params = {
    track: circuit,
    total_laps: undefined, // Use circuit default
    starting_fuel: 110.0,
    position: 5,
    available_compounds: ["C1", "C2", "C3"]
  };

  console.log(`\nOptimizing strategy for ${circuit}...`);

  const resultJson = f1nexus.optimizeStrategy(JSON.stringify(params));
  const strategy = JSON.parse(resultJson);

  console.log(`\nOptimal Strategy:`);
  console.log(`  Starting compound: ${strategy.starting_compound}`);
  console.log(`  Predicted race time: ${strategy.predicted_race_time.toFixed(1)}s`);
  console.log(`  Confidence: ${(strategy.confidence * 100).toFixed(1)}%`);
  console.log(`\nPit Stops:`);

  strategy.pit_stops.forEach((stop, i) => {
    console.log(`  ${i + 1}. Lap ${stop.lap} → ${stop.compound}`);
    console.log(`     Loss: ${stop.pit_loss.toFixed(1)}s, Reason: ${stop.reason}`);
  });
}

// Run for multiple circuits
const circuits = f1nexus.getCircuits();
circuits.slice(0, 3).forEach(optimizeForCircuit);
```

### Batch Race Simulation

```javascript
const f1nexus = require('@f1-nexus/node');
const fs = require('fs');

async function batchSimulate(configurations, outputFile) {
  const results = [];

  for (const config of configurations) {
    console.log(`Running ${config.name}...`);

    const params = {
      track: config.track,
      num_simulations: 10000, // High accuracy
      starting_compound: config.starting,
      pit_stops: config.stops
    };

    const resultJson = f1nexus.simulateRace(JSON.stringify(params));
    const simulation = JSON.parse(resultJson);

    results.push({
      name: config.name,
      track: config.track,
      ...simulation
    });
  }

  // Save to JSON file
  fs.writeFileSync(outputFile, JSON.stringify(results, null, 2));
  console.log(`\nResults saved to ${outputFile}`);

  return results;
}

// Configuration matrix
const configs = [
  { name: "Monaco 1-stop", track: "monaco", starting: "C3", stops: [{ lap: 40, compound: "C2" }] },
  { name: "Monaco 2-stop", track: "monaco", starting: "C4", stops: [{ lap: 25, compound: "C3" }, { lap: 52, compound: "C2" }] },
  { name: "Spa 1-stop", track: "spa", starting: "C3", stops: [{ lap: 22, compound: "C2" }] },
  { name: "Spa 2-stop", track: "spa", starting: "C4", stops: [{ lap: 15, compound: "C3" }, { lap: 30, compound: "C2" }] }
];

batchSimulate(configs, './simulation-results.json');
```

### Tire Degradation Service

```javascript
const f1nexus = require('@f1-nexus/node');

class TireDegradationService {
  constructor() {
    this.compounds = f1nexus.getTireCompounds().filter(c => c.startsWith('C'));
  }

  predictDegradation(compound, ageLaps, trackTemp, trackSeverity) {
    const params = {
      compound,
      age_laps: ageLaps,
      track_temp: trackTemp,
      track_severity: trackSeverity
    };

    const resultJson = f1nexus.predictTireLife(JSON.stringify(params));
    return JSON.parse(resultJson);
  }

  compareCompounds(ageLaps, trackTemp) {
    const results = {};

    for (const compound of this.compounds) {
      const prediction = this.predictDegradation(compound, ageLaps, trackTemp, 1.0);
      results[compound] = {
        wear: prediction.current_wear_percent,
        remaining: prediction.estimated_remaining_laps,
        grip: prediction.grip_multiplier
      };
    }

    return results;
  }

  findOptimalCompound(targetStintLength, trackTemp) {
    const scores = {};

    for (const compound of this.compounds) {
      const prediction = this.predictDegradation(compound, targetStintLength, trackTemp, 1.0);

      // Score = grip * (1 - wear)
      scores[compound] = prediction.grip_multiplier * (1 - prediction.current_wear_percent / 100);
    }

    // Return compound with highest score
    return Object.entries(scores).sort((a, b) => b[1] - a[1])[0][0];
  }
}

// Usage
const service = new TireDegradationService();

console.log('Tire comparison at lap 20:');
console.log(service.compareCompounds(20, 30.0));

console.log('\nOptimal compound for 25-lap stint at 35°C:');
console.log(service.findOptimalCompound(25, 35.0));
```

---

## React Integration

### React Hook

```javascript
// useF1Strategy.js
import { useState, useEffect } from 'react';
import init, { F1Nexus } from '@f1-nexus/wasm';

export function useF1Strategy() {
  const [nexus, setNexus] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function initWasm() {
      await init();
      setNexus(new F1Nexus());
      setLoading(false);
    }
    initWasm();
  }, []);

  const optimizeStrategy = (params) => {
    if (!nexus) return null;
    return nexus.optimizeStrategy(params);
  };

  const simulateRace = (params) => {
    if (!nexus) return null;
    return nexus.simulateRace(params);
  };

  const predictTireLife = (params) => {
    if (!nexus) return null;
    return nexus.predictTireLife(params);
  };

  return {
    loading,
    optimizeStrategy,
    simulateRace,
    predictTireLife,
    circuits: nexus?.getCircuits() || [],
    compounds: nexus?.getTireCompounds() || []
  };
}
```

### React Component

```jsx
// StrategyOptimizer.jsx
import React, { useState } from 'react';
import { useF1Strategy } from './useF1Strategy';

export function StrategyOptimizer() {
  const { loading, optimizeStrategy, circuits } = useF1Strategy();
  const [circuit, setCircuit] = useState('monaco');
  const [strategy, setStrategy] = useState(null);

  const handleOptimize = () => {
    const result = optimizeStrategy({
      track: circuit,
      available_compounds: ["C1", "C2", "C3"]
    });
    setStrategy(result);
  };

  if (loading) {
    return <div>Loading F1 Nexus...</div>;
  }

  return (
    <div>
      <h2>Strategy Optimizer</h2>

      <select value={circuit} onChange={(e) => setCircuit(e.target.value)}>
        {circuits.map(c => <option key={c} value={c}>{c}</option>)}
      </select>

      <button onClick={handleOptimize}>Optimize</button>

      {strategy && (
        <div>
          <h3>Optimal Strategy for {circuit}</h3>
          <p>Starting: {strategy.starting_compound}</p>
          <p>Predicted time: {strategy.predicted_race_time.toFixed(1)}s</p>

          <h4>Pit Stops:</h4>
          <ul>
            {strategy.pit_stops.map((stop, i) => (
              <li key={i}>
                Lap {stop.lap}: {stop.compound}
                ({stop.pit_loss.toFixed(1)}s loss, {stop.reason})
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
```

---

## Vue Integration

```vue
<!-- StrategyDashboard.vue -->
<template>
  <div v-if="!loading">
    <h2>F1 Strategy Dashboard</h2>

    <select v-model="selectedCircuit">
      <option v-for="circuit in circuits" :key="circuit" :value="circuit">
        {{ circuit }}
      </option>
    </select>

    <button @click="optimize">Optimize Strategy</button>

    <div v-if="strategy">
      <h3>Strategy for {{ selectedCircuit }}</h3>
      <p>Starting: {{ strategy.starting_compound }}</p>
      <p>Time: {{ strategy.predicted_race_time.toFixed(1) }}s</p>

      <div v-for="(stop, i) in strategy.pit_stops" :key="i">
        Lap {{ stop.lap }}: {{ stop.compound }}
      </div>
    </div>
  </div>
  <div v-else>
    Loading...
  </div>
</template>

<script>
import { ref, onMounted } from 'vue';
import init, { F1Nexus } from '@f1-nexus/wasm';

export default {
  setup() {
    const loading = ref(true);
    const nexus = ref(null);
    const circuits = ref([]);
    const selectedCircuit = ref('monaco');
    const strategy = ref(null);

    onMounted(async () => {
      await init();
      nexus.value = new F1Nexus();
      circuits.value = nexus.value.getCircuits();
      loading.value = false;
    });

    const optimize = () => {
      strategy.value = nexus.value.optimizeStrategy({
        track: selectedCircuit.value,
        available_compounds: ["C1", "C2", "C3"]
      });
    };

    return {
      loading,
      circuits,
      selectedCircuit,
      strategy,
      optimize
    };
  }
};
</script>
```

---

## Express.js API

```javascript
const express = require('express');
const f1nexus = require('@f1-nexus/node');

const app = express();
app.use(express.json());

// Optimize strategy endpoint
app.post('/api/optimize', (req, res) => {
  try {
    const params = {
      track: req.body.track || 'monaco',
      total_laps: req.body.total_laps,
      starting_fuel: req.body.starting_fuel || 110.0,
      position: req.body.position || 5,
      available_compounds: req.body.available_compounds || ["C1", "C2", "C3"]
    };

    const resultJson = f1nexus.optimizeStrategy(JSON.stringify(params));
    const strategy = JSON.parse(resultJson);

    res.json({ success: true, data: strategy });
  } catch (error) {
    res.status(400).json({ success: false, error: error.message });
  }
});

// Simulate race endpoint
app.post('/api/simulate', (req, res) => {
  try {
    const params = {
      track: req.body.track || 'spa',
      num_simulations: req.body.num_simulations || 100,
      starting_compound: req.body.starting_compound || 'C3',
      pit_stops: req.body.pit_stops || []
    };

    const resultJson = f1nexus.simulateRace(JSON.stringify(params));
    const simulation = JSON.parse(resultJson);

    res.json({ success: true, data: simulation });
  } catch (error) {
    res.status(400).json({ success: false, error: error.message });
  }
});

// Tire life prediction endpoint
app.post('/api/tire-life', (req, res) => {
  try {
    const params = {
      compound: req.body.compound || 'C3',
      age_laps: req.body.age_laps || 0,
      track_temp: req.body.track_temp || 30.0,
      track_severity: req.body.track_severity || 1.0
    };

    const resultJson = f1nexus.predictTireLife(JSON.stringify(params));
    const prediction = JSON.parse(resultJson);

    res.json({ success: true, data: prediction });
  } catch (error) {
    res.status(400).json({ success: false, error: error.message });
  }
});

// Get supported circuits
app.get('/api/circuits', (req, res) => {
  res.json({ success: true, data: f1nexus.getCircuits() });
});

// Get tire compounds
app.get('/api/compounds', (req, res) => {
  res.json({ success: true, data: f1nexus.getTireCompounds() });
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
  console.log(`F1 Nexus API running on port ${PORT}`);
  console.log(`F1 Nexus version: ${f1nexus.version()}`);
});
```

---

## CLI Usage

```bash
# Optimize strategy for Monaco
f1-nexus-cli optimize --track monaco --compounds C1,C2,C3

# Simulate race at Spa
f1-nexus-cli simulate --track spa --pit-stops 22:C2

# Predict tire life
f1-nexus-cli tire-life --compound C3 --age 20 --temp 35

# Get version
f1-nexus-cli --version
```

---

## Advanced Scenarios

### Multi-Strategy Comparison with Confidence Intervals

```javascript
async function compareStrategiesWithCI(track, strategies, confidence = 0.95) {
  const results = [];

  for (const strat of strategies) {
    const simulation = nexus.simulateRace({
      track,
      num_simulations: 10000, // Large sample size
      starting_compound: strat.starting,
      pit_stops: strat.stops
    });

    // Calculate confidence interval
    const margin = (simulation.max_race_time - simulation.min_race_time) / 2;
    const ciLower = simulation.mean_race_time - margin * (1 - confidence);
    const ciUpper = simulation.mean_race_time + margin * (1 - confidence);

    results.push({
      name: strat.name,
      mean: simulation.mean_race_time,
      ci: [ciLower, ciUpper],
      probability_fastest: 0 // Will calculate
    });
  }

  // Calculate probability of being fastest
  for (let i = 0; i < results.length; i++) {
    let fasterCount = 0;
    for (let j = 0; j < results.length; j++) {
      if (i !== j && results[i].mean < results[j].ci[1]) {
        fasterCount++;
      }
    }
    results[i].probability_fastest = fasterCount / (results.length - 1);
  }

  return results;
}
```

### Weather-Adaptive Strategy

```javascript
async function getWeatherAdaptiveStrategy(circuit, apiKey) {
  // Get weather forecast
  const weatherParams = { circuit, api_key: apiKey };
  const weather = await fetch('/mcp/get_weather_forecast', {
    method: 'POST',
    body: JSON.stringify(weatherParams)
  }).then(r => r.json());

  // Determine tire strategy based on rain probability
  let compounds;
  if (weather.forecast.rain_probability > 0.7) {
    compounds = ["Intermediate", "Wet"];
  } else if (weather.forecast.rain_probability > 0.3) {
    compounds = ["C2", "C3", "Intermediate"];
  } else {
    compounds = ["C1", "C2", "C3"];
  }

  // Optimize with weather-appropriate compounds
  const strategy = nexus.optimizeStrategy({
    track: circuit,
    available_compounds: compounds
  });

  return {
    strategy,
    weather: weather.forecast,
    risk_level: weather.forecast.rain_probability > 0.5 ? 'high' : 'low'
  };
}
```

---

## More Examples

For more examples, see:
- [Integration Tests](../tests/integration/)
- [Benchmark Suite](../crates/f1-nexus-bench/)
- [Example Projects](./examples/)

---

## Support

Questions? Check out:
- [API Documentation](./API.md)
- [GitHub Issues](https://github.com/mrkingsleyobi/f1-nexus/issues)
- [Discord Community](https://discord.gg/f1-nexus)
