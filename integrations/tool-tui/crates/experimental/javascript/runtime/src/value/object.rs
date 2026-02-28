//! Object representation

use std::collections::HashMap;

/// JavaScript object
#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    /// Properties
    properties: HashMap<String, super::Value>,
    /// Prototype
    prototype: Option<Box<Object>>,
    /// Whether the object is frozen (no modifications allowed)
    frozen: bool,
    /// Whether the object is sealed (no new properties, but existing can be modified)
    sealed: bool,
}

impl Object {
    /// Create a new empty object
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
            prototype: None,
            frozen: false,
            sealed: false,
        }
    }

    /// Get a property
    pub fn get(&self, key: &str) -> Option<&super::Value> {
        self.properties
            .get(key)
            .or_else(|| self.prototype.as_ref().and_then(|p| p.get(key)))
    }

    /// Set a property
    pub fn set(&mut self, key: String, value: super::Value) -> bool {
        if self.frozen {
            return false;
        }
        if self.sealed && !self.properties.contains_key(&key) {
            return false;
        }
        self.properties.insert(key, value);
        true
    }

    /// Check if has own property
    pub fn has_own(&self, key: &str) -> bool {
        self.properties.contains_key(key)
    }

    /// Get all own keys
    pub fn keys(&self) -> Vec<&String> {
        self.properties.keys().collect()
    }

    /// Get all own keys as owned strings
    pub fn keys_owned(&self) -> Vec<String> {
        self.properties.keys().cloned().collect()
    }

    /// Get all own values
    pub fn values(&self) -> Vec<&super::Value> {
        self.properties.values().collect()
    }

    /// Get all own values as cloned
    pub fn values_cloned(&self) -> Vec<super::Value> {
        self.properties.values().cloned().collect()
    }

    /// Get all own entries as (key, value) pairs
    pub fn entries(&self) -> Vec<(&String, &super::Value)> {
        self.properties.iter().collect()
    }

    /// Get all own entries as cloned (key, value) pairs
    pub fn entries_cloned(&self) -> Vec<(String, super::Value)> {
        self.properties.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Freeze the object (prevent all modifications)
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    /// Check if object is frozen
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    /// Seal the object (prevent adding new properties)
    pub fn seal(&mut self) {
        self.sealed = true;
    }

    /// Check if object is sealed
    pub fn is_sealed(&self) -> bool {
        self.sealed
    }

    /// Copy all enumerable properties from another object
    pub fn assign_from(&mut self, source: &Object) {
        if self.frozen {
            return;
        }
        for (key, value) in &source.properties {
            if !self.sealed || self.properties.contains_key(key) {
                self.properties.insert(key.clone(), value.clone());
            }
        }
    }

    /// Get the number of own properties
    pub fn len(&self) -> usize {
        self.properties.len()
    }

    /// Check if object has no own properties
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }
}

impl Default for Object {
    fn default() -> Self {
        Self::new()
    }
}
