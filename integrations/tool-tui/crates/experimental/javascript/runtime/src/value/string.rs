//! Interned string implementation
//!
//! Thread-safe string interning for deduplication.
//! Uses parking_lot::RwLock for efficient concurrent access without lock poisoning.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Global string interner with thread-safe access
/// Uses parking_lot::RwLock which:
/// - Does not have lock poisoning (always succeeds)
/// - Is more efficient than std::sync::RwLock
/// - Provides fair scheduling to prevent writer starvation
static STRING_INTERNER: OnceLock<RwLock<StringInterner>> = OnceLock::new();

/// Get or initialize the global string interner
fn get_interner() -> &'static RwLock<StringInterner> {
    STRING_INTERNER.get_or_init(|| RwLock::new(StringInterner::new()))
}

/// String interner for deduplication
///
/// Thread-safe when accessed through the global `intern` and `get_interned` functions.
/// The interner maintains a bidirectional mapping between strings and IDs.
pub struct StringInterner {
    strings: HashMap<String, u32>,
    by_id: Vec<String>,
}

impl StringInterner {
    /// Create a new interner
    pub fn new() -> Self {
        Self {
            strings: HashMap::new(),
            by_id: Vec::new(),
        }
    }

    /// Intern a string
    ///
    /// Returns the ID for the string. If the string was already interned,
    /// returns the existing ID. Otherwise, assigns a new ID.
    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(&id) = self.strings.get(s) {
            return id;
        }

        let id = self.by_id.len() as u32;
        self.strings.insert(s.to_string(), id);
        self.by_id.push(s.to_string());
        id
    }

    /// Get string by ID
    pub fn get(&self, id: u32) -> Option<&str> {
        self.by_id.get(id as usize).map(|s| s.as_str())
    }

    /// Check if a string is already interned (without interning it)
    pub fn contains(&self, s: &str) -> bool {
        self.strings.contains_key(s)
    }

    /// Get the ID for a string if it's already interned
    pub fn get_id(&self, s: &str) -> Option<u32> {
        self.strings.get(s).copied()
    }

    /// Get the number of interned strings
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Check if the interner is empty
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the global interner
///
/// This is called automatically by `intern` and `get_interned`, but can be
/// called explicitly for eager initialization.
pub fn init_interner() {
    // Force initialization of the global interner
    let _ = get_interner();
}

/// Intern a string globally (thread-safe)
///
/// Returns a unique ID for the string. Identical strings will always
/// return the same ID. This function is safe to call from multiple threads.
///
/// # Thread Safety
///
/// This function uses a read-write lock internally:
/// - First attempts a read lock to check if string exists
/// - Only acquires write lock if string needs to be added
/// - parking_lot::RwLock ensures no lock poisoning
pub fn intern(s: &str) -> u32 {
    let interner = get_interner();

    // First, try to find the string with a read lock (fast path)
    {
        let guard = interner.read();
        if let Some(&id) = guard.strings.get(s) {
            return id;
        }
    }

    // String not found, need to intern it with a write lock
    let mut guard = interner.write();

    // Double-check in case another thread interned it while we waited
    if let Some(&id) = guard.strings.get(s) {
        return id;
    }

    // Intern the string
    guard.intern(s)
}

/// Get interned string by ID (thread-safe)
///
/// Returns the string associated with the given ID, or None if the ID
/// is not valid. This function is safe to call from multiple threads.
pub fn get_interned(id: u32) -> Option<String> {
    let interner = get_interner();
    let guard = interner.read();
    guard.get(id).map(|s| s.to_string())
}

/// Check if a string is already interned (thread-safe)
///
/// Returns true if the string has been interned, false otherwise.
/// This does not intern the string if it's not already present.
pub fn is_interned(s: &str) -> bool {
    let interner = get_interner();
    let guard = interner.read();
    guard.contains(s)
}

/// Get the ID for a string if it's already interned (thread-safe)
///
/// Returns Some(id) if the string is interned, None otherwise.
/// This does not intern the string if it's not already present.
pub fn get_id(s: &str) -> Option<u32> {
    let interner = get_interner();
    let guard = interner.read();
    guard.get_id(s)
}

/// Get the number of interned strings (thread-safe)
pub fn interned_count() -> usize {
    let interner = get_interner();
    let guard = interner.read();
    guard.len()
}
