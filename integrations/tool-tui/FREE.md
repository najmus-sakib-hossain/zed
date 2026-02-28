# Keyless LLM API Models - Complete Reference (February 2026)

> **Zero signup, zero API keys, instant access**  
> Tested and verified on February 21, 2026

---

## 1. Pollinations.ai

**Status:** ✓ Fully Operational  
**Endpoint:** `https://text.pollinations.ai/`  
**Authentication:** None required  
**Rate Limits:** None (community-funded)

### Available Model

**Primary Model:** GPT-OSS 20B Reasoning LLM (OVH)

| Model ID | Aliases | Features |
|----------|---------|----------|
| `openai-fast` | `openai`, `gpt-oss`, `gpt-oss-20b`, `ovh-reasoning` | Text generation, reasoning, tool support |

**Capabilities:**
- ✓ Text generation
- ✓ Reasoning tasks
- ✓ Tool/function calling
- ✗ Vision (not supported)
- ✗ Audio (not supported)

**Tier:** Anonymous (free, unlimited)

**Note:** As of February 2026, Pollinations.ai legacy API supports only GPT-OSS 20B. Use any of the aliases interchangeably.

### Usage Examples

**GET Method (Simplest):**
```bash
curl "https://text.pollinations.ai/What is 2+2?"
# Response: 2 + 2 = 4.
```

**POST Method (Model Selection):**
```bash
curl -X POST https://text.pollinations.ai/ \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [{"role": "user", "content": "Explain Rust"}],
    "model": "gpt-oss-20b"
  }'
```

**Python:**
```python
import requests

# Simple GET
response = requests.get("https://text.pollinations.ai/What is Rust?")
print(response.text)

# POST with model selection
response = requests.post("https://text.pollinations.ai/", json={
    "messages": [{"role": "user", "content": "Hello"}],
    "model": "openai"  # or "gpt-oss-20b", "openai-fast", etc.
})
print(response.text)
```

---

## 2. mlvoca.com

**Status:** ✓ Fully Operational  
**Endpoint:** `https://mlvoca.com/api/generate`  
**Authentication:** None required  
**Rate Limits:** None (limited hardware resources)

### Available Models

| Model ID | Parameters | Description | Best For |
|----------|------------|-------------|----------|
| `tinyllama` | 1.1B | Extremely fast, lightweight | Simple tasks, classification, quick responses |
| `deepseek-r1:1.5b` | 1.5B | Small reasoning model | Logic, analysis, code understanding |

### Usage Examples

**Non-Streaming:**
```bash
curl -X POST https://mlvoca.com/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "tinyllama",
    "prompt": "What is 2+2?",
    "stream": false
  }'
```

**Streaming:**
```bash
curl -X POST https://mlvoca.com/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "deepseek-r1:1.5b",
    "prompt": "Explain Rust"
  }'
```

**Python:**
```python
import requests

response = requests.post("https://mlvoca.com/api/generate", json={
    "model": "deepseek-r1:1.5b",
    "prompt": "Write a Python hello world",
    "stream": False
})
print(response.json()["response"])
```

### Parameters

- `model` (required): `tinyllama` or `deepseek-r1:1.5b`
- `prompt` (required): Input text
- `stream` (optional): `true` for streaming, `false` for single response
- `temperature` (optional): Randomness (0.0-1.0)
- `system` (optional): System message override
- `format` (optional): Response format (`json` or schema)

---

## 3. AI Horde (stablehorde.net)

**Status:** ✓ Fully Operational (Crowdsourced)  
**Endpoint:** `https://stablehorde.net/api/v2/generate/text/async`  
**Authentication:** `0000000000` (10 zeros - anonymous key)  
**Rate Limits:** Queue-based (3-10 second wait)

### Available Models (27+ Active)

#### Popular Models (February 2026)

| Model Name | Type | Parameters | Queue Status |
|------------|------|------------|--------------|
| `TheDrummer/Cydonia-24B-v4.3` | KoboldCPP | 24B | ✓ Active (16 workers) |
| `TheDrummer/Skyfall-31B-v4` | KoboldCPP | 31B | ✓ Active (4 workers) |
| `koboldcpp/L3-8B-Stheno-v3.2` | Llama 3 | 8B | ✓ Active (4 workers) |
| `koboldcpp/L3-8B-Stheno-v3.3` | Llama 3 | 8B | ✓ Active |
| `koboldcpp/Llama-3.1-8B-Lexi-Uncensored-V2` | Llama 3.1 | 8B | ✓ Active |
| `koboldcpp/mini-magnum-12b-v1.1` | Magnum | 12B | ✓ Active |
| `koboldcpp/NeonMaid-12B-v2` | Creative | 12B | ✓ Active (2 workers) |
| `koboldcpp/Rocinante-X-12B-v1` | Roleplay | 12B | ✓ Active |
| `koboldcpp/Fimbulvetr-11B-v2` | Creative | 11B | ✓ Active |
| `koboldcpp/Magidonia-24B-v4.3` | Creative | 24B | ✓ Active |
| `koboldcpp/Dark-Nexus-24B-v2.0` | Creative | 24B | ✓ Active |
| `aphrodite/TheDrummer/Behemoth-X-123B-v2.1` | Large | 123B | ✓ Active |
| `Qwen/Qwen3-Coder-480B-A35B-Instruct` | Coding | 35B (MoE) | ✓ Active (2 workers) |
| `zai-org/GLM-4.6` | General | 4.6B | ✓ Active |

#### Lightweight Models

| Model Name | Parameters | Best For |
|------------|------------|----------|
| `koboldcpp/Falcon-H1-Tiny-90M-Instruct` | 90M | Ultra-fast responses |
| `koboldcpp/gemma-3-1B-it` | 1B | Quick tasks |
| `koboldcpp/KobbleTiny-1.1B` | 1.1B | Simple queries |
| `koboldcpp/LFM2.5-1.2B-Instruct` | 1.2B | Fast inference |
| `koboldcpp/Qwen3-0.6B` | 600M | Minimal latency |

### Usage Example

**Python (Async Queue):**
```python
import requests
import time

headers = {"apikey": "0000000000"}

# Submit job
response = requests.post(
    "https://stablehorde.net/api/v2/generate/text/async",
    json={
        "prompt": "Write a haiku about coding",
        "params": {
            "n": 1,
            "max_length": 80,
            "max_context_length": 1024
        },
        "models": ["koboldcpp/L3-8B-Stheno-v3.2"]  # Optional
    },
    headers=headers
)

job_id = response.json()["id"]
print(f"Job submitted: {job_id}")

# Poll for result
for _ in range(10):
    time.sleep(3)
    status = requests.get(
        f"https://stablehorde.net/api/v2/generate/text/status/{job_id}",
        headers=headers
    ).json()
    
    if status["done"]:
        print(status["generations"][0]["text"])
        break
```

**Bash:**
```bash
# Submit job
JOB_ID=$(curl -s -X POST https://stablehorde.net/api/v2/generate/text/async \
  -H "Content-Type: application/json" \
  -H "apikey: 0000000000" \
  -d '{"prompt": "Hello world","params": {"n": 1, "max_length": 50}}' \
  | grep -o '"id":"[^"]*"' | cut -d'"' -f4)

# Check status
sleep 5
curl -s "https://stablehorde.net/api/v2/generate/text/status/$JOB_ID" \
  -H "apikey: 0000000000"
```

### Model Categories

- **Creative Writing:** Stheno, NeonMaid, Magidonia, Fimbulvetr
- **Roleplay:** Rocinante, Lexi-Uncensored, Dark-Nexus
- **Coding:** Qwen3-Coder
- **General Purpose:** Llama 3/3.1, Cydonia, Skyfall
- **Large Context:** Behemoth-X (123B)

---

## Comparison Matrix

| Service | Models | Speed | Queue | Best For |
|---------|--------|-------|-------|----------|
| **Pollinations.ai** | 1 (GPT-OSS 20B) | Instant | None | General use, reasoning, tools |
| **mlvoca.com** | 2 (TinyLlama, DeepSeek-R1) | Fast | None | Small models, reasoning |
| **AI Horde** | 27+ (Llama, Qwen, Creative) | 3-10s | Yes | Variety, creative tasks |

---

## Quick Start Guide

### 1. Fastest Response (Pollinations)
```bash
curl "https://text.pollinations.ai/What is Rust?"
```

### 2. Best Small Model (mlvoca)
```python
import requests
r = requests.post("https://mlvoca.com/api/generate", json={
    "model": "deepseek-r1:1.5b",
    "prompt": "Explain Python",
    "stream": False
})
print(r.json()["response"])
```

### 3. Most Model Variety (AI Horde)
```python
import requests, time
headers = {"apikey": "0000000000"}
job = requests.post(
    "https://stablehorde.net/api/v2/generate/text/async",
    json={"prompt": "Hello", "params": {"max_length": 50}},
    headers=headers
).json()
time.sleep(5)
result = requests.get(
    f"https://stablehorde.net/api/v2/generate/text/status/{job['id']}",
    headers=headers
).json()
print(result["generations"][0]["text"])
```

---

## Notes

- **Pollinations.ai:** Community-funded, no limits, GPT-OSS 20B only
- **mlvoca.com:** Limited hardware, encourage scientific use
- **AI Horde:** Crowdsourced GPUs, 3-10s wait, 27+ models

All services tested and verified working on **February 21, 2026**.

---

## Test Files

- `test_pollinations.py` - Test Pollinations API
- `test_mlvoca.py` - Test mlvoca API  
- `test_ai_horde.py` - Test AI Horde async queue
- `test_keyless_llms.sh` - Bash test script

Run tests: `python test_pollinations.py` or `bash test_keyless_llms.sh`

---

## Model Count Summary

- **Pollinations.ai:** 1 model (GPT-OSS 20B with 5 aliases)
- **mlvoca.com:** 2 models (TinyLlama, DeepSeek-R1 1.5B)
- **AI Horde:** 27+ active models (Llama 3/3.1, Qwen, creative/roleplay models)
- **Total:** 30+ unique models across all services
