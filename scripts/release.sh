#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BINARY_NAME="pagepouch-rs"

usage() {
    echo "Usage: $0"
    echo ""
    echo "This script automatically releases a new version based on CHANGELOG.md and Cargo.toml"
    echo "It will:"
    echo "  - Extract version from Cargo.toml"
    echo "  - Validate it matches the most recent release in CHANGELOG.md"
    echo "  - Extract release notes from CHANGELOG.md"
    echo "  - Create and push git tag"
    echo "  - Create GitHub release with binary"
    exit 1
}

if [[ $# -gt 0 ]]; then
    usage
fi

# Function to extract version from Cargo.toml
get_cargo_version() {
    grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'
}

# Function to get the most recent version from CHANGELOG.md (first non-Unreleased section)
get_changelog_version() {
    grep -E '^## \[[0-9]+\.[0-9]+\.[0-9]+\]' CHANGELOG.md | head -1 | sed 's/^## \[\([0-9]*\.[0-9]*\.[0-9]*\)\].*/\1/'
}

# Function to extract release notes for a specific version from CHANGELOG.md
get_release_notes() {
    local version="$1"
    # Find the section for this version and extract content until the next ## section
    awk "/^## \[$version\]/ {found=1; next} found && /^## / {exit} found {print}" CHANGELOG.md | sed '/^$/d'
}

# Function to validate changelog content has substance
validate_changelog_content() {
    local version="$1"
    local notes
    notes=$(get_release_notes "$version")

    # Check if there's at least one ### heading with bullet points
    if ! echo "$notes" | grep -q "^### "; then
        echo -e "${RED}❌ CHANGELOG.md for version $version has no subsections (### headings)${NC}"
        return 1
    fi

    if ! echo "$notes" | grep -q "^- "; then
        echo -e "${RED}❌ CHANGELOG.md for version $version has no bullet points (- items)${NC}"
        return 1
    fi

    return 0
}

# Get versions
CARGO_VERSION=$(get_cargo_version)
CHANGELOG_VERSION=$(get_changelog_version)
VERSION="v$CARGO_VERSION"

echo -e "${BLUE}📋 Validating release configuration${NC}"

# Validate Cargo.toml version format
if [[ ! "$CARGO_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}❌ Cargo.toml version '$CARGO_VERSION' must be in format X.Y.Z${NC}"
    exit 1
fi

# Validate CHANGELOG.md exists and is readable
if [[ ! -r CHANGELOG.md ]]; then
    echo -e "${RED}❌ CHANGELOG.md not found or not readable${NC}"
    exit 1
fi

# Check if CHANGELOG.md has the expected structure
if ! grep -q "^## \[Unreleased\]" CHANGELOG.md; then
    echo -e "${RED}❌ CHANGELOG.md missing '## [Unreleased]' section${NC}"
    exit 1
fi

# Validate versions match
if [[ "$CARGO_VERSION" != "$CHANGELOG_VERSION" ]]; then
    echo -e "${RED}❌ Version mismatch:${NC}"
    echo -e "  Cargo.toml: $CARGO_VERSION"
    echo -e "  CHANGELOG.md: $CHANGELOG_VERSION"
    echo -e "  Please update one to match the other"
    exit 1
fi

# Validate changelog content
if ! validate_changelog_content "$CARGO_VERSION"; then
    exit 1
fi

echo -e "${GREEN}✅ Version validation passed: $VERSION${NC}"

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

# Pull latest changes and tags
echo -e "${YELLOW}📥 Pulling latest changes and tags${NC}"
git pull origin "$CURRENT_BRANCH"
git fetch --tags

# Check if tag already exists locally or remotely
if git tag -l | grep -q "^${VERSION}$"; then
    echo -e "${RED}❌ Tag ${VERSION} already exists locally${NC}"
    exit 1
fi

if git ls-remote --tags origin | grep -q "refs/tags/${VERSION}$"; then
    echo -e "${RED}❌ Tag ${VERSION} already exists on remote${NC}"
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

# Extract release notes from CHANGELOG.md
echo -e "${YELLOW}📝 Extracting release notes from CHANGELOG.md${NC}"
RELEASE_NOTES=$(get_release_notes "$CARGO_VERSION")

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
