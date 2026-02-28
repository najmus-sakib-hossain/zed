#!/usr/bin/env python3
"""
RLM Test with LARGE Document - Actual Token Measurement
"""

import json
import urllib.request
import urllib.error
import time

GEMINI_API_KEY = "AIzaSyDkGMc89MIF6umVwpoAuezYN7m7xsqOiZ0"
GEMINI_API_URL = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-lite:generateContent"

def call_api(prompt, max_tokens=1024):
    """Make API call"""
    url = f"{GEMINI_API_URL}?key={GEMINI_API_KEY}"
    
    data = {
        "contents": [{"parts": [{"text": prompt}]}],
        "generationConfig": {"maxOutputTokens": max_tokens, "temperature": 0.7}
    }
    
    req = urllib.request.Request(
        url,
        data=json.dumps(data).encode('utf-8'),
        headers={"Content-Type": "application/json"},
        method='POST'
    )
    
    try:
        with urllib.request.urlopen(req) as response:
            return json.loads(response.read().decode('utf-8'))
    except urllib.error.HTTPError as e:
        error_body = e.read().decode('utf-8')
        return {"error": json.loads(error_body) if error_body else {}, "status_code": e.code}

def create_large_document():
    """Create a VERY LARGE document - 200 sections"""
    sections = []
    for i in range(200):
        sections.append(f"""
=== SECTION {i+1}: System Component Analysis ===

Component Overview:
Component {i+1} is a critical microservice handling data processing, validation, 
and transformation. It processes approximately 50,000 requests per day with 
99.9% uptime SLA requirements.

Performance Optimizations:
1. Caching Layer: Redis cache with 300s TTL, achieving 82% hit ratio
2. Database: PostgreSQL with connection pool (20-100 connections)
3. Async Processing: RabbitMQ message queue for background jobs
4. Memory: Lazy loading and streaming for large datasets
5. Network: HTTP/2 with gzip compression

Code Architecture:
- Service layer with dependency injection
- Repository pattern for data access
- Event-driven architecture with pub/sub
- Circuit breaker for external API calls
- Distributed tracing with OpenTelemetry

Metrics:
- P50 latency: 45ms
- P95 latency: 120ms  
- P99 latency: 250ms
- Throughput: 2,500 req/s
- Error rate: 0.05%
- CPU usage: 40% average
- Memory: 1.2GB average
""")
    return "\n".join(sections)

def rlm_test(document, query):
    """Test RLM approach"""
    print("\n" + "="*80)
    print("RLM TEST - LARGE DOCUMENT")
    print("="*80)
    
    print(f"\nDocument: {len(document):,} characters")
    print(f"Query: {query}")
    
    # Split into chunks
    words = document.split()
    chunk_size = 4000
    chunks = []
    for i in range(0, len(words), chunk_size):
        chunks.append(" ".join(words[i:i+chunk_size]))
    
    print(f"Chunks: {len(chunks)}")
    
    total_input = 0
    total_output = 0
    
    # Phase 1: Process chunks
    print(f"\nPhase 1: Processing {len(chunks)} chunks...")
    summaries = []
    
    for idx, chunk in enumerate(chunks):
        prompt = f"List the key optimization patterns in this section:\n\n{chunk}"
        
        result = call_api(prompt, max_tokens=150)
        
        if "error" in result:
            print(f"  Chunk {idx+1} failed")
            continue
        
        try:
            summary = result['candidates'][0]['content']['parts'][0]['text']
            summaries.append(summary)
            
            usage = result.get('usageMetadata', {})
            total_input += usage.get('promptTokenCount', 0)
            total_output += usage.get('candidatesTokenCount', 0)
            
            print(f"  Chunk {idx+1}/{len(chunks)} - Input: {usage.get('promptTokenCount', 0)}, Output: {usage.get('candidatesTokenCount', 0)}")
            time.sleep(1.5)
            
        except Exception as e:
            print(f"  Chunk {idx+1} error: {e}")
            continue
    
    # Phase 2: Synthesize
    print(f"\nPhase 2: Synthesizing from {len(summaries)} summaries...")
    
    combined = "\n".join([f"{i+1}. {s}" for i, s in enumerate(summaries)])
    prompt = f"Answer: {query}\n\nSummaries:\n{combined}"
    
    result = call_api(prompt, max_tokens=400)
    
    if "error" not in result:
        try:
            usage = result.get('usageMetadata', {})
            total_input += usage.get('promptTokenCount', 0)
            total_output += usage.get('candidatesTokenCount', 0)
            print(f"  Synthesis - Input: {usage.get('promptTokenCount', 0)}, Output: {usage.get('candidatesTokenCount', 0)}")
        except:
            pass
    
    print("\n" + "="*80)
    print("RLM RESULTS")
    print("="*80)
    print(f"\nTotal Input Tokens: {total_input:,}")
    print(f"Total Output Tokens: {total_output:,}")
    print(f"TOTAL TOKENS: {total_input + total_output:,}")
    
    return total_input + total_output

def main():
    print("\n" + "="*80)
    print("LARGE DOCUMENT TOKEN SAVINGS TEST")
    print("="*80)
    
    document = create_large_document()
    query = "What are the common performance optimization patterns?"
    
    print(f"\nDocument size: {len(document):,} characters")
    print(f"Estimated if sent as single prompt: ~{int(len(document) * 0.25):,} tokens")
    
    rlm_tokens = rlm_test(document, query)
    
    estimated_traditional = int(len(document) * 0.25)
    
    print("\n" + "="*80)
    print("COMPARISON")
    print("="*80)
    print(f"\nTraditional (estimated): {estimated_traditional:,} tokens")
    print(f"RLM (actual): {rlm_tokens:,} tokens")
    
    if rlm_tokens < estimated_traditional:
        savings = estimated_traditional - rlm_tokens
        savings_pct = (savings / estimated_traditional) * 100
        print(f"\nSAVINGS: {savings:,} tokens ({savings_pct:.1f}%)")
    else:
        extra = rlm_tokens - estimated_traditional
        extra_pct = (extra / estimated_traditional) * 100
        print(f"\nRLM used {extra:,} MORE tokens ({extra_pct:.1f}%)")
        print("Note: RLM trades tokens for ability to handle unlimited context")

if __name__ == "__main__":
    main()
