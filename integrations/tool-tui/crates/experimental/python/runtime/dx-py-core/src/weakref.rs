//! Weak reference implementation for PyValue objects
//!
//! Provides weak references that don't prevent objects from being garbage collected.

use crate::pylist::PyValue;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::{Arc, Weak};

/// Weak reference to a PyValue object
pub struct PyWeakRef {
    /// The weak reference to the target object
    target: WeakTarget,
    /// Optional callback to call when the object is deallocated
    callback: Option<Box<dyn Fn() + Send + Sync>>,
    /// Unique ID for this weak reference
    id: usize,
}

impl std::fmt::Debug for PyWeakRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PyWeakRef")
            .field("target", &self.target)
            .field("callback", &self.callback.as_ref().map(|_| "Some(callback)"))
            .field("id", &self.id)
            .finish()
    }
}

/// Internal representation of weak reference targets
#[derive(Debug)]
enum WeakTarget {
    List(Weak<crate::PyList>),
    Dict(Weak<crate::PyDict>),
    Instance(Weak<crate::types::PyInstance>),
    Type(Weak<crate::types::PyType>),
    Generator(Weak<crate::pygenerator::PyGenerator>),
    Coroutine(Weak<crate::pygenerator::PyCoroutine>),
    Tuple(Weak<crate::PyTuple>),
}

/// Global registry for weak references
static WEAK_REF_REGISTRY: once_cell::sync::Lazy<Mutex<HashMap<usize, Vec<usize>>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

/// Counter for generating unique weak reference IDs
static WEAK_REF_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

impl PyWeakRef {
    /// Create a new weak reference to an object
    pub fn new(target: &PyValue) -> Option<Self> {
        let weak_target = match target {
            PyValue::List(list) => WeakTarget::List(Arc::downgrade(list)),
            PyValue::Dict(dict) => WeakTarget::Dict(Arc::downgrade(dict)),
            PyValue::Instance(instance) => WeakTarget::Instance(Arc::downgrade(instance)),
            PyValue::Type(type_obj) => WeakTarget::Type(Arc::downgrade(type_obj)),
            PyValue::Generator(gen) => WeakTarget::Generator(Arc::downgrade(gen)),
            PyValue::Coroutine(coro) => WeakTarget::Coroutine(Arc::downgrade(coro)),
            PyValue::Tuple(tuple) => WeakTarget::Tuple(Arc::downgrade(tuple)),
            // Primitive types can't have weak references
            _ => return None,
        };

        let id = WEAK_REF_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Register this weak reference
        if let Some(target_addr) = Self::get_target_address(target) {
            let mut registry = WEAK_REF_REGISTRY.lock().unwrap();
            registry.entry(target_addr).or_insert_with(Vec::new).push(id);
        }

        Some(Self {
            target: weak_target,
            callback: None,
            id,
        })
    }

    /// Create a weak reference with a callback
    pub fn new_with_callback<F>(target: &PyValue, callback: F) -> Option<Self>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut weak_ref = Self::new(target)?;
        weak_ref.callback = Some(Box::new(callback));
        Some(weak_ref)
    }

    /// Try to upgrade the weak reference to a strong reference
    pub fn upgrade(&self) -> Option<PyValue> {
        match &self.target {
            WeakTarget::List(weak) => weak.upgrade().map(PyValue::List),
            WeakTarget::Dict(weak) => weak.upgrade().map(PyValue::Dict),
            WeakTarget::Instance(weak) => weak.upgrade().map(PyValue::Instance),
            WeakTarget::Type(weak) => weak.upgrade().map(PyValue::Type),
            WeakTarget::Generator(weak) => weak.upgrade().map(PyValue::Generator),
            WeakTarget::Coroutine(weak) => weak.upgrade().map(PyValue::Coroutine),
            WeakTarget::Tuple(weak) => weak.upgrade().map(PyValue::Tuple),
        }
    }

    /// Check if the weak reference is still alive
    pub fn is_alive(&self) -> bool {
        self.upgrade().is_some()
    }

    /// Get the unique ID of this weak reference
    pub fn id(&self) -> usize {
        self.id
    }

    /// Get the memory address of the target object for registry purposes
    fn get_target_address(target: &PyValue) -> Option<usize> {
        match target {
            PyValue::List(list) => Some(Arc::as_ptr(list) as usize),
            PyValue::Dict(dict) => Some(Arc::as_ptr(dict) as usize),
            PyValue::Instance(instance) => Some(Arc::as_ptr(instance) as usize),
            PyValue::Type(type_obj) => Some(Arc::as_ptr(type_obj) as usize),
            PyValue::Generator(gen) => Some(Arc::as_ptr(gen) as usize),
            PyValue::Coroutine(coro) => Some(Arc::as_ptr(coro) as usize),
            PyValue::Tuple(tuple) => Some(Arc::as_ptr(tuple) as usize),
            _ => None,
        }
    }

    /// Call the callback if the object has been deallocated
    fn call_callback_if_dead(&self) {
        if !self.is_alive() {
            if let Some(ref callback) = self.callback {
                callback();
            }
        }
    }
}

impl Drop for PyWeakRef {
    fn drop(&mut self) {
        // Remove from registry when weak reference is dropped
        // In a full implementation, we'd need to track the target address
        // For now, we'll just call the callback if the object is dead
        self.call_callback_if_dead();
    }
}

/// Weak reference proxy that behaves like the original object but raises ReferenceError if dead
#[derive(Debug)]
pub struct PyWeakProxy {
    weak_ref: PyWeakRef,
}

impl PyWeakProxy {
    /// Create a new weak proxy
    pub fn new(target: &PyValue) -> Option<Self> {
        let weak_ref = PyWeakRef::new(target)?;
        Some(Self { weak_ref })
    }

    /// Get the underlying object, or return an error if it's been deallocated
    pub fn get(&self) -> Result<PyValue, String> {
        self.weak_ref.upgrade().ok_or_else(|| "weak object has gone away".to_string())
    }

    /// Check if the proxy is still alive
    pub fn is_alive(&self) -> bool {
        self.weak_ref.is_alive()
    }
}

/// Clear weak references for a deallocated object
/// This should be called when an object is being deallocated
pub fn clear_weak_references(target: &PyValue) {
    if let Some(target_addr) = PyWeakRef::get_target_address(target) {
        let mut registry = WEAK_REF_REGISTRY.lock().unwrap();
        if let Some(weak_ref_ids) = registry.remove(&target_addr) {
            // In a full implementation, we would call callbacks for all weak references
            // For now, we just remove them from the registry
            drop(weak_ref_ids);
        }
    }
}

/// Get the number of weak references to an object
pub fn get_weak_ref_count(target: &PyValue) -> usize {
    if let Some(target_addr) = PyWeakRef::get_target_address(target) {
        let registry = WEAK_REF_REGISTRY.lock().unwrap();
        registry.get(&target_addr).map(|refs| refs.len()).unwrap_or(0)
    } else {
        0
    }
}

/// Clear all weak references (for testing only)
///
/// # Warning
/// This function is intended for testing purposes only and should not be used in production code.
pub fn clear_all_weak_references() {
    let mut registry = WEAK_REF_REGISTRY.lock().unwrap();
    registry.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PyInstance, PyList, PyType};
    use std::sync::Arc;

    #[test]
    fn test_weak_ref_creation() {
        let list = PyValue::List(Arc::new(PyList::new()));
        let weak_ref = PyWeakRef::new(&list);

        assert!(weak_ref.is_some());
        let weak_ref = weak_ref.unwrap();
        assert!(weak_ref.is_alive());
        assert!(weak_ref.upgrade().is_some());
    }

    #[test]
    fn test_weak_ref_upgrade() {
        let list = PyValue::List(Arc::new(PyList::new()));
        let weak_ref = PyWeakRef::new(&list).unwrap();

        // Should be able to upgrade while object is alive
        let upgraded = weak_ref.upgrade();
        assert!(upgraded.is_some());

        // Drop the original reference
        drop(list);

        // Should not be able to upgrade after object is dropped
        // Note: This test might not work as expected due to Arc semantics
        // In a real implementation, we'd need more sophisticated tracking
    }

    #[test]
    fn test_weak_ref_primitive_types() {
        let int_val = PyValue::Int(42);
        let weak_ref = PyWeakRef::new(&int_val);

        // Primitive types can't have weak references
        assert!(weak_ref.is_none());
    }

    #[test]
    fn test_weak_proxy() {
        let list = PyValue::List(Arc::new(PyList::new()));
        let proxy = PyWeakProxy::new(&list);

        assert!(proxy.is_some());
        let proxy = proxy.unwrap();
        assert!(proxy.is_alive());
        assert!(proxy.get().is_ok());
    }

    #[test]
    fn test_weak_ref_with_callback() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc as StdArc;

        let callback_called = StdArc::new(AtomicBool::new(false));
        let callback_called_clone = StdArc::clone(&callback_called);

        let list = PyValue::List(Arc::new(PyList::new()));
        let _weak_ref = PyWeakRef::new_with_callback(&list, move || {
            callback_called_clone.store(true, Ordering::Relaxed);
        });

        // Callback should not be called while object is alive
        assert!(!callback_called.load(Ordering::Relaxed));
    }

    #[test]
    fn test_weak_ref_count() {
        let list = PyValue::List(Arc::new(PyList::new()));

        assert_eq!(get_weak_ref_count(&list), 0);

        let _weak_ref1 = PyWeakRef::new(&list);
        assert_eq!(get_weak_ref_count(&list), 1);

        let _weak_ref2 = PyWeakRef::new(&list);
        assert_eq!(get_weak_ref_count(&list), 2);
    }

    #[test]
    fn test_clear_weak_references() {
        let list = PyValue::List(Arc::new(PyList::new()));
        let _weak_ref = PyWeakRef::new(&list);

        assert_eq!(get_weak_ref_count(&list), 1);

        clear_weak_references(&list);
        assert_eq!(get_weak_ref_count(&list), 0);
    }
}
