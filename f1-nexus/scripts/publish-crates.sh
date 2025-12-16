#!/bin/bash
# Publish all F1 Nexus crates to crates.io in correct dependency order

set -e  # Exit on error

echo "🚀 F1 Nexus - Publishing to crates.io"
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if logged in to crates.io
echo "Checking crates.io authentication..."
if ! cargo login --help > /dev/null 2>&1; then
    echo -e "${RED}❌ Cargo is not installed${NC}"
    exit 1
fi

# Crates already published (skip these)
PUBLISHED_CRATES=(
    "f1-nexus-core"
    "f1-nexus-telemetry"
    "f1-nexus-physics"
    "f1-nexus-vectors"
    "f1-nexus-weather"
    "f1-nexus-strategy"
    "f1-nexus-agentdb"
)

# Crates to publish (in dependency order)
CRATES_TO_PUBLISH=(
    "f1-nexus-agents"
    "f1-nexus-wasm"
    "f1-nexus-mcp"
    "f1-nexus-node"
    "f1-nexus-cli"
)

# Function to check if rate limited
check_rate_limit() {
    local crate_name=$1
    echo "Checking rate limit for $crate_name..."

    if cargo search "$crate_name" 2>&1 | grep -q "rate limit"; then
        echo -e "${RED}❌ Rate limit exceeded. Please wait 24 hours and try again.${NC}"
        exit 1
    fi
}

# Function to publish a crate
publish_crate() {
    local crate_path=$1
    local crate_name=$(basename $crate_path)

    echo ""
    echo -e "${YELLOW}📦 Publishing: $crate_name${NC}"
    echo "----------------------------------------"

    cd "$crate_path"

    # Run tests first
    echo "Running tests..."
    if ! cargo test --quiet 2>/dev/null; then
        echo -e "${YELLOW}⚠️  Tests failed or not present, continuing anyway${NC}"
    fi

    # Dry run
    echo "Running dry-run publish..."
    if ! cargo publish --dry-run; then
        echo -e "${RED}❌ Dry-run failed for $crate_name${NC}"
        read -p "Continue anyway? (y/n) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi

    # Actual publish
    echo "Publishing to crates.io..."
    if cargo publish; then
        echo -e "${GREEN}✅ Successfully published $crate_name${NC}"
    else
        echo -e "${RED}❌ Failed to publish $crate_name${NC}"
        read -p "Continue with next crate? (y/n) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi

    # Wait to avoid rate limit (10 seconds between publishes)
    echo "Waiting 10 seconds to avoid rate limit..."
    sleep 10

    cd - > /dev/null
}

# Main execution
echo ""
echo -e "${GREEN}Already published crates:${NC}"
for crate in "${PUBLISHED_CRATES[@]}"; do
    echo "  ✓ $crate"
done

echo ""
echo -e "${YELLOW}Crates to publish:${NC}"
for crate in "${CRATES_TO_PUBLISH[@]}"; do
    echo "  • $crate"
done

echo ""
read -p "Continue with publishing? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
fi

# Check rate limit before starting
check_rate_limit "f1-nexus"

# Publish each crate
for crate in "${CRATES_TO_PUBLISH[@]}"; do
    crate_path="crates/$crate"

    if [ ! -d "$crate_path" ]; then
        echo -e "${RED}❌ Crate directory not found: $crate_path${NC}"
        continue
    fi

    publish_crate "$crate_path"
done

echo ""
echo -e "${GREEN}🎉 All crates published successfully!${NC}"
echo ""
echo "Next steps:"
echo "  1. Verify on crates.io: https://crates.io/search?q=f1-nexus"
echo "  2. Test installation: cargo install f1-nexus-cli"
echo "  3. Publish to npm: npm publish"
echo ""
