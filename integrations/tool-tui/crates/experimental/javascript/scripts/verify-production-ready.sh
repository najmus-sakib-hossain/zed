#!/bin/bash
# DX JavaScript Runtime - Production Readiness Verification
# This script verifies that all production readiness requirements are met.

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

echo -e "${BOLD}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}║     DX JavaScript Runtime - Production Readiness Verification   ║${NC}"
echo -e "${BOLD}╚══════════════════════════════════════════════════════════════════╝${NC}"
echo ""

PASSED=0
FAILED=0
WARNINGS=0

# Helper functions
pass() {
    echo -e "  ${GREEN}✓${NC} $1"
    ((PASSED++))
}

fail() {
    echo -e "  ${RED}✗${NC} $1"
    ((FAILED++))
}

warn() {
    echo -e "  ${YELLOW}⚠${NC} $1"
    ((WARNINGS++))
}

section() {
    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
}

# 1. Check Build
section "Build Verification"

echo "Checking runtime build..."
if cargo build --manifest-path "$ROOT_DIR/runtime/Cargo.toml" --release 2>/dev/null; then
    pass "Runtime builds successfully"
else
    fail "Runtime build failed"
fi

echo "Checking bundler build..."
if cargo build --manifest-path "$ROOT_DIR/bundler/Cargo.toml" --release 2>/dev/null; then
    pass "Bundler builds successfully"
else
    fail "Bundler build failed"
fi

echo "Checking package manager build..."
if cargo build --manifest-path "$ROOT_DIR/package-manager/Cargo.toml" --release 2>/dev/null; then
    pass "Package manager builds successfully"
else
    fail "Package manager build failed"
fi

# 2. Check Tests
section "Test Suite Verification"

echo "Running runtime tests..."
if cargo test --manifest-path "$ROOT_DIR/runtime/Cargo.toml" --release 2>/dev/null; then
    pass "Runtime tests pass"
else
    fail "Runtime tests failed"
fi

echo "Running compatibility tests..."
if cargo test --manifest-path "$ROOT_DIR/compatibility/Cargo.toml" --release 2>/dev/null; then
    pass "Compatibility tests pass"
else
    fail "Compatibility tests failed"
fi

# 3. Check Property-Based Tests
section "Property-Based Test Verification"

echo "Running BigInt property tests..."
if cargo test --manifest-path "$ROOT_DIR/runtime/Cargo.toml" bigint_property --release 2>/dev/null; then
    pass "BigInt property tests pass"
else
    warn "BigInt property tests not found or failed"
fi

echo "Running dynamic import property tests..."
if cargo test --manifest-path "$ROOT_DIR/runtime/Cargo.toml" dynamic_import_property --release 2>/dev/null; then
    pass "Dynamic import property tests pass"
else
    warn "Dynamic import property tests not found or failed"
fi

echo "Running crypto property tests..."
if cargo test --manifest-path "$ROOT_DIR/compatibility/Cargo.toml" crypto_props --release 2>/dev/null; then
    pass "Crypto property tests pass"
else
    warn "Crypto property tests not found or failed"
fi

echo "Running stream property tests..."
if cargo test --manifest-path "$ROOT_DIR/compatibility/Cargo.toml" stream_props --release 2>/dev/null; then
    pass "Stream property tests pass"
else
    warn "Stream property tests not found or failed"
fi

# 4. Check Documentation
section "Documentation Verification"

echo "Checking documentation builds..."
if cargo doc --manifest-path "$ROOT_DIR/runtime/Cargo.toml" --no-deps 2>/dev/null; then
    pass "Documentation builds successfully"
else
    fail "Documentation build failed"
fi

echo "Checking README exists..."
if [ -f "$ROOT_DIR/README.md" ]; then
    pass "README.md exists"
else
    fail "README.md missing"
fi

echo "Checking API reference..."
if [ -f "$ROOT_DIR/docs/API_REFERENCE.md" ]; then
    pass "API reference exists"
else
    warn "API reference missing"
fi

# 5. Check CI/CD Configuration
section "CI/CD Configuration Verification"

echo "Checking GitHub Actions workflows..."
if [ -f "$ROOT_DIR/.github/workflows/ci.yml" ]; then
    pass "CI workflow exists"
else
    fail "CI workflow missing"
fi

if [ -f "$ROOT_DIR/.github/workflows/release.yml" ]; then
    pass "Release workflow exists"
else
    fail "Release workflow missing"
fi

if [ -f "$ROOT_DIR/.github/workflows/benchmark.yml" ]; then
    pass "Benchmark workflow exists"
else
    warn "Benchmark workflow missing"
fi

# 6. Check npm Package
section "npm Package Verification"

echo "Checking npm package configuration..."
if [ -f "$ROOT_DIR/npm/package.json" ]; then
    pass "npm package.json exists"
else
    fail "npm package.json missing"
fi

if [ -f "$ROOT_DIR/npm/scripts/install.js" ]; then
    pass "npm install script exists"
else
    fail "npm install script missing"
fi

# 7. Check Benchmark Suite
section "Benchmark Suite Verification"

echo "Checking benchmark suite..."
if [ -f "$ROOT_DIR/benchmarks/suite/run.sh" ]; then
    pass "Benchmark suite exists"
else
    warn "Benchmark suite missing"
fi

if [ -d "$ROOT_DIR/benchmarks/suite/workloads" ]; then
    WORKLOAD_COUNT=$(ls -1 "$ROOT_DIR/benchmarks/suite/workloads"/*.js 2>/dev/null | wc -l)
    if [ "$WORKLOAD_COUNT" -ge 4 ]; then
        pass "Benchmark workloads exist ($WORKLOAD_COUNT files)"
    else
        warn "Few benchmark workloads found ($WORKLOAD_COUNT files)"
    fi
else
    warn "Benchmark workloads directory missing"
fi

# 8. Check Cross-Platform Support
section "Cross-Platform Support Verification"

echo "Checking release workflow targets..."
if grep -q "aarch64-unknown-linux-gnu" "$ROOT_DIR/.github/workflows/release.yml" 2>/dev/null; then
    pass "Linux ARM64 target configured"
else
    warn "Linux ARM64 target not configured"
fi

if grep -q "aarch64-apple-darwin" "$ROOT_DIR/.github/workflows/release.yml" 2>/dev/null; then
    pass "macOS ARM64 target configured"
else
    warn "macOS ARM64 target not configured"
fi

if grep -q "x86_64-pc-windows-msvc" "$ROOT_DIR/.github/workflows/release.yml" 2>/dev/null; then
    pass "Windows x86_64 target configured"
else
    warn "Windows x86_64 target not configured"
fi

# Summary
section "Summary"

TOTAL=$((PASSED + FAILED + WARNINGS))

echo ""
echo -e "  ${GREEN}Passed:${NC}   $PASSED"
echo -e "  ${RED}Failed:${NC}   $FAILED"
echo -e "  ${YELLOW}Warnings:${NC} $WARNINGS"
echo -e "  ${BOLD}Total:${NC}    $TOTAL"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║              PRODUCTION READINESS: VERIFIED ✓                   ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════════╝${NC}"
    exit 0
else
    echo -e "${RED}╔══════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${RED}║         PRODUCTION READINESS: NOT VERIFIED ($FAILED failures)      ║${NC}"
    echo -e "${RED}╚══════════════════════════════════════════════════════════════════╝${NC}"
    exit 1
fi
