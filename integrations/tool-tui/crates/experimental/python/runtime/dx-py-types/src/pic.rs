//! Polymorphic Inline Cache (PIC) for 2-4 types

use dx_py_jit::profile::PyType;
use std::sync::atomic::{AtomicPtr, AtomicU8, Ordering};

/// Maximum entries in a PIC
pub const PIC_MAX_ENTRIES: usize = 4;

/// Entry in a polymorphic inline cache
#[repr(C)]
pub struct PicEntry {
    /// Type tag
    type_tag: AtomicU8,
    /// Pointer to specialized code
    code_ptr: AtomicPtr<u8>,
}

impl PicEntry {
    fn new() -> Self {
        Self {
            type_tag: AtomicU8::new(PyType::Unknown as u8),
            code_ptr: AtomicPtr::new(std::ptr::null_mut()),
        }
    }
}

/// Polymorphic Inline Cache (PIC) - up to 4 types
#[repr(C)]
pub struct PolymorphicInlineCache {
    /// Cache entries
    entries: [PicEntry; PIC_MAX_ENTRIES],
    /// Number of entries used
    entry_count: AtomicU8,
}

impl PolymorphicInlineCache {
    /// Create a new empty PIC
    pub fn new() -> Self {
        Self {
            entries: [
                PicEntry::new(),
                PicEntry::new(),
                PicEntry::new(),
                PicEntry::new(),
            ],
            entry_count: AtomicU8::new(0),
        }
    }

    /// Look up specialized code for a type
    ///
    /// Returns the code pointer if found, None otherwise.
    #[inline(always)]
    pub fn lookup(&self, obj_type: PyType) -> Option<*const u8> {
        let count = self.entry_count.load(Ordering::Relaxed) as usize;
        let type_byte = obj_type as u8;

        for i in 0..count.min(PIC_MAX_ENTRIES) {
            if self.entries[i].type_tag.load(Ordering::Relaxed) == type_byte {
                let code = self.entries[i].code_ptr.load(Ordering::Acquire);
                if !code.is_null() {
                    return Some(code as *const u8);
                }
            }
        }

        None
    }

    /// Add an entry to the PIC
    ///
    /// Returns true if the entry was added, false if the PIC is full.
    pub fn add_entry(&self, obj_type: PyType, code: *const u8) -> bool {
        let count = self.entry_count.load(Ordering::Relaxed) as usize;

        if count >= PIC_MAX_ENTRIES {
            return false; // PIC is full, should transition to megamorphic
        }

        // Check if type already exists
        let type_byte = obj_type as u8;
        for i in 0..count {
            if self.entries[i].type_tag.load(Ordering::Relaxed) == type_byte {
                // Update existing entry
                self.entries[i].code_ptr.store(code as *mut u8, Ordering::Release);
                return true;
            }
        }

        // Add new entry
        self.entries[count].type_tag.store(type_byte, Ordering::Relaxed);
        self.entries[count].code_ptr.store(code as *mut u8, Ordering::Release);
        self.entry_count.fetch_add(1, Ordering::Release);

        true
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entry_count.load(Ordering::Relaxed) as usize
    }

    /// Check if the PIC is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if the PIC is full
    pub fn is_full(&self) -> bool {
        self.len() >= PIC_MAX_ENTRIES
    }

    /// Get all types in the PIC
    pub fn get_types(&self) -> Vec<PyType> {
        let count = self.len();
        (0..count)
            .map(|i| PyType::from_u8(self.entries[i].type_tag.load(Ordering::Relaxed)))
            .collect()
    }

    /// Reset the PIC
    pub fn reset(&self) {
        for entry in &self.entries {
            entry.type_tag.store(PyType::Unknown as u8, Ordering::Relaxed);
            entry.code_ptr.store(std::ptr::null_mut(), Ordering::Release);
        }
        self.entry_count.store(0, Ordering::Release);
    }
}

impl Default for PolymorphicInlineCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pic_new() {
        let pic = PolymorphicInlineCache::new();
        assert!(pic.is_empty());
        assert!(!pic.is_full());
    }

    #[test]
    fn test_pic_add_and_lookup() {
        let pic = PolymorphicInlineCache::new();

        let code1 = 0x1000 as *const u8;
        let code2 = 0x2000 as *const u8;

        assert!(pic.add_entry(PyType::Int, code1));
        assert!(pic.add_entry(PyType::Float, code2));

        assert_eq!(pic.len(), 2);

        assert_eq!(pic.lookup(PyType::Int), Some(code1));
        assert_eq!(pic.lookup(PyType::Float), Some(code2));
        assert_eq!(pic.lookup(PyType::Str), None);
    }

    #[test]
    fn test_pic_full() {
        let pic = PolymorphicInlineCache::new();

        assert!(pic.add_entry(PyType::Int, std::ptr::null()));
        assert!(pic.add_entry(PyType::Float, std::ptr::null()));
        assert!(pic.add_entry(PyType::Str, std::ptr::null()));
        assert!(pic.add_entry(PyType::List, std::ptr::null()));

        assert!(pic.is_full());

        // Should fail to add more
        assert!(!pic.add_entry(PyType::Dict, std::ptr::null()));
    }

    #[test]
    fn test_pic_update_existing() {
        let pic = PolymorphicInlineCache::new();

        let code1 = 0x1000 as *const u8;
        let code2 = 0x2000 as *const u8;

        pic.add_entry(PyType::Int, code1);
        assert_eq!(pic.lookup(PyType::Int), Some(code1));

        // Update with new code
        pic.add_entry(PyType::Int, code2);
        assert_eq!(pic.lookup(PyType::Int), Some(code2));

        // Should still have only 1 entry
        assert_eq!(pic.len(), 1);
    }

    #[test]
    fn test_pic_get_types() {
        let pic = PolymorphicInlineCache::new();

        pic.add_entry(PyType::Int, std::ptr::null());
        pic.add_entry(PyType::Float, std::ptr::null());

        let types = pic.get_types();
        assert_eq!(types.len(), 2);
        assert!(types.contains(&PyType::Int));
        assert!(types.contains(&PyType::Float));
    }
}
