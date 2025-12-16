# F1 Nexus Release Roadmap
## From 75% → 100% Completion & Publishing Guide

**Current Status: 75% Complete**
- ✅ Core Strategy Engine: 100%
- ✅ Telemetry Processing: 100%
- ✅ Real-time Streaming: 100%
- ⚠️ CLI: 90% (needs fixes)
- ❌ MCP Tools: 0%
- ❌ Weather APIs: 0%
- ⏳ Publishing: 54% (7/13 crates)
- ❌ WASM: 10%
- ❌ npm Package: 10%

---

## 🎯 Phase 1: Fix Critical Issues (2-3 hours)

### Task 1.1: Fix CLI Compilation Errors
**Current Issue:** 12 struct field mismatches in CLI commands

**Steps:**
```bash
cd /home/user/f1-nexus/f1-nexus

# Check the actual struct definitions
grep -r "pub struct RaceStrategy" crates/f1-nexus-core/src/
grep -r "pub struct PitStop" crates/f1-nexus-core/src/
grep -r "pub struct FuelStrategy" crates/f1-nexus-core/src/
grep -r "pub struct ErsDeploymentPlan" crates/f1-nexus-core/src/
grep -r "pub struct StrategyMetadata" crates/f1-nexus-core/src/

# Fix the mismatches in:
# - crates/f1-nexus-cli/src/commands/optimize.rs
# - crates/f1-nexus-cli/src/commands/simulate.rs

# Test compilation
cargo build -p f1-nexus-cli

# Run CLI tests
cargo test -p f1-nexus-cli
```

**Files to Fix:**
1. `crates/f1-nexus-cli/src/commands/simulate.rs` - Lines 20-50
2. `crates/f1-nexus-cli/src/commands/optimize.rs` - Lines 56-80

**Expected Outcome:** CLI compiles without errors and basic commands work.

---

## 🔧 Phase 2: Implement Missing Features (4-6 hours)

### Task 2.1: Implement Real MCP Tools
**Location:** `crates/f1-nexus-mcp/src/lib.rs`

**Tools to Implement:**
1. **Strategy Optimizer Tool**
```rust
// Add to lib.rs
pub fn strategy_optimizer_tool() -> McpTool {
    McpTool {
        name: "optimize_strategy".to_string(),
        description: "Optimize F1 race strategy for a given circuit".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "circuit": { "type": "string", "description": "Circuit ID (monaco, spa, silverstone)" },
                "current_lap": { "type": "integer", "description": "Current lap number" },
                "fuel_load": { "type": "number", "description": "Current fuel load in kg" },
                "tire_age": { "type": "integer", "description": "Current tire age in laps" }
            },
            "required": ["circuit"]
        }),
        handler: Box::new(|params| {
            // Implementation using f1_nexus_strategy::optimize_pit_strategy
            // Return JSON result
        })
    }
}
```

2. **Telemetry Analyzer Tool**
3. **Race Simulator Tool**
4. **Historical Data Query Tool**

**Steps:**
```bash
# Edit MCP tools
vim crates/f1-nexus-mcp/src/lib.rs

# Test MCP server
cargo run -p f1-nexus-cli -- mcp --transport stdio
```

### Task 2.2: Integrate Weather APIs
**Location:** `crates/f1-nexus-weather/src/api.rs`

**Implementation:**
```rust
// OpenWeatherMap integration
pub struct OpenWeatherMapClient {
    api_key: String,
    base_url: String,
}

impl OpenWeatherMapClient {
    pub async fn get_forecast(&self, lat: f64, lon: f64) -> Result<WeatherForecast> {
        // HTTP request to OpenWeatherMap API
        // Parse response into WeatherForecast struct
    }
}
```

**Get API Key:**
1. Sign up at https://openweathermap.org/api
2. Get free API key
3. Add to environment: `OPENWEATHER_API_KEY=your_key`

**Test:**
```bash
cargo test -p f1-nexus-weather
```

---

## 📦 Phase 3: Complete WASM & NAPI Bindings (3-4 hours)

### Task 3.1: Complete WASM Bindings
**Location:** `crates/f1-nexus-wasm/src/lib.rs`

**Implementation:**
```rust
use wasm_bindgen::prelude::*;
use f1_nexus_strategy::*;

#[wasm_bindgen]
pub struct WasmStrategyOptimizer {
    config: OptimizationConfig,
}

#[wasm_bindgen]
impl WasmStrategyOptimizer {
    #[wasm_bindgen(constructor)]
    pub fn new(circuit_json: &str) -> Result<WasmStrategyOptimizer, JsValue> {
        // Parse JSON to Circuit
        // Create OptimizationConfig
        // Return optimizer
    }

    pub fn optimize(&self) -> Result<JsValue, JsValue> {
        // Run optimization
        // Serialize result to JSON
        // Return as JsValue
    }
}
```

**Build:**
```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build WASM
cd crates/f1-nexus-wasm
wasm-pack build --target web --out-dir ../../pkg

# Test in browser
wasm-pack test --headless --firefox
```

### Task 3.2: Complete NAPI-RS Bindings
**Location:** `crates/f1-nexus-napi/src/lib.rs`

**Implementation:**
```rust
#[napi]
pub struct F1Nexus {
    // Internal state
}

#[napi]
impl F1Nexus {
    #[napi(constructor)]
    pub fn new() -> Self {
        F1Nexus {}
    }

    #[napi]
    pub async fn optimize_strategy(&self, circuit: String) -> napi::Result<String> {
        // Call Rust optimization
        // Return JSON result
    }

    #[napi]
    pub async fn simulate_race(&self, config: String) -> napi::Result<String> {
        // Call race simulator
        // Return JSON result
    }
}
```

**Build:**
```bash
cd crates/f1-nexus-napi
npm install
npm run build

# Test Node.js bindings
npm test
```

---

## 📚 Phase 4: Documentation (2-3 hours)

### Task 4.1: API Documentation
```bash
# Generate docs for all crates
cargo doc --no-deps --all-features --open

# Add examples to README.md
```

### Task 4.2: Usage Examples
Create `examples/` directory with:
1. `basic_optimization.rs`
2. `race_simulation.rs`
3. `telemetry_streaming.rs`
4. `mcp_server.rs`

**Example:**
```rust
// examples/basic_optimization.rs
use f1_nexus_strategy::*;

fn main() -> anyhow::Result<()> {
    // Create circuit
    let circuit = Circuit { /* ... */ };

    // Setup config
    let config = OptimizationConfig { /* ... */ };

    // Optimize
    let strategy = optimize_pit_strategy(&config)?;

    println!("Optimal strategy: {:?}", strategy);
    Ok(())
}
```

---

## 🚀 Phase 5: Publishing to crates.io (1-2 hours)

### Publish Remaining 6 Crates

**Status:** 7/13 published, 6 remaining

**Remaining Crates:**
1. f1-nexus-agents
2. f1-nexus-wasm
3. f1-nexus-cli
4. f1-nexus-mcp
5. f1-nexus-napi
6. f1-nexus (workspace)

**Publishing Order:**
```bash
# Wait for rate limit reset (check status)
cargo search f1-nexus-agents
# If "Rate limit exceeded", wait 24 hours from last publish

# When ready, publish in dependency order:

# 1. Agents (no new dependencies)
cd crates/f1-nexus-agents
cargo publish --dry-run  # Test first
cargo publish

# 2. WASM (depends on core, strategy)
cd ../f1-nexus-wasm
cargo publish --dry-run
cargo publish

# 3. MCP (depends on core, strategy)
cd ../f1-nexus-mcp
cargo publish --dry-run
cargo publish

# 4. NAPI (depends on core, strategy)
cd ../f1-nexus-napi
cargo publish --dry-run
cargo publish

# 5. CLI (depends on all)
cd ../f1-nexus-cli
cargo publish --dry-run
cargo publish

# 6. Workspace (meta-package)
cd ../..
cargo publish --dry-run
cargo publish
```

**Pre-publish Checklist:**
- ✅ All dependencies use published versions (not path)
- ✅ Version numbers are consistent
- ✅ README.md exists
- ✅ License files included
- ✅ Examples compile
- ✅ Tests pass
- ✅ Documentation complete

**Handle Rate Limits:**
```bash
# Check when you can publish next
# crates.io allows 10 publishes per 10 minutes
# Wait if you hit the limit

# Use --dry-run to test without publishing
cargo publish --dry-run
```

---

## 📦 Phase 6: Publishing to npm (1-2 hours)

### Prepare npm Package

**Location:** Root directory needs `package.json`

**Create package.json:**
```json
{
  "name": "@f1-nexus/core",
  "version": "1.0.0-alpha.1",
  "description": "Next-generation Formula 1 strategy optimization platform",
  "main": "index.js",
  "types": "index.d.ts",
  "files": [
    "index.js",
    "index.d.ts",
    "pkg/",
    "crates/f1-nexus-napi/index.node"
  ],
  "scripts": {
    "build": "npm run build:wasm && npm run build:napi",
    "build:wasm": "cd crates/f1-nexus-wasm && wasm-pack build",
    "build:napi": "cd crates/f1-nexus-napi && npm run build",
    "test": "node test/integration.test.js",
    "prepublishOnly": "npm run build"
  },
  "keywords": [
    "formula1",
    "f1",
    "racing",
    "strategy",
    "optimization",
    "telemetry",
    "wasm",
    "rust"
  ],
  "author": "F1 Nexus Team",
  "license": "Apache-2.0",
  "repository": {
    "type": "git",
    "url": "https://github.com/mrkingsleyobi/f1-nexus"
  },
  "engines": {
    "node": ">=16.0.0"
  },
  "dependencies": {},
  "devDependencies": {
    "@napi-rs/cli": "^2.18.0",
    "wasm-pack": "^0.12.0"
  },
  "napi": {
    "name": "f1-nexus-napi",
    "triples": {
      "additional": [
        "x86_64-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "x86_64-pc-windows-msvc"
      ]
    }
  }
}
```

**Create TypeScript Definitions (index.d.ts):**
```typescript
export interface Circuit {
  id: string;
  name: string;
  country: string;
  length: number;
  num_turns: number;
  lap_record: number;
}

export interface OptimizationResult {
  predicted_time: number;
  strategy: RaceStrategy;
}

export interface RaceStrategy {
  starting_compound: string;
  pit_stops: PitStop[];
}

export interface PitStop {
  lap: number;
  reason: string;
}

export class F1Nexus {
  constructor();
  optimizeStrategy(circuit: string): Promise<OptimizationResult>;
  simulateRace(config: string): Promise<string>;
}

export function optimizeStrategy(circuit: Circuit): Promise<OptimizationResult>;
export function simulateRace(laps: number): Promise<number>;
```

**Build npm Package:**
```bash
# Build all bindings
npm run build

# Test locally
npm link
cd ~/test-project
npm link @f1-nexus/core
node test.js

# Unlink after testing
npm unlink @f1-nexus/core
```

**Publish to npm:**
```bash
# Login to npm (one time)
npm login

# Publish (use --dry-run first)
npm publish --access public --dry-run
npm publish --access public

# Publish with tag for alpha
npm publish --access public --tag alpha
```

**Verify Publication:**
```bash
# Check on npm
npm view @f1-nexus/core

# Install in test project
npm install @f1-nexus/core@alpha
```

---

## ✅ Phase 7: Final Testing & CI/CD (2-3 hours)

### Task 7.1: Integration Tests
```bash
# Create tests/integration/
mkdir -p tests/integration

# Write end-to-end tests
# Test CLI commands
# Test MCP server
# Test WASM in browser
# Test Node.js bindings
```

### Task 7.2: Set Up GitHub Actions
**Create `.github/workflows/ci.yml`:**
```yaml
name: CI

on:
  push:
    branches: [ main, claude/* ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      - name: Run tests
        run: cargo test --all-features
      - name: Build CLI
        run: cargo build --release -p f1-nexus-cli
      - name: Build WASM
        run: |
          curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
          cd crates/f1-nexus-wasm
          wasm-pack build

  publish:
    if: startsWith(github.ref, 'refs/tags/v')
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Publish to crates.io
        run: cargo publish --token ${{ secrets.CARGO_TOKEN }}
      - name: Publish to npm
        run: |
          echo "//registry.npmjs.org/:_authToken=${{ secrets.NPM_TOKEN }}" > ~/.npmrc
          npm publish --access public
```

### Task 7.3: Performance Benchmarks
```bash
# Add criterion benchmarks
mkdir -p benches

# Run benchmarks
cargo bench
```

---

## 📊 Completion Checklist

### Development (75% → 100%)
- [ ] **Phase 1:** Fix CLI compilation errors (90% → 100%)
- [ ] **Phase 2:** Implement MCP tools (0% → 100%)
- [ ] **Phase 2:** Integrate weather APIs (0% → 100%)
- [ ] **Phase 3:** Complete WASM bindings (10% → 100%)
- [ ] **Phase 3:** Complete NAPI bindings (10% → 100%)
- [ ] **Phase 4:** Write documentation (0% → 100%)
- [ ] **Phase 7:** Add integration tests (0% → 100%)
- [ ] **Phase 7:** Set up CI/CD (0% → 100%)

### Publishing
- [ ] **Phase 5:** Publish 6 remaining crates to crates.io (54% → 100%)
- [ ] **Phase 6:** Publish npm package (0% → 100%)
- [ ] **Phase 7:** Create GitHub release
- [ ] **Phase 7:** Update documentation website

---

## 🎯 Quick Start Commands

### For Immediate Development:
```bash
# 1. Fix CLI (highest priority)
cd /home/user/f1-nexus/f1-nexus
cargo build -p f1-nexus-cli 2>&1 | grep "error\[" > errors.txt
# Fix each error in errors.txt

# 2. Test everything
cargo test --all-features

# 3. Build release binaries
cargo build --release
```

### For Publishing:
```bash
# Check rate limit status
cargo search f1-nexus-agents

# When ready (after 24h), publish all:
./scripts/publish-all.sh  # Create this script
```

### For npm Package:
```bash
# Build and test
npm run build
npm test

# Publish
npm publish --access public --tag alpha
```

---

## 📈 Timeline Estimate

| Phase | Time | Total |
|-------|------|-------|
| Phase 1: Fix CLI | 2-3h | 77% |
| Phase 2: Features | 4-6h | 85% |
| Phase 3: Bindings | 3-4h | 92% |
| Phase 4: Docs | 2-3h | 95% |
| Phase 5: crates.io | 1-2h | 97% |
| Phase 6: npm | 1-2h | 99% |
| Phase 7: Testing/CI | 2-3h | 100% |
| **TOTAL** | **15-23 hours** | **100%** |

**Fast Track (MVP Release):**
- Phase 1 (Fix CLI): 2h → 77%
- Phase 2 (MCP only): 2h → 82%
- Phase 5 (Publish crates): 1h → 97%
- **Total: 5 hours → 97% (MVP Ready)**

---

## 🚀 Ready to Release!

Once all phases complete:
1. Tag release: `git tag -a v1.0.0-alpha.1 -m "First alpha release"`
2. Push tag: `git push origin v1.0.0-alpha.1`
3. GitHub Actions will auto-publish
4. Announce on social media
5. Update documentation site

**Questions?** Check [CONTRIBUTING.md](CONTRIBUTING.md) or open an issue.
