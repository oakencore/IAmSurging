#!/bin/bash
# Test runner for I Am Surging
# Run from project root: ./scripts/run-tests.sh

set -e

# Project root (parent of scripts directory)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "=========================================="
echo "I Am Surging - Test Suite"
echo "=========================================="
echo ""

# Colours for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No colour

PASSED=0
FAILED=0
SKIPPED=0

# Helper function
run_test() {
    local name="$1"
    local cmd="$2"
    local skip_reason="$3"

    if [ -n "$skip_reason" ]; then
        echo -e "${YELLOW}SKIP${NC} - $name ($skip_reason)"
        SKIPPED=$((SKIPPED + 1))
        return
    fi

    if eval "$cmd" > /dev/null 2>&1; then
        echo -e "${GREEN}PASS${NC} - $name"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}FAIL${NC} - $name"
        FAILED=$((FAILED + 1))
    fi
}

echo "1. Build Tests"
echo "------------------------------------------"

run_test "Cargo build (debug)" "cargo build"
run_test "Cargo build (release)" "cargo build --release"

echo ""
echo "2. Unit Tests"
echo "------------------------------------------"

run_test "Cargo test" "cargo test"

echo ""
echo "3. Helper Script Tests"
echo "------------------------------------------"

run_test "discover-feeds.js exists" "test -f scripts/discover-feeds.js"
run_test "setup.sh exists" "test -f scripts/setup.sh"
run_test "Node.js available" "node --version"
run_test "Node.js version >= 18" "node -e \"process.exit(parseInt(process.version.slice(1)) >= 18 ? 0 : 1)\""

echo ""
echo "4. Feed Discovery Tests"
echo "------------------------------------------"

run_test "discover-feeds.js --search BTC" "node scripts/discover-feeds.js --search BTC"
run_test "discover-feeds.js finds feeds" "node scripts/discover-feeds.js --search USD | grep -q 'Total:'"

echo ""
echo "5. Feed Loader Tests"
echo "------------------------------------------"

# Check if feedIds.json exists, create a test one if not
if [ ! -f feedIds.json ]; then
    echo -e "${YELLOW}INFO${NC} - Creating test feedIds.json for testing..."
    cat > feedIds.json << 'EOF'
{
  "BTC/USD": "4cd1cad962425681af07b9254b7d804de3ca3446fbfd1371bb258d2c75059812",
  "ETH/USD": "a0950ee5ee117b2e2c30f154a69e17bfb489a7610c508dc5f67eb2a14616d8ea",
  "SOL/USD": "822512ee9add93518eca1c105a38422841a76c590db079eebb283deb2c14caa9"
}
EOF
    CREATED_TEST_FEEDS=true
fi

run_test "feedIds.json exists" "test -f feedIds.json"
run_test "feedIds.json is valid JSON" "python3 -c \"import json; json.load(open('feedIds.json'))\""
run_test "feedIds.json has entries" "python3 -c \"import json; d=json.load(open('feedIds.json')); assert len(d) > 0\""
run_test "BTC/USD feed exists" "python3 -c \"import json; d=json.load(open('feedIds.json')); assert 'BTC/USD' in d\""
run_test "ETH/USD feed exists" "python3 -c \"import json; d=json.load(open('feedIds.json')); assert 'ETH/USD' in d\""
run_test "SOL/USD feed exists" "python3 -c \"import json; d=json.load(open('feedIds.json')); assert 'SOL/USD' in d\""

echo ""
echo "6. CLI Tests"
echo "------------------------------------------"

CLI="./target/release/surge"

run_test "CLI binary exists" "test -f $CLI"
run_test "CLI --help" "$CLI --help"
run_test "CLI list command" "$CLI list --limit 5"
run_test "CLI list with filter" "$CLI list --filter BTC --limit 5"

echo ""
echo "7. API Tests (require network)"
echo "------------------------------------------"

if [ -z "$SKIP_API_TESTS" ]; then
    # Basic API tests - these hit the live Switchboard API
    run_test "Get BTC price" "$CLI btc"
    run_test "Get multiple prices" "$CLI btc eth sol"
    run_test "JSON output format" "$CLI --json btc"
else
    run_test "Get BTC price" "" "SKIP_API_TESTS set"
    run_test "Get multiple prices" "" "SKIP_API_TESTS set"
    run_test "JSON output format" "" "SKIP_API_TESTS set"
fi

# Clean up test feedIds.json if we created it
if [ "$CREATED_TEST_FEEDS" = true ]; then
    rm -f feedIds.json
    echo -e "${YELLOW}INFO${NC} - Removed test feedIds.json"
fi

echo ""
echo "8. Security/Privacy Checks"
echo "------------------------------------------"

# Files to check (exclude build artifacts, node_modules, and scripts)
CHECK_FILES="src/*.rs Cargo.toml README.md API.md"

# Check for API keys or secrets
run_test "No hardcoded API keys" "! grep -r -i -E '(api_key|apikey|secret|token)\s*=\s*[\"'\''][A-Za-z0-9]{20,}' $CHECK_FILES 2>/dev/null"

# Check for personal directory paths
run_test "No personal directory paths" "! grep -r -E '/Users/[a-zA-Z]+|/home/[a-zA-Z]+' $CHECK_FILES 2>/dev/null"

echo ""
echo "9. Release Readiness Checks"
echo "------------------------------------------"

# Licence file exists
run_test "LICENSE file exists" "test -f LICENSE || test -f LICENSE.md || test -f LICENSE.txt"

# No debug statements (dbg! macro in Rust)
run_test "No dbg! in Rust" "! grep -r 'dbg!' src/*.rs 2>/dev/null"

# README has essential sections
run_test "README exists" "test -f README.md"
run_test "README has Quick Start" "grep -q -i 'quick start' README.md"
run_test "README has Usage" "grep -q -i 'usage' README.md"

# Cargo.lock exists for reproducible builds
run_test "Cargo.lock exists" "test -f Cargo.lock"

# Scripts are executable
run_test "setup.sh is executable" "test -x scripts/setup.sh"
run_test "run-tests.sh is executable" "test -x scripts/run-tests.sh"
run_test "discover-feeds.js is executable" "test -x scripts/discover-feeds.js"

echo ""
echo "=========================================="
echo "Test Results"
echo "=========================================="
echo -e "${GREEN}Passed:${NC}  $PASSED"
echo -e "${RED}Failed:${NC}  $FAILED"
echo -e "${YELLOW}Skipped:${NC} $SKIPPED"
echo ""

if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
fi
