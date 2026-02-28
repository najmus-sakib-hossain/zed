//! Binary Task Graph (BTG) format
//!
//! Pre-compiled task pipelines with u32 indices and parallel execution maps.

use crate::error::TaskError;
use crate::types::TaskEntry;
use crate::{BTG_MAGIC, FORMAT_VERSION};
use bytemuck::{Pod, Zeroable};

/// Binary Task Graph header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BtgHeader {
    /// Magic bytes: "DXTG"
    pub magic: [u8; 4],
    /// Format version
    pub version: u32,
    /// Total number of task definitions
    pub task_count: u32,
    /// Offset to task entries
    pub tasks_offset: u64,
    /// Offset to dependency edges (u32 pairs)
    pub edges_offset: u64,
    /// Number of edges
    pub edge_count: u32,
    /// Offset to parallel execution groups
    pub parallel_groups_offset: u64,
    /// Offset to topological order
    pub topo_order_offset: u64,
    /// Offset to string table
    pub strings_offset: u64,
    /// Blake3 hash of content
    pub content_hash: [u8; 32],
}

impl BtgHeader {
    /// Size of header in bytes
    pub const SIZE: usize = std::mem::size_of::<Self>();

    /// Create a new header
    pub fn new(task_count: u32, edge_count: u32) -> Self {
        Self {
            magic: BTG_MAGIC,
            version: FORMAT_VERSION,
            task_count,
            tasks_offset: Self::SIZE as u64,
            edges_offset: 0,
            edge_count,
            parallel_groups_offset: 0,
            topo_order_offset: 0,
            strings_offset: 0,
            content_hash: [0; 32],
        }
    }

    /// Validate magic bytes
    pub fn validate_magic(&self) -> Result<(), TaskError> {
        if self.magic != BTG_MAGIC {
            return Err(TaskError::InvalidMagic { found: self.magic });
        }
        Ok(())
    }
}

/// Parallel execution group
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq)]
pub struct ParallelGroup {
    /// Offset to task indices in this group
    pub tasks_offset: u32,
    /// Number of tasks in this group
    pub task_count: u16,
    /// Group level (0 = no dependencies, 1 = depends on level 0, etc.)
    pub level: u16,
}

/// BTG Serializer
pub struct BtgSerializer;

impl BtgSerializer {
    /// Serialize task graph to BTG format
    pub fn serialize(data: &TaskGraphData) -> Result<Vec<u8>, TaskError> {
        let mut buffer = Vec::new();

        // Calculate sizes
        let tasks_size = data.tasks.len() * TaskEntry::SIZE;
        let edges_size = data.dependency_edges.len() * 8;
        let groups_size = data.parallel_groups.len() * std::mem::size_of::<ParallelGroup>();
        let topo_size = data.topological_order.len() * 4;

        // Calculate offsets
        let tasks_offset = BtgHeader::SIZE as u64;
        let edges_offset = tasks_offset + tasks_size as u64;
        let groups_offset = edges_offset + edges_size as u64;
        let topo_offset = groups_offset + groups_size as u64;
        let strings_offset = topo_offset + topo_size as u64;

        // Build string table
        let (string_table, string_indices) = Self::build_string_table(data);

        // Create header
        let mut header =
            BtgHeader::new(data.tasks.len() as u32, data.dependency_edges.len() as u32);
        header.tasks_offset = tasks_offset;
        header.edges_offset = edges_offset;
        header.parallel_groups_offset = groups_offset;
        header.topo_order_offset = topo_offset;
        header.strings_offset = strings_offset;

        // Write header
        buffer.extend_from_slice(bytemuck::bytes_of(&header));

        // Write task entries
        for task in &data.tasks {
            let mut entry = TaskEntry::new(
                string_indices[&task.name] as u32,
                task.package_idx,
                string_indices[&task.command] as u32,
            );
            entry.definition_hash = task.definition_hash;
            entry.frame_budget_us = task.frame_budget_us;
            if task.cacheable {
                entry.set_cacheable(true);
            }
            buffer.extend_from_slice(bytemuck::bytes_of(&entry));
        }

        // Write dependency edges
        for (from, to) in &data.dependency_edges {
            buffer.extend_from_slice(&from.to_le_bytes());
            buffer.extend_from_slice(&to.to_le_bytes());
        }

        // Write parallel groups
        for group in &data.parallel_groups {
            buffer.extend_from_slice(bytemuck::bytes_of(group));
        }

        // Write topological order
        for idx in &data.topological_order {
            buffer.extend_from_slice(&idx.to_le_bytes());
        }

        // Write string table
        buffer.extend_from_slice(&string_table);

        // Compute content hash
        // content_hash offset in packed BtgHeader: 4 + 4 + 4 + 8 + 8 + 4 + 8 + 8 + 8 = 56
        let content_hash = blake3::hash(&buffer[BtgHeader::SIZE..]);
        buffer[56..88].copy_from_slice(content_hash.as_bytes());

        Ok(buffer)
    }

    /// Deserialize BTG format
    pub fn deserialize(data: &[u8]) -> Result<TaskGraphData, TaskError> {
        if data.len() < BtgHeader::SIZE {
            return Err(TaskError::ExecutionFailed {
                exit_code: -1,
                stderr: "data too small for header".to_string(),
            });
        }

        let header: &BtgHeader = bytemuck::from_bytes(&data[..BtgHeader::SIZE]);
        header.validate_magic()?;

        // Read string table
        let strings_start = header.strings_offset as usize;
        let string_table = Self::parse_string_table(&data[strings_start..]);

        // Read task entries
        let tasks_start = header.tasks_offset as usize;
        let mut tasks = Vec::with_capacity(header.task_count as usize);

        for i in 0..header.task_count as usize {
            let offset = tasks_start + i * TaskEntry::SIZE;
            let entry: &TaskEntry = bytemuck::from_bytes(&data[offset..offset + TaskEntry::SIZE]);

            tasks.push(TaskData {
                name: string_table.get(entry.name_idx as usize).cloned().unwrap_or_default(),
                package_idx: entry.package_idx,
                command: string_table.get(entry.command_idx as usize).cloned().unwrap_or_default(),
                definition_hash: entry.definition_hash,
                frame_budget_us: entry.frame_budget_us,
                cacheable: entry.is_cacheable(),
            });
        }

        // Read dependency edges
        let edges_start = header.edges_offset as usize;
        let mut dependency_edges = Vec::with_capacity(header.edge_count as usize);

        for i in 0..header.edge_count as usize {
            let offset = edges_start + i * 8;
            let from = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            let to = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
            dependency_edges.push((from, to));
        }

        // Read topological order
        let topo_start = header.topo_order_offset as usize;
        let topo_count = header.task_count as usize;
        let mut topological_order = Vec::with_capacity(topo_count);

        for i in 0..topo_count {
            let offset = topo_start + i * 4;
            if offset + 4 <= data.len() {
                let idx = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
                topological_order.push(idx);
            }
        }

        // Read parallel groups
        let groups_start = header.parallel_groups_offset as usize;
        let groups_end = header.topo_order_offset as usize;
        let group_count = (groups_end - groups_start) / std::mem::size_of::<ParallelGroup>();
        let mut parallel_groups = Vec::with_capacity(group_count);

        for i in 0..group_count {
            let offset = groups_start + i * std::mem::size_of::<ParallelGroup>();
            let group: &ParallelGroup =
                bytemuck::from_bytes(&data[offset..offset + std::mem::size_of::<ParallelGroup>()]);
            parallel_groups.push(*group);
        }

        Ok(TaskGraphData {
            tasks,
            dependency_edges,
            topological_order,
            parallel_groups,
        })
    }

    fn build_string_table(
        data: &TaskGraphData,
    ) -> (Vec<u8>, std::collections::HashMap<String, usize>) {
        use std::collections::HashMap;

        let mut table = Vec::new();
        let mut indices = HashMap::new();
        let mut string_index = 0usize;

        for task in &data.tasks {
            for s in [&task.name, &task.command] {
                if !indices.contains_key(s) {
                    indices.insert(s.clone(), string_index);
                    table.extend_from_slice(s.as_bytes());
                    table.push(0);
                    string_index += 1;
                }
            }
        }

        (table, indices)
    }

    fn parse_string_table(data: &[u8]) -> Vec<String> {
        let mut strings = Vec::new();
        let mut start = 0;

        for (i, &byte) in data.iter().enumerate() {
            if byte == 0 {
                if let Ok(s) = std::str::from_utf8(&data[start..i]) {
                    strings.push(s.to_string());
                }
                start = i + 1;
            }
        }

        strings
    }
}

/// Task graph data for serialization
#[derive(Debug, Clone, PartialEq)]
pub struct TaskGraphData {
    /// Task definitions
    pub tasks: Vec<TaskData>,
    /// Dependency edges as (from_idx, to_idx)
    pub dependency_edges: Vec<(u32, u32)>,
    /// Pre-computed topological order
    pub topological_order: Vec<u32>,
    /// Parallel execution groups
    pub parallel_groups: Vec<ParallelGroup>,
}

/// Task data for serialization
#[derive(Debug, Clone, PartialEq)]
pub struct TaskData {
    /// Task name (e.g., "build", "test")
    pub name: String,
    /// Package index this task belongs to
    pub package_idx: u32,
    /// Command to execute
    pub command: String,
    /// Pre-computed hash of task definition
    pub definition_hash: [u8; 8],
    /// Frame budget in microseconds
    pub frame_budget_us: u32,
    /// Whether output is cacheable
    pub cacheable: bool,
}

impl TaskGraphData {
    /// Create empty task graph
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            dependency_edges: Vec::new(),
            topological_order: Vec::new(),
            parallel_groups: Vec::new(),
        }
    }

    /// Compute parallel execution groups
    pub fn compute_parallel_groups(&mut self) {
        let n = self.tasks.len();
        if n == 0 {
            return;
        }

        // Compute levels (distance from root)
        let mut levels = vec![0u16; n];
        let mut adj: Vec<Vec<u32>> = vec![Vec::new(); n];

        for &(from, to) in &self.dependency_edges {
            adj[from as usize].push(to);
        }

        // BFS to compute levels
        for &root in &self.topological_order {
            for &neighbor in &adj[root as usize] {
                levels[neighbor as usize] =
                    levels[neighbor as usize].max(levels[root as usize] + 1);
            }
        }

        // Group tasks by level
        let max_level = *levels.iter().max().unwrap_or(&0);
        self.parallel_groups.clear();

        for level in 0..=max_level {
            let tasks_at_level: Vec<u32> =
                (0..n as u32).filter(|&i| levels[i as usize] == level).collect();

            if !tasks_at_level.is_empty() {
                self.parallel_groups.push(ParallelGroup {
                    tasks_offset: 0, // Would be set during serialization
                    task_count: tasks_at_level.len() as u16,
                    level,
                });
            }
        }
    }
}

impl Default for TaskGraphData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btg_header_size() {
        // Packed struct size: 4 + 4 + 4 + 8 + 8 + 4 + 8 + 8 + 8 + 32 = 88 bytes
        assert_eq!(BtgHeader::SIZE, 88);
    }

    #[test]
    fn test_parallel_group_computation() {
        let mut data = TaskGraphData {
            tasks: vec![
                TaskData {
                    name: "build:a".to_string(),
                    package_idx: 0,
                    command: "npm run build".to_string(),
                    definition_hash: [0; 8],
                    frame_budget_us: 0,
                    cacheable: true,
                },
                TaskData {
                    name: "build:b".to_string(),
                    package_idx: 1,
                    command: "npm run build".to_string(),
                    definition_hash: [0; 8],
                    frame_budget_us: 0,
                    cacheable: true,
                },
                TaskData {
                    name: "build:c".to_string(),
                    package_idx: 2,
                    command: "npm run build".to_string(),
                    definition_hash: [0; 8],
                    frame_budget_us: 0,
                    cacheable: true,
                },
            ],
            dependency_edges: vec![(0, 2), (1, 2)], // a,b -> c
            topological_order: vec![0, 1, 2],
            parallel_groups: vec![],
        };

        data.compute_parallel_groups();

        // Should have 2 levels: [a, b] at level 0, [c] at level 1
        assert_eq!(data.parallel_groups.len(), 2);
        // Copy field values to avoid unaligned access on packed struct
        let level0 = { data.parallel_groups[0].level };
        let count0 = { data.parallel_groups[0].task_count };
        let level1 = { data.parallel_groups[1].level };
        let count1 = { data.parallel_groups[1].task_count };
        assert_eq!(level0, 0);
        assert_eq!(count0, 2);
        assert_eq!(level1, 1);
        assert_eq!(count1, 1);
    }
}
