//! Swiss table dictionary with SIMD probe

use std::mem;

/// Control byte values for Swiss table
mod ctrl {
    pub const EMPTY: u8 = 0b1111_1111;
    #[allow(dead_code)]
    pub const DELETED: u8 = 0b1000_0000;

    /// Check if control byte indicates empty slot
    #[inline]
    pub fn is_empty(ctrl: u8) -> bool {
        ctrl == EMPTY
    }

    /// Check if control byte indicates deleted slot
    #[inline]
    #[allow(dead_code)]
    pub fn is_deleted(ctrl: u8) -> bool {
        ctrl == DELETED
    }

    /// Check if control byte indicates full slot
    #[inline]
    pub fn is_full(ctrl: u8) -> bool {
        ctrl & 0b1000_0000 == 0
    }

    /// Get the H2 hash (lower 7 bits)
    #[inline]
    pub fn h2(hash: u64) -> u8 {
        (hash & 0x7F) as u8
    }
}

/// Entry in the Swiss dict
#[derive(Clone)]
struct Entry<K, V> {
    key: K,
    value: V,
}

/// Swiss table dictionary with SIMD probe
pub struct SwissDict<K, V> {
    /// Control bytes for SIMD matching
    ctrl: Vec<u8>,
    /// Entries (parallel to ctrl)
    entries: Vec<Option<Entry<K, V>>>,
    /// Number of occupied slots
    len: usize,
    /// Number of slots (power of 2)
    capacity: usize,
    /// Growth threshold
    growth_left: usize,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> SwissDict<K, V> {
    /// Create a new empty dictionary
    pub fn new() -> Self {
        Self::with_capacity(16)
    }

    /// Create with initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.next_power_of_two().max(16);
        let ctrl = vec![ctrl::EMPTY; capacity + 16]; // Extra for SIMD
        let entries = (0..capacity).map(|_| None).collect();

        Self {
            ctrl,
            entries,
            len: 0,
            capacity,
            growth_left: capacity * 7 / 8, // 87.5% load factor
        }
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Hash a key
    fn hash_key(key: &K) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// Get H1 (bucket index)
    #[inline]
    fn h1(&self, hash: u64) -> usize {
        (hash >> 7) as usize & (self.capacity - 1)
    }

    /// Find a slot for the given key
    fn find_slot(&self, key: &K) -> Option<usize> {
        let hash = Self::hash_key(key);
        let h1 = self.h1(hash);
        let h2 = ctrl::h2(hash);

        // Linear probe through the table
        let mut pos = h1;
        let mut probes = 0;
        loop {
            // Check current position
            let idx = pos & (self.capacity - 1);
            let ctrl_byte = self.ctrl[idx];

            // If we find a matching h2, check the key
            if ctrl_byte == h2 {
                if let Some(ref entry) = self.entries[idx] {
                    if entry.key == *key {
                        return Some(idx);
                    }
                }
            }

            // If we hit an empty slot, the key doesn't exist
            if ctrl::is_empty(ctrl_byte) {
                return None;
            }

            // Linear probe to next slot
            pos = pos.wrapping_add(1);
            probes += 1;

            // Safety: prevent infinite loop
            if probes >= self.capacity {
                return None;
            }
        }
    }

    /// Find an empty slot for insertion
    fn find_insert_slot(&self, key: &K) -> usize {
        let hash = Self::hash_key(key);
        let h1 = self.h1(hash);

        let mut pos = h1;
        let mut probes = 0;
        loop {
            let idx = pos & (self.capacity - 1);
            if !ctrl::is_full(self.ctrl[idx]) {
                return idx;
            }

            pos = pos.wrapping_add(1);
            probes += 1;

            if probes >= self.capacity {
                panic!("Table is full");
            }
        }
    }

    /// Match control bytes using SIMD (kept for potential future use)
    #[allow(dead_code)]
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    fn match_byte(&self, pos: usize, h2: u8) -> Vec<usize> {
        use std::arch::x86_64::*;

        unsafe {
            let ctrl_ptr = self.ctrl.as_ptr().add(pos);
            let ctrl_vec = _mm_loadu_si128(ctrl_ptr as *const __m128i);
            let h2_vec = _mm_set1_epi8(h2 as i8);
            let cmp = _mm_cmpeq_epi8(ctrl_vec, h2_vec);
            let mask = _mm_movemask_epi8(cmp) as u32;

            let mut matches = Vec::new();
            let mut m = mask;
            while m != 0 {
                let idx = m.trailing_zeros() as usize;
                matches.push(idx);
                m &= m - 1;
            }
            matches
        }
    }

    #[allow(dead_code)]
    #[cfg(not(all(target_arch = "x86_64", target_feature = "sse2")))]
    fn match_byte(&self, pos: usize, h2: u8) -> Vec<usize> {
        self.match_byte_scalar(pos, h2)
    }

    #[allow(dead_code)]
    fn match_byte_scalar(&self, pos: usize, h2: u8) -> Vec<usize> {
        let mut matches = Vec::new();
        for i in 0..16 {
            let idx = (pos + i) & (self.capacity - 1);
            if self.ctrl[idx] == h2 {
                matches.push(i);
            }
        }
        matches
    }

    /// Match empty slots using SIMD (kept for potential future use)
    #[allow(dead_code)]
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    fn match_empty(&self, pos: usize) -> Vec<usize> {
        use std::arch::x86_64::*;

        unsafe {
            let ctrl_ptr = self.ctrl.as_ptr().add(pos);
            let ctrl_vec = _mm_loadu_si128(ctrl_ptr as *const __m128i);
            let empty_vec = _mm_set1_epi8(ctrl::EMPTY as i8);
            let cmp = _mm_cmpeq_epi8(ctrl_vec, empty_vec);
            let mask = _mm_movemask_epi8(cmp) as u32;

            let mut matches = Vec::new();
            let mut m = mask;
            while m != 0 {
                let idx = m.trailing_zeros() as usize;
                matches.push(idx);
                m &= m - 1;
            }
            matches
        }
    }

    #[allow(dead_code)]
    #[cfg(not(all(target_arch = "x86_64", target_feature = "sse2")))]
    fn match_empty(&self, pos: usize) -> Vec<usize> {
        self.match_empty_scalar(pos)
    }

    #[allow(dead_code)]
    fn match_empty_scalar(&self, pos: usize) -> Vec<usize> {
        let mut matches = Vec::new();
        for i in 0..16 {
            let idx = (pos + i) & (self.capacity - 1);
            if ctrl::is_empty(self.ctrl[idx]) {
                matches.push(i);
            }
        }
        matches
    }

    /// Get a value by key
    pub fn get(&self, key: &K) -> Option<&V> {
        self.find_slot(key).map(|idx| &self.entries[idx].as_ref().unwrap().value)
    }

    /// Get a mutable value by key
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.find_slot(key).map(|idx| &mut self.entries[idx].as_mut().unwrap().value)
    }

    /// Insert a key-value pair
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        // Check if key exists
        if let Some(idx) = self.find_slot(&key) {
            let old = mem::replace(&mut self.entries[idx].as_mut().unwrap().value, value);
            return Some(old);
        }

        // Check if we need to grow
        if self.growth_left == 0 {
            self.grow();
        }

        // Find insert slot
        let hash = Self::hash_key(&key);
        let h2 = ctrl::h2(hash);
        let idx = self.find_insert_slot(&key);

        self.ctrl[idx] = h2;
        self.entries[idx] = Some(Entry { key, value });
        self.len += 1;
        self.growth_left -= 1;

        None
    }

    /// Remove a key
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(idx) = self.find_slot(key) {
            self.ctrl[idx] = ctrl::DELETED;
            let entry = self.entries[idx].take().unwrap();
            self.len -= 1;
            return Some(entry.value);
        }
        None
    }

    /// Check if key exists
    pub fn contains_key(&self, key: &K) -> bool {
        self.find_slot(key).is_some()
    }

    /// Grow the table
    fn grow(&mut self) {
        let new_capacity = self.capacity * 2;
        let mut new_dict = SwissDict::with_capacity(new_capacity);

        for entry in self.entries.iter().flatten() {
            new_dict.insert(entry.key.clone(), entry.value.clone());
        }

        *self = new_dict;
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.ctrl.fill(ctrl::EMPTY);
        self.entries.iter_mut().for_each(|e| *e = None);
        self.len = 0;
        self.growth_left = self.capacity * 7 / 8;
    }

    /// Iterate over key-value pairs
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().filter_map(|e| e.as_ref()).map(|e| (&e.key, &e.value))
    }

    /// Iterate over keys
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.iter().map(|(k, _)| k)
    }

    /// Iterate over values
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, v)| v)
    }
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> Default for SwissDict<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut dict = SwissDict::new();
        dict.insert("key1", 100);
        dict.insert("key2", 200);

        assert_eq!(dict.get(&"key1"), Some(&100));
        assert_eq!(dict.get(&"key2"), Some(&200));
        assert_eq!(dict.get(&"key3"), None);
    }

    #[test]
    fn test_update() {
        let mut dict = SwissDict::new();
        dict.insert("key", 100);
        let old = dict.insert("key", 200);

        assert_eq!(old, Some(100));
        assert_eq!(dict.get(&"key"), Some(&200));
    }

    #[test]
    fn test_remove() {
        let mut dict = SwissDict::new();
        dict.insert("key", 100);

        let removed = dict.remove(&"key");
        assert_eq!(removed, Some(100));
        assert_eq!(dict.get(&"key"), None);
    }

    #[test]
    fn test_contains_key() {
        let mut dict = SwissDict::new();
        dict.insert("key", 100);

        assert!(dict.contains_key(&"key"));
        assert!(!dict.contains_key(&"other"));
    }

    #[test]
    fn test_len() {
        let mut dict = SwissDict::new();
        assert_eq!(dict.len(), 0);
        assert!(dict.is_empty());

        dict.insert("a", 1);
        dict.insert("b", 2);
        assert_eq!(dict.len(), 2);

        dict.remove(&"a");
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_grow() {
        let mut dict = SwissDict::with_capacity(16);

        for i in 0..100 {
            dict.insert(i, i * 2);
        }

        assert_eq!(dict.len(), 100);

        for i in 0..100 {
            assert_eq!(dict.get(&i), Some(&(i * 2)));
        }
    }

    #[test]
    fn test_clear() {
        let mut dict = SwissDict::new();
        dict.insert("a", 1);
        dict.insert("b", 2);

        dict.clear();
        assert!(dict.is_empty());
        assert_eq!(dict.get(&"a"), None);
    }

    #[test]
    fn test_iter() {
        let mut dict = SwissDict::new();
        dict.insert(1, "a");
        dict.insert(2, "b");
        dict.insert(3, "c");

        let mut pairs: Vec<_> = dict.iter().map(|(&k, &v)| (k, v)).collect();
        pairs.sort_by_key(|(k, _)| *k);

        assert_eq!(pairs, vec![(1, "a"), (2, "b"), (3, "c")]);
    }
}
