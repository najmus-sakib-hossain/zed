use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use forge::chunking::cdc::{chunk_data, ChunkConfig};
use forge::store::cas::ChunkStore;
use forge::store::compression;
use tempfile::tempdir;

fn patterned_data(size: usize) -> Vec<u8> {
    (0..size)
        .map(|i| (((i % 251) as u8) ^ ((i / 97 % 37) as u8)).wrapping_add((i % 11) as u8))
        .collect()
}

fn bench_chunking(c: &mut Criterion) {
    let cfg = ChunkConfig::default();
    let mut group = c.benchmark_group("chunking");

    for size in [1024 * 1024, 10 * 1024 * 1024] {
        let data = patterned_data(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("chunk", size), &data, |b, data| {
            b.iter(|| {
                let _ = chunk_data(data, &cfg);
            })
        });
    }

    group.finish();
}

fn bench_hash(c: &mut Criterion) {
    let data = patterned_data(1024 * 1024);
    let mut group = c.benchmark_group("hashing");
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("hash_1mb", |b| b.iter(|| blake3::hash(&data)));
    group.finish();
}

fn bench_compress(c: &mut Criterion) {
    let data = patterned_data(1024 * 1024);
    let mut group = c.benchmark_group("compression");
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("compress_1mb", |b| {
        b.iter(|| compression::compress(&data, 8).unwrap())
    });
    group.finish();
}

fn bench_full_pipeline(c: &mut Criterion) {
    let data = patterned_data(1024 * 1024);
    let cfg = ChunkConfig::default();
    let dir = tempdir().unwrap();
    let store = ChunkStore::new(dir.path().join("cas"));

    let mut group = c.benchmark_group("pipeline");
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("full_pipeline_1mb", |b| {
        b.iter(|| {
            let chunks = chunk_data(&data, &cfg);
            for chunk in chunks {
                let raw = &data[chunk.offset..chunk.offset + chunk.length];
                let compressed = compression::compress(raw, 8).unwrap();
                let _ = store.store(&chunk.hash, &compressed).unwrap();
            }
        })
    });
    group.finish();
}

criterion_group!(benches, bench_chunking, bench_hash, bench_compress, bench_full_pipeline);
criterion_main!(benches);
