#!/bin/bash

echo "Stopping any running Forge processes..."
pkill -f forge-cli 2>/dev/null
if [ $? -eq 0 ]; then
    echo "Forge process stopped."
    sleep 2
else
    echo "No running Forge process found."
fi

echo ""
echo "Building Forge binary..."
cargo build --release

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Build successful!"
    if [ -f "target/release/forge-cli.exe" ]; then
        echo "Binary location: target/release/forge-cli.exe"
    else
        echo "Binary location: target/release/forge-cli"
    fi
else
    echo ""
    echo "❌ Build failed!"
    exit 1
fi
