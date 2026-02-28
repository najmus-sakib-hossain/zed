//! Performance benchmarks for dx-js-runtime

use std::time::Instant;

fn benchmark<F: Fn()>(name: &str, iterations: usize, f: F) -> f64 {
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed().as_secs_f64();
    let ops_per_sec = iterations as f64 / elapsed;
    println!(
        "{}: {:.2} ops/sec ({:.2}ms per op)",
        name,
        ops_per_sec,
        elapsed * 1000.0 / iterations as f64
    );
    ops_per_sec
}

fn main() {
    println!("=== Dx JS Runtime Benchmarks ===\n");

    // RegExp benchmark
    use dx_js_runtime::runtime::regexp::RegExp;
    benchmark("RegExp.test()", 100_000, || {
        let re = RegExp::new("test", "i").unwrap();
        assert!(re.test("Testing 123"));
    });

    benchmark("RegExp.replace()", 50_000, || {
        let re = RegExp::new("world", "g").unwrap();
        let _ = re.replace("hello world world", "rust");
    });

    // DateTime benchmark
    use dx_js_runtime::runtime::datetime::DateTime;
    benchmark("DateTime.now()", 1_000_000, || {
        let _ = DateTime::now();
    });

    benchmark("DateTime.to_iso_string()", 100_000, || {
        let dt = DateTime::now();
        let _ = dt.to_iso_string();
    });

    // URL parsing benchmark
    use dx_js_runtime::runtime::url::URL;
    benchmark("URL.parse()", 100_000, || {
        let _ = URL::new("https://example.com:8080/path?q=test#hash").unwrap();
    });

    benchmark("URLSearchParams.get()", 100_000, || {
        let url = URL::new("https://api.com/search?q=rust&page=1").unwrap();
        let params = url.search_params();
        assert_eq!(params.get("q"), Some(&"rust".to_string()));
    });

    // Streams benchmark
    use dx_js_runtime::runtime::streams::*;
    benchmark("Stream.write()", 50_000, || {
        let mut stream = WritableStream::new();
        stream.write(b"Hello World").unwrap();
    });

    benchmark("Stream.pipe()", 10_000, || {
        let mut readable = ReadableStream::new();
        readable.push(b"Test data");
        readable.push_end();
        let mut writable = WritableStream::new();
        readable.pipe(&mut writable).unwrap();
    });

    // EventEmitter benchmark
    use dx_js_runtime::runtime::events::EventEmitter;
    benchmark("EventEmitter.emit()", 100_000, || {
        let mut emitter = EventEmitter::new();
        emitter.on("test", |_data| {});
        emitter.emit("test", b"data");
    });

    // Crypto benchmark
    use dx_js_runtime::runtime::crypto::CryptoModule;
    let crypto = CryptoModule::new();
    benchmark("crypto.randomUUID()", 50_000, || {
        let _ = crypto.random_uuid();
    });

    benchmark("crypto.randomBytes(32)", 50_000, || {
        let _ = crypto.random_bytes(32);
    });

    // Array operations (via builtins)
    benchmark("Array operations", 10_000, || {
        let data: Vec<i32> = (0..100).collect();
        let _: Vec<i32> = data.iter().map(|x| x * 2).filter(|x| x % 3 == 0).collect();
    });

    println!("\n=== Benchmark Complete ===");
}
