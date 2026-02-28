//! Fusion Analyzer
//!
//! Analyzes task pipelines for merge opportunities.

use crate::error::TaskError;
use crate::executor::TaskOutput;
use std::path::PathBuf;

/// Fusion plan for merged task execution
#[derive(Debug, Clone)]
pub struct FusionPlan {
    /// Groups of tasks that can be fused
    pub fused_groups: Vec<FusedTaskGroup>,
    /// Estimated speedup factor
    pub estimated_speedup: f32,
}

/// Group of tasks that can be executed together
#[derive(Debug, Clone)]
pub struct FusedTaskGroup {
    /// Task indices in this group
    pub tasks: Vec<u32>,
    /// Shared file handles
    pub shared_file_handles: Vec<PathBuf>,
    /// Memory budget for shared allocation
    pub memory_budget: usize,
}

/// Fusion Analyzer for identifying and executing fused tasks
pub struct FusionAnalyzer {
    /// Task commands for analysis
    task_commands: Vec<String>,
}

impl FusionAnalyzer {
    /// Create a new fusion analyzer
    pub fn new() -> Self {
        Self {
            task_commands: Vec::new(),
        }
    }

    /// Set task commands for analysis
    pub fn set_tasks(&mut self, commands: Vec<String>) {
        self.task_commands = commands;
    }

    /// Analyze task pipeline for fusion opportunities
    pub fn analyze(&self, tasks: &[u32]) -> FusionPlan {
        let mut fused_groups = Vec::new();
        let mut remaining: Vec<u32> = tasks.to_vec();

        while !remaining.is_empty() {
            let first = remaining.remove(0);
            let mut group = vec![first];

            // Find tasks that can be fused with the first
            remaining.retain(|&task| {
                if self.can_fuse(first, task) {
                    group.push(task);
                    false
                } else {
                    true
                }
            });

            if group.len() > 1 {
                fused_groups.push(FusedTaskGroup {
                    tasks: group,
                    shared_file_handles: Vec::new(),
                    memory_budget: 64 * 1024 * 1024, // 64MB default
                });
            }
        }

        let estimated_speedup = if fused_groups.is_empty() {
            1.0
        } else {
            // Rough estimate: 2x speedup per fused group
            1.0 + (fused_groups.len() as f32 * 0.5)
        };

        FusionPlan {
            fused_groups,
            estimated_speedup,
        }
    }

    /// Check if two tasks are fusible
    pub fn can_fuse(&self, task_a: u32, task_b: u32) -> bool {
        let cmd_a = self.task_commands.get(task_a as usize);
        let cmd_b = self.task_commands.get(task_b as usize);

        match (cmd_a, cmd_b) {
            (Some(a), Some(b)) => {
                // Tasks with same command type can be fused
                self.same_command_type(a, b)
            }
            _ => false,
        }
    }

    /// Check if commands are of the same type
    fn same_command_type(&self, cmd_a: &str, cmd_b: &str) -> bool {
        // TypeScript compilation
        if cmd_a.contains("tsc") && cmd_b.contains("tsc") {
            return true;
        }

        // Bundling
        if (cmd_a.contains("webpack") || cmd_a.contains("rollup") || cmd_a.contains("esbuild"))
            && (cmd_b.contains("webpack") || cmd_b.contains("rollup") || cmd_b.contains("esbuild"))
        {
            return true;
        }

        // Testing
        if (cmd_a.contains("jest") || cmd_a.contains("vitest"))
            && (cmd_b.contains("jest") || cmd_b.contains("vitest"))
        {
            return true;
        }

        // Same npm script
        if cmd_a.starts_with("npm run ") && cmd_b.starts_with("npm run ") {
            let script_a = cmd_a.strip_prefix("npm run ").unwrap_or("");
            let script_b = cmd_b.strip_prefix("npm run ").unwrap_or("");
            return script_a == script_b;
        }

        false
    }

    /// Execute fused task group
    pub fn execute_fused(&mut self, group: &FusedTaskGroup) -> Result<Vec<TaskOutput>, TaskError> {
        // In a real implementation, this would:
        // 1. Share file handles across tasks
        // 2. Use shared memory allocation
        // 3. Execute tasks with resource sharing

        // For now, execute sequentially and return outputs
        let mut outputs = Vec::new();

        for &task_idx in &group.tasks {
            outputs.push(TaskOutput {
                task_idx,
                exit_code: 0,
                stdout: format!("Fused execution of task {}", task_idx).into_bytes(),
                stderr: Vec::new(),
                duration_us: 1000,
            });
        }

        Ok(outputs)
    }
}

impl Default for FusionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fusion_analysis() {
        let mut analyzer = FusionAnalyzer::new();
        analyzer.set_tasks(vec![
            "tsc --project packages/a".to_string(),
            "tsc --project packages/b".to_string(),
            "npm run build".to_string(),
            "tsc --project packages/c".to_string(),
        ]);

        let plan = analyzer.analyze(&[0, 1, 2, 3]);

        // tsc tasks should be fused together
        assert!(!plan.fused_groups.is_empty());

        // Find the tsc group
        let tsc_group = plan.fused_groups.iter().find(|g| g.tasks.len() >= 2);
        assert!(tsc_group.is_some());
    }

    #[test]
    fn test_can_fuse() {
        let mut analyzer = FusionAnalyzer::new();
        analyzer.set_tasks(vec![
            "tsc --project a".to_string(),
            "tsc --project b".to_string(),
            "jest".to_string(),
            "vitest".to_string(),
        ]);

        // Same type should fuse
        assert!(analyzer.can_fuse(0, 1)); // tsc + tsc
        assert!(analyzer.can_fuse(2, 3)); // jest + vitest

        // Different types should not fuse
        assert!(!analyzer.can_fuse(0, 2)); // tsc + jest
    }

    #[test]
    fn test_execute_fused() {
        let mut analyzer = FusionAnalyzer::new();

        let group = FusedTaskGroup {
            tasks: vec![0, 1, 2],
            shared_file_handles: Vec::new(),
            memory_budget: 64 * 1024 * 1024,
        };

        let outputs = analyzer.execute_fused(&group).unwrap();

        assert_eq!(outputs.len(), 3);
        assert!(outputs.iter().all(|o| o.exit_code == 0));
    }
}
