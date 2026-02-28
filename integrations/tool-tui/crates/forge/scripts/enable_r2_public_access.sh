#!/bin/bash
# Quick fix for R2 400 errors - Enable public access

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  Forge R2 - Fix 400 Errors by Enabling Public Access        ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Check if .env exists
if [ ! -f ".env" ]; then
    echo "❌ .env file not found!"
    exit 1
fi

# Load environment variables
export $(grep -v '^#' .env | xargs)

echo "📋 Configuration:"
echo "   Account ID: ${R2_ACCOUNT_ID:0:8}..."
echo "   Bucket: $R2_BUCKET_NAME"
echo ""

# Enable public access via Cloudflare API
echo "🔓 Enabling public access on R2 bucket..."

# Note: The API token in .env.example is the secret access key, not the API bearer token
# We'll use the dashboard instructions instead

echo ""
echo "⚠️  Automatic public access requires a Cloudflare API token (not R2 access key)."
echo ""
echo "📖 Please enable public access manually:"
echo ""
echo "   1. Visit: https://dash.cloudflare.com/?to=/:account/r2/overview/buckets/$R2_BUCKET_NAME"
echo "   2. Click on your bucket name: '$R2_BUCKET_NAME'"
echo "   3. Go to 'Settings' tab"
echo "   4. Scroll to 'Public access' section"
echo "   5. Click 'Allow Access' button"
echo "   6. Copy the public bucket URL (looks like: https://pub-xxxxx.r2.dev)"
echo "   7. Add to .env: R2_PUBLIC_URL=https://pub-xxxxx.r2.dev"
echo ""
echo "🌐 Direct link to bucket settings:"
echo "   https://dash.cloudflare.com/?to=/:account/r2/overview/buckets/$R2_BUCKET_NAME"
echo ""
echo "After enabling public access, run the demo again:"
echo "   cargo run --example r2_demo"
echo ""
