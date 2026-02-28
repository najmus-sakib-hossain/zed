use std::time::Duration;

use dx::registry::{ConfigWatcher, WatchEvent};

#[tokio::test]
async fn test_config_watcher_triggers_reload() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut watcher = ConfigWatcher::new();
    watcher.watch(temp_dir.path().to_path_buf()).unwrap();

    let mut rx = watcher.subscribe();

    let test_file = temp_dir.path().join("dx-config.toml");
    std::fs::write(&test_file, "version = 1").unwrap();

    let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for watcher event")
        .expect("watcher channel closed");

    match event {
        WatchEvent::Created(_) | WatchEvent::Modified(_) | WatchEvent::Batch(_) => {}
        other => panic!("unexpected event: {other:?}"),
    }
}
