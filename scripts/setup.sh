#!/bin/bash
# Setup script for I Am Surging
# Initializes the project with required configuration and feed data
#
# Usage:
#   ./scripts/setup.sh              # Interactive setup
#   ./scripts/setup.sh --quick      # Non-interactive, use defaults
#   ./scripts/setup.sh --help       # Show help

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Project root (parent of scripts directory)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

print_header() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}  I Am Surging - Project Setup${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo ""
}

print_help() {
    echo "Usage: ./scripts/setup.sh [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --quick       Non-interactive mode, use all defaults"
    echo "  --api-key     Set your Surge API key (for server auth)"
    echo "  --feeds-api   Custom Switchboard feeds API URL"
    echo "  --help        Show this help message"
    echo ""
    echo "Examples:"
    echo "  ./scripts/setup.sh"
    echo "  ./scripts/setup.sh --quick"
    echo "  ./scripts/setup.sh --api-key your-key-here"
    echo "  ./scripts/setup.sh --feeds-api https://custom.api.url/feeds"
    echo ""
}

check_dependencies() {
    echo -e "${YELLOW}Checking dependencies...${NC}"

    local missing=0

    # Check Node.js
    if command -v node &> /dev/null; then
        local node_version=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
        if [ "$node_version" -ge 18 ]; then
            echo -e "  ${GREEN}✓${NC} Node.js $(node -v)"
        else
            echo -e "  ${RED}✗${NC} Node.js $(node -v) - requires v18+"
            missing=1
        fi
    else
        echo -e "  ${RED}✗${NC} Node.js not found"
        missing=1
    fi

    # Check Rust/Cargo (optional but recommended)
    if command -v cargo &> /dev/null; then
        echo -e "  ${GREEN}✓${NC} Cargo $(cargo --version | cut -d' ' -f2)"
    else
        echo -e "  ${YELLOW}!${NC} Cargo not found (needed to build the project)"
    fi

    # Check curl (for API calls)
    if command -v curl &> /dev/null; then
        echo -e "  ${GREEN}✓${NC} curl available"
    else
        echo -e "  ${RED}✗${NC} curl not found"
        missing=1
    fi

    echo ""

    if [ $missing -eq 1 ]; then
        echo -e "${RED}Missing required dependencies. Please install them and try again.${NC}"
        exit 1
    fi
}

setup_env() {
    echo -e "${YELLOW}Setting up environment...${NC}"

    if [ -f .env ]; then
        echo -e "  ${GREEN}✓${NC} .env already exists"

        if [ "$INTERACTIVE" = true ]; then
            read -p "  Overwrite existing .env? [y/N] " -n 1 -r
            echo
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                return
            fi
        else
            return
        fi
    fi

    if [ ! -f .env.example ]; then
        echo -e "  ${RED}✗${NC} .env.example not found"
        return
    fi

    cp .env.example .env

    # Set API key if provided
    if [ -n "$API_KEY" ]; then
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' "s/SURGE_API_KEY=.*/SURGE_API_KEY=$API_KEY/" .env
        else
            sed -i "s/SURGE_API_KEY=.*/SURGE_API_KEY=$API_KEY/" .env
        fi
        echo -e "  ${GREEN}✓${NC} API key configured"
    elif [ "$INTERACTIVE" = true ]; then
        echo ""
        echo "  The API key is used for server authentication (optional)."
        echo "  Leave blank to disable authentication."
        read -p "  Enter your Surge API key (or press Enter to skip): " input_key

        if [ -n "$input_key" ]; then
            if [[ "$OSTYPE" == "darwin"* ]]; then
                sed -i '' "s/SURGE_API_KEY=.*/SURGE_API_KEY=$input_key/" .env
            else
                sed -i "s/SURGE_API_KEY=.*/SURGE_API_KEY=$input_key/" .env
            fi
            echo -e "  ${GREEN}✓${NC} API key configured"
        fi
    fi

    echo -e "  ${GREEN}✓${NC} Created .env from template"
}

generate_feeds() {
    echo -e "${YELLOW}Generating feed IDs...${NC}"

    if [ -f feedIds.json ]; then
        local count=$(grep -c ":" feedIds.json 2>/dev/null || echo "0")
        echo -e "  ${GREEN}✓${NC} feedIds.json exists ($count feeds)"

        if [ "$INTERACTIVE" = true ]; then
            read -p "  Regenerate feed IDs? [y/N] " -n 1 -r
            echo
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                return
            fi
        else
            return
        fi
    fi

    # Use custom API URL if provided
    local api_url="${FEEDS_API_URL:-https://explorer.switchboardlabs.xyz/api/feeds}"

    echo -e "  Fetching feeds from Switchboard..."
    echo -e "  API: $api_url"
    echo ""

    # Run the discover-feeds script with --all flag
    if [ -f scripts/discover-feeds.js ]; then
        FEEDS_API_URL="$api_url" node scripts/discover-feeds.js --all
    elif [ -f discover-feeds.js ]; then
        FEEDS_API_URL="$api_url" node discover-feeds.js --all
    else
        echo -e "  ${RED}✗${NC} discover-feeds.js not found"
        echo -e "  Creating minimal feedIds.json with common feeds..."

        cat > feedIds.json << 'EOF'
{
  "BTC/USD": "4cd1cad962425681af07b9254b7d804de3ca3446fbfd1371bb258d2c75059812",
  "ETH/USD": "a0950ee5ee117b2e2c30f154a69e17bfb489a7610c508dc5f67eb2a14616d8ea",
  "SOL/USD": "822512ee9add93518eca1c105a38422841a76c590db079eebb283deb2c14caa9"
}
EOF
        echo -e "  ${YELLOW}!${NC} Created minimal feedIds.json (3 feeds)"
        echo -e "  ${YELLOW}!${NC} Run 'node scripts/discover-feeds.js --all' for full feed list"
        return
    fi

    if [ -f feedIds.json ]; then
        local count=$(grep -c ":" feedIds.json 2>/dev/null || echo "0")
        echo -e "  ${GREEN}✓${NC} Generated feedIds.json ($count feeds)"
    else
        echo -e "  ${RED}✗${NC} Failed to generate feedIds.json"
    fi
}

build_project() {
    if ! command -v cargo &> /dev/null; then
        echo -e "${YELLOW}Skipping build (Cargo not installed)${NC}"
        return
    fi

    if [ "$INTERACTIVE" = true ]; then
        read -p "Build the project? [Y/n] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Nn]$ ]]; then
            return
        fi
    fi

    echo -e "${YELLOW}Building project...${NC}"
    cargo build --release
    echo -e "  ${GREEN}✓${NC} Build complete"
}

print_summary() {
    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  Setup Complete!${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "Next steps:"
    echo ""
    echo "  1. Get a price:"
    echo "     cargo run --release -- btc"
    echo ""
    echo "  2. Stream live prices:"
    echo "     cargo run --release -- stream btc eth sol"
    echo ""
    echo "  3. Start the API server:"
    echo "     cargo run --release --bin surge-server"
    echo ""
    echo "  4. Run tests:"
    echo "     ./scripts/run-tests.sh"
    echo ""

    if [ -f .env ]; then
        if grep -q "SURGE_API_KEY=your-api-key-here" .env 2>/dev/null; then
            echo -e "${YELLOW}Note: API key not configured. Server will run without authentication.${NC}"
            echo -e "${YELLOW}Edit .env to add your API key for production use.${NC}"
            echo ""
        fi
    fi
}

# Parse arguments
INTERACTIVE=true
API_KEY=""
FEEDS_API_URL=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --quick)
            INTERACTIVE=false
            shift
            ;;
        --api-key)
            API_KEY="$2"
            shift 2
            ;;
        --feeds-api)
            FEEDS_API_URL="$2"
            shift 2
            ;;
        --help|-h)
            print_help
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            print_help
            exit 1
            ;;
    esac
done

# Main
print_header
check_dependencies
setup_env
generate_feeds
build_project
print_summary
