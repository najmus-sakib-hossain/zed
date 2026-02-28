#!/bin/bash
# Demo script for the DX chat command with Google AI Studio

echo "=== DX Chat Command Demo ==="
echo ""

# List all available models
echo "1. Listing all available models (including Gemma 3):"
cargo run --manifest-path crates/cli/Cargo.toml -- chat --list-models
echo ""

# Example 1: Simple chat with default model (gemini-2.5-flash)
echo "2. Simple chat with default model:"
echo "   Command: dx chat \"What is Rust?\""
# Uncomment to run (requires API key):
# cargo run --manifest-path crates/cli/Cargo.toml -- chat "What is Rust?"
echo ""

# Example 2: Chat with Gemma 3 model
echo "3. Chat with Gemma 3 27B instruction-tuned model:"
echo "   Command: dx chat -m gemma-3-27b-it \"Explain async/await in Rust\""
# Uncomment to run (requires API key):
# cargo run --manifest-path crates/cli/Cargo.toml -- chat -m gemma-3-27b-it "Explain async/await in Rust"
echo ""

# Example 3: Chat with Gemini 3 Pro
echo "4. Chat with Gemini 3 Pro (most intelligent):"
echo "   Command: dx chat -m gemini-3-pro-preview \"Write a complex algorithm\""
# Uncomment to run (requires API key):
# cargo run --manifest-path crates/cli/Cargo.toml -- chat -m gemini-3-pro-preview "Write a complex algorithm"
echo ""

# Example 4: Using environment variable for API key
echo "5. Using environment variable for API key:"
echo "   export GOOGLE_AI_STUDIO_KEY=your_api_key_here"
echo "   dx chat \"Hello, AI!\""
echo ""

# Example 6: Interactive mode (read from stdin)
echo "6. Interactive mode (no prompt argument):"
echo "   dx chat"
echo "   (Then type your message and press Ctrl+D)"
echo ""

echo "=== Setup Instructions ==="
echo "1. Get your API key from: https://aistudio.google.com/apikey"
echo "2. Set it as environment variable: export GOOGLE_AI_STUDIO_KEY=your_key"
echo "3. Or pass it with --api-key flag: dx chat --api-key YOUR_KEY \"message\""
echo ""
echo "=== Available Models ==="
echo "• Gemini 3: gemini-3-pro-preview, gemini-3-flash-preview"
echo "• Gemini 2.5: gemini-2.5-flash (default), gemini-2.5-pro"
echo "• Gemma 3: gemma-3-1b-it, gemma-3-4b-it, gemma-3-12b-it, gemma-3-27b-it"
echo "• TranslateGemma: translategemma-4b, translategemma-12b, translategemma-27b"
