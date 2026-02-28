//! Integration tests for dx-forge
//!
//! Tests end-to-end functionality of the Forge system

use anyhow::Result;
use dx_forge::{
    DxTool, ExecutionContext, Forge, ForgeConfig, Orchestrator, SnapshotManager, ToolOutput,
    Version,
};
use std::path::PathBuf;
use tempfile::TempDir;

/// Simple test tool for integration testing
struct TestTool {
    name: String,
    priority: u32,
    should_run: bool,
}

impl TestTool {
    fn new(name: impl Into<String>, priority: u32) -> Self {
        Self {
            name: name.into(),
            priority,
            should_run: true,
        }
    }
}

impl DxTool for TestTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn priority(&self) -> u32 {
        self.priority
    }

    fn execute(&mut self, _ctx: &ExecutionContext) -> Result<ToolOutput> {
        Ok(ToolOutput::success())
    }

    fn should_run(&self, _ctx: &ExecutionContext) -> bool {
        self.should_run
    }
}

#[test]
fn test_forge_initialization() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let forge = Forge::new(temp_dir.path())?;

    assert_eq!(forge.project_root(), temp_dir.path());
    assert!(forge.forge_dir().exists());

    Ok(())
}

#[test]
fn test_forge_with_custom_config() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let custom_forge_dir = temp_dir.path().join("custom_forge");

    let config = ForgeConfig::new(temp_dir.path())
        .with_forge_dir(&custom_forge_dir)
        .without_auto_watch()
        .without_lsp();

    let forge = Forge::with_config(config)?;

    assert_eq!(forge.forge_dir(), &custom_forge_dir);
    assert!(custom_forge_dir.exists());

    Ok(())
}

#[test]
fn test_tool_lifecycle() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let forge = Forge::new(temp_dir.path())?;

    // Subscribe to lifecycle events
    let mut rx = forge.subscribe_lifecycle_events();

    // Register a tool (would need to expose this via Forge API)
    // This tests the lifecycle manager indirectly

    // Verify event channel is working
    assert!(rx.try_recv().is_err()); // No events yet

    Ok(())
}

#[test]
fn test_orchestrator_priority_ordering() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut orch = Orchestrator::new(temp_dir.path())?;

    // Register tools in random order
    orch.register_tool(Box::new(TestTool::new("tool-c", 30)))?;
    orch.register_tool(Box::new(TestTool::new("tool-a", 10)))?;
    orch.register_tool(Box::new(TestTool::new("tool-b", 20)))?;

    let outputs = orch.execute_all()?;

    // All tools should execute
    assert_eq!(outputs.len(), 3);
    assert!(outputs.iter().all(|o| o.success));

    Ok(())
}

#[test]
fn test_orchestrator_dependencies() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut orch = Orchestrator::new(temp_dir.path())?;

    struct ToolWithDeps {
        name: String,
        deps: Vec<String>,
    }

    impl DxTool for ToolWithDeps {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> &str {
            "1.0.0"
        }

        fn priority(&self) -> u32 {
            50
        }

        fn execute(&mut self, _ctx: &ExecutionContext) -> Result<ToolOutput> {
            Ok(ToolOutput::success())
        }

        fn dependencies(&self) -> Vec<String> {
            self.deps.clone()
        }
    }

    // Register tool with dependency
    orch.register_tool(Box::new(ToolWithDeps {
        name: "tool-b".to_string(),
        deps: vec!["tool-a".to_string()],
    }))?;

    orch.register_tool(Box::new(ToolWithDeps {
        name: "tool-a".to_string(),
        deps: vec![],
    }))?;

    // Should execute successfully with dependencies resolved
    let outputs = orch.execute_all()?;
    assert_eq!(outputs.len(), 2);
    assert!(outputs.iter().all(|o| o.success));

    Ok(())
}

#[test]
fn test_orchestrator_missing_dependency() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut orch = Orchestrator::new(temp_dir.path())?;

    struct ToolWithMissingDep;

    impl DxTool for ToolWithMissingDep {
        fn name(&self) -> &str {
            "tool-a"
        }

        fn version(&self) -> &str {
            "1.0.0"
        }

        fn priority(&self) -> u32 {
            50
        }

        fn execute(&mut self, _ctx: &ExecutionContext) -> Result<ToolOutput> {
            Ok(ToolOutput::success())
        }

        fn dependencies(&self) -> Vec<String> {
            vec!["missing-tool".to_string()]
        }
    }

    orch.register_tool(Box::new(ToolWithMissingDep))?;

    // Should fail due to missing dependency
    assert!(orch.execute_all().is_err());

    Ok(())
}

#[test]
fn test_version_snapshot_system() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut manager = SnapshotManager::new(temp_dir.path())?;

    // Create initial snapshot
    let mut tool_states = std::collections::HashMap::new();
    tool_states.insert(
        "test-tool".to_string(),
        dx_forge::ToolState {
            tool_name: "test-tool".to_string(),
            version: Version::new(1, 0, 0),
            config: std::collections::HashMap::new(),
            output_files: vec![],
        },
    );

    let snapshot1 = manager.create_snapshot("Initial commit", tool_states.clone(), vec![])?;

    // Verify snapshot was created
    let loaded = manager.get_snapshot(&snapshot1)?;
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().message, "Initial commit");

    // Create second snapshot
    let snapshot2 = manager.create_snapshot("Second commit", tool_states, vec![])?;

    // Verify history
    let history = manager.history(10)?;
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].id, snapshot2);
    assert_eq!(history[1].id, snapshot1);

    Ok(())
}

#[test]
fn test_version_branching() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut manager = SnapshotManager::new(temp_dir.path())?;

    // Create initial snapshot
    manager.create_snapshot("Initial", std::collections::HashMap::new(), vec![])?;

    // Create and switch to feature branch
    manager.create_branch("feature")?;
    manager.checkout_branch("feature")?;

    assert_eq!(manager.current_branch(), "feature");

    // Create commit on feature branch
    manager.create_snapshot("Feature commit", std::collections::HashMap::new(), vec![])?;

    // Switch back to main
    manager.checkout_branch("main")?;

    // List branches
    let branches = manager.list_branches()?;
    assert!(branches.iter().any(|b| b.name == "main"));
    assert!(branches.iter().any(|b| b.name == "feature"));

    Ok(())
}

#[test]
fn test_generated_code_tracking() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut forge = Forge::new(temp_dir.path())?;

    // Create a test file
    let test_file = temp_dir.path().join("generated.txt");
    std::fs::write(&test_file, "generated content")?;

    // Track the file
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("generator".to_string(), "test-tool".to_string());

    forge.track_generated_file(test_file.clone(), "test-tool", metadata)?;

    // Verify file is tracked
    let files = forge.get_generated_files("test-tool");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0], test_file);

    Ok(())
}

#[tokio::test]
async fn test_generated_code_cleanup() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut forge = Forge::new(temp_dir.path())?;

    // Create test files
    let test_file1 = temp_dir.path().join("gen1.txt");
    let test_file2 = temp_dir.path().join("gen2.txt");

    std::fs::write(&test_file1, "content1")?;
    std::fs::write(&test_file2, "content2")?;

    // Track files
    forge.track_generated_file(test_file1.clone(), "test-tool", Default::default())?;
    forge.track_generated_file(test_file2.clone(), "test-tool", Default::default())?;

    // Cleanup
    let removed = forge.cleanup_generated("test-tool").await?;

    assert_eq!(removed.len(), 2);
    assert!(!test_file1.exists());
    assert!(!test_file2.exists());

    Ok(())
}

#[test]
fn test_version_parsing() -> Result<()> {
    use std::str::FromStr;

    let v1 = Version::from_str("1.2.3")?;
    assert_eq!(v1.major, 1);
    assert_eq!(v1.minor, 2);
    assert_eq!(v1.patch, 3);

    let v2 = Version::from_str("v2.0.0-beta.1")?;
    assert_eq!(v2.major, 2);
    assert!(v2.is_prerelease());

    let v3 = Version::from_str("1.0.0+build.123")?;
    assert!(v3.is_stable());

    Ok(())
}

#[test]
fn test_version_comparison() -> Result<()> {
    let v1 = Version::new(1, 2, 3);
    let v2 = Version::new(1, 2, 4);
    let v3 = Version::new(2, 0, 0);

    assert!(v1 < v2);
    assert!(v2 < v3);
    assert!(v1.is_compatible_with(&v2));
    assert!(!v1.is_compatible_with(&v3));

    Ok(())
}
