# F1 Nexus Quick Start Guide
## 🏎️ From Development to Production

**Current Status: 75% Complete → Ready for 100%**

---

## 🎯 Fastest Path to 100% (5 hours)

### Option A: MVP Release (97% Complete)
```bash
# 1. Fix CLI (2 hours)
cd /home/user/f1-nexus/f1-nexus
cargo build -p f1-nexus-cli 2>&1 | grep "error" > cli-errors.txt
# Fix struct mismatches in cli-errors.txt

# 2. Implement MCP Tools (2 hours)
vim crates/f1-nexus-mcp/src/lib.rs
# Add real tool implementations

# 3. Publish to crates.io (1 hour)
./scripts/publish-crates.sh

# DONE! 97% Complete, fully usable
```

### Option B: Full Release (100% Complete)
Follow the [RELEASE-ROADMAP.md](RELEASE-ROADMAP.md) for complete implementation.

---

## 📦 Publishing Commands

### Publish to crates.io
```bash
# Automated script
./scripts/publish-crates.sh

# Or manually:
cd crates/f1-nexus-agents && cargo publish
cd ../f1-nexus-wasm && cargo publish
cd ../f1-nexus-mcp && cargo publish
cd ../f1-nexus-napi && cargo publish
cd ../f1-nexus-cli && cargo publish
```

### Publish to npm
```bash
# Automated script
./scripts/publish-npm.sh

# Or manually:
npm run build
npm publish --access public --tag alpha
```

---

## 🔧 Fix CLI Issues (First Priority)

### The Problem
12 compilation errors due to struct field mismatches.

### The Fix
```bash
# 1. Identify mismatches
cargo build -p f1-nexus-cli 2>&1 | grep "no field"

# 2. Check actual structs
grep -A 20 "pub struct RaceStrategy" crates/f1-nexus-core/src/strategy.rs
grep -A 20 "pub struct PitStop" crates/f1-nexus-core/src/types.rs

# 3. Update files:
#    - crates/f1-nexus-cli/src/commands/simulate.rs (lines 20-50)
#    - crates/f1-nexus-cli/src/commands/optimize.rs (lines 56-80)

# 4. Verify
cargo build -p f1-nexus-cli
cargo run -p f1-nexus-cli -- optimize --track monaco
```

---

## 🛠️ Implement MCP Tools (Second Priority)

### Location
`crates/f1-nexus-mcp/src/lib.rs`

### Quick Implementation
```rust
use f1_nexus_strategy::*;
use f1_nexus_core::*;
use serde_json::json;

pub fn get_mcp_tools() -> Vec<McpTool> {
    vec![
        strategy_optimizer_tool(),
        race_simulator_tool(),
        telemetry_analyzer_tool(),
    ]
}

fn strategy_optimizer_tool() -> McpTool {
    McpTool {
        name: "optimize_strategy".to_string(),
        description: "Optimize pit stop strategy".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "circuit": {"type": "string"},
                "fuel": {"type": "number"}
            }
        }),
        handler: Box::new(|params| {
            // Parse params
            // Call optimize_pit_strategy()
            // Return JSON result
            Ok(json!({"success": true}))
        })
    }
}
```

### Test
```bash
cargo run -p f1-nexus-cli -- mcp --transport stdio
```

---

## 📚 Documentation Checklist

- [ ] README.md with examples
- [ ] API documentation (cargo doc)
- [ ] Usage examples in examples/
- [ ] CHANGELOG.md
- [ ] CONTRIBUTING.md

### Generate Docs
```bash
cargo doc --no-deps --all-features --open
```

---

## ✅ Pre-Publish Checklist

### Before crates.io
- [ ] All tests pass: `cargo test --all-features`
- [ ] Lints pass: `cargo clippy --all-features`
- [ ] Format check: `cargo fmt --check`
- [ ] Version numbers consistent
- [ ] Dependencies use published versions (not `path`)
- [ ] README.md exists
- [ ] LICENSE file included
- [ ] No secrets in code

### Before npm
- [ ] WASM builds: `cd crates/f1-nexus-wasm && wasm-pack build`
- [ ] NAPI builds: `cd crates/f1-nexus-napi && npm run build`
- [ ] package.json complete
- [ ] TypeScript definitions (index.d.ts)
- [ ] Tests pass: `npm test`
- [ ] Logged in: `npm whoami`

---

## 🚀 Release Process

### 1. Version Bump
```bash
# Update version in all Cargo.toml files
VERSION="1.0.0-alpha.2"
find . -name "Cargo.toml" -exec sed -i "s/version = \"1.0.0-alpha.1\"/version = \"$VERSION\"/" {} \;

# Update package.json
sed -i "s/\"version\": \"1.0.0-alpha.1\"/\"version\": \"$VERSION\"/" package.json
```

### 2. Commit & Tag
```bash
git add -A
git commit -m "Release v$VERSION"
git tag -a "v$VERSION" -m "Release v$VERSION"
git push origin claude/publish-rust-npm-01AjEo53J9BLKqPXFKFPgKz8
git push origin "v$VERSION"
```

### 3. Publish
```bash
# crates.io
./scripts/publish-crates.sh

# npm
./scripts/publish-npm.sh
```

### 4. Announce
- Create GitHub Release
- Post on social media
- Update documentation site

---

## 📊 Progress Tracker

### Core Development
- [x] Strategy Engine (100%)
- [x] Telemetry Processing (100%)
- [x] Real-time Streaming (100%)
- [ ] CLI (90% - fix errors)
- [ ] MCP Tools (0% - implement)
- [ ] Weather APIs (0% - integrate)

### Bindings
- [ ] WASM (10% - complete)
- [ ] NAPI (10% - complete)

### Publishing
- [x] crates.io (54% - 7/13 published)
- [ ] npm (0% - not published)

### Documentation
- [ ] API Docs (0%)
- [ ] Examples (0%)
- [ ] Tutorials (0%)

---

## 🆘 Troubleshooting

### "Rate limit exceeded" on crates.io
**Solution:** Wait 24 hours between publishes. Max 10 publishes per 10 minutes.

### npm publish fails with "401 Unauthorized"
**Solution:** Run `npm login` first.

### WASM build fails
**Solution:**
```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

### NAPI build fails
**Solution:**
```bash
npm install -g @napi-rs/cli
cd crates/f1-nexus-napi
npm install
```

### Tests fail
**Solution:** Check logs, fix issues, ensure all dependencies are correct.

---

## 🎯 Next Actions

**Today:**
1. ✅ Read this guide
2. ⏳ Fix CLI compilation errors
3. ⏳ Implement basic MCP tools

**This Week:**
4. ⏳ Publish to crates.io
5. ⏳ Complete WASM/NAPI bindings
6. ⏳ Publish to npm

**Next Week:**
7. ⏳ Write documentation
8. ⏳ Add integration tests
9. ⏳ Set up CI/CD

---

## 📞 Support

- **Documentation:** [RELEASE-ROADMAP.md](RELEASE-ROADMAP.md)
- **Issues:** https://github.com/mrkingsleyobi/f1-nexus/issues
- **Discussions:** https://github.com/mrkingsleyobi/f1-nexus/discussions

---

**Ready to reach 100%? Start with:** `./scripts/publish-crates.sh --dry-run`
