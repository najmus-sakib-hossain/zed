//! Task Executor
//!
//! Loads the Binary Task Graph and executes tasks with parallel scheduling.

use crate::btg::{BtgHeader, BtgSerializer, TaskData, TaskGraphData};
use crate::error::TaskError;
use crate::types::TaskInstance;
use bitvec::prelude::*;
use memmap2::Mmap;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Task output from execution
#[derive(Debug, Clone)]
pub struct TaskOutput {
    /// Task index
    pub task_idx: u32,
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Standard output
    pub stdout: Vec<u8>,
    /// Standard error
    pub stderr: Vec<u8>,
    /// Execution time in microseconds
    pub duration_us: u64,
}

/// Internal command output
struct CommandOutput {
    exit_code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// Task Executor for loading and running task graphs
pub struct TaskExecutor {
    /// Memory-mapped task graph
    mmap: Option<Mmap>,
    /// Parsed task graph data
    data: Option<TaskGraphData>,
    /// Path to task graph file
    graph_path: Option<PathBuf>,
    /// Task name to index lookup
    task_index: rustc_hash::FxHashMap<(u32, String), u32>,
    /// Completed tasks bitset
    completed: BitVec,
}

impl TaskExecutor {
    /// Create a new task executor
    pub fn new() -> Self {
        Self {
            mmap: None,
            data: None,
            graph_path: None,
            task_index: rustc_hash::FxHashMap::default(),
            completed: BitVec::new(),
        }
    }

    /// Load task graph from memory-mapped file
    pub fn load(&mut self, path: &Path) -> Result<(), TaskError> {
        let file = std::fs::File::open(path).map_err(|_| TaskError::GraphNotFound {
            path: path.to_path_buf(),
        })?;

        let mmap = unsafe { Mmap::map(&file) }?;

        if mmap.len() < BtgHeader::SIZE {
            return Err(TaskError::ExecutionFailed {
                exit_code: -1,
                stderr: "file too small".to_string(),
            });
        }

        let header: &BtgHeader = bytemuck::from_bytes(&mmap[..BtgHeader::SIZE]);
        header.validate_magic()?;

        let data = BtgSerializer::deserialize(&mmap)?;
        self.build_task_index(&data);
        self.completed = bitvec![0; data.tasks.len()];

        self.mmap = Some(mmap);
        self.data = Some(data);
        self.graph_path = Some(path.to_path_buf());

        Ok(())
    }

    /// Load from raw bytes (for testing)
    pub fn load_from_bytes(&mut self, bytes: &[u8]) -> Result<(), TaskError> {
        let data = BtgSerializer::deserialize(bytes)?;
        self.build_task_index(&data);
        self.completed = bitvec![0; data.tasks.len()];
        self.data = Some(data);
        Ok(())
    }

    /// Build task lookup index
    fn build_task_index(&mut self, data: &TaskGraphData) {
        self.task_index.clear();
        for (idx, task) in data.tasks.iter().enumerate() {
            self.task_index.insert((task.package_idx, task.name.clone()), idx as u32);
        }
    }

    /// Get task by package and name
    pub fn get_task(&self, package_idx: u32, name: &str) -> Option<&TaskData> {
        let idx = *self.task_index.get(&(package_idx, name.to_string()))?;
        self.data.as_ref()?.tasks.get(idx as usize)
    }

    /// Get task by index
    pub fn get_task_by_index(&self, idx: u32) -> Option<&TaskData> {
        self.data.as_ref()?.tasks.get(idx as usize)
    }

    /// Get tasks that can run in parallel at current stage
    pub fn parallel_tasks(&self) -> Vec<u32> {
        let data = match &self.data {
            Some(d) => d,
            None => return Vec::new(),
        };

        let mut ready = Vec::new();

        for (idx, _task) in data.tasks.iter().enumerate() {
            if self.completed[idx] {
                continue;
            }

            // Check if all dependencies are completed
            let deps_complete = data
                .dependency_edges
                .iter()
                .filter(|(_, to)| *to == idx as u32)
                .all(|(from, _)| self.completed[*from as usize]);

            if deps_complete {
                ready.push(idx as u32);
            }
        }

        ready
    }

    /// Clone a task template for execution (zero-allocation)
    #[inline]
    pub fn clone_task(&self, task_idx: u32) -> TaskInstance {
        TaskInstance::new(task_idx)
    }

    /// Check if task should yield due to frame budget
    #[inline]
    pub fn should_yield(&self, task: &TaskInstance, now_ns: u64) -> bool {
        let data = match &self.data {
            Some(d) => d,
            None => return false,
        };

        let task_data = match data.tasks.get(task.task_idx as usize) {
            Some(t) => t,
            None => return false,
        };

        if task_data.frame_budget_us == 0 {
            return false;
        }

        task.elapsed_us(now_ns) >= task_data.frame_budget_us as u64
    }

    /// Execute a task
    pub fn execute(&mut self, task_idx: u32) -> Result<TaskOutput, TaskError> {
        let data = self.data.as_ref().ok_or_else(|| TaskError::ExecutionFailed {
            exit_code: -1,
            stderr: "no task graph loaded".to_string(),
        })?;

        let task = data.tasks.get(task_idx as usize).ok_or_else(|| TaskError::TaskNotFound {
            package: "unknown".to_string(),
            task: format!("index {}", task_idx),
        })?;

        let start = Instant::now();

        // Check dependencies
        for (from, to) in &data.dependency_edges {
            if *to == task_idx && !self.completed[*from as usize] {
                return Err(TaskError::DependencyFailed {
                    task_idx: *from,
                    reason: "dependency not completed".to_string(),
                });
            }
        }

        // Actually execute the command
        let output = self.run_command(&task.command)?;

        let duration_us = start.elapsed().as_micros() as u64;

        let result = TaskOutput {
            task_idx,
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
            duration_us,
        };

        // Only mark as completed if successful
        if result.exit_code == 0 {
            self.completed.set(task_idx as usize, true);
        }

        Ok(result)
    }

    /// Run a shell command and capture output
    fn run_command(&self, command: &str) -> Result<CommandOutput, TaskError> {
        use std::process::{Command, Stdio};

        // Determine shell based on platform
        #[cfg(windows)]
        let (shell, shell_arg) = ("cmd", "/C");
        #[cfg(not(windows))]
        let (shell, shell_arg) = ("sh", "-c");

        let output = Command::new(shell)
            .arg(shell_arg)
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| TaskError::ExecutionFailed {
                exit_code: -1,
                stderr: format!("Failed to execute command: {}", e),
            })?;

        Ok(CommandOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    /// Mark task as completed
    pub fn mark_completed(&mut self, task_idx: u32) {
        if (task_idx as usize) < self.completed.len() {
            self.completed.set(task_idx as usize, true);
        }
    }

    /// Check if task is completed
    pub fn is_completed(&self, task_idx: u32) -> bool {
        self.completed.get(task_idx as usize).map(|b| *b).unwrap_or(false)
    }

    /// Get number of tasks
    pub fn task_count(&self) -> usize {
        self.data.as_ref().map(|d| d.tasks.len()).unwrap_or(0)
    }

    /// Reset all tasks to not completed
    pub fn reset(&mut self) {
        self.completed.fill(false);
    }

    /// Execute all tasks in dependency order
    pub fn execute_all(&mut self) -> Result<Vec<TaskOutput>, TaskError> {
        let mut outputs = Vec::new();

        loop {
            let ready = self.parallel_tasks();
            if ready.is_empty() {
                break;
            }

            // Execute ready tasks (could be parallelized with rayon)
            for task_idx in ready {
                let output = self.execute(task_idx)?;

                // Check for failure
                if output.exit_code != 0 {
                    return Err(TaskError::ExecutionFailed {
                        exit_code: output.exit_code,
                        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    });
                }

                outputs.push(output);
            }
        }

        Ok(outputs)
    }

    /// Execute tasks in parallel using rayon
    #[cfg(feature = "parallel")]
    pub fn execute_parallel(&mut self) -> Result<Vec<TaskOutput>, TaskError> {
        use rayon::prelude::*;

        let mut outputs = Vec::new();

        loop {
            let ready = self.parallel_tasks();
            if ready.is_empty() {
                break;
            }

            // Execute ready tasks in parallel
            let results: Vec<_> = ready
                .par_iter()
                .map(|&task_idx| {
                    let data = self.data.as_ref().unwrap();
                    let task = &data.tasks[task_idx as usize];
                    let start = Instant::now();

                    let cmd_output = self.run_command(&task.command);

                    match cmd_output {
                        Ok(output) => Ok(TaskOutput {
                            task_idx,
                            exit_code: output.exit_code,
                            stdout: output.stdout,
                            stderr: output.stderr,
                            duration_us: start.elapsed().as_micros() as u64,
                        }),
                        Err(e) => Err(e),
                    }
                })
                .collect();

            // Process results
            for result in results {
                let output = result?;

                if output.exit_code != 0 {
                    return Err(TaskError::ExecutionFailed {
                        exit_code: output.exit_code,
                        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    });
                }

                self.completed.set(output.task_idx as usize, true);
                outputs.push(output);
            }
        }

        Ok(outputs)
    }

    /// Execute a single task by name
    pub fn execute_by_name(
        &mut self,
        package_idx: u32,
        name: &str,
    ) -> Result<TaskOutput, TaskError> {
        let task_idx = *self.task_index.get(&(package_idx, name.to_string())).ok_or_else(|| {
            TaskError::TaskNotFound {
                package: format!("package {}", package_idx),
                task: name.to_string(),
            }
        })?;

        // Execute dependencies first
        self.execute_dependencies(task_idx)?;

        // Execute the task
        self.execute(task_idx)
    }

    /// Execute all dependencies of a task
    fn execute_dependencies(&mut self, task_idx: u32) -> Result<(), TaskError> {
        let data = self.data.as_ref().ok_or_else(|| TaskError::ExecutionFailed {
            exit_code: -1,
            stderr: "no task graph loaded".to_string(),
        })?;

        // Find all dependencies
        let deps: Vec<u32> = data
            .dependency_edges
            .iter()
            .filter(|(_, to)| *to == task_idx)
            .map(|(from, _)| *from)
            .collect();

        // Execute each dependency (recursively)
        for dep_idx in deps {
            if !self.is_completed(dep_idx) {
                self.execute_dependencies(dep_idx)?;
                let output = self.execute(dep_idx)?;

                if output.exit_code != 0 {
                    return Err(TaskError::DependencyFailed {
                        task_idx: dep_idx,
                        reason: String::from_utf8_lossy(&output.stderr).to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Get task execution order (topological sort)
    pub fn execution_order(&self) -> Vec<u32> {
        self.data.as_ref().map(|d| d.topological_order.clone()).unwrap_or_default()
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TaskState;

    fn create_test_graph() -> TaskGraphData {
        // Use platform-independent echo commands that always succeed
        #[cfg(windows)]
        let build_cmd = "echo build-complete";
        #[cfg(not(windows))]
        let build_cmd = "echo build-complete";

        #[cfg(windows)]
        let test_cmd = "echo test-complete";
        #[cfg(not(windows))]
        let test_cmd = "echo test-complete";

        let mut data = TaskGraphData {
            tasks: vec![
                TaskData {
                    name: "build".to_string(),
                    package_idx: 0,
                    command: build_cmd.to_string(),
                    definition_hash: [0; 8],
                    frame_budget_us: 0,
                    cacheable: true,
                },
                TaskData {
                    name: "build".to_string(),
                    package_idx: 1,
                    command: build_cmd.to_string(),
                    definition_hash: [0; 8],
                    frame_budget_us: 0,
                    cacheable: true,
                },
                TaskData {
                    name: "test".to_string(),
                    package_idx: 0,
                    command: test_cmd.to_string(),
                    definition_hash: [0; 8],
                    frame_budget_us: 16000, // 16ms frame budget
                    cacheable: false,
                },
            ],
            dependency_edges: vec![(0, 2)], // build:0 -> test:0
            topological_order: vec![0, 1, 2],
            parallel_groups: vec![],
        };
        data.compute_parallel_groups();
        data
    }

    #[test]
    fn test_task_executor_load() {
        let data = create_test_graph();
        let bytes = BtgSerializer::serialize(&data).unwrap();

        let mut executor = TaskExecutor::new();
        executor.load_from_bytes(&bytes).unwrap();

        assert_eq!(executor.task_count(), 3);
    }

    #[test]
    fn test_task_lookup() {
        let data = create_test_graph();
        let bytes = BtgSerializer::serialize(&data).unwrap();

        let mut executor = TaskExecutor::new();
        executor.load_from_bytes(&bytes).unwrap();

        let task = executor.get_task(0, "build").unwrap();
        assert!(task.command.contains("echo"));

        let task = executor.get_task(0, "test").unwrap();
        assert!(task.command.contains("echo"));

        assert!(executor.get_task(0, "nonexistent").is_none());
    }

    #[test]
    fn test_parallel_tasks() {
        let data = create_test_graph();
        let bytes = BtgSerializer::serialize(&data).unwrap();

        let mut executor = TaskExecutor::new();
        executor.load_from_bytes(&bytes).unwrap();

        // Initially, build:0 and build:1 can run in parallel
        let parallel = executor.parallel_tasks();
        assert!(parallel.contains(&0));
        assert!(parallel.contains(&1));
        assert!(!parallel.contains(&2)); // test:0 depends on build:0

        // After completing build:0, test:0 becomes available
        executor.mark_completed(0);
        let parallel = executor.parallel_tasks();
        assert!(!parallel.contains(&0)); // already completed
        assert!(parallel.contains(&1));
        assert!(parallel.contains(&2)); // now available
    }

    #[test]
    fn test_task_instance_zero_allocation() {
        let executor = TaskExecutor::new();

        // This should be stack-allocated
        let instance = executor.clone_task(5);
        assert_eq!(instance.task_idx, 5);
        assert_eq!(instance.state, TaskState::Pending);

        // Verify size is reasonable for stack allocation
        assert!(TaskInstance::SIZE <= 96);
    }

    #[test]
    fn test_frame_budget_check() {
        let data = create_test_graph();
        let bytes = BtgSerializer::serialize(&data).unwrap();

        let mut executor = TaskExecutor::new();
        executor.load_from_bytes(&bytes).unwrap();

        let mut instance = executor.clone_task(2); // test task with 16ms budget
        instance.start(0);

        // At 10ms, should not yield
        assert!(!executor.should_yield(&instance, 10_000_000)); // 10ms in ns

        // At 20ms, should yield
        assert!(executor.should_yield(&instance, 20_000_000)); // 20ms in ns
    }

    #[test]
    fn test_execute_with_dependencies() {
        let data = create_test_graph();
        let bytes = BtgSerializer::serialize(&data).unwrap();

        let mut executor = TaskExecutor::new();
        executor.load_from_bytes(&bytes).unwrap();

        // Can't execute test:0 before build:0
        let result = executor.execute(2);
        assert!(matches!(result, Err(TaskError::DependencyFailed { .. })));

        // Execute build:0 first
        executor.execute(0).unwrap();
        assert!(executor.is_completed(0));

        // Now test:0 can execute
        let output = executor.execute(2).unwrap();
        assert_eq!(output.exit_code, 0);
        assert!(executor.is_completed(2));
    }

    /// Test that shell commands work correctly on the current platform
    #[test]
    fn test_platform_shell_execution() {
        // Create a task graph with platform-specific commands
        #[cfg(windows)]
        let cmd = "echo Hello from Windows";
        #[cfg(not(windows))]
        let cmd = "echo Hello from Unix";

        let mut data = TaskGraphData {
            tasks: vec![TaskData {
                name: "shell-test".to_string(),
                package_idx: 0,
                command: cmd.to_string(),
                definition_hash: [0; 8],
                frame_budget_us: 0,
                cacheable: false,
            }],
            dependency_edges: vec![],
            topological_order: vec![0],
            parallel_groups: vec![],
        };
        data.compute_parallel_groups();

        let bytes = BtgSerializer::serialize(&data).unwrap();
        let mut executor = TaskExecutor::new();
        executor.load_from_bytes(&bytes).unwrap();

        let output = executor.execute(0).unwrap();
        assert_eq!(output.exit_code, 0);

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Hello"));
    }

    /// Test that environment variables work in shell commands
    #[test]
    fn test_shell_environment_variables() {
        // Use platform-specific environment variable syntax
        #[cfg(windows)]
        let cmd = "echo %PATH%";
        #[cfg(not(windows))]
        let cmd = "echo $PATH";

        let mut data = TaskGraphData {
            tasks: vec![TaskData {
                name: "env-test".to_string(),
                package_idx: 0,
                command: cmd.to_string(),
                definition_hash: [0; 8],
                frame_budget_us: 0,
                cacheable: false,
            }],
            dependency_edges: vec![],
            topological_order: vec![0],
            parallel_groups: vec![],
        };
        data.compute_parallel_groups();

        let bytes = BtgSerializer::serialize(&data).unwrap();
        let mut executor = TaskExecutor::new();
        executor.load_from_bytes(&bytes).unwrap();

        let output = executor.execute(0).unwrap();
        assert_eq!(output.exit_code, 0);

        // PATH should be non-empty
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(!stdout.trim().is_empty());
    }

    /// Test that command chaining works on the current platform
    #[test]
    fn test_shell_command_chaining() {
        // Use platform-specific command chaining
        #[cfg(windows)]
        let cmd = "echo first & echo second";
        #[cfg(not(windows))]
        let cmd = "echo first && echo second";

        let mut data = TaskGraphData {
            tasks: vec![TaskData {
                name: "chain-test".to_string(),
                package_idx: 0,
                command: cmd.to_string(),
                definition_hash: [0; 8],
                frame_budget_us: 0,
                cacheable: false,
            }],
            dependency_edges: vec![],
            topological_order: vec![0],
            parallel_groups: vec![],
        };
        data.compute_parallel_groups();

        let bytes = BtgSerializer::serialize(&data).unwrap();
        let mut executor = TaskExecutor::new();
        executor.load_from_bytes(&bytes).unwrap();

        let output = executor.execute(0).unwrap();
        assert_eq!(output.exit_code, 0);

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("first"));
        assert!(stdout.contains("second"));
    }

    /// Test that failing commands return non-zero exit code
    #[test]
    fn test_shell_command_failure() {
        // Use a command that will fail on all platforms
        #[cfg(windows)]
        let cmd = "exit /b 1";
        #[cfg(not(windows))]
        let cmd = "exit 1";

        let mut data = TaskGraphData {
            tasks: vec![TaskData {
                name: "fail-test".to_string(),
                package_idx: 0,
                command: cmd.to_string(),
                definition_hash: [0; 8],
                frame_budget_us: 0,
                cacheable: false,
            }],
            dependency_edges: vec![],
            topological_order: vec![0],
            parallel_groups: vec![],
        };
        data.compute_parallel_groups();

        let bytes = BtgSerializer::serialize(&data).unwrap();
        let mut executor = TaskExecutor::new();
        executor.load_from_bytes(&bytes).unwrap();

        let output = executor.execute(0).unwrap();
        assert_ne!(output.exit_code, 0);
    }

    /// Test Unix-specific shell features
    #[cfg(not(windows))]
    #[test]
    fn test_unix_shell_features() {
        // Test pipe support
        let mut data = TaskGraphData {
            tasks: vec![TaskData {
                name: "pipe-test".to_string(),
                package_idx: 0,
                command: "echo 'hello world' | tr 'a-z' 'A-Z'".to_string(),
                definition_hash: [0; 8],
                frame_budget_us: 0,
                cacheable: false,
            }],
            dependency_edges: vec![],
            topological_order: vec![0],
            parallel_groups: vec![],
        };
        data.compute_parallel_groups();

        let bytes = BtgSerializer::serialize(&data).unwrap();
        let mut executor = TaskExecutor::new();
        executor.load_from_bytes(&bytes).unwrap();

        let output = executor.execute(0).unwrap();
        assert_eq!(output.exit_code, 0);

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("HELLO WORLD"));
    }

    /// Test Unix shell with subshell
    #[cfg(not(windows))]
    #[test]
    fn test_unix_subshell() {
        let mut data = TaskGraphData {
            tasks: vec![TaskData {
                name: "subshell-test".to_string(),
                package_idx: 0,
                command: "(echo 'in subshell')".to_string(),
                definition_hash: [0; 8],
                frame_budget_us: 0,
                cacheable: false,
            }],
            dependency_edges: vec![],
            topological_order: vec![0],
            parallel_groups: vec![],
        };
        data.compute_parallel_groups();

        let bytes = BtgSerializer::serialize(&data).unwrap();
        let mut executor = TaskExecutor::new();
        executor.load_from_bytes(&bytes).unwrap();

        let output = executor.execute(0).unwrap();
        assert_eq!(output.exit_code, 0);

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("in subshell"));
    }
}
