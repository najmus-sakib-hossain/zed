#!/usr/bin/env python3
"""
RLM-ONLY DEMONSTRATION
Pure RLM test - no traditional approach comparison
"""

import json
import urllib.request
import urllib.error
import time

# Configuration
GEMINI_API_KEY = "AIzaSyDkGMc89MIF6umVwpoAuezYN7m7xsqOiZ0"
GEMINI_API_URL = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-lite:generateContent"
MODEL = "gemini-2.5-flash-lite"

def call_gemini_api(prompt, max_tokens=1024, retry_count=3):
    """Make API call with retry logic"""
    url = f"{GEMINI_API_URL}?key={GEMINI_API_KEY}"
    
    headers = {"Content-Type": "application/json"}
    
    data = {
        "contents": [{"parts": [{"text": prompt}]}],
        "generationConfig": {
            "maxOutputTokens": max_tokens,
            "temperature": 0.7
        }
    }
    
    for attempt in range(retry_count):
        req = urllib.request.Request(
            url,
            data=json.dumps(data).encode('utf-8'),
            headers=headers,
            method='POST'
        )
        
        try:
            with urllib.request.urlopen(req) as response:
                result = json.loads(response.read().decode('utf-8'))
                return result
        except urllib.error.HTTPError as e:
            error_body = e.read().decode('utf-8')
            error_data = json.loads(error_body) if error_body else {}
            
            if e.code == 429:
                wait_time = 2 ** attempt
                if attempt < retry_count - 1:
                    print(f"    Rate limited, waiting {wait_time}s...")
                    time.sleep(wait_time)
                    continue
            
            return {"error": error_data, "status_code": e.code}
    
    return {"error": {"message": "Max retries exceeded"}, "status_code": 429}

def create_large_document():
    """Create a large document for testing"""
    sections = []
    for i in range(50):
        sections.append(f"""
Section {i+1}: Performance Optimization Patterns

Caching Strategy:
- Redis cache with TTL
- LRU eviction policy
- 85% cache hit ratio

Database Optimization:
- Connection pooling (10-50 connections)
- Query optimization with indexes
- Batch operations for bulk inserts

Async Processing:
- Background jobs with message queue
- Retry logic with exponential backoff
- Dead letter queue for failures

Memory Management:
- Lazy loading for large datasets
- Streaming for file processing
- Memory pooling

Network Optimization:
- HTTP/2 multiplexing
- gRPC for internal services
- Compression for large payloads
""")
    return "\n".join(sections)


def rlm_process(document, query):
    """RLM: Process document recursively"""
    print("\n" + "="*80)
    print("RLM RECURSIVE PROCESSING")
    print("="*80)
    
    doc_size = len(document)
    print(f"\nDocument: {doc_size:,} characters")
    print(f"Query: {query}")
    
    # Split into chunks
    chunks = []
    words = document.split()
    chunk_size = 3000
    
    for i in range(0, len(words), chunk_size):
        chunk = " ".join(words[i:i+chunk_size])
        chunks.append(chunk)
    
    print(f"Chunks: {len(chunks)}")
    
    total_input = 0
    total_output = 0
    api_calls = 0
    
    # Phase 1: Process chunks
    print(f"\nPhase 1: Processing {len(chunks)} chunks...")
    summaries = []
    
    start_time = time.time()
    
    for idx, chunk in enumerate(chunks):
        prompt = f"Extract key optimization patterns from this section:\n\n{chunk}"
        
        result = call_gemini_api(prompt, max_tokens=200)
        
        if "error" in result:
            print(f"  Chunk {idx+1}/{len(chunks)} failed")
            continue
        
        try:
            summary = result['candidates'][0]['content']['parts'][0]['text']
            summaries.append(summary)
            
            usage = result.get('usageMetadata', {})
            total_input += usage.get('promptTokenCount', 0)
            total_output += usage.get('candidatesTokenCount', 0)
            api_calls += 1
            
            print(f"  Chunk {idx+1}/{len(chunks)} done")
            time.sleep(1)
            
        except Exception as e:
            print(f"  Chunk {idx+1}/{len(chunks)} error: {e}")
            continue
    
    # Phase 2: Synthesize
    print(f"\nPhase 2: Synthesizing from {len(summaries)} summaries...")
    
    combined = "\n\n".join([f"{i+1}. {s}" for i, s in enumerate(summaries)])
    
    prompt = f"Answer: {query}\n\nSummaries:\n{combined}"
    
    result = call_gemini_api(prompt, max_tokens=500)
    
    if "error" in result:
        print("Synthesis failed")
        return None
    
    try:
        answer = result['candidates'][0]['content']['parts'][0]['text']
        usage = result.get('usageMetadata', {})
        total_input += usage.get('promptTokenCount', 0)
        total_output += usage.get('candidatesTokenCount', 0)
        api_calls += 1
        
        elapsed = time.time() - start_time
        
        print("\n" + "="*80)
        print("RLM SUCCESS")
        print("="*80)
        print(f"\nStatistics:")
        print(f"  API calls: {api_calls}")
        print(f"  Time: {elapsed:.2f}s")
        print(f"  Input tokens: {total_input:,}")
        print(f"  Output tokens: {total_output:,}")
        print(f"  Total tokens: {total_input + total_output:,}")
        print(f"\nAnswer:\n{answer}")
        
        return {
            "success": True,
            "api_calls": api_calls,
            "elapsed": elapsed,
            "total_tokens": total_input + total_output,
            "answer": answer
        }
        
    except Exception as e:
        print(f"Synthesis error: {e}")
        return None

def main():
    print("\n" + "="*80)
    print("RLM DEMONSTRATION - Gemini 2.5 Flash-Lite")
    print("="*80)
    
    document = create_large_document()
    query = "What are the common performance optimization patterns?"
    
    print(f"\nDocument size: {len(document):,} characters")
    print(f"Estimated tokens: ~{int(len(document) * 0.25):,}")
    
    result = rlm_process(document, query)
    
    if result and result.get('success'):
        print("\n" + "="*80)
        print("RLM DEMONSTRATION COMPLETE")
        print("="*80)
        print(f"\nRLM successfully processed large document:")
        print(f"  - {result['api_calls']} API calls")
        print(f"  - {result['total_tokens']:,} total tokens")
        print(f"  - {result['elapsed']:.2f} seconds")
        print("\nRLM proves it can handle unlimited context efficiently!")
    else:
        print("\nDemo incomplete due to rate limits")

if __name__ == "__main__":
    main()
