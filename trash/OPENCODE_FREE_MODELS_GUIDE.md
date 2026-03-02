# OpenCode Free Models - Complete Usage Guide

## Overview

OpenCode provides **3 completely free AI models** that work without any API key. These models are accessible through their Zen API gateway and cost $0.00 per request.

## Free Models Available

### 1. trinity-large-preview-free
- **Provider**: Arcee AI
- **Context Window**: 131,072 tokens (~131K)
- **Output Limit**: 131,072 tokens
- **Features**: Standard text generation
- **Best For**: General purpose tasks, quick responses

### 2. big-pickle
- **Provider**: Minimax (M2.5)
- **Context Window**: 200,000 tokens (~200K)
- **Output Limit**: 128,000 tokens
- **Features**: Reasoning model with internal thought process
- **Best For**: Complex reasoning, problem-solving, debugging

### 3. minimax-m2.5-free
- **Provider**: Minimax (M2.5)
- **Context Window**: 204,800 tokens (~204K)
- **Output Limit**: 131,072 tokens
- **Features**: Reasoning model with internal thought process
- **Best For**: Complex reasoning, long-context tasks

## API Configuration

### Endpoint
```
https://opencode.ai/zen/v1/chat/completions
```

### Authentication
```
Authorization: Bearer public
```

No API key required - just use the literal string `"public"` as your bearer token.

## Usage Examples

### cURL Example

```bash
curl -X POST "https://opencode.ai/zen/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer public" \
  -d '{
    "model": "trinity-large-preview-free",
    "messages": [
      {"role": "user", "content": "Explain quantum computing"}
    ],
    "max_tokens": 500
  }'
```

### Python Example

```python
import requests

url = "https://opencode.ai/zen/v1/chat/completions"
headers = {
    "Content-Type": "application/json",
    "Authorization": "Bearer public"
}

payload = {
    "model": "big-pickle",
    "messages": [
        {"role": "user", "content": "Write a Python function to sort a list"}
    ],
    "max_tokens": 1000
}

response = requests.post(url, json=payload, headers=headers)
print(response.json())
```

### JavaScript/Node.js Example

```javascript
const response = await fetch('https://opencode.ai/zen/v1/chat/completions', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer public'
  },
  body: JSON.stringify({
    model: 'minimax-m2.5-free',
    messages: [
      { role: 'user', content: 'Explain async/await in JavaScript' }
    ],
    max_tokens: 800
  })
});

const data = await response.json();
console.log(data.choices[0].message.content);
```

### OpenAI SDK Compatible

These models work with OpenAI-compatible SDKs:

```python
from openai import OpenAI

client = OpenAI(
    base_url="https://opencode.ai/zen/v1",
    api_key="public"
)

response = client.chat.completions.create(
    model="trinity-large-preview-free",
    messages=[
        {"role": "user", "content": "Hello, how are you?"}
    ]
)

print(response.choices[0].message.content)
```

## Request Parameters

### Required Parameters
- `model`: One of `trinity-large-preview-free`, `big-pickle`, or `minimax-m2.5-free`
- `messages`: Array of message objects with `role` and `content`

### Optional Parameters
- `max_tokens`: Maximum tokens to generate (default varies by model)
- `temperature`: Sampling temperature 0-2 (default: 1.0)
- `top_p`: Nucleus sampling parameter (default: 1.0)
- `stream`: Boolean for streaming responses (default: false)

## Response Format

### Standard Response
```json
{
  "id": "gen-1772192332-...",
  "provider": "Arcee AI",
  "model": "arcee-ai/trinity-large-preview:free",
  "object": "chat.completion",
  "created": 1772192332,
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "The response text here",
        "reasoning": null
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 16,
    "completion_tokens": 14,
    "total_tokens": 30,
    "cost": 0
  }
}
```

### Reasoning Model Response (big-pickle, minimax-m2.5-free)
```json
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "The final answer",
        "reasoning": "Internal thought process...",
        "reasoning_details": [
          {
            "type": "reasoning.text",
            "text": "Step-by-step reasoning..."
          }
        ]
      }
    }
  ]
}
```

## Model Selection Guide

### Use **trinity-large-preview-free** when:
- You need fast, straightforward responses
- Simple Q&A or text generation
- You don't need reasoning traces
- Lower token usage is preferred

### Use **big-pickle** when:
- You need to see the model's reasoning process
- Complex problem-solving tasks
- Debugging or understanding model logic
- Medium-length contexts (up to 200K tokens)

### Use **minimax-m2.5-free** when:
- You need the largest context window (204K tokens)
- Long document analysis
- Complex reasoning with extensive context
- You want reasoning traces for transparency

## Rate Limits & Restrictions

- **No explicit rate limits documented** - use responsibly
- **No API key required** - truly free access
- **No credit card needed** - no hidden costs
- Models may have server-side throttling during high load
- Some models marked as "deprecated" may be removed in future

## Integration with OpenCode IDE

If using the OpenCode IDE, these models are automatically available:

1. No configuration needed
2. Models appear in the model selector
3. Automatically uses `apiKey: "public"` when no key is set
4. Paid models are hidden unless you add an API key

## Troubleshooting

### Empty Responses
If you get empty responses, try:
- Increasing `max_tokens` parameter
- Using a different model
- Simplifying your prompt

### Model Not Found
Ensure you're using the exact model ID:
- ✅ `trinity-large-preview-free`
- ✅ `big-pickle`
- ✅ `minimax-m2.5-free`
- ❌ `gpt-5-nano` (currently broken)

### Authentication Errors
Make sure you're using:
```
Authorization: Bearer public
```
Not `Authorization: public` or any other format.

## Cost Comparison

| Model | Input Cost | Output Cost | Total Cost |
|-------|-----------|-------------|------------|
| trinity-large-preview-free | $0.00 | $0.00 | **$0.00** |
| big-pickle | $0.00 | $0.00 | **$0.00** |
| minimax-m2.5-free | $0.00 | $0.00 | **$0.00** |

Compare to paid alternatives:
- GPT-4: $10-75 per 1M tokens
- Claude Opus: $15-75 per 1M tokens
- Gemini Pro: $2-12 per 1M tokens

## Legal & Terms

- These models are provided by OpenCode through their Zen API gateway
- Actual model hosting by Arcee AI and Minimax
- Use responsibly and ethically
- No guarantees on uptime or availability
- Check [OpenCode's terms](https://opencode.ai/docs/zen) for latest policies

## Additional Resources

- [OpenCode Documentation](https://opencode.ai/docs/zen)
- [Models.dev API Reference](https://models.dev)
- [OpenCode GitHub](https://github.com/opencode-ai)

---

**Last Updated**: February 2026  
**Tested**: All 3 models verified working with curl requests
