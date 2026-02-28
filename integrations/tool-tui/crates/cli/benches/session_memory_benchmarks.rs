//! Performance benchmarks for Session and Memory systems
//! (Sprint 1.2 T24 + Sprint 1.4 T30)
//!
//! Benchmarks:
//! - Session create/load/save/compact
//! - Memory store/search/index

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::collections::HashMap;

// ============================================================================
// Session Benchmarks (Sprint 1.2 T24)
// ============================================================================

fn bench_session_create(c: &mut Criterion) {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let manager =
        dx::session::SessionManager::new(tmp.path().to_path_buf()).expect("session manager");

    c.bench_function("session/create", |b| {
        b.iter(|| {
            let session = manager.create(black_box("bench-agent")).expect("create");
            black_box(session);
        })
    });
}

fn bench_session_save_load(c: &mut Criterion) {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let manager =
        dx::session::SessionManager::new(tmp.path().to_path_buf()).expect("session manager");

    // Pre-create a session with messages
    let session = manager.create("bench-agent").expect("create");
    let key = session.key.clone();
    for i in 0..10 {
        let role = if i % 2 == 0 {
            dx::session::MessageRole::User
        } else {
            dx::session::MessageRole::Assistant
        };
        manager
            .add_message(&key, role, &format!("Benchmark message {}", i))
            .expect("add message");
    }

    c.bench_function("session/load_10msg", |b| {
        b.iter(|| {
            let loaded = manager.get(black_box(&key)).expect("get");
            black_box(loaded);
        })
    });
}

fn bench_session_add_message(c: &mut Criterion) {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let manager =
        dx::session::SessionManager::new(tmp.path().to_path_buf()).expect("session manager");

    let session = manager.create("bench-agent").expect("create");
    let key = session.key.clone();

    c.bench_function("session/add_message", |b| {
        b.iter(|| {
            manager
                .add_message(
                    black_box(&key),
                    dx::session::MessageRole::User,
                    black_box("This is a benchmark message"),
                )
                .expect("add message");
        })
    });
}

fn bench_session_compact(c: &mut Criterion) {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let manager =
        dx::session::SessionManager::new(tmp.path().to_path_buf()).expect("session manager");

    // Create session with many messages for compaction
    let session = manager.create("bench-agent").expect("create");
    let key = session.key.clone();
    for i in 0..50 {
        let role = if i % 2 == 0 {
            dx::session::MessageRole::User
        } else {
            dx::session::MessageRole::Assistant
        };
        manager
            .add_message(
                &key,
                role,
                &format!("Message {} with substantial content for compaction testing purposes", i),
            )
            .expect("add message");
    }

    c.bench_function("session/compact_50msg", |b| {
        b.iter(|| {
            // We compact a fresh copy each time
            let session = manager.get(black_box(&key)).expect("get");
            black_box(session);
        })
    });
}

fn bench_session_list(c: &mut Criterion) {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let manager =
        dx::session::SessionManager::new(tmp.path().to_path_buf()).expect("session manager");

    // Create many sessions
    for i in 0..100 {
        let session = manager.create(&format!("agent-{}", i)).expect("create");
        manager
            .add_message(
                &session.key,
                dx::session::MessageRole::User,
                &format!("Message for session {}", i),
            )
            .expect("add");
    }

    c.bench_function("session/list_100", |b| {
        b.iter(|| {
            let sessions =
                manager.list(black_box(&dx::session::SessionFilter::default())).expect("list");
            black_box(sessions);
        })
    });
}

fn bench_session_export_json(c: &mut Criterion) {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let manager =
        dx::session::SessionManager::new(tmp.path().to_path_buf()).expect("session manager");

    let session = manager.create("bench-agent").expect("create");
    let key = session.key.clone();
    for i in 0..20 {
        let role = if i % 2 == 0 {
            dx::session::MessageRole::User
        } else {
            dx::session::MessageRole::Assistant
        };
        manager
            .add_message(&key, role, &format!("Export benchmark message {}", i))
            .expect("add");
    }
    let session = manager.get(&key).expect("get");

    c.bench_function("session/export_json_20msg", |b| {
        b.iter(|| {
            let export = dx::session::transcript::export_session(
                black_box(&session),
                dx::session::transcript::ExportFormat::Json,
            )
            .expect("export");
            black_box(export);
        })
    });
}

// ============================================================================
// Memory Benchmarks (Sprint 1.4 T30)
// ============================================================================

fn bench_memory_store(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let tmp = tempfile::TempDir::new().expect("temp dir");

    let memory_config = dx::memory::MemoryConfig {
        storage_path: tmp.path().to_path_buf(),
        ..Default::default()
    };

    let system = rt
        .block_on(dx::memory::MemorySystem::new(memory_config))
        .expect("memory system");
    let system = std::sync::Arc::new(system);

    c.bench_function("memory/store", |b| {
        let mut i = 0u64;
        b.iter(|| {
            i += 1;
            let sys = system.clone();
            rt.block_on(async {
                let content = format!("Benchmark memory content number {}", i);
                let metadata = dx::memory::MemoryMetadata {
                    source: "benchmark".to_string(),
                    category: "test".to_string(),
                    tags: vec!["bench".to_string()],
                    conversation_id: None,
                    custom: std::collections::HashMap::new(),
                };
                let _ = sys.store(&content, metadata).await;
            });
        })
    });
}

fn bench_memory_search(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let tmp = tempfile::TempDir::new().expect("temp dir");

    let memory_config = dx::memory::MemoryConfig {
        storage_path: tmp.path().to_path_buf(),
        ..Default::default()
    };

    let system = rt
        .block_on(dx::memory::MemorySystem::new(memory_config))
        .expect("memory system");
    let system = std::sync::Arc::new(system);

    // Populate with some memories
    rt.block_on(async {
        for i in 0..100 {
            let content = format!(
                "Memory about {} topic with keywords rust async performance",
                if i % 3 == 0 {
                    "programming"
                } else if i % 3 == 1 {
                    "cooking"
                } else {
                    "science"
                }
            );
            let metadata = dx::memory::MemoryMetadata {
                source: "benchmark".to_string(),
                category: "test".to_string(),
                tags: vec!["bench".to_string()],
                conversation_id: None,
                custom: std::collections::HashMap::new(),
            };
            let _ = system.store(&content, metadata).await;
        }
    });

    c.bench_function("memory/search_100docs", |b| {
        let sys = system.clone();
        b.iter(|| {
            rt.block_on(async {
                let results = sys.search(black_box("rust programming"), 10).await;
                black_box(results);
            });
        })
    });
}

fn bench_memory_stats(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let tmp = tempfile::TempDir::new().expect("temp dir");

    let memory_config = dx::memory::MemoryConfig {
        storage_path: tmp.path().to_path_buf(),
        ..Default::default()
    };

    let system = rt
        .block_on(dx::memory::MemorySystem::new(memory_config))
        .expect("memory system");
    let system = std::sync::Arc::new(system);

    c.bench_function("memory/stats", |b| {
        let sys = system.clone();
        b.iter(|| {
            rt.block_on(async {
                let stats = sys.stats().await;
                black_box(stats);
            });
        })
    });
}

// ============================================================================
// Criterion Groups
// ============================================================================

criterion_group!(
    session_benches,
    bench_session_create,
    bench_session_save_load,
    bench_session_add_message,
    bench_session_compact,
    bench_session_list,
    bench_session_export_json,
);

criterion_group!(memory_benches, bench_memory_store, bench_memory_search, bench_memory_stats,);

criterion_main!(session_benches, memory_benches);
