#!/bin/bash
# Quick test of Groq API using curl

echo "🧪 Testing Groq API with curl..."
echo ""

curl -s https://api.groq.com/openai/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer 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" \
  -H "User-Agent: Mozilla/5.0" \
  -d '{
    "model": "llama-3.3-70b-versatile",
    "messages": [
      {
        "role": "user",
        "content": "What is 2+2? Answer in one word."
      }
    ],
    "max_tokens": 10
  }' | python3 -m json.tool

echo ""
echo "✅ If you see a response above, the API is working!"
echo "✅ The Rust RLM code is ready - just need to fix compilation"
