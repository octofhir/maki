#!/bin/bash
#
# Real IG Integration Tests
#
# Tests MAKI against production FHIR Implementation Guides:
# - mCODE (Minimal Common Oncology Data Elements)
# - US Core
# - Others as configured
#
# Usage: ./test-real-igs.sh

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MAKI_BIN="${SCRIPT_DIR}/target/release/maki"
TEST_DIR="/tmp/maki-ig-tests"
RESULTS_FILE="${TEST_DIR}/test-results.md"

# Ensure MAKI is built
if [ ! -f "$MAKI_BIN" ]; then
    echo -e "${YELLOW}Building MAKI in release mode...${NC}"
    cargo build --release --bin maki
fi

# Create test directory
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

# Initialize results file
cat > "$RESULTS_FILE" <<EOF
# MAKI Real IG Test Results

**Date**: $(date)
**MAKI Version**: $(${MAKI_BIN} --version || echo "dev")

---

EOF

echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           MAKI Real IG Integration Tests                      ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Test counter
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

#==============================================================================
# Test Function
#==============================================================================
test_ig() {
    local ig_name="$1"
    local ig_dir="$2"
    local description="$3"

    TESTS_RUN=$((TESTS_RUN + 1))

    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}Testing: ${ig_name}${NC}"
    echo -e "${BLUE}Description: ${description}${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""

    if [ ! -d "$ig_dir" ]; then
        echo -e "${RED}✗ IG directory not found: ${ig_dir}${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        cat >> "$RESULTS_FILE" <<EOF
## ${ig_name}

**Status**: ❌ FAILED
**Reason**: IG directory not found: \`${ig_dir}\`

EOF
        return 1
    fi

    cd "$ig_dir"

    # Check for sushi-config.yaml
    if [ ! -f "sushi-config.yaml" ]; then
        echo -e "${RED}✗ No sushi-config.yaml found${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        cat >> "$RESULTS_FILE" <<EOF
## ${ig_name}

**Status**: ❌ FAILED
**Reason**: No sushi-config.yaml found

EOF
        return 1
    fi

    # Count FSH files
    local fsh_count=$(find input/fsh -name "*.fsh" 2>/dev/null | wc -l)
    echo -e "${YELLOW}  📄 FSH files: ${fsh_count}${NC}"

    # Run MAKI build
    echo -e "${YELLOW}  🔨 Running MAKI build...${NC}"
    local start_time=$(date +%s)

    if ${MAKI_BIN} build > build.log 2>&1; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))

        # Count generated resources
        local resource_count=$(find fsh-generated -name "*.json" 2>/dev/null | wc -l)

        echo -e "${GREEN}  ✓ Build successful (${duration}s)${NC}"
        echo -e "${GREEN}  ✓ Generated ${resource_count} FHIR resources${NC}"

        # Validate JSON files
        local invalid_count=0
        for json_file in fsh-generated/fsh-generated/resources/*.json; do
            if [ -f "$json_file" ]; then
                if ! python3 -m json.tool "$json_file" > /dev/null 2>&1; then
                    invalid_count=$((invalid_count + 1))
                fi
            fi
        done

        if [ $invalid_count -gt 0 ]; then
            echo -e "${YELLOW}  ⚠ ${invalid_count} invalid JSON files${NC}"
        else
            echo -e "${GREEN}  ✓ All JSON files valid${NC}"
        fi

        TESTS_PASSED=$((TESTS_PASSED + 1))

        # Write results
        cat >> "$RESULTS_FILE" <<EOF
## ${ig_name}

**Status**: ✅ PASSED
**Description**: ${description}
**FSH Files**: ${fsh_count}
**Generated Resources**: ${resource_count}
**Build Time**: ${duration}s
**JSON Validation**: $([ $invalid_count -eq 0 ] && echo "✓ All valid" || echo "⚠ ${invalid_count} invalid")

### Build Log
\`\`\`
$(tail -20 build.log)
\`\`\`

EOF

    else
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))

        echo -e "${RED}  ✗ Build failed (${duration}s)${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))

        # Write results
        cat >> "$RESULTS_FILE" <<EOF
## ${ig_name}

**Status**: ❌ FAILED
**Description**: ${description}
**FSH Files**: ${fsh_count}
**Build Time**: ${duration}s

### Error Log
\`\`\`
$(tail -50 build.log)
\`\`\`

EOF
    fi

    echo ""
    cd "$TEST_DIR"
}

#==============================================================================
# Test Cases
#==============================================================================

# Test 1: mCODE IG (Local copy)
test_ig \
    "mCODE" \
    "/tmp/mcode-test-build" \
    "Minimal Common Oncology Data Elements (HL7 FHIR US)"

# Test 2: Simple test IG (parallel export test)
test_ig \
    "Test Parallel Export" \
    "/tmp/test-parallel-export" \
    "Simple test IG for parallel export validation"

#==============================================================================
# Summary
#==============================================================================

echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                      Test Summary                              ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "  Total Tests: ${TESTS_RUN}"
echo -e "  ${GREEN}Passed: ${TESTS_PASSED}${NC}"
echo -e "  ${RED}Failed: ${TESTS_FAILED}${NC}"
echo ""

cat >> "$RESULTS_FILE" <<EOF

---

## Summary

**Total Tests**: ${TESTS_RUN}
**Passed**: ${TESTS_PASSED} ✅
**Failed**: ${TESTS_FAILED} ❌
**Success Rate**: $(( TESTS_PASSED * 100 / TESTS_RUN ))%

EOF

echo -e "${YELLOW}Results written to: ${RESULTS_FILE}${NC}"
echo ""

# Exit with appropriate code
if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed! 🎉${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed.${NC}"
    exit 1
fi
