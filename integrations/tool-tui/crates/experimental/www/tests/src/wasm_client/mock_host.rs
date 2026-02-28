//! Mock Host Functions for WASM Client Testing
//!
//! This module provides a test harness that simulates the JavaScript host
//! environment for testing the WASM client without a browser.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// DOM operation types for verification
#[derive(Debug, Clone, PartialEq)]
pub enum DomOp {
    CloneTemplate {
        template_id: u32,
        result_node: u32,
    },
    CacheTemplate {
        id: u32,
        html: Vec<u8>,
    },
    Append {
        parent: u32,
        child: u32,
    },
    Remove {
        node: u32,
    },
    SetText {
        node: u32,
        text: String,
    },
    SetAttr {
        node: u32,
        key: String,
        value: String,
    },
    ToggleClass {
        node: u32,
        class: String,
        enable: bool,
    },
    Listen {
        node: u32,
        event_type: u32,
        handler_id: u32,
    },
    Log {
        value: u32,
    },
    GetCachedBase {
        cache_id: u32,
        size: usize,
    },
    StorePatched {
        cache_id: u32,
        data: Vec<u8>,
    },
}

/// Mock host state for testing
#[derive(Default, Clone)]
pub struct MockHostState {
    /// Cached templates (id -> HTML bytes)
    pub templates: HashMap<u32, Vec<u8>>,
    /// Next node ID to assign
    pub next_node_id: u32,
    /// DOM operations log
    pub dom_ops: Vec<DomOp>,
    /// Event listeners (node_id, event_type, handler_id)
    pub listeners: Vec<(u32, u32, u32)>,
    /// Cached base data for delta patching
    pub cache: HashMap<u32, Vec<u8>>,
    /// Patched results
    pub patched_results: HashMap<u32, Vec<u8>>,
}

impl MockHostState {
    pub fn new() -> Self {
        Self {
            next_node_id: 1, // Start from 1, 0 is root
            ..Default::default()
        }
    }

    /// Reset state for a new test
    pub fn reset(&mut self) {
        self.templates.clear();
        self.next_node_id = 1;
        self.dom_ops.clear();
        self.listeners.clear();
        self.cache.clear();
        self.patched_results.clear();
    }

    /// Pre-populate cache with base data for delta testing
    pub fn set_cached_base(&mut self, cache_id: u32, data: Vec<u8>) {
        self.cache.insert(cache_id, data);
    }

    /// Get the patched result for a cache ID
    pub fn get_patched_result(&self, cache_id: u32) -> Option<&Vec<u8>> {
        self.patched_results.get(&cache_id)
    }

    /// Verify expected DOM operations occurred
    pub fn verify_ops(&self, expected: &[DomOp]) -> bool {
        if self.dom_ops.len() != expected.len() {
            return false;
        }
        self.dom_ops.iter().zip(expected.iter()).all(|(a, b)| a == b)
    }

    /// Check if a specific operation occurred
    pub fn has_op(&self, op: &DomOp) -> bool {
        self.dom_ops.contains(op)
    }

    /// Get all operations of a specific type
    pub fn get_ops_of_type<F>(&self, predicate: F) -> Vec<&DomOp>
    where
        F: Fn(&DomOp) -> bool,
    {
        self.dom_ops.iter().filter(|op| predicate(op)).collect()
    }

    /// Simulate host_clone_template
    pub fn clone_template(&mut self, template_id: u32) -> u32 {
        let node_id = self.next_node_id;
        self.next_node_id += 1;
        self.dom_ops.push(DomOp::CloneTemplate {
            template_id,
            result_node: node_id,
        });
        node_id
    }

    /// Simulate host_cache_template
    pub fn cache_template(&mut self, id: u32, html: &[u8]) {
        self.templates.insert(id, html.to_vec());
        self.dom_ops.push(DomOp::CacheTemplate {
            id,
            html: html.to_vec(),
        });
    }

    /// Simulate host_append
    pub fn append(&mut self, parent: u32, child: u32) {
        self.dom_ops.push(DomOp::Append { parent, child });
    }

    /// Simulate host_remove
    pub fn remove(&mut self, node: u32) {
        self.dom_ops.push(DomOp::Remove { node });
    }

    /// Simulate host_set_text
    pub fn set_text(&mut self, node: u32, text: &str) {
        self.dom_ops.push(DomOp::SetText {
            node,
            text: text.to_string(),
        });
    }

    /// Simulate host_set_attr
    pub fn set_attr(&mut self, node: u32, key: &str, value: &str) {
        self.dom_ops.push(DomOp::SetAttr {
            node,
            key: key.to_string(),
            value: value.to_string(),
        });
    }

    /// Simulate host_toggle_class
    pub fn toggle_class(&mut self, node: u32, class: &str, enable: bool) {
        self.dom_ops.push(DomOp::ToggleClass {
            node,
            class: class.to_string(),
            enable,
        });
    }

    /// Simulate host_listen
    pub fn listen(&mut self, node: u32, event_type: u32, handler_id: u32) {
        self.listeners.push((node, event_type, handler_id));
        self.dom_ops.push(DomOp::Listen {
            node,
            event_type,
            handler_id,
        });
    }

    /// Simulate host_get_cached_base
    pub fn get_cached_base(&mut self, cache_id: u32, buf: &mut [u8]) -> usize {
        if let Some(data) = self.cache.get(&cache_id) {
            let len = data.len().min(buf.len());
            buf[..len].copy_from_slice(&data[..len]);
            self.dom_ops.push(DomOp::GetCachedBase {
                cache_id,
                size: data.len(),
            });
            data.len()
        } else {
            0
        }
    }

    /// Simulate host_store_patched
    pub fn store_patched(&mut self, cache_id: u32, data: &[u8]) {
        self.patched_results.insert(cache_id, data.to_vec());
        self.dom_ops.push(DomOp::StorePatched {
            cache_id,
            data: data.to_vec(),
        });
    }
}

// Global mock host state (thread-local for test isolation)
thread_local! {
    pub static MOCK_HOST: Arc<Mutex<MockHostState>> = Arc::new(Mutex::new(MockHostState::new()));
}

/// Initialize mock host for a test
pub fn init_mock_host() {
    MOCK_HOST.with(|host| {
        host.lock().unwrap().reset();
    });
}

/// Get a clone of the mock host state for verification
pub fn get_mock_host_state() -> MockHostState {
    MOCK_HOST.with(|host| host.lock().unwrap().clone())
}

/// Access mock host state mutably
pub fn with_mock_host<F, R>(f: F) -> R
where
    F: FnOnce(&mut MockHostState) -> R,
{
    MOCK_HOST.with(|host| {
        let mut guard = host.lock().unwrap();
        f(&mut guard)
    })
}
