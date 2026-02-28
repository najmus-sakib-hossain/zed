//! Benchmarks for the module system
//!
//! Tests performance of module installation, dependency resolution, and namespacing.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::fs;
use tempfile::TempDir;

use driven::modules::{Module, ModuleDependency, ModuleManager};

/// Create a test module directory with manifest
fn create_test_module(dir: &std::path::Path, id: &str, name: &str, version: &str) {
    fs::create_dir_all(dir).unwrap();
    let manifest = format!(
        r#"# Test Module
id|{}
nm|{}
v|{}
desc|A test module for benchmarking
agent.0|test-agent-1
agent.1|test-agent-2
workflow.0|test-workflow-1
workflow.1|test-workflow-2
template.0|test-template
"#,
        id, name, version
    );
    fs::write(dir.join("module.dx"), manifest).unwrap();
}

fn bench_module_creation(c: &mut Criterion) {
    c.bench_function("module_creation", |b| {
        b.iter(|| {
            black_box(
                Module::new("test-module", "Test Module", "1.0.0")
                    .with_description("A test module")
                    .with_agent("agent1")
                    .with_agent("agent2")
                    .with_workflow("workflow1")
                    .with_template("template1"),
            )
        })
    });
}

fn bench_version_satisfaction(c: &mut Criterion) {
    let module = Module::new("test", "Test", "1.2.3");

    let mut group = c.benchmark_group("version_satisfaction");

    group.bench_function("exact", |b| b.iter(|| black_box(module.satisfies_version("1.2.3"))));

    group.bench_function("wildcard", |b| b.iter(|| black_box(module.satisfies_version("*"))));

    group.bench_function("caret", |b| b.iter(|| black_box(module.satisfies_version("^1"))));

    group.bench_function("tilde", |b| b.iter(|| black_box(module.satisfies_version("~1.2"))));

    group.finish();
}

fn bench_namespacing(c: &mut Criterion) {
    let module = Module::new("my-module", "My Module", "1.0.0")
        .with_agent("agent1")
        .with_workflow("workflow1");

    let mut group = c.benchmark_group("namespacing");

    group.bench_function("agent", |b| b.iter(|| black_box(module.namespaced_agent("agent1"))));

    group.bench_function("workflow", |b| {
        b.iter(|| black_box(module.namespaced_workflow("workflow1")))
    });

    group.finish();
}

fn bench_module_installation(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let module_path = temp_dir.path().join("test-module");
    create_test_module(&module_path, "test-module", "Test Module", "1.0.0");

    c.bench_function("module_installation", |b| {
        b.iter_with_setup(
            || {
                let registry_path =
                    temp_dir.path().join(format!("registry-{}", rand::random::<u64>()));
                ModuleManager::new(&registry_path)
            },
            |mut manager| black_box(manager.install(&module_path).unwrap()),
        )
    });
}

fn bench_dependency_resolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("dependency_resolution");

    for depth in [1, 5, 10].iter() {
        group.bench_with_input(BenchmarkId::new("chain_depth", depth), depth, |b, &depth| {
            let temp_dir = TempDir::new().unwrap();
            let registry_path = temp_dir.path().join("registry");
            let mut manager = ModuleManager::new(&registry_path);

            // Create a chain of dependencies
            for i in 0..depth {
                let module_path = temp_dir.path().join(format!("module-{}", i));
                fs::create_dir_all(&module_path).unwrap();

                let dep_line = if i > 0 {
                    format!("dep.module-{}|*\n", i - 1)
                } else {
                    String::new()
                };

                let manifest = format!("id|module-{}\nnm|Module {}\nv|1.0.0\n{}", i, i, dep_line);
                fs::write(module_path.join("module.dx"), manifest).unwrap();
                manager.install(&module_path).unwrap();
            }

            // Create the final module that depends on the last in chain
            let final_module = Module::new("final", "Final", "1.0.0")
                .with_dependency(ModuleDependency::new(format!("module-{}", depth - 1), "*"));

            b.iter(|| black_box(manager.resolve_dependencies(&final_module).unwrap()))
        });
    }

    group.finish();
}

fn bench_get_all_resources(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let registry_path = temp_dir.path().join("registry");
    let mut manager = ModuleManager::new(&registry_path);

    // Install 10 modules with resources
    for i in 0..10 {
        let module_path = temp_dir.path().join(format!("module-{}", i));
        create_test_module(
            &module_path,
            &format!("module-{}", i),
            &format!("Module {}", i),
            "1.0.0",
        );
        manager.install(&module_path).unwrap();
    }

    let mut group = c.benchmark_group("get_all_resources");

    group.bench_function("agents", |b| b.iter(|| black_box(manager.get_all_agents())));

    group.bench_function("workflows", |b| b.iter(|| black_box(manager.get_all_workflows())));

    group.bench_function("templates", |b| b.iter(|| black_box(manager.get_all_templates())));

    group.finish();
}

criterion_group!(
    benches,
    bench_module_creation,
    bench_version_satisfaction,
    bench_namespacing,
    bench_module_installation,
    bench_dependency_resolution,
    bench_get_all_resources,
);

criterion_main!(benches);
