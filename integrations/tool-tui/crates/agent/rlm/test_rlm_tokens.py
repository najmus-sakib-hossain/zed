#!/usr/bin/env python3
import json
import urllib.request
import time

GEMINI_API_KEY = "AIzaSyDkGMc89MIF6umVwpoAuezYN7m7xsqOiZ0"
API_URL = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-lite:generateContent"

def call_api(prompt, max_tokens=200):
    url = f"{API_URL}?key={GEMINI_API_KEY}"
    data = {
        "contents": [{"parts": [{"text": prompt}]}],
        "generationConfig": {"maxOutputTokens": max_tokens, "temperature": 0.7}
    }
    req = urllib.request.Request(url, data=json.dumps(data).encode('utf-8'), 
                                  headers={"Content-Type": "application/json"}, method='POST')
    try:
        with urllib.request.urlopen(req) as response:
            return json.loads(response.read().decode('utf-8'))
    except:
        return {"error": True}

# Read document
with open('test_document.txt', 'r') as f:
    document = f.read()

print(f"Document size: {len(document):,} characters")
print(f"Actual tokens (Gemini): 46,224 (from dx token command)")

# RLM Processing
words = document.split()
chunk_size = 4000
chunks = [" ".join(words[i:i+chunk_size]) for i in range(0, len(words), chunk_size)]

print(f"\nRLM: Processing {len(chunks)} chunks...")

total_input = 0
total_output = 0
summaries = []

for idx, chunk in enumerate(chunks):
    result = call_api(f"List key optimization patterns:\n\n{chunk}", 150)
    if "error" not in result:
        try:
            summaries.append(result['candidates'][0]['content']['parts'][0]['text'])
            usage = result.get('usageMetadata', {})
            total_input += usage.get('promptTokenCount', 0)
            total_output += usage.get('candidatesTokenCount', 0)
            print(f"  Chunk {idx+1}: {usage.get('promptTokenCount', 0)} in, {usage.get('candidatesTokenCount', 0)} out")
        except:
            pass
    time.sleep(1.5)

# Synthesis
combined = "\n".join([f"{i+1}. {s}" for i, s in enumerate(summaries)])
result = call_api(f"Summarize patterns:\n{combined}", 300)
if "error" not in result:
    try:
        usage = result.get('usageMetadata', {})
        total_input += usage.get('promptTokenCount', 0)
        total_output += usage.get('candidatesTokenCount', 0)
        print(f"  Synthesis: {usage.get('promptTokenCount', 0)} in, {usage.get('candidatesTokenCount', 0)} out")
    except:
        pass

print(f"\n{'='*60}")
print("RESULTS")
print(f"{'='*60}")
print(f"Traditional (full doc): 46,224 tokens")
print(f"RLM (actual):           {total_input + total_output:,} tokens")
print(f"{'='*60}")

if total_input + total_output < 46224:
    savings = 46224 - (total_input + total_output)
    pct = (savings / 46224) * 100
    print(f"SAVINGS: {savings:,} tokens ({pct:.1f}%)")
else:
    extra = (total_input + total_output) - 46224
    pct = (extra / 46224) * 100
    print(f"RLM used {extra:,} MORE tokens (+{pct:.1f}%)")
