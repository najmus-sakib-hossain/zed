"""
Benchmark CPython's asyncio for comparison with dx-py-reactor

This benchmark measures async I/O performance:
- File read/write operations
- Batch file operations
- Network-like operations (simulated)
- DNS resolution
"""
import asyncio
import time
import tempfile
import os
import socket
import sys

# Number of iterations for each benchmark
ITERATIONS = 1000
FILE_SIZE = 4096  # 4KB files

def measure_sync(name, func, iterations=ITERATIONS):
    """Measure synchronous function execution time"""
    # Warmup
    for _ in range(min(10, iterations // 10)):
        func()
    
    start = time.perf_counter_ns()
    for _ in range(iterations):
        func()
    end = time.perf_counter_ns()
    
    mean_ns = (end - start) / iterations
    return name, mean_ns

async def measure_async(name, coro_func, iterations=ITERATIONS):
    """Measure async function execution time"""
    # Warmup
    for _ in range(min(10, iterations // 10)):
        await coro_func()
    
    start = time.perf_counter_ns()
    for _ in range(iterations):
        await coro_func()
    end = time.perf_counter_ns()
    
    mean_ns = (end - start) / iterations
    return name, mean_ns

# ============================================================================
# File I/O Benchmarks
# ============================================================================

class FileIOBenchmarks:
    def __init__(self):
        self.temp_dir = tempfile.mkdtemp()
        self.test_file = os.path.join(self.temp_dir, "test.bin")
        self.test_data = os.urandom(FILE_SIZE)
        
        # Create test file
        with open(self.test_file, 'wb') as f:
            f.write(self.test_data)
        
        # Create multiple test files for batch operations
        self.batch_files = []
        for i in range(100):
            path = os.path.join(self.temp_dir, f"batch_{i}.bin")
            with open(path, 'wb') as f:
                f.write(os.urandom(1024))  # 1KB each
            self.batch_files.append(path)
    
    def cleanup(self):
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)
    
    # Synchronous benchmarks
    def sync_read_file(self):
        with open(self.test_file, 'rb') as f:
            return f.read()
    
    def sync_write_file(self):
        path = os.path.join(self.temp_dir, "write_test.bin")
        with open(path, 'wb') as f:
            f.write(self.test_data)
    
    def sync_read_batch(self):
        """Read 10 files sequentially"""
        results = []
        for path in self.batch_files[:10]:
            with open(path, 'rb') as f:
                results.append(f.read())
        return results
    
    # Async benchmarks using asyncio
    async def async_read_file(self):
        """Async file read using run_in_executor"""
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, self.sync_read_file)
    
    async def async_write_file(self):
        """Async file write using run_in_executor"""
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, self.sync_write_file)
    
    async def async_read_batch_sequential(self):
        """Read 10 files sequentially with async"""
        results = []
        for path in self.batch_files[:10]:
            loop = asyncio.get_event_loop()
            data = await loop.run_in_executor(None, lambda p=path: open(p, 'rb').read())
            results.append(data)
        return results
    
    async def async_read_batch_parallel(self):
        """Read 10 files in parallel with asyncio.gather"""
        loop = asyncio.get_event_loop()
        tasks = [
            loop.run_in_executor(None, lambda p=path: open(p, 'rb').read())
            for path in self.batch_files[:10]
        ]
        return await asyncio.gather(*tasks)
    
    async def async_read_100_files_parallel(self):
        """Read 100 files in parallel"""
        loop = asyncio.get_event_loop()
        tasks = [
            loop.run_in_executor(None, lambda p=path: open(p, 'rb').read())
            for path in self.batch_files
        ]
        return await asyncio.gather(*tasks)

# ============================================================================
# Network-like Benchmarks (simulated with pipes)
# ============================================================================

class NetworkBenchmarks:
    def __init__(self):
        pass
    
    def sync_socket_pair_roundtrip(self):
        """Create socket pair and do a roundtrip"""
        # Use localhost TCP for more realistic test
        server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server.bind(('127.0.0.1', 0))
        server.listen(1)
        port = server.getsockname()[1]
        
        client = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        client.connect(('127.0.0.1', port))
        
        conn, _ = server.accept()
        
        # Roundtrip
        client.send(b'hello')
        data = conn.recv(1024)
        conn.send(b'world')
        response = client.recv(1024)
        
        client.close()
        conn.close()
        server.close()
        
        return response
    
    async def async_socket_roundtrip(self):
        """Async socket roundtrip using asyncio"""
        # Create server
        server = await asyncio.start_server(
            lambda r, w: None,  # Dummy handler
            '127.0.0.1', 0
        )
        port = server.sockets[0].getsockname()[1]
        
        # Connect client
        reader, writer = await asyncio.open_connection('127.0.0.1', port)
        
        writer.write(b'hello')
        await writer.drain()
        
        writer.close()
        await writer.wait_closed()
        server.close()
        await server.wait_closed()

# ============================================================================
# DNS Resolution Benchmarks
# ============================================================================

class DNSBenchmarks:
    def sync_resolve_localhost(self):
        return socket.gethostbyname('localhost')
    
    async def async_resolve_localhost(self):
        loop = asyncio.get_event_loop()
        return await loop.getaddrinfo('localhost', None)

# ============================================================================
# Main Benchmark Runner
# ============================================================================

async def run_benchmarks():
    print(f"CPython {sys.version.split()[0]} Async I/O Benchmarks")
    print("=" * 70)
    print()
    
    results = []
    
    # File I/O benchmarks
    print("File I/O Benchmarks:")
    print("-" * 70)
    
    file_bench = FileIOBenchmarks()
    
    try:
        # Sync file operations
        results.append(measure_sync("sync_read_4kb", file_bench.sync_read_file, 10000))
        results.append(measure_sync("sync_write_4kb", file_bench.sync_write_file, 10000))
        results.append(measure_sync("sync_read_10_files", file_bench.sync_read_batch, 1000))
        
        # Async file operations
        results.append(await measure_async("async_read_4kb", file_bench.async_read_file, 1000))
        results.append(await measure_async("async_write_4kb", file_bench.async_write_file, 1000))
        results.append(await measure_async("async_read_10_seq", file_bench.async_read_batch_sequential, 100))
        results.append(await measure_async("async_read_10_par", file_bench.async_read_batch_parallel, 100))
        results.append(await measure_async("async_read_100_par", file_bench.async_read_100_files_parallel, 10))
        
    finally:
        file_bench.cleanup()
    
    # DNS benchmarks
    print("\nDNS Resolution Benchmarks:")
    print("-" * 70)
    
    dns_bench = DNSBenchmarks()
    results.append(measure_sync("sync_dns_localhost", dns_bench.sync_resolve_localhost, 1000))
    results.append(await measure_async("async_dns_localhost", dns_bench.async_resolve_localhost, 100))
    
    # Print results
    print()
    print(f"{'Benchmark':<25} {'Mean':>15} {'Ops/sec':>15}")
    print("=" * 70)
    
    for name, mean_ns in results:
        if mean_ns > 0:
            ops_per_sec = 1_000_000_000 / mean_ns
            if mean_ns >= 1_000_000_000:
                mean_str = f"{mean_ns/1_000_000_000:.3f}s"
            elif mean_ns >= 1_000_000:
                mean_str = f"{mean_ns/1_000_000:.3f}ms"
            elif mean_ns >= 1_000:
                mean_str = f"{mean_ns/1_000:.3f}µs"
            else:
                mean_str = f"{mean_ns:.0f}ns"
            print(f"{name:<25} {mean_str:>15} {ops_per_sec:>15,.0f}")
        else:
            print(f"{name:<25} {'<1ns':>15} {'∞':>15}")
    
    print()
    print("=" * 70)
    print("Notes:")
    print("- sync_* = synchronous blocking I/O")
    print("- async_* = asyncio with run_in_executor (thread pool)")
    print("- *_par = parallel execution with asyncio.gather")
    print("- *_seq = sequential async execution")
    print()
    
    return results

if __name__ == "__main__":
    asyncio.run(run_benchmarks())
