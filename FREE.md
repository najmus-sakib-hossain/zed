# Truly Keyless LLM Providers - BRUTAL REALITY CHECK

> **Zero API Keys • Zero Signup • Zero Authentication**  
> Only providers that ACTUALLY WORK right now

**Last Tested:** February 21, 2026  
**Reality Check:** Every model brutally tested, no BS

---

## The Brutal Truth

**Only 2 providers are truly usable:**
1. **Pollinations.ai** - 1 model, works perfectly, no limits
2. **mlvoca.com** - 2 small models, reliable, no limits

**What's mostly useless:**
- **ApiAirforce** - 87 models but only 1 works without rate limits (99% useless)

**What doesn't work:**
- AI Horde - No active workers
- HuggingFace (no auth) - API deprecated

---

## 1. Pollinations.ai ⭐ BEST CHOICE

**Endpoint:** `https://text.pollinations.ai/`  
**Authentication:** None  
**Reality:** Actually works as advertised. No rate limits, no BS.  
**Status:** ✓ Works perfectly

### Available Model

**1 model:**
- `openai-fast` (aliases: `openai`, `gpt-oss`, `gpt-oss-20b`, `ovh-reasoning`)
- 20B parameter reasoning model
- Supports tools and reasoning
- Fast responses
- No limits

### Usage

**GET (Simplest - RECOMMENDED):**
```bash
curl "https://text.pollinations.ai/What is 2+2?"
```

**Python GET:**
```python
import requests

response = requests.get("https://text.pollinations.ai/Explain Rust in one sentence")
print(response.text)
```

**POST (with options):**
```python
import requests

response = requests.post("https://text.pollinations.ai/", json={
    "model": "openai",
    "messages": [{"role": "user", "content": "Hello"}]
})
print(response.text)
```

**Get model info:**
```bash
curl https://text.pollinations.ai/models
```

**Pros:**
- ✓ Actually free with no limits
- ✓ Fast responses
- ✓ Simple to use
- ✓ No promotional spam
- ✓ Reliable
- ✓ 20B reasoning model

**Cons:**
- Only 1 model

---

## 2. mlvoca.com ⭐ RELIABLE

**Endpoint:** `https://mlvoca.com/api/generate`  
**Authentication:** None  
**Reality:** Works reliably. Small models but they actually respond.  
**Format:** Ollama-compatible  
**Status:** ✓ Works

### Available Models

**2 models:**
1. `tinyllama` - 1.1B parameters, fast
2. `deepseek-r1:1.5b` - 1.5B parameters, reasoning

### Usage

```python
import requests

response = requests.post("https://mlvoca.com/api/generate", json={
    "model": "deepseek-r1:1.5b",
    "prompt": "Explain Python",
    "stream": False
})

print(response.json()["response"])
```

**Bash:**
```bash
curl -X POST https://mlvoca.com/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "tinyllama",
    "prompt": "Hello",
    "stream": false
  }'
```

**Pros:**
- ✓ Ollama-compatible API
- ✓ No rate limits
- ✓ Reliable
- ✓ Small models = fast responses

**Cons:**
- Only 2 small models (1-1.5B)
- Limited capabilities compared to larger models

---

## 3. ApiAirforce ⚠️ 99% USELESS

**Endpoint:** `https://api.airforce/v1/chat/completions`  
**Authentication:** None  
**Reality:** BRUTAL - Tested all 87 models, only 1 works without rate limits!  
**Format:** OpenAI-compatible  
**Status:** ⚠️ Mostly useless

### What Actually Works (Tested ALL 87 Models)

**Only 1 model works without rate limits:**
- `step-3.5-flash:free` ✓ (StepFun model)

**All other 86 models are immediately rate limited:**
- `gpt-4o-mini` ✗ Rate limited
- `gpt-3.5-turbo` ✗ Rate limited
- `claude-sonnet-4.5` ✗ Rate limited
- `claude-haiku-4.5` ✗ Rate limited
- `gemini-3-flash` ✗ Rate limited
- `gemini-2.5-flash` ✗ Rate limited
- `deepseek-v3.2` ✗ Rate limited
- `deepseek-r1` ✗ Rate limited
- `llama-4-scout` ✗ Rate limited
- `llama-4-maverick` ✗ Rate limited
- `grok-4-mini:free` ✗ Rate limited
- `grok-4` ✗ Rate limited
- `qwen3.5` ✗ Rate limited
- `glm-5` ✗ Rate limited
- `kimi-k2` ✗ Rate limited
- All other 71 models ✗ Rate limited

**Reality:** They advertise 87 chat models but 99% are immediately rate limited. Every response includes "Ratelimit Exceeded! Please join: https://discord.g" or returns HTTP 429. This is a bait-and-switch to get you to join their Discord.

### Usage (for the 1 working model)

```python
import requests

response = requests.post("https://api.airforce/v1/chat/completions", json={
    "model": "step-3.5-flash:free",
    "messages": [{"role": "user", "content": "Hello"}]
})

if response.status_code == 200:
    print(response.json()["choices"][0]["message"]["content"])
```

**Pros:**
- OpenAI-compatible API
- 1 model works

**Cons:**
- 86 out of 87 models are rate limited
- Constant promotional messages
- Pushes you to join Discord
- Essentially useless for most use cases

**Verdict:** Not recommended. Use Pollinations.ai or mlvoca.com instead.

---

## What DOESN'T Work

### AI Horde ✗
- **Status:** No active text workers
- **Reality:** Depends on community GPU donations. Currently offline for text generation.
- **Tested:** 0 active text models available
- **Verdict:** Don't rely on it

### HuggingFace (no auth) ✗
- **Status:** API deprecated
- **Reality:** `api-inference.huggingface.co` is no longer supported
- **Error:** HTTP 410 Gone
- **Verdict:** Dead without API key

---

## Comparison - The Real Deal

| Provider | Models | Actually Free? | Rate Limits | Reliability | Verdict |
|----------|--------|----------------|-------------|-------------|---------|
| **Pollinations.ai** | 1 (20B) | YES | None | High | ⭐ USE THIS |
| **mlvoca.com** | 2 (1-1.5B) | YES | None | High | ⭐ USE THIS |
| **ApiAirforce** | 1/87 usable | Sort of | 99% rate limited | Low | ⚠️ AVOID |

---

## Brutal Recommendations

### For Production
**Use Pollinations.ai**
- Only truly free provider with a decent model
- 20B reasoning model
- Reliable, fast, no rate limits
- No catches, no BS

### For Quick Testing
**Use mlvoca.com**
- Small models but reliable
- Ollama-compatible
- Good for simple tasks
- No rate limits

### Avoid
**ApiAirforce**
- 99% of models are rate limited
- Bait-and-switch marketing
- Constant Discord promotion
- Not worth the hassle

---

## Quick Start - What Actually Works

### Pollinations.ai (Recommended)
```bash
# Simplest possible LLM call - actually works!
curl "https://text.pollinations.ai/What is Rust?"
```

```python
import requests

# GET method - dead simple
response = requests.get("https://text.pollinations.ai/Explain async/await")
print(response.text)

# POST method - more control
response = requests.post("https://text.pollinations.ai/", json={
    "model": "openai",
    "messages": [{"role": "user", "content": "Hello"}]
})
print(response.text)
```

### mlvoca.com
```python
import requests

r = requests.post("https://mlvoca.com/api/generate", json={
    "model": "deepseek-r1:1.5b",
    "prompt": "Explain Python",
    "stream": False
})
print(r.json()["response"])
```

---

## Test Script - Verify Yourself

```python
#!/usr/bin/env python3
import requests

print("Testing truly keyless providers...\n")

# 1. Pollinations.ai
print("1. Pollinations.ai:")
try:
    r = requests.get("https://text.pollinations.ai/Say hello", timeout=10)
    print(f"   ✓ {r.text[:50]}\n")
except Exception as e:
    print(f"   ✗ {e}\n")

# 2. mlvoca.com
print("2. mlvoca.com:")
try:
    r = requests.post("https://mlvoca.com/api/generate",
        json={"model": "tinyllama", "prompt": "Say hello", "stream": False},
        timeout=10)
    print(f"   ✓ {r.json()['response'][:50]}\n")
except Exception as e:
    print(f"   ✗ {e}\n")

# 3. ApiAirforce (the only working model)
print("3. ApiAirforce (step-3.5-flash:free):")
try:
    r = requests.post("https://api.airforce/v1/chat/completions",
        json={"model": "step-3.5-flash:free", "messages": [{"role": "user", "content": "Say hello"}]},
        timeout=10)
    if r.status_code == 200:
        print(f"   ✓ {r.json()['choices'][0]['message']['content'][:50]}\n")
    else:
        print(f"   ✗ HTTP {r.status_code}\n")
except Exception as e:
    print(f"   ✗ {e}\n")
```

---

## The Bottom Line

**2 providers actually work without any catches:**

1. **Pollinations.ai** - Best choice, 20B model, truly free, reliable
2. **mlvoca.com** - Small models, reliable, good for simple tasks

**1 provider is mostly useless:**

3. **ApiAirforce** - 99% rate limited (86/87 models), bait-and-switch

**Everything else:**
- Requires API keys
- Is currently offline
- Has been deprecated

---

## Total Reality

- **2 truly usable providers**
- **3 working models total** (1 + 2)
- **Zero authentication required**
- **Zero rate limits** (for the 2 good providers)

**Final Verdict:**
- Use **Pollinations.ai** for anything serious
- Use **mlvoca.com** for small/simple tasks
- Avoid **ApiAirforce** unless you specifically need their 1 working model

**Verified:** February 21, 2026 - All 87 ApiAirforce models tested individually
