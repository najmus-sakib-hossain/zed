# DX JavaScript Runtime - Production Readiness Verification (PowerShell)
# This script verifies that all production readiness requirements are met.

$ErrorActionPreference = "Continue"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent $ScriptDir

Write-Host ""
Write-Host "╔══════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║     DX JavaScript Runtime - Production Readiness Verification   ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

$Passed = 0
$Failed = 0
$Warnings = 0

function Pass($message) {
    Write-Host "  ✓ $message" -ForegroundColor Green
    $script:Passed++
}

function Fail($message) {
    Write-Host "  ✗ $message" -ForegroundColor Red
    $script:Failed++
}

function Warn($message) {
    Write-Host "  ⚠ $message" -ForegroundColor Yellow
    $script:Warnings++
}

function Section($title) {
    Write-Host ""
    Write-Host "═══════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
    Write-Host "  $title" -ForegroundColor Cyan
    Write-Host "═══════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
}

# 1. Check Build
Section "Build Verification"

Write-Host "Checking runtime build..."
$buildResult = cargo build --manifest-path "$RootDir/runtime/Cargo.toml" --release 2>&1
if ($LASTEXITCODE -eq 0) {
    Pass "Runtime builds successfully"
} else {
    Fail "Runtime build failed"
}

Write-Host "Checking bundler build..."
$buildResult = cargo build --manifest-path "$RootDir/bundler/Cargo.toml" --release 2>&1
if ($LASTEXITCODE -eq 0) {
    Pass "Bundler builds successfully"
} else {
    Fail "Bundler build failed"
}

Write-Host "Checking package manager build..."
$buildResult = cargo build --manifest-path "$RootDir/package-manager/Cargo.toml" --release 2>&1
if ($LASTEXITCODE -eq 0) {
    Pass "Package manager builds successfully"
} else {
    Fail "Package manager build failed"
}

# 2. Check Tests
Section "Test Suite Verification"

Write-Host "Running runtime tests..."
$testResult = cargo test --manifest-path "$RootDir/runtime/Cargo.toml" --release 2>&1
if ($LASTEXITCODE -eq 0) {
    Pass "Runtime tests pass"
} else {
    Fail "Runtime tests failed"
}

Write-Host "Running compatibility tests..."
$testResult = cargo test --manifest-path "$RootDir/compatibility/Cargo.toml" --release 2>&1
if ($LASTEXITCODE -eq 0) {
    Pass "Compatibility tests pass"
} else {
    Fail "Compatibility tests failed"
}

# 3. Check Documentation
Section "Documentation Verification"

Write-Host "Checking README exists..."
if (Test-Path "$RootDir/README.md") {
    Pass "README.md exists"
} else {
    Fail "README.md missing"
}

Write-Host "Checking API reference..."
if (Test-Path "$RootDir/docs/API_REFERENCE.md") {
    Pass "API reference exists"
} else {
    Warn "API reference missing"
}

# 4. Check CI/CD Configuration
Section "CI/CD Configuration Verification"

Write-Host "Checking GitHub Actions workflows..."
if (Test-Path "$RootDir/.github/workflows/ci.yml") {
    Pass "CI workflow exists"
} else {
    Fail "CI workflow missing"
}

if (Test-Path "$RootDir/.github/workflows/release.yml") {
    Pass "Release workflow exists"
} else {
    Fail "Release workflow missing"
}

if (Test-Path "$RootDir/.github/workflows/benchmark.yml") {
    Pass "Benchmark workflow exists"
} else {
    Warn "Benchmark workflow missing"
}

# 5. Check npm Package
Section "npm Package Verification"

Write-Host "Checking npm package configuration..."
if (Test-Path "$RootDir/npm/package.json") {
    Pass "npm package.json exists"
} else {
    Fail "npm package.json missing"
}

if (Test-Path "$RootDir/npm/scripts/install.js") {
    Pass "npm install script exists"
} else {
    Fail "npm install script missing"
}

# 6. Check Benchmark Suite
Section "Benchmark Suite Verification"

Write-Host "Checking benchmark suite..."
if (Test-Path "$RootDir/benchmarks/suite/run.ps1") {
    Pass "Benchmark suite exists"
} else {
    Warn "Benchmark suite missing"
}

if (Test-Path "$RootDir/benchmarks/suite/workloads") {
    $workloadCount = (Get-ChildItem "$RootDir/benchmarks/suite/workloads/*.js" -ErrorAction SilentlyContinue).Count
    if ($workloadCount -ge 4) {
        Pass "Benchmark workloads exist ($workloadCount files)"
    } else {
        Warn "Few benchmark workloads found ($workloadCount files)"
    }
} else {
    Warn "Benchmark workloads directory missing"
}

# Summary
Section "Summary"

$Total = $Passed + $Failed + $Warnings

Write-Host ""
Write-Host "  Passed:   $Passed" -ForegroundColor Green
Write-Host "  Failed:   $Failed" -ForegroundColor Red
Write-Host "  Warnings: $Warnings" -ForegroundColor Yellow
Write-Host "  Total:    $Total"
Write-Host ""

if ($Failed -eq 0) {
    Write-Host "╔══════════════════════════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "║              PRODUCTION READINESS: VERIFIED ✓                   ║" -ForegroundColor Green
    Write-Host "╚══════════════════════════════════════════════════════════════════╝" -ForegroundColor Green
    exit 0
} else {
    Write-Host "╔══════════════════════════════════════════════════════════════════╗" -ForegroundColor Red
    Write-Host "║         PRODUCTION READINESS: NOT VERIFIED ($Failed failures)      ║" -ForegroundColor Red
    Write-Host "╚══════════════════════════════════════════════════════════════════╝" -ForegroundColor Red
    exit 1
}
