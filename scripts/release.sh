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
    cat << EOF
Usage: $0 [OPTION]

Release PagePouch based on CHANGELOG.md and Cargo.toml

OPTIONS:
    --make-release      create a new release version interactively
    -h, --help         display this help and exit

By default, releases the current version from Cargo.toml after validating
it matches CHANGELOG.md. With --make-release, prompts for a new version,
updates both files, commits changes, and then proceeds with the release.

EOF
    exit 1
}

# Parse arguments
MAKE_RELEASE=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --make-release)
            MAKE_RELEASE=true
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done

# Function to extract version from Cargo.toml
get_cargo_version() {
    grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'
}

# Function to compare two semantic versions (returns 0 if v1 > v2, 1 if v1 <= v2)
version_greater() {
    local v1="$1"
    local v2="$2"
    
    # Split versions into arrays
    IFS='.' read -ra V1 <<< "$v1"
    IFS='.' read -ra V2 <<< "$v2"
    
    # Compare major, minor, patch
    for i in {0..2}; do
        local n1=${V1[i]:-0}
        local n2=${V2[i]:-0}
        if (( n1 > n2 )); then
            return 0
        elif (( n1 < n2 )); then
            return 1
        fi
    done
    return 1  # Equal versions
}

# Function to increment patch version
increment_patch() {
    local version="$1"
    IFS='.' read -ra V <<< "$version"
    echo "${V[0]}.${V[1]}.$((V[2] + 1))"
}

# Function to check if unreleased section has content
check_unreleased_changes() {
    # Get content between ## [Unreleased] and the next ## section
    local unreleased_content
    unreleased_content=$(awk '/^## \[Unreleased\]/ {found=1; next} found && /^## / {exit} found {print}' CHANGELOG.md)
    
    # Check if there's at least one ### heading
    if ! echo "$unreleased_content" | grep -q "^### "; then
        echo -e "${RED}❌ CHANGELOG.md has no changes under ## [Unreleased]${NC}"
        echo -e "   Add at least one ### heading with changes before creating a release"
        return 1
    fi
    return 0
}

# Function to update Cargo.toml version
update_cargo_version() {
    local new_version="$1"
    sed -i "s/^version = \".*\"/version = \"$new_version\"/" Cargo.toml
}

# Function to add new release section to CHANGELOG.md
update_changelog() {
    local new_version="$1"
    local current_date
    current_date=$(date '+%Y-%m-%d')
    
    # Get the previous version for comparison link
    local prev_version
    prev_version=$(get_changelog_version)
    
    # Determine the comparison link
    local comparison_link
    if [[ -n "$prev_version" ]]; then
        comparison_link="https://github.com/anson-vandoren/pagepouch-rs/compare/v${prev_version}...v${new_version}"
    else
        comparison_link="https://github.com/anson-vandoren/pagepouch-rs.git"
    fi
    
    # Create temporary file with the new content
    local temp_file
    temp_file=$(mktemp)
    
    # Add everything up to and including the [Unreleased] line
    awk '/^## \[Unreleased\]/ {print; print ""; exit} {print}' CHANGELOG.md > "$temp_file"
    
    # Add the new release section
    echo "## [${new_version}](${comparison_link}) - ${current_date}" >> "$temp_file"
    
    # Add the unreleased content (skip empty lines at the start)
    awk '/^## \[Unreleased\]/ {found=1; next} found && /^## / {exit} found && NF > 0 {print}' CHANGELOG.md >> "$temp_file"
    
    # Add a blank line before the next section
    echo "" >> "$temp_file"
    
    # Add the rest of the file starting from the next ## section
    awk '/^## \[Unreleased\]/ {found=1; next} found && /^## / {print_rest=1} print_rest {print}' CHANGELOG.md >> "$temp_file"
    
    # Replace the original file
    mv "$temp_file" CHANGELOG.md
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

# Handle --make-release workflow
if [[ "$MAKE_RELEASE" == "true" ]]; then
    echo -e "${BLUE}🚀 Interactive release creation${NC}"
    
    # Check if we're in a clean git state
    if [[ -n $(git status --porcelain) ]]; then
        echo -e "${RED}❌ Working directory is not clean. Commit your changes first.${NC}"
        exit 1
    fi
    
    # Validate CHANGELOG.md has unreleased changes
    if ! check_unreleased_changes; then
        exit 1
    fi
    
    # Get current versions
    current_cargo_version=$(get_cargo_version)
    current_changelog_version=$(get_changelog_version)
    
    echo -e "${YELLOW}📋 Current version information:${NC}"
    echo -e "  Cargo.toml: $current_cargo_version"
    echo -e "  Latest CHANGELOG.md: $current_changelog_version"
    
    # Determine the default next version (increment patch)
    default_version=$(increment_patch "$current_cargo_version")
    
    # Prompt for new version
    echo ""
    echo -e "${YELLOW}🔢 Enter new version number (default: $default_version):${NC}"
    read -r new_version
    
    # Use default if empty
    if [[ -z "$new_version" ]]; then
        new_version="$default_version"
    fi
    
    # Validate version format
    if [[ ! "$new_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo -e "${RED}❌ Version must be in format X.Y.Z${NC}"
        exit 1
    fi
    
    # Check that new version is greater than current versions
    if ! version_greater "$new_version" "$current_cargo_version"; then
        echo -e "${RED}❌ New version $new_version must be greater than current Cargo.toml version $current_cargo_version${NC}"
        exit 1
    fi
    
    if [[ -n "$current_changelog_version" ]] && ! version_greater "$new_version" "$current_changelog_version"; then
        echo -e "${RED}❌ New version $new_version must be greater than current CHANGELOG.md version $current_changelog_version${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✅ Version $new_version is valid${NC}"
    
    # Update files
    echo -e "${YELLOW}📝 Updating Cargo.toml...${NC}"
    update_cargo_version "$new_version"
    
    echo -e "${YELLOW}📝 Updating CHANGELOG.md...${NC}"
    update_changelog "$new_version"
    
    # Show what will be released
    echo ""
    echo -e "${BLUE}📋 Changes for version $new_version:${NC}"
    echo -e "${BLUE}======================================================================================================${NC}"
    get_release_notes "$new_version"
    echo -e "${BLUE}======================================================================================================${NC}"
    echo ""
    
    # Confirm changes
    echo -e "${YELLOW}❓ Proceed with these changes? (y/N)${NC}"
    read -r response
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        echo -e "${RED}❌ Aborted${NC}"
        exit 1
    fi
    
    # Stage files
    echo -e "${YELLOW}📦 Staging files for commit...${NC}"
    git add CHANGELOG.md Cargo.toml Cargo.lock
    
    # Show what will be committed
    echo -e "${YELLOW}📋 Files to be committed:${NC}"
    git status --porcelain
    
    # Confirm commit
    commit_message="🏗️ deploy $new_version"
    echo -e "${YELLOW}❓ Create commit with message '$commit_message'? (y/N)${NC}"
    read -r response
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        echo -e "${RED}❌ Aborted${NC}"
        exit 1
    fi
    
    # Create commit
    git commit -m "$commit_message"
    echo -e "${GREEN}✅ Commit created${NC}"
    
    # Confirm push
    current_branch=$(git branch --show-current)
    echo -e "${YELLOW}❓ Push changes to remote '$current_branch'? (y/N)${NC}"
    read -r response
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        echo -e "${RED}❌ Changes committed locally but not pushed. Run 'git push origin $current_branch' manually.${NC}"
        exit 1
    fi
    
    # Push changes
    git push origin "$current_branch"
    echo -e "${GREEN}✅ Changes pushed to remote${NC}"
    
    echo -e "${GREEN}🎉 Release version $new_version prepared successfully!${NC}"
    echo -e "${YELLOW}⏳ Proceeding with release process...${NC}"
    echo ""
fi

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
git tag -s "$VERSION" -m "Release $VERSION"
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
