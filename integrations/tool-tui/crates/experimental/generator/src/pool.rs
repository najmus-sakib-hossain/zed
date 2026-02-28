//! Template Pool - Feature #12
//!
//! Memory pool for templates using SharedArrayBuffer-style linear memory.
//! Enables zero-copy template access and parallel generation.

use crate::MAX_POOL_SIZE;
use crate::error::{GeneratorError, Result};
use crate::template::{Template, TemplateHandle};
use parking_lot::RwLock;
use std::sync::Arc;

// ============================================================================
// Pool Slot
// ============================================================================

/// A slot in the template pool.
#[derive(Debug)]
struct PoolSlot {
    /// Template data (if occupied).
    template: Option<Arc<Template>>,
    /// Generation counter (for ABA prevention).
    generation: u16,
}

impl Default for PoolSlot {
    fn default() -> Self {
        Self {
            template: None,
            generation: 0,
        }
    }
}

// ============================================================================
// Template Pool
// ============================================================================

/// Memory pool for compiled templates.
///
/// Templates are stored in a fixed-size pool and accessed by handle.
/// This enables:
/// - Zero-copy template sharing across threads
/// - Predictable memory usage
/// - Fast template lookup by ID
///
/// # Example
///
/// ```rust,ignore
/// use dx_generator::TemplatePool;
///
/// let pool = TemplatePool::new(256);
///
/// // Add a template
/// let handle = pool.insert(template)?;
///
/// // Access by handle
/// if let Some(template) = pool.get(handle) {
///     // Use template
/// }
/// ```
#[derive(Debug)]
pub struct TemplatePool {
    /// Pool slots.
    slots: RwLock<Vec<PoolSlot>>,
    /// Template ID to slot mapping.
    id_to_slot: RwLock<std::collections::HashMap<u32, u16>>,
    /// Maximum pool size.
    max_size: usize,
    /// Free slot list.
    free_slots: RwLock<Vec<u16>>,
}

impl TemplatePool {
    /// Create a new template pool.
    #[must_use]
    pub fn new(max_size: usize) -> Self {
        let max_size = max_size.min(MAX_POOL_SIZE);
        let mut slots = Vec::with_capacity(max_size);
        let mut free_slots = Vec::with_capacity(max_size);

        for i in 0..max_size {
            slots.push(PoolSlot::default());
            free_slots.push(i as u16);
        }

        // Reverse so we pop from the front (lower indices first)
        free_slots.reverse();

        Self {
            slots: RwLock::new(slots),
            id_to_slot: RwLock::new(std::collections::HashMap::new()),
            max_size,
            free_slots: RwLock::new(free_slots),
        }
    }

    /// Insert a template into the pool.
    pub fn insert(&self, template: Template) -> Result<TemplateHandle> {
        let template_id = template.id();

        // Check if already in pool
        {
            let id_map = self.id_to_slot.read();
            if let Some(&slot) = id_map.get(&template_id) {
                let slots = self.slots.read();
                if let Some(pool_slot) = slots.get(slot as usize) {
                    return Ok(TemplateHandle::new(template_id, slot, pool_slot.generation));
                }
            }
        }

        // Allocate a slot
        let slot = {
            let mut free = self.free_slots.write();
            free.pop().ok_or_else(|| GeneratorError::CacheFull {
                count: self.max_size,
                max_count: self.max_size,
            })?
        };

        // Insert template
        let generation = {
            let mut slots = self.slots.write();
            let pool_slot = &mut slots[slot as usize];
            pool_slot.generation = pool_slot.generation.wrapping_add(1);
            pool_slot.template = Some(Arc::new(template));
            pool_slot.generation
        };

        // Update ID mapping
        {
            let mut id_map = self.id_to_slot.write();
            id_map.insert(template_id, slot);
        }

        Ok(TemplateHandle::new(template_id, slot, generation))
    }

    /// Get a template by handle.
    #[must_use]
    pub fn get(&self, handle: TemplateHandle) -> Option<Arc<Template>> {
        if handle.is_null() {
            return None;
        }

        let slots = self.slots.read();
        let slot = slots.get(handle.slot as usize)?;

        // Check generation
        if slot.generation != handle.generation {
            return None;
        }

        slot.template.clone()
    }

    /// Get a template by ID.
    #[must_use]
    pub fn get_by_id(&self, template_id: u32) -> Option<Arc<Template>> {
        let id_map = self.id_to_slot.read();
        let slot = *id_map.get(&template_id)?;

        let slots = self.slots.read();
        slots.get(slot as usize)?.template.clone()
    }

    /// Remove a template by handle.
    pub fn remove(&self, handle: TemplateHandle) -> Option<Arc<Template>> {
        if handle.is_null() {
            return None;
        }

        let template = {
            let mut slots = self.slots.write();
            let slot = slots.get_mut(handle.slot as usize)?;

            if slot.generation != handle.generation {
                return None;
            }

            slot.template.take()
        };

        if template.is_some() {
            // Return slot to free list
            let mut free = self.free_slots.write();
            free.push(handle.slot);

            // Remove ID mapping
            let mut id_map = self.id_to_slot.write();
            id_map.remove(&handle.id);
        }

        template
    }

    /// Get the number of templates in the pool.
    #[must_use]
    pub fn len(&self) -> usize {
        self.max_size - self.free_slots.read().len()
    }

    /// Check if the pool is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if the pool is full.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.free_slots.read().is_empty()
    }

    /// Get pool capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.max_size
    }

    /// Clear all templates from the pool.
    pub fn clear(&self) {
        let mut slots = self.slots.write();
        let mut free = self.free_slots.write();
        let mut id_map = self.id_to_slot.write();

        for (i, slot) in slots.iter_mut().enumerate() {
            if slot.template.take().is_some() {
                slot.generation = slot.generation.wrapping_add(1);
                free.push(i as u16);
            }
        }

        id_map.clear();
    }

    /// Get all template handles.
    #[must_use]
    pub fn handles(&self) -> Vec<TemplateHandle> {
        let slots = self.slots.read();
        let id_map = self.id_to_slot.read();

        id_map
            .iter()
            .filter_map(|(&id, &slot)| {
                let pool_slot = slots.get(slot as usize)?;
                if pool_slot.template.is_some() {
                    Some(TemplateHandle::new(id, slot, pool_slot.generation))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for TemplatePool {
    fn default() -> Self {
        Self::new(256)
    }
}

// ============================================================================
// Shared Pool
// ============================================================================

/// Thread-safe shared template pool.
pub type SharedPool = Arc<TemplatePool>;

/// Create a shared template pool.
#[must_use]
pub fn shared_pool(max_size: usize) -> SharedPool {
    Arc::new(TemplatePool::new(max_size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary::BinaryTemplate;

    fn make_test_template(name: &str) -> Template {
        let builder = BinaryTemplate::builder(name);
        let binary = builder.build();
        Template::from_bytes(binary.to_bytes()).unwrap()
    }

    #[test]
    fn test_pool_insert_get() {
        let pool = TemplatePool::new(10);
        let template = make_test_template("test");

        let handle = pool.insert(template).unwrap();
        assert!(!handle.is_null());

        let retrieved = pool.get(handle).unwrap();
        assert_eq!(retrieved.name(), "test");
    }

    #[test]
    fn test_pool_get_by_id() {
        let pool = TemplatePool::new(10);
        let template = make_test_template("test");
        let id = template.id();

        pool.insert(template).unwrap();

        let retrieved = pool.get_by_id(id).unwrap();
        assert_eq!(retrieved.name(), "test");
    }

    #[test]
    fn test_pool_remove() {
        let pool = TemplatePool::new(10);
        let template = make_test_template("test");

        let handle = pool.insert(template).unwrap();
        assert_eq!(pool.len(), 1);

        pool.remove(handle);
        assert_eq!(pool.len(), 0);
        assert!(pool.get(handle).is_none());
    }

    #[test]
    fn test_pool_generation() {
        let pool = TemplatePool::new(10);

        // Insert and remove
        let template1 = make_test_template("test1");
        let handle1 = pool.insert(template1).unwrap();
        pool.remove(handle1);

        // Insert new template in same slot
        let template2 = make_test_template("test2");
        let handle2 = pool.insert(template2).unwrap();

        // Old handle should be invalid (generation mismatch)
        assert!(pool.get(handle1).is_none());
        assert!(pool.get(handle2).is_some());
    }

    #[test]
    fn test_pool_full() {
        let pool = TemplatePool::new(2);

        pool.insert(make_test_template("t1")).unwrap();
        pool.insert(make_test_template("t2")).unwrap();

        // Pool is full
        let result = pool.insert(make_test_template("t3"));
        assert!(result.is_err());
    }

    #[test]
    fn test_pool_clear() {
        let pool = TemplatePool::new(10);

        pool.insert(make_test_template("t1")).unwrap();
        pool.insert(make_test_template("t2")).unwrap();
        assert_eq!(pool.len(), 2);

        pool.clear();
        assert_eq!(pool.len(), 0);
        assert!(pool.is_empty());
    }
}
