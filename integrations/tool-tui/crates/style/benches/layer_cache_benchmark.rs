use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dx_style::core::{LayerCache, layer_gen};

fn benchmark_layer_generation(c: &mut Criterion) {
    let classes: Vec<String> = vec![
        "flex".to_string(),
        "p-4".to_string(),
        "bg-blue-500".to_string(),
        "text-white".to_string(),
        "rounded".to_string(),
    ];

    c.bench_function("layer_gen_cold_cache", |b| {
        b.iter(|| {
            let mut cache = LayerCache::default();
            layer_gen::generate_layers_cached(black_box(&classes), &mut cache, true)
        })
    });

    c.bench_function("layer_gen_warm_cache_no_colors", |b| {
        let mut cache = LayerCache::default();
        // Warm up cache
        layer_gen::generate_layers_cached(&classes, &mut cache, true);

        // Test with non-color classes (should reuse cache)
        let non_color_classes: Vec<String> = vec![
            "flex".to_string(),
            "p-4".to_string(),
            "rounded".to_string(),
            "m-2".to_string(),
        ];

        b.iter(|| {
            layer_gen::generate_layers_cached(black_box(&non_color_classes), &mut cache, false)
        })
    });

    c.bench_function("layer_gen_warm_cache_with_colors", |b| {
        let mut cache = LayerCache::default();
        // Warm up cache
        layer_gen::generate_layers_cached(&classes, &mut cache, true);

        // Test with new color classes (should regenerate theme)
        let new_color_classes: Vec<String> = vec![
            "flex".to_string(),
            "p-4".to_string(),
            "bg-red-500".to_string(), // New color
            "text-white".to_string(),
        ];

        b.iter(|| {
            layer_gen::generate_layers_cached(black_box(&new_color_classes), &mut cache, false)
        })
    });
}

criterion_group!(benches, benchmark_layer_generation);
criterion_main!(benches);
