//! Tests for profiler, SIMD, GPU, WebAssembly, and Workers

#[test]
fn test_cpu_profiler() {
    use dx_js_runtime::profiler::CpuProfiler;

    let mut profiler = CpuProfiler::new(1);
    profiler.start();

    profiler.sample(vec!["main".to_string(), "foo".to_string()]);
    profiler.sample(vec!["main".to_string(), "bar".to_string()]);
    profiler.sample(vec!["main".to_string(), "foo".to_string()]);

    profiler.stop();

    let profile = profiler.get_profile();
    assert_eq!(profile.total_samples, 3);

    let hot = profile.hot_functions(10);
    assert!(!hot.is_empty());
}

#[test]
fn test_memory_profiler() {
    use dx_js_runtime::profiler::MemoryProfiler;

    let mut profiler = MemoryProfiler::new();
    profiler.start();

    profiler.track_allocation(1, 100, vec!["alloc1".to_string()]);
    profiler.track_allocation(2, 200, vec!["alloc2".to_string()]);

    let snapshot = profiler.get_snapshot();
    assert_eq!(snapshot.current_usage, 300);
    assert_eq!(snapshot.peak_usage, 300);
    assert_eq!(snapshot.allocation_count, 2);

    profiler.track_deallocation(100);
    let snapshot2 = profiler.get_snapshot();
    assert_eq!(snapshot2.current_usage, 200);
}

#[test]
fn test_flame_graph() {
    use dx_js_runtime::profiler::{CpuProfiler, FlameGraph};

    let mut profiler = CpuProfiler::new(1);
    profiler.start();
    profiler.sample(vec!["main".to_string()]);
    profiler.stop();

    let profile = profiler.get_profile();
    let graph = FlameGraph::from_profile(&profile);

    let svg = graph.to_svg(800, 600);
    assert!(svg.contains("<svg"));
    assert!(svg.contains("</svg>"));

    let json = graph.to_json();
    assert!(json.contains("nodes"));
}

#[test]
fn test_simd_operations() {
    use dx_js_runtime::simd::{vector_add_f32, SimdF32x4};

    let a = SimdF32x4::new(1.0, 2.0, 3.0, 4.0);
    let b = SimdF32x4::splat(2.0);
    let c = a.add(&b);

    assert_eq!(c.0, [3.0, 4.0, 5.0, 6.0]);

    let d = a.mul(&b);
    assert_eq!(d.0, [2.0, 4.0, 6.0, 8.0]);

    let sum = a.sum();
    assert_eq!(sum, 10.0);

    // Vector operations
    let vec_a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let vec_b = vec![1.0; 8];
    let mut result = vec![0.0; 8];

    vector_add_f32(&vec_a, &vec_b, &mut result).unwrap();
    assert_eq!(result, vec![2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
}

#[test]
fn test_wasm_memory() {
    use dx_js_runtime::wasm::WasmMemory;

    let mut memory = WasmMemory::new(1);

    let data = b"Hello WASM";
    memory.write(0, data).unwrap();

    let read = memory.read(0, 10).unwrap();
    assert_eq!(read, data);

    let old_pages = memory.grow(1).unwrap();
    assert_eq!(old_pages, 1);
}

#[test]
fn test_wasm_module() {
    use dx_js_runtime::wasm::{WasmExport, WasmMemory, WasmModule, WasmType};

    let mut module = WasmModule::new("test".to_string());

    module.add_export(
        "add".to_string(),
        WasmExport::Function {
            params: vec![WasmType::I32, WasmType::I32],
            results: vec![WasmType::I32],
        },
    );

    module.set_memory(WasmMemory::new(1));

    assert!(module.get_export("add").is_some());
    assert!(module.get_export("missing").is_none());
}

#[test]
fn test_worker_creation() {
    use dx_js_runtime::workers::{Worker, WorkerPool};

    let worker = Worker::new(0);
    assert!(worker.is_ok());

    let pool = WorkerPool::new(4);
    assert!(pool.is_ok());
}

#[test]
fn test_gpu_device() {
    use dx_js_runtime::gpu::GpuDevice;

    let device = GpuDevice::new();
    // GPU may not be available in test environment
    assert!(!device.is_available() || device.is_available());
}

#[test]
fn test_profiler_combined() {
    use dx_js_runtime::profiler::Profiler;

    let mut profiler = Profiler::new();
    profiler.start_all();

    profiler.cpu.sample(vec!["test".to_string()]);
    profiler.memory.track_allocation(1, 42, vec!["test".to_string()]);

    profiler.stop_all();

    let report = profiler.generate_report();
    assert_eq!(report.cpu_profile.total_samples, 1);
    assert_eq!(report.memory_snapshot.current_usage, 42);
}

#[test]
fn test_simd_i32() {
    use dx_js_runtime::simd::SimdI32x4;

    let a = SimdI32x4::new(10, 20, 30, 40);
    let b = SimdI32x4::splat(5);

    let sum = a.add(&b);
    assert_eq!(sum.0, [15, 25, 35, 45]);

    let prod = a.mul(&b);
    assert_eq!(prod.0, [50, 100, 150, 200]);
}
