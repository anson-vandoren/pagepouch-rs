#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
BINARY_NAME="pagepouch"
REPO_NAME="pagepouch-rs"

usage() {
    echo "Usage: $0 <version> [release-notes]"
    echo "Example: $0 v1.0.0 'Initial release with bookmark management'"
    echo "Example: $0 v1.0.1 'Bug fixes and performance improvements'"
    exit 1
}

if [[ $# -lt 1 ]]; then
    usage
fi

VERSION="$1"
RELEASE_NOTES="${2:-Release $VERSION}"

# Validate version format
if [[ ! "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}❌ Version must be in format vX.Y.Z (e.g., v1.0.0)${NC}"
    exit 1
fi

echo -e "${YELLOW}🚀 Starting release process for ${VERSION}${NC}"

# Check if we're in a clean git state
if [[ -n $(git status --porcelain) ]]; then
    echo -e "${RED}❌ Working directory is not clean. Commit your changes first.${NC}"
    exit 1
fi

# Make sure we're on main branch
CURRENT_BRANCH=$(git branch --show-current)
if [[ "$CURRENT_BRANCH" != "main" ]]; then
    echo -e "${YELLOW}⚠️  Not on main branch (currently on $CURRENT_BRANCH). Continue? (y/N)${NC}"
    read -r response
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Pull latest changes
echo -e "${YELLOW}📥 Pulling latest changes${NC}"
git pull origin "$CURRENT_BRANCH"

# Check if tag already exists
if git tag -l | grep -q "^${VERSION}$"; then
    echo -e "${RED}❌ Tag ${VERSION} already exists${NC}"
    exit 1
fi

# Run tests
echo -e "${YELLOW}🧪 Running tests${NC}"
if command -v mise >/dev/null 2>&1; then
    mise run test
else
    cargo test
fi

# Run clippy
echo -e "${YELLOW}🔍 Running clippy${NC}"
if command -v mise >/dev/null 2>&1; then
    mise run clippy
else
    cargo clippy --all-targets --all-features -- -D warnings
fi

# Build release binary
echo -e "${YELLOW}🔨 Building release binary${NC}"
cargo build --release

# Verify binary was created
BINARY_PATH="target/release/${BINARY_NAME}"
if [[ ! -f "$BINARY_PATH" ]]; then
    echo -e "${RED}❌ Binary not found at ${BINARY_PATH}${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Binary built successfully: $(ls -lh $BINARY_PATH | awk '{print $5}')${NC}"

# Create and push git tag
echo -e "${YELLOW}🏷️  Creating git tag ${VERSION}${NC}"
git tag -a "$VERSION" -m "Release $VERSION"
git push origin "$VERSION"

# Create GitHub release
echo -e "${YELLOW}📦 Creating GitHub release${NC}"
if ! command -v gh >/dev/null 2>&1; then
    echo -e "${RED}❌ GitHub CLI (gh) is required but not installed${NC}"
    echo "Install it with: brew install gh (macOS) or https://cli.github.com/manual/installation"
    exit 1
fi

# Check if logged in to GitHub
if ! gh auth status >/dev/null 2>&1; then
    echo -e "${YELLOW}🔐 Please log in to GitHub CLI${NC}"
    gh auth login
fi

# Create the release
gh release create "$VERSION" \
    --title "PagePouch $VERSION" \
    --notes "$RELEASE_NOTES" \
    "$BINARY_PATH#${BINARY_NAME}-${VERSION}"

echo -e "${GREEN}✅ Release $VERSION created successfully!${NC}"
echo -e "${GREEN}📡 Webhook will deploy to production automatically${NC}"
echo -e "${GREEN}🌐 Monitor deployment at: https://pagepouch.com${NC}"

# Optional: wait a bit and check deployment
echo -e "${YELLOW}⏳ Waiting 30 seconds before checking deployment...${NC}"
sleep 30

echo -e "${YELLOW}🔍 Checking if deployment was successful...${NC}"
if curl -sf https://pagepouch.com/health >/dev/null 2>&1; then
    echo -e "${GREEN}✅ Deployment appears successful - health check passed${NC}"
else
    echo -e "${YELLOW}⚠️  Health check failed - check production logs${NC}"
fi