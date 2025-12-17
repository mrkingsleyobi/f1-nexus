#!/bin/bash
# Build and publish F1 Nexus to npm

set -e  # Exit on error

echo "📦 F1 Nexus - Publishing to npm"
echo "================================"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check if npm is installed
if ! command -v npm &> /dev/null; then
    echo -e "${RED}❌ npm is not installed${NC}"
    exit 1
fi

# Check if logged in to npm
echo "Checking npm authentication..."
if ! npm whoami &> /dev/null; then
    echo -e "${YELLOW}⚠️  Not logged in to npm${NC}"
    echo "Please run: npm login"
    exit 1
fi

NPM_USER=$(npm whoami)
echo -e "${GREEN}✓ Logged in as: $NPM_USER${NC}"

# Check current directory
if [ ! -f "package.json" ]; then
    echo -e "${RED}❌ package.json not found. Are you in the project root?${NC}"
    exit 1
fi

# Show package info
PKG_NAME=$(node -p "require('./package.json').name")
PKG_VERSION=$(node -p "require('./package.json').version")

echo ""
echo "Package: $PKG_NAME"
echo "Version: $PKG_VERSION"
echo ""

# Build WASM
echo -e "${YELLOW}Building WASM...${NC}"
if [ -d "crates/f1-nexus-wasm" ]; then
    cd crates/f1-nexus-wasm

    # Install wasm-pack if not present
    if ! command -v wasm-pack &> /dev/null; then
        echo "Installing wasm-pack..."
        curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
    fi

    # Build WASM
    wasm-pack build --target web --out-dir ../../pkg
    echo -e "${GREEN}✓ WASM built successfully${NC}"

    cd ../..
else
    echo -e "${YELLOW}⚠️  WASM crate not found, skipping${NC}"
fi

# Build NAPI
echo ""
echo -e "${YELLOW}Building NAPI bindings...${NC}"
if [ -d "crates/f1-nexus-napi" ]; then
    cd crates/f1-nexus-napi

    # Install dependencies
    if [ -f "package.json" ]; then
        npm install
        npm run build
        echo -e "${GREEN}✓ NAPI built successfully${NC}"
    else
        echo -e "${YELLOW}⚠️  package.json not found in NAPI crate${NC}"
    fi

    cd ../..
else
    echo -e "${YELLOW}⚠️  NAPI crate not found, skipping${NC}"
fi

# Run tests
echo ""
echo -e "${YELLOW}Running tests...${NC}"
if [ -f "test/integration.test.js" ]; then
    node test/integration.test.js
    echo -e "${GREEN}✓ Tests passed${NC}"
else
    echo -e "${YELLOW}⚠️  No tests found, skipping${NC}"
fi

# Check what will be published
echo ""
echo -e "${YELLOW}Checking package contents...${NC}"
npm pack --dry-run

echo ""
echo "Files to be published:"
npm publish --dry-run 2>&1 | grep -A 100 "package size"

# Confirm publish
echo ""
read -p "Publish $PKG_NAME@$PKG_VERSION to npm? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
fi

# Publish to npm
echo ""
echo -e "${YELLOW}Publishing to npm...${NC}"

# For alpha/beta versions, use tags
if [[ $PKG_VERSION == *"alpha"* ]]; then
    TAG="alpha"
elif [[ $PKG_VERSION == *"beta"* ]]; then
    TAG="beta"
else
    TAG="latest"
fi

echo "Publishing with tag: $TAG"

if npm publish --access public --tag $TAG; then
    echo ""
    echo -e "${GREEN}🎉 Successfully published to npm!${NC}"
    echo ""
    echo "Install with:"
    echo "  npm install $PKG_NAME@$TAG"
    echo ""
    echo "View on npm:"
    echo "  https://www.npmjs.com/package/${PKG_NAME//@/}"
    echo ""
else
    echo -e "${RED}❌ Failed to publish to npm${NC}"
    exit 1
fi

# Create git tag
echo ""
read -p "Create git tag v$PKG_VERSION? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    git tag -a "v$PKG_VERSION" -m "Release v$PKG_VERSION"
    git push origin "v$PKG_VERSION"
    echo -e "${GREEN}✓ Git tag created and pushed${NC}"
fi

echo ""
echo -e "${GREEN}All done! 🚀${NC}"
