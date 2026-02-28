//! Plugin System Benchmarks
//!
//! Measures performance of key plugin operations:
//! - Registry lookup
//! - Sandbox capability checks
//! - KV store operations (host functions)
//! - Signature verification
//! - Resource tracker alloc/dealloc

use std::time::Instant;

fn bench<F: Fn()>(label: &str, iterations: usize, f: F) {
    // Warm up
    for _ in 0..100 {
        f();
    }

    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed();

    let per_op = elapsed / iterations as u32;
    println!("{:<40} {:>10} ops in {:>10?}  ({:>8?}/op)", label, iterations, elapsed, per_op);
}

fn main() {
    println!("=== DX Plugin System Benchmarks ===\n");

    // -----------------------------------------------------------------------
    // Sandbox capability checks
    // -----------------------------------------------------------------------
    {
        use dx_cli::plugin::sandbox::{PluginSandbox, SandboxConfig};
        use dx_cli::plugin::traits::Capability;

        let config = SandboxConfig::default().with_capability(Capability::Network);
        let mut sandbox = PluginSandbox::new(config);

        bench("sandbox: capability check (allowed)", 1_000_000, || {
            let _ = sandbox.check_capability(Capability::Network);
        });

        bench("sandbox: capability check (denied)", 1_000_000, || {
            let _ = sandbox.check_capability(Capability::Shell);
        });
    }

    // -----------------------------------------------------------------------
    // Host function KV store
    // -----------------------------------------------------------------------
    {
        use dx_cli::plugin::host_functions::{HostState, host_kv_delete, host_kv_get, host_kv_set};

        let state = HostState::new();
        host_kv_set(&state, "bench-key", b"bench-value");

        bench("host_kv_set", 1_000_000, || {
            host_kv_set(&state, "bench-key", b"bench-value");
        });

        bench("host_kv_get (hit)", 1_000_000, || {
            let _ = host_kv_get(&state, "bench-key");
        });

        bench("host_kv_get (miss)", 1_000_000, || {
            let _ = host_kv_get(&state, "nonexistent");
        });
    }

    // -----------------------------------------------------------------------
    // Resource tracker
    // -----------------------------------------------------------------------
    {
        use dx_cli::plugin::resource_limiter::{ResourceLimits, ResourceTracker};

        let limits = ResourceLimits {
            max_memory_bytes: usize::MAX,
            max_fuel: u64::MAX,
            ..Default::default()
        };

        bench("resource_tracker: alloc_memory", 1_000_000, || {
            let tracker = ResourceTracker::new(limits.clone());
            let _ = tracker.alloc_memory(1024);
        });

        bench("resource_tracker: consume_fuel", 1_000_000, || {
            let tracker = ResourceTracker::new(limits.clone());
            let _ = tracker.consume_fuel(100);
        });

        bench("resource_tracker: check_timeout", 1_000_000, || {
            let tracker = ResourceTracker::new(limits.clone());
            let _ = tracker.check_timeout();
        });

        bench("resource_tracker: snapshot", 1_000_000, || {
            let tracker = ResourceTracker::new(limits.clone());
            let _ = tracker.snapshot();
        });
    }

    // -----------------------------------------------------------------------
    // Signature
    // -----------------------------------------------------------------------
    {
        use dx_cli::plugin::signature::{
            SignatureVerifier, TrustedKey, generate_keypair, sign_plugin,
        };
        use ed25519_dalek::SigningKey;

        let (sk_bytes, vk_bytes) = generate_keypair();
        let sk = SigningKey::from_bytes(&sk_bytes);
        let data = vec![0u8; 1024]; // 1 KB fake binary

        bench("sign_plugin (1KB)", 10_000, || {
            let _ = sign_plugin(&data, &sk);
        });

        let sig = sign_plugin(&data, &sk);
        let trusted = TrustedKey::from_bytes("bench", &vk_bytes).unwrap();
        let mut verifier = SignatureVerifier::new();
        verifier.add_trusted_key(trusted);

        bench("verify_bytes (1KB)", 10_000, || {
            let _ = verifier.verify_bytes(&data, &sig);
        });

        // Larger binary
        let big_data = vec![0u8; 1_000_000]; // 1 MB
        let big_sig = sign_plugin(&big_data, &sk);

        bench("sign_plugin (1MB)", 100, || {
            let _ = sign_plugin(&big_data, &sk);
        });

        bench("verify_bytes (1MB)", 100, || {
            let _ = verifier.verify_bytes(&big_data, &big_sig);
        });
    }

    // -----------------------------------------------------------------------
    // Host function logging
    // -----------------------------------------------------------------------
    {
        use dx_cli::plugin::host_functions::{HostState, host_log};

        let state = HostState::new();

        bench("host_log", 100_000, || {
            host_log(&state, 2, "benchmark log message");
        });
    }

    println!("\n=== Benchmarks complete ===");
}
