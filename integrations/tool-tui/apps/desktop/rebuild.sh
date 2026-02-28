#!/bin/bash
# Close any running app instances first, then run this script

echo "Cleaning build artifacts..."
cargo clean

echo "Building desktop app..."
cargo build --bin app

echo "Build complete! Run the app with: cargo run --bin app"
