//! Node Registry: Track cloned nodes by ID
//!
//! Fixed-size array for predictable memory layout.

use web_sys::Node;

/// Maximum nodes (matches dx_packet::MAX_NODES)
const MAX_NODES: usize = 65536;

/// Node registry using fixed-size array
pub struct NodeRegistry {
    /// Registered nodes (None = slot empty)
    nodes: Vec<Option<Node>>,
    /// Next available ID
    next_id: u16,
}

impl NodeRegistry {
    /// Create new registry
    pub fn new() -> Self {
        // Pre-allocate with None (lazy initialization)
        let mut nodes = Vec::with_capacity(MAX_NODES);
        nodes.resize_with(MAX_NODES, || None);

        Self {
            nodes,
            next_id: 1, // 0 reserved for root
        }
    }

    /// Register a node and return its ID
    pub fn register(&mut self, node: Node) -> u16 {
        let id = self.next_id;
        if (id as usize) < MAX_NODES {
            self.nodes[id as usize] = Some(node);
            self.next_id = self.next_id.wrapping_add(1);
        }
        id
    }

    /// Get a node by ID
    pub fn get(&self, id: u16) -> Option<&Node> {
        self.nodes.get(id as usize).and_then(|n| n.as_ref())
    }

    /// Remove a node by ID
    pub fn remove(&mut self, id: u16) -> Option<Node> {
        self.nodes.get_mut(id as usize).and_then(|n| n.take())
    }

    /// Get count of registered nodes
    pub fn count(&self) -> u32 {
        self.nodes.iter().filter(|n| n.is_some()).count() as u32
    }

    /// Clear all nodes
    pub fn clear(&mut self) {
        for slot in self.nodes.iter_mut() {
            *slot = None;
        }
        self.next_id = 1;
    }
}
