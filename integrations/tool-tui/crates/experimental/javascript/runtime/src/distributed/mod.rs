//! Distributed computing primitives

use std::collections::HashMap;
use std::net::SocketAddr;

pub struct DistributedRuntime {
    nodes: HashMap<String, NodeInfo>,
    /// Local node identifier - reserved for distributed execution
    #[allow(dead_code)]
    local_node: String,
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: String,
    pub addr: SocketAddr,
    pub capabilities: NodeCapabilities,
    pub load: f32,
}

#[derive(Debug, Clone)]
pub struct NodeCapabilities {
    pub cpu_cores: usize,
    pub memory_gb: usize,
    pub gpu_available: bool,
}

impl DistributedRuntime {
    pub fn new(local_id: String) -> Self {
        Self {
            nodes: HashMap::new(),
            local_node: local_id,
        }
    }

    pub fn register_node(&mut self, info: NodeInfo) {
        self.nodes.insert(info.id.clone(), info);
    }

    pub fn remove_node(&mut self, id: &str) {
        self.nodes.remove(id);
    }

    pub fn get_available_nodes(&self) -> Vec<&NodeInfo> {
        self.nodes.values().filter(|n| n.load < 0.8).collect()
    }

    pub fn select_node_for_task(&self, required_caps: &NodeCapabilities) -> Option<&NodeInfo> {
        self.nodes
            .values()
            .filter(|n| {
                n.capabilities.cpu_cores >= required_caps.cpu_cores
                    && n.capabilities.memory_gb >= required_caps.memory_gb
                    && (!required_caps.gpu_available || n.capabilities.gpu_available)
            })
            .min_by(|a, b| a.load.partial_cmp(&b.load).unwrap_or(std::cmp::Ordering::Equal))
    }
}

pub struct Task {
    pub id: u64,
    pub code: Vec<u8>,
    pub requirements: NodeCapabilities,
}

impl Task {
    pub fn new(id: u64, code: Vec<u8>) -> Self {
        Self {
            id,
            code,
            requirements: NodeCapabilities {
                cpu_cores: 1,
                memory_gb: 1,
                gpu_available: false,
            },
        }
    }

    pub fn with_requirements(mut self, req: NodeCapabilities) -> Self {
        self.requirements = req;
        self
    }
}

pub struct TaskScheduler {
    pending: Vec<Task>,
    running: HashMap<u64, String>,
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            running: HashMap::new(),
        }
    }

    pub fn submit(&mut self, task: Task) {
        self.pending.push(task);
    }

    pub fn schedule(&mut self, runtime: &DistributedRuntime) -> Vec<(u64, String)> {
        let mut scheduled = Vec::new();
        self.pending.retain(|task| {
            if let Some(node) = runtime.select_node_for_task(&task.requirements) {
                self.running.insert(task.id, node.id.clone());
                scheduled.push((task.id, node.id.clone()));
                false
            } else {
                true
            }
        });
        scheduled
    }

    pub fn complete_task(&mut self, task_id: u64) {
        self.running.remove(&task_id);
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}
