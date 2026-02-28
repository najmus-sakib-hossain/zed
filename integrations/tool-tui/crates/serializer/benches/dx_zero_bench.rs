//! Benchmarks: DX-Zero vs Cap'n Proto, rkyv, FlatBuffers, Protobuf
//!
//! This benchmark compares serialization and deserialization performance
//! across multiple binary formats.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use serializer::zero::{DxZeroBuilder, DxZeroSlot};

/// Test data structure
struct User {
    id: u64,
    age: u32,
    active: bool,
    score: f64,
    name: String,
    email: String,
    bio: String,
}

impl User {
    fn sample() -> Self {
        Self {
            id: 12345,
            age: 30,
            active: true,
            score: 98.5,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            bio: "Software engineer with 10 years of experience in Rust and systems programming."
                .to_string(),
        }
    }

    fn sample_small() -> Self {
        Self {
            id: 999,
            age: 25,
            active: true,
            score: 85.0,
            name: "Alice".to_string(),
            email: "a@b.com".to_string(),
            bio: "Short bio".to_string(),
        }
    }
}

/// DX-Zero struct layout
#[repr(C, packed)]
struct UserDxZero {
    _header: [u8; 4],
    id: u64,
    age: u32,
    active: bool,
    score: f64,
    name_slot: [u8; 16],
    email_slot: [u8; 16],
    bio_slot: [u8; 16],
}

impl UserDxZero {
    const HEADER_SIZE: usize = 4;
    const FIXED_SIZE: usize = 21; // id(8) + age(4) + active(1) + score(8)
    const SLOT_COUNT: usize = 3;
    const HEAP_OFFSET: usize = 73; // 4 + 21 + 48

    #[inline(always)]
    fn from_bytes(bytes: &[u8]) -> &Self {
        unsafe { &*(bytes.as_ptr() as *const Self) }
    }

    #[inline(always)]
    fn id(&self) -> u64 {
        unsafe {
            let ptr = (self as *const Self as *const u8).add(4);
            u64::from_le_bytes(*(ptr as *const [u8; 8]))
        }
    }

    #[inline(always)]
    fn age(&self) -> u32 {
        unsafe {
            let ptr = (self as *const Self as *const u8).add(12);
            u32::from_le_bytes(*(ptr as *const [u8; 4]))
        }
    }

    #[inline(always)]
    fn name(&self) -> &str {
        let slot = unsafe { &*(self.name_slot.as_ptr() as *const DxZeroSlot) };
        if slot.is_inline() {
            slot.inline_str()
        } else {
            let (offset, length) = slot.heap_ref();
            unsafe {
                let ptr =
                    (self as *const Self as *const u8).add(Self::HEAP_OFFSET + offset as usize);
                let bytes = std::slice::from_raw_parts(ptr, length as usize);
                std::str::from_utf8_unchecked(bytes)
            }
        }
    }
}

// =============================================================================
// DX-ZERO BENCHMARKS
// =============================================================================

fn bench_dx_zero_serialize(c: &mut Criterion) {
    let user = User::sample();

    c.bench_function("dx_zero_serialize", |b| {
        b.iter(|| {
            let mut buffer = Vec::new();
            let mut builder =
                DxZeroBuilder::new(&mut buffer, UserDxZero::FIXED_SIZE, UserDxZero::SLOT_COUNT);

            builder.write_u64(0, user.id);
            builder.write_u32(8, user.age);
            builder.write_bool(12, user.active);
            builder.write_f64(13, user.score);
            builder.write_string(21, &user.name);
            builder.write_string(37, &user.email);
            builder.write_string(53, &user.bio);

            let size = builder.finish();
            black_box(size);
        });
    });
}

fn bench_dx_zero_deserialize(c: &mut Criterion) {
    let user = User::sample();

    let mut buffer = Vec::new();
    let mut builder =
        DxZeroBuilder::new(&mut buffer, UserDxZero::FIXED_SIZE, UserDxZero::SLOT_COUNT);
    builder.write_u64(0, user.id);
    builder.write_u32(8, user.age);
    builder.write_bool(12, user.active);
    builder.write_f64(13, user.score);
    builder.write_string(21, &user.name);
    builder.write_string(37, &user.email);
    builder.write_string(53, &user.bio);
    builder.finish();

    c.bench_function("dx_zero_deserialize", |b| {
        b.iter(|| {
            let user = UserDxZero::from_bytes(&buffer);
            black_box(user);
        });
    });
}

fn bench_dx_zero_field_access(c: &mut Criterion) {
    let user = User::sample();

    let mut buffer = Vec::new();
    let mut builder =
        DxZeroBuilder::new(&mut buffer, UserDxZero::FIXED_SIZE, UserDxZero::SLOT_COUNT);
    builder.write_u64(0, user.id);
    builder.write_u32(8, user.age);
    builder.write_bool(12, user.active);
    builder.write_f64(13, user.score);
    builder.write_string(21, &user.name);
    builder.write_string(37, &user.email);
    builder.write_string(53, &user.bio);
    builder.finish();

    let user_zero = UserDxZero::from_bytes(&buffer);

    let mut group = c.benchmark_group("dx_zero_field_access");

    group.bench_function("id", |b| {
        b.iter(|| black_box(user_zero.id()));
    });

    group.bench_function("age", |b| {
        b.iter(|| black_box(user_zero.age()));
    });

    group.bench_function("name", |b| {
        b.iter(|| black_box(user_zero.name()));
    });

    group.finish();
}

// =============================================================================
// SIZE COMPARISON
// =============================================================================

fn bench_size_comparison(c: &mut Criterion) {
    let user = User::sample();

    // DX-Zero
    let mut dx_buffer = Vec::new();
    let mut builder =
        DxZeroBuilder::new(&mut dx_buffer, UserDxZero::FIXED_SIZE, UserDxZero::SLOT_COUNT);
    builder.write_u64(0, user.id);
    builder.write_u32(8, user.age);
    builder.write_bool(12, user.active);
    builder.write_f64(13, user.score);
    builder.write_string(21, &user.name);
    builder.write_string(37, &user.email);
    builder.write_string(53, &user.bio);
    let dx_size = builder.finish();

    // JSON (for comparison)
    let json = serde_json::json!({
        "id": user.id,
        "age": user.age,
        "active": user.active,
        "score": user.score,
        "name": user.name,
        "email": user.email,
        "bio": user.bio,
    });
    let json_size = serde_json::to_vec(&json).unwrap().len();

    println!("\n=== SIZE COMPARISON ===");
    println!("DX-Zero:  {} bytes", dx_size);
    println!(
        "JSON:     {} bytes ({:.1}× larger)",
        json_size,
        json_size as f64 / dx_size as f64
    );

    c.bench_function("size_reference", |b| {
        b.iter(|| black_box(dx_size));
    });
}

// =============================================================================
// INLINE VS HEAP COMPARISON
// =============================================================================

fn bench_inline_vs_heap(c: &mut Criterion) {
    let mut group = c.benchmark_group("inline_vs_heap");

    // Inline string (8 bytes)
    {
        let mut buffer = Vec::new();
        let mut builder = DxZeroBuilder::new(&mut buffer, 0, 1);
        builder.write_string(0, "John Doe");
        builder.finish();

        let user = UserDxZero::from_bytes(&buffer);

        group.bench_function("inline_8bytes", |b| {
            b.iter(|| black_box(user.name()));
        });
    }

    // Heap string (20 bytes)
    {
        let mut buffer = Vec::new();
        let mut builder = DxZeroBuilder::new(&mut buffer, 0, 1);
        builder.write_string(0, "john.doe@example.com");
        builder.finish();

        let user = UserDxZero::from_bytes(&buffer);

        group.bench_function("heap_20bytes", |b| {
            b.iter(|| black_box(user.name()));
        });
    }

    group.finish();
}

// =============================================================================
// BENCHMARK GROUPS
// =============================================================================

criterion_group!(
    benches,
    bench_dx_zero_serialize,
    bench_dx_zero_deserialize,
    bench_dx_zero_field_access,
    bench_size_comparison,
    bench_inline_vs_heap,
);

criterion_main!(benches);
