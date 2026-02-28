#!/bin/bash
# Forge R2 Demo Runner
# This script runs the complete Forge demo with R2 integration

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  Forge R2 Demo - Setup and Execution                        ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Check if .env exists
if [ ! -f ".env" ]; then
    echo "⚠️  .env file not found!"
    echo "📋 Copying .env.example to .env..."
    cp .env.example .env
    echo "✓ Created .env file"
    echo ""
    echo "⚠️  Please edit .env and add your R2 credentials before running again!"
    exit 1
fi

# Verify R2 credentials are set
if grep -q "your_access_key_id_here" .env || grep -q "your_secret_access_key_here" .env; then
    echo "⚠️  R2 credentials not configured in .env!"
    echo "Please edit .env and add your R2 Access Key ID and Secret Access Key"
    exit 1
fi

echo "✓ .env file found and configured"
echo ""

# Load environment variables
export $(grep -v '^#' .env | xargs)

echo "📋 Configuration:"
echo "   Account ID: ${R2_ACCOUNT_ID:0:8}..."
echo "   Bucket: $R2_BUCKET_NAME"
echo "   Access Key: ${R2_ACCESS_KEY_ID:0:8}..."
echo ""

# Check if forge-demo directory exists
if [ ! -d "examples/forge-demo" ]; then
    echo "✗ examples/forge-demo directory not found!"
    echo "Please ensure the demo repository is created first."
    exit 1
fi

echo "✓ Forge demo directory found"
echo ""

# Build the project
echo "🔨 Building Forge..."
cargo build --example r2_demo 2>&1 | grep -E "(Compiling|Finished)" || true
echo "✓ Build complete"
echo ""

# Run the demo
echo "🚀 Running Forge R2 Demo..."
echo "════════════════════════════════════════════════════════════════"
echo ""

cargo run --example r2_demo

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "✓ Demo execution complete!"
echo ""
echo "🌐 View your R2 bucket:"
echo "   https://dash.cloudflare.com/?to=/:account/r2/overview/buckets/$R2_BUCKET_NAME"
echo ""
