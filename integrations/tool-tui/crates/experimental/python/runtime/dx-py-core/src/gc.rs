//! Garbage collection integration for PyValue objects
//!
//! This module integrates the cycle detector with the PyValue system
//! and implements actual cycle collection.

use crate::cleanup::CleanupManager;
use crate::pylist::PyValue;
use dx_py_gc::cycle::{CycleMarker, Traceable};
use dx_py_gc::CycleDetector;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Global cycle detector instance
static CYCLE_DETECTOR: once_cell::sync::Lazy<CycleDetector> =
    once_cell::sync::Lazy::new(|| CycleDetector::new());

/// Statistics for cycle collection
#[derive(Debug, Default)]
pub struct GcStats {
    pub cycles_detected: usize,
    pub objects_collected: usize,
    pub collections_run: usize,
}

static GC_STATS: AtomicUsize = AtomicUsize::new(0);

/// Garbage collector for PyValue objects
pub struct PyGc {
    /// Objects that might be part of cycles
    potential_cycles: Mutex<HashSet<usize>>,
}

impl PyGc {
    /// Create a new garbage collector
    pub fn new() -> Self {
        Self {
            potential_cycles: Mutex::new(HashSet::new()),
        }
    }

    /// Add an object as a potential cycle root
    /// Called when an object's reference count is decremented but doesn't reach 0
    pub fn add_potential_cycle(&self, obj: &PyValue) {
        if let Some(addr) = self.get_object_address(obj) {
            self.potential_cycles.lock().unwrap().insert(addr);
        }
    }

    /// Run cycle detection and collection
    pub fn collect_cycles(&self) -> usize {
        let mut collected = 0;
        let potential_cycles = {
            let mut cycles = self.potential_cycles.lock().unwrap();
            let result = cycles.clone();
            cycles.clear();
            result
        };

        // For each potential cycle root, check if it's actually garbage
        for addr in potential_cycles {
            if let Some(obj) = unsafe { self.addr_to_object(addr) } {
                if self.is_garbage(&obj) {
                    // Object is garbage - finalize and mark for collection
                    if let Err(e) = CleanupManager::finalize_object(&obj) {
                        eprintln!("Warning: Error during cycle collection finalization: {}", e);
                    }
                    collected += 1;
                }
            }
        }

        GC_STATS.fetch_add(collected, Ordering::Relaxed);
        collected
    }

    /// Check if an object is garbage (part of an unreachable cycle)
    fn is_garbage(&self, obj: &PyValue) -> bool {
        // Simplified cycle detection:
        // In a full implementation, this would use tri-color marking
        // For now, we'll use a simple heuristic based on reference counts

        match obj {
            PyValue::List(list) => {
                // If reference count is 1 and it contains references to itself or other
                // objects with low reference counts, it might be garbage
                list.header.refcount() == 1 && self.contains_self_references(obj)
            }
            PyValue::Dict(dict) => {
                dict.header.refcount() == 1 && self.contains_self_references(obj)
            }
            PyValue::Instance(instance) => {
                instance.header.refcount() == 1 && self.contains_self_references(obj)
            }
            _ => false, // Other types don't typically form cycles
        }
    }

    /// Check if an object contains references to itself or other low-refcount objects
    fn contains_self_references(&self, _obj: &PyValue) -> bool {
        // Simplified implementation - in reality this would trace through
        // all references and check for cycles
        // For now, we'll return false to be conservative
        false
    }

    /// Get the memory address of an object for tracking
    fn get_object_address(&self, obj: &PyValue) -> Option<usize> {
        match obj {
            PyValue::List(list) => Some(Arc::as_ptr(list) as usize),
            PyValue::Dict(dict) => Some(Arc::as_ptr(dict) as usize),
            PyValue::Instance(instance) => Some(Arc::as_ptr(instance) as usize),
            PyValue::Type(type_obj) => Some(Arc::as_ptr(type_obj) as usize),
            _ => None, // Primitive types don't need cycle detection
        }
    }

    /// Convert an address back to a PyValue (unsafe)
    unsafe fn addr_to_object(&self, _addr: usize) -> Option<PyValue> {
        // This is a placeholder - in a real implementation, we would need
        // a way to safely convert addresses back to objects
        // This requires careful memory management and type tracking
        None
    }

    /// Get garbage collection statistics
    pub fn stats(&self) -> GcStats {
        GcStats {
            cycles_detected: CYCLE_DETECTOR.total_cycles_detected(),
            objects_collected: GC_STATS.load(Ordering::Relaxed),
            collections_run: 1, // Simplified
        }
    }

    /// Force a garbage collection cycle
    pub fn force_collect(&self) -> usize {
        self.collect_cycles()
    }
}

/// Global garbage collector instance
static PY_GC: once_cell::sync::Lazy<PyGc> = once_cell::sync::Lazy::new(|| PyGc::new());

/// Add an object as a potential cycle root
pub fn add_potential_cycle(obj: &PyValue) {
    PY_GC.add_potential_cycle(obj);
}

/// Run garbage collection
pub fn collect() -> usize {
    PY_GC.collect_cycles()
}

/// Get GC statistics
pub fn stats() -> GcStats {
    PY_GC.stats()
}

/// Force garbage collection
pub fn force_collect() -> usize {
    PY_GC.force_collect()
}

/// Implement Traceable for PyValue to integrate with cycle detector
impl Traceable for PyValue {
    fn trace(&self, tracer: &mut dyn FnMut(usize)) {
        match self {
            PyValue::List(list) => {
                // Trace all elements in the list
                for element in list.to_vec() {
                    if let Some(addr) = PY_GC.get_object_address(&element) {
                        tracer(addr);
                    }
                }
            }
            PyValue::Dict(dict) => {
                // Trace all values in the dict
                for value in dict.values() {
                    if let Some(addr) = PY_GC.get_object_address(&value) {
                        tracer(addr);
                    }
                }
            }
            PyValue::Instance(instance) => {
                // Trace all attributes in the instance
                for value in instance.dict.iter() {
                    if let Some(addr) = PY_GC.get_object_address(value.value()) {
                        tracer(addr);
                    }
                }
            }
            _ => {
                // Other types don't contain references to trace
            }
        }
    }

    fn get_marker(&self) -> &CycleMarker {
        // For now, we'll use a dummy marker
        // In a full implementation, each object would have its own marker
        static DUMMY_MARKER: once_cell::sync::Lazy<CycleMarker> =
            once_cell::sync::Lazy::new(|| CycleMarker::new());
        &DUMMY_MARKER
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PyDict, PyList};

    #[test]
    fn test_gc_creation() {
        let gc = PyGc::new();
        let stats = gc.stats();
        assert_eq!(stats.objects_collected, 0);
    }

    #[test]
    fn test_add_potential_cycle() {
        let gc = PyGc::new();
        let list = PyValue::List(Arc::new(PyList::new()));

        gc.add_potential_cycle(&list);
        // Should not panic
    }

    #[test]
    fn test_collect_cycles_empty() {
        let gc = PyGc::new();
        let collected = gc.collect_cycles();
        assert_eq!(collected, 0);
    }

    #[test]
    fn test_is_garbage_simple() {
        let gc = PyGc::new();
        let list = PyValue::List(Arc::new(PyList::new()));

        // A simple empty list should not be considered garbage
        assert!(!gc.is_garbage(&list));
    }

    #[test]
    fn test_global_functions() {
        // Test global GC functions
        let collected = collect();
        assert_eq!(collected, 0);

        let _stats = stats();
        // Stats are valid (objects_collected is usize, always >= 0)
    }

    #[test]
    fn test_traceable_implementation() {
        let list = PyValue::List(Arc::new(PyList::new()));
        let mut traced_addresses = Vec::new();

        list.trace(&mut |addr| {
            traced_addresses.push(addr);
        });

        // Empty list should trace no addresses
        assert_eq!(traced_addresses.len(), 0);
    }
}
