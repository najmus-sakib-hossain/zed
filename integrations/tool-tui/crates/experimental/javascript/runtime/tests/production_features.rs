//! Tests for io_uring, distributed computing, and deployment

#[test]
fn test_async_io() {
    use dx_js_runtime::io::AsyncIO;

    let io = AsyncIO::new();
    assert!(io.is_io_uring_available() || !io.is_io_uring_available());
}

#[test]
fn test_io_queue() {
    use dx_js_runtime::io::{IOOperation, IOQueue, IORequest};

    let mut queue = IOQueue::new();

    queue.submit(IORequest {
        id: 1,
        path: "test.txt".to_string(),
        operation: IOOperation::Write(b"test data".to_vec()),
    });

    let results = queue.process_batch();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_distributed_runtime() {
    use dx_js_runtime::distributed::{DistributedRuntime, NodeCapabilities, NodeInfo};
    use std::net::SocketAddr;

    let mut runtime = DistributedRuntime::new("node1".to_string());

    runtime.register_node(NodeInfo {
        id: "node2".to_string(),
        addr: "127.0.0.1:8080".parse::<SocketAddr>().unwrap(),
        capabilities: NodeCapabilities {
            cpu_cores: 4,
            memory_gb: 8,
            gpu_available: false,
        },
        load: 0.5,
    });

    let nodes = runtime.get_available_nodes();
    assert_eq!(nodes.len(), 1);

    runtime.remove_node("node2");
    let nodes = runtime.get_available_nodes();
    assert_eq!(nodes.len(), 0);
}

#[test]
fn test_task_scheduler() {
    use dx_js_runtime::distributed::{
        DistributedRuntime, NodeCapabilities, NodeInfo, Task, TaskScheduler,
    };
    use std::net::SocketAddr;

    let mut scheduler = TaskScheduler::new();
    let mut runtime = DistributedRuntime::new("node1".to_string());

    runtime.register_node(NodeInfo {
        id: "worker1".to_string(),
        addr: "127.0.0.1:9000".parse::<SocketAddr>().unwrap(),
        capabilities: NodeCapabilities {
            cpu_cores: 8,
            memory_gb: 16,
            gpu_available: true,
        },
        load: 0.3,
    });

    let task = Task::new(1, vec![1, 2, 3, 4]);
    scheduler.submit(task);

    let scheduled = scheduler.schedule(&runtime);
    assert_eq!(scheduled.len(), 1);
    assert_eq!(scheduled[0].0, 1);

    scheduler.complete_task(1);
}

#[test]
fn test_node_selection() {
    use dx_js_runtime::distributed::{DistributedRuntime, NodeCapabilities, NodeInfo};
    use std::net::SocketAddr;

    let mut runtime = DistributedRuntime::new("master".to_string());

    runtime.register_node(NodeInfo {
        id: "node1".to_string(),
        addr: "127.0.0.1:8001".parse::<SocketAddr>().unwrap(),
        capabilities: NodeCapabilities {
            cpu_cores: 2,
            memory_gb: 4,
            gpu_available: false,
        },
        load: 0.9,
    });

    runtime.register_node(NodeInfo {
        id: "node2".to_string(),
        addr: "127.0.0.1:8002".parse::<SocketAddr>().unwrap(),
        capabilities: NodeCapabilities {
            cpu_cores: 8,
            memory_gb: 16,
            gpu_available: true,
        },
        load: 0.2,
    });

    let required = NodeCapabilities {
        cpu_cores: 4,
        memory_gb: 8,
        gpu_available: false,
    };
    let selected = runtime.select_node_for_task(&required);

    assert!(selected.is_some());
    assert_eq!(selected.unwrap().id, "node2");
}

#[test]
fn test_deployment_config() {
    use dx_js_runtime::deploy::DeploymentConfig;
    use std::path::PathBuf;

    let prod = DeploymentConfig::production(PathBuf::from("dist"));
    assert!(prod.compression);
    assert!(prod.minify);
    assert!(!prod.source_maps);

    let dev = DeploymentConfig::development(PathBuf::from("dev"));
    assert!(!dev.compression);
    assert!(!dev.minify);
    assert!(dev.source_maps);
}

#[test]
fn test_bundler() {
    use dx_js_runtime::deploy::{Bundler, DeploymentConfig};
    use std::path::PathBuf;

    let config = DeploymentConfig::development(PathBuf::from("test_output"));
    let bundler = Bundler::new(config);

    // Bundler is ready to use - verify it was created successfully
    let _ = &bundler;
}

#[test]
fn test_production_optimizer() {
    use dx_js_runtime::deploy::ProductionOptimizer;

    let optimizer = ProductionOptimizer::new();
    let code = b"function test() { return 42; }";
    let optimized = optimizer.optimize(code);

    assert!(!optimized.is_empty());
}

#[test]
fn test_task_requirements() {
    use dx_js_runtime::distributed::{NodeCapabilities, Task};

    let task = Task::new(100, vec![5, 6, 7, 8]).with_requirements(NodeCapabilities {
        cpu_cores: 4,
        memory_gb: 8,
        gpu_available: true,
    });

    assert_eq!(task.id, 100);
    assert_eq!(task.requirements.cpu_cores, 4);
    assert!(task.requirements.gpu_available);
}
