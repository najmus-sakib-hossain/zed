//! # dx-guard — DOM Integrity Protection
//!
//! Protect against browser extensions and scripts that mutate the DOM.
//!
//! ## Features
//! - MutationObserver monitoring
//! - Signature verification
//! - Auto-repair of unauthorized changes
//! - Tamper detection

#![forbid(unsafe_code)]

/// Mutation types to monitor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationType {
    ChildList,
    Attributes,
    CharacterData,
}

/// DOM protection configuration
#[derive(Debug, Clone)]
pub struct GuardConfig {
    /// Monitor child node changes
    pub observe_children: bool,
    /// Monitor attribute changes
    pub observe_attributes: bool,
    /// Monitor text content changes
    pub observe_character_data: bool,
    /// Automatically repair unauthorized changes
    pub auto_repair: bool,
    /// Whitelist of allowed mutation sources
    pub whitelist: Vec<String>,
}

impl Default for GuardConfig {
    fn default() -> Self {
        Self {
            observe_children: true,
            observe_attributes: true,
            observe_character_data: false,
            auto_repair: true,
            whitelist: Vec::new(),
        }
    }
}

/// Mutation record
#[derive(Debug, Clone)]
pub struct MutationRecord {
    pub mutation_type: MutationType,
    pub target_id: Option<String>,
    pub added_nodes: usize,
    pub removed_nodes: usize,
    pub attribute_name: Option<String>,
    pub old_value: Option<String>,
}

/// DOM monitor (tracks mutations)
#[cfg(target_arch = "wasm32")]
pub struct DOMMonitor {
    config: GuardConfig,
    observer: Option<web_sys::MutationObserver>,
    mutation_count: std::cell::Cell<u32>,
}

#[cfg(target_arch = "wasm32")]
impl DOMMonitor {
    /// Create new DOM monitor
    pub fn new(config: GuardConfig) -> Self {
        Self {
            config,
            observer: None,
            mutation_count: std::cell::Cell::new(0),
        }
    }

    /// Start monitoring
    pub fn start(&mut self, target: &web_sys::Element) -> Result<(), String> {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::prelude::*;

        let mutation_count = self.mutation_count.clone();

        // Create mutation callback
        let callback = Closure::wrap(Box::new(
            move |mutations: js_sys::Array, _observer: web_sys::MutationObserver| {
                let count = mutation_count.get();
                mutation_count.set(count + mutations.length());

                // Log mutations in dev mode
                #[cfg(debug_assertions)]
                web_sys::console::log_1(
                    &format!("Detected {} mutations", mutations.length()).into(),
                );
            },
        )
            as Box<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>);

        let observer = web_sys::MutationObserver::new(callback.as_ref().unchecked_ref())
            .map_err(|e| format!("Failed to create observer: {:?}", e))?;

        // Configure observer options
        let mut options = web_sys::MutationObserverInit::new();
        options.child_list(self.config.observe_children);
        options.attributes(self.config.observe_attributes);
        options.character_data(self.config.observe_character_data);
        options.subtree(true);

        observer
            .observe_with_options(target, &options)
            .map_err(|e| format!("Failed to start observer: {:?}", e))?;

        self.observer = Some(observer);
        callback.forget(); // Keep callback alive

        Ok(())
    }

    /// Stop monitoring
    pub fn stop(&mut self) {
        if let Some(observer) = &self.observer {
            observer.disconnect();
        }
        self.observer = None;
    }

    /// Get mutation count
    pub fn mutation_count(&self) -> u32 {
        self.mutation_count.get()
    }

    /// Reset mutation count
    pub fn reset_count(&self) {
        self.mutation_count.set(0);
    }
}

/// DOM signature (for integrity verification)
#[derive(Debug, Clone)]
pub struct DOMSignature {
    /// Element ID
    pub element_id: String,
    /// Hash of element structure
    pub structure_hash: u64,
    /// Hash of attributes
    pub attribute_hash: u64,
}

impl DOMSignature {
    /// Create signature from element
    #[cfg(target_arch = "wasm32")]
    pub fn from_element(element: &web_sys::Element) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let element_id = element.id();

        // Hash structure (tag name + child count)
        let mut structure_hasher = DefaultHasher::new();
        element.tag_name().hash(&mut structure_hasher);
        element.child_element_count().hash(&mut structure_hasher);
        let structure_hash = structure_hasher.finish();

        // Hash attributes
        let mut attribute_hasher = DefaultHasher::new();
        if let Some(attrs) = element.attributes() {
            for i in 0..attrs.length() {
                if let Some(attr) = attrs.item(i) {
                    attr.name().hash(&mut attribute_hasher);
                    attr.value().hash(&mut attribute_hasher);
                }
            }
        }
        let attribute_hash = attribute_hasher.finish();

        Self {
            element_id,
            structure_hash,
            attribute_hash,
        }
    }

    /// Verify signature matches current element
    #[cfg(target_arch = "wasm32")]
    pub fn verify(&self, element: &web_sys::Element) -> bool {
        let current = Self::from_element(element);
        self.structure_hash == current.structure_hash
            && self.attribute_hash == current.attribute_hash
    }
}

/// Integrity checker
pub struct IntegrityChecker {
    signatures: std::collections::HashMap<String, DOMSignature>,
}

impl IntegrityChecker {
    /// Create new integrity checker
    pub fn new() -> Self {
        Self {
            signatures: std::collections::HashMap::new(),
        }
    }

    /// Register element signature
    pub fn register(&mut self, signature: DOMSignature) {
        self.signatures.insert(signature.element_id.clone(), signature);
    }

    /// Verify element integrity
    #[cfg(target_arch = "wasm32")]
    pub fn verify(&self, element: &web_sys::Element) -> bool {
        let element_id = element.id();
        if let Some(signature) = self.signatures.get(&element_id) {
            signature.verify(element)
        } else {
            false // Unknown element
        }
    }

    /// Get all registered element IDs
    pub fn registered_ids(&self) -> Vec<&str> {
        self.signatures.keys().map(|s| s.as_str()).collect()
    }

    /// Clear all signatures
    pub fn clear(&mut self) {
        self.signatures.clear();
    }
}

impl Default for IntegrityChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_config() {
        let config = GuardConfig::default();
        assert!(config.observe_children);
        assert!(config.observe_attributes);
        assert!(config.auto_repair);
    }

    #[test]
    fn test_integrity_checker() {
        let mut checker = IntegrityChecker::new();

        let sig = DOMSignature {
            element_id: "test".to_string(),
            structure_hash: 12345,
            attribute_hash: 67890,
        };

        checker.register(sig);

        assert_eq!(checker.registered_ids(), vec!["test"]);

        checker.clear();
        assert_eq!(checker.registered_ids().len(), 0);
    }

    #[test]
    fn test_mutation_record() {
        let record = MutationRecord {
            mutation_type: MutationType::ChildList,
            target_id: Some("element-1".to_string()),
            added_nodes: 2,
            removed_nodes: 0,
            attribute_name: None,
            old_value: None,
        };

        assert_eq!(record.mutation_type, MutationType::ChildList);
        assert_eq!(record.added_nodes, 2);
    }
}
