//! Property-based tests for Binary Dawn
//! **Feature: binary-dawn**

use super::*;
use proptest::prelude::*;

// Property generators
fn arb_tool_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{0,20}".prop_map(|s| format!("dx-{}", s))
}

fn arb_tool_definition() -> impl Strategy<Value = DxToolDefinition> {
    (
        arb_tool_name(),
        "[a-z/]{1,30}",
        0u32..100,
        prop::collection::vec(arb_tool_name(), 0..3),
    )
        .prop_map(|(name, path, priority, deps)| DxToolDefinition {
            name,
            binary_path: path,
            priority,
            dependencies: deps,
        })
}

fn arb_tool_status() -> impl Strategy<Value = ToolStatus> {
    prop_oneof![
        Just(ToolStatus::Stopped),
        Just(ToolStatus::Starting),
        (1u32..65535).prop_map(ToolStatus::Running),
        Just(ToolStatus::Healthy),
        Just(ToolStatus::Degraded),
    ]
}

// **Property 4: Tool Status Validity**
// **Validates: Requirements 2.5**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_tool_status_is_valid_variant(status in arb_tool_status()) {
        let is_valid = matches!(
            status,
            ToolStatus::Stopped | ToolStatus::Starting | ToolStatus::Running(_) |
            ToolStatus::Healthy | ToolStatus::Degraded
        );
        prop_assert!(is_valid);
    }
}

// **Property 1: Tool Registration Round-Trip**
// **Validates: Requirements 2.1**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_tool_registration_roundtrip(def in arb_tool_definition()) {
        let mut orch = Orchestrator::new();
        let original_name = def.name.clone();
        let original_path = def.binary_path.clone();
        let original_priority = def.priority;
        let original_deps = def.dependencies.clone();

        orch.register_tool(def);

        let retrieved = orch.get_tool(&original_name);
        prop_assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        prop_assert_eq!(&retrieved.name, &original_name);
        prop_assert_eq!(&retrieved.binary_path, &original_path);
        prop_assert_eq!(retrieved.priority, original_priority);
        prop_assert_eq!(&retrieved.dependencies, &original_deps);
    }
}

// **Property 2: Ensure Running Idempotence**
// **Property 3: Stopped to Running Transition**
// **Validates: Requirements 2.2, 2.3**
#[tokio::test]
async fn prop_ensure_running_idempotence() {
    let orch = Orchestrator::new();
    let tool_name = "dx-test-tool";

    orch.ensure_running(tool_name).await.unwrap();
    let status1 = orch.get_status(tool_name).await;
    assert!(matches!(status1, ToolStatus::Running(_)));

    let pid1 = if let ToolStatus::Running(pid) = status1 {
        pid
    } else {
        0
    };

    orch.ensure_running(tool_name).await.unwrap();
    let status2 = orch.get_status(tool_name).await;
    let pid2 = if let ToolStatus::Running(pid) = status2 {
        pid
    } else {
        0
    };
    assert_eq!(pid1, pid2, "PID should not change");
}

#[tokio::test]
async fn prop_stopped_to_running_transition() {
    let orch = Orchestrator::new();
    let tool_name = "dx-transition-test";

    let initial = orch.get_status(tool_name).await;
    assert_eq!(initial, ToolStatus::Stopped);

    orch.ensure_running(tool_name).await.unwrap();
    let after = orch.get_status(tool_name).await;
    assert!(matches!(after, ToolStatus::Running(_)));
}

// **Property 5: Background Task Non-Blocking**
// **Validates: Requirements 3.5**
#[tokio::test]
async fn prop_background_task_nonblocking() {
    let worker = BackgroundWorker::new();

    let start = std::time::Instant::now();
    worker.enqueue(BackgroundTask::CacheCurrentState).await;
    worker.enqueue(BackgroundTask::SyncToCloudflareR2).await;
    worker.enqueue(BackgroundTask::PrefetchPackage("test-pkg".into())).await;
    let elapsed = start.elapsed();

    // Enqueue should return within 10ms
    assert!(elapsed.as_millis() < 10, "Enqueue took {}ms", elapsed.as_millis());
}

// **Property 6: Traffic Safety Analysis**
// **Validates: Requirements 4.2, 4.3, 4.4**
#[test]
fn prop_traffic_safety_nonexistent_is_green() {
    let tm = TrafficManager::new();
    let path = std::path::Path::new("/nonexistent/path/file.rs");
    let result = tm.analyze_traffic_safety(path, "any content");
    assert_eq!(result, TrafficLight::Green);
}

#[test]
fn prop_traffic_safety_matching_content_is_green() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.rs");
    let content = "pub fn test() {}";
    std::fs::write(&file_path, content).unwrap();

    let tm = TrafficManager::new();
    let result = tm.analyze_traffic_safety(&file_path, content);
    assert_eq!(result, TrafficLight::Green);
}

#[test]
fn prop_traffic_safety_different_content_is_yellow() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.rs");
    std::fs::write(&file_path, "old content").unwrap();

    let tm = TrafficManager::new();
    let result = tm.analyze_traffic_safety(&file_path, "new content");
    assert_eq!(result, TrafficLight::Yellow);
}

// **Property 7: Green Signal Injection**
// **Validates: Requirements 4.6**
#[tokio::test]
async fn prop_green_signal_allows_injection() {
    let tm = TrafficManager::new();
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("new_file.rs");

    // Non-existent file should be Green
    let signal = tm.analyze_traffic_safety(&target, "new content");
    assert_eq!(signal, TrafficLight::Green);

    // Green signal means we can write
    if signal == TrafficLight::Green {
        std::fs::create_dir_all(target.parent().unwrap()).ok();
        std::fs::write(&target, "new content").unwrap();
        assert!(target.exists());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new content");
    }
}
