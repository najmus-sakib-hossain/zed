//! Stack-allocated list with heap fallback

use std::mem::MaybeUninit;

/// A stack-allocated list with fixed capacity and heap fallback
pub struct StackList<const CAP: usize> {
    /// Number of elements
    len: usize,
    /// Stack storage
    stack: [MaybeUninit<u64>; CAP],
    /// Heap overflow storage (None if still on stack)
    heap: Option<Vec<u64>>,
}

impl<const CAP: usize> StackList<CAP> {
    /// Create a new empty stack list
    pub fn new() -> Self {
        Self {
            len: 0,
            stack: unsafe { MaybeUninit::uninit().assume_init() },
            heap: None,
        }
    }

    /// Create with initial capacity hint
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity > CAP {
            Self {
                len: 0,
                stack: unsafe { MaybeUninit::uninit().assume_init() },
                heap: Some(Vec::with_capacity(capacity)),
            }
        } else {
            Self::new()
        }
    }

    /// Get the number of elements
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Check if using heap storage
    #[inline]
    pub fn is_on_heap(&self) -> bool {
        self.heap.is_some()
    }

    /// Get the stack capacity
    #[inline]
    pub const fn stack_capacity(&self) -> usize {
        CAP
    }

    /// Push an element
    pub fn push(&mut self, value: u64) {
        if let Some(ref mut heap) = self.heap {
            heap.push(value);
            self.len += 1;
        } else if self.len < CAP {
            self.stack[self.len] = MaybeUninit::new(value);
            self.len += 1;
        } else {
            // Overflow to heap
            self.overflow_to_heap();
            self.heap.as_mut().unwrap().push(value);
            self.len += 1;
        }
    }

    /// Pop an element
    pub fn pop(&mut self) -> Option<u64> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;

        if let Some(ref mut heap) = self.heap {
            heap.pop()
        } else {
            Some(unsafe { self.stack[self.len].assume_init() })
        }
    }

    /// Get an element by index
    #[inline]
    pub fn get(&self, index: usize) -> Option<u64> {
        if index >= self.len {
            return None;
        }

        if let Some(ref heap) = self.heap {
            heap.get(index).copied()
        } else {
            Some(unsafe { self.stack[index].assume_init() })
        }
    }

    /// Set an element by index
    #[inline]
    pub fn set(&mut self, index: usize, value: u64) -> bool {
        if index >= self.len {
            return false;
        }

        if let Some(ref mut heap) = self.heap {
            heap[index] = value;
        } else {
            self.stack[index] = MaybeUninit::new(value);
        }
        true
    }

    /// Move data from stack to heap
    fn overflow_to_heap(&mut self) {
        let mut heap = Vec::with_capacity(CAP * 2);
        for i in 0..self.len {
            heap.push(unsafe { self.stack[i].assume_init() });
        }
        self.heap = Some(heap);
    }

    /// Iterate over elements
    pub fn iter(&self) -> impl Iterator<Item = u64> + '_ {
        (0..self.len).map(|i| self.get(i).unwrap())
    }

    /// Convert to Vec
    pub fn to_vec(&self) -> Vec<u64> {
        if let Some(ref heap) = self.heap {
            heap.clone()
        } else {
            self.iter().collect()
        }
    }

    /// Clear the list
    pub fn clear(&mut self) {
        self.len = 0;
        if let Some(ref mut heap) = self.heap {
            heap.clear();
        }
    }

    /// Extend from an iterator
    pub fn extend<I: IntoIterator<Item = u64>>(&mut self, iter: I) {
        for value in iter {
            self.push(value);
        }
    }
}

impl<const CAP: usize> Default for StackList<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> Clone for StackList<CAP> {
    fn clone(&self) -> Self {
        if let Some(ref heap) = self.heap {
            Self {
                len: self.len,
                stack: unsafe { MaybeUninit::uninit().assume_init() },
                heap: Some(heap.clone()),
            }
        } else {
            let mut new = Self::new();
            for i in 0..self.len {
                new.stack[i] = self.stack[i];
            }
            new.len = self.len;
            new
        }
    }
}

/// Common list sizes
pub type SmallList = StackList<8>;
pub type MediumList = StackList<32>;
pub type LargeList = StackList<128>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_list() {
        let list: StackList<8> = StackList::new();
        assert!(list.is_empty());
        assert!(!list.is_on_heap());
    }

    #[test]
    fn test_push_pop() {
        let mut list: StackList<4> = StackList::new();
        list.push(1);
        list.push(2);
        list.push(3);

        assert_eq!(list.len(), 3);
        assert_eq!(list.pop(), Some(3));
        assert_eq!(list.pop(), Some(2));
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_overflow_to_heap() {
        let mut list: StackList<2> = StackList::new();
        list.push(1);
        list.push(2);
        assert!(!list.is_on_heap());

        list.push(3); // Overflow
        assert!(list.is_on_heap());
        assert_eq!(list.len(), 3);
        assert_eq!(list.get(2), Some(3));
    }

    #[test]
    fn test_get_set() {
        let mut list: StackList<4> = StackList::new();
        list.push(10);
        list.push(20);

        assert_eq!(list.get(0), Some(10));
        assert_eq!(list.get(1), Some(20));
        assert_eq!(list.get(2), None);

        assert!(list.set(0, 100));
        assert_eq!(list.get(0), Some(100));
    }

    #[test]
    fn test_iter() {
        let mut list: StackList<8> = StackList::new();
        list.extend([1, 2, 3, 4, 5]);

        let sum: u64 = list.iter().sum();
        assert_eq!(sum, 15);
    }
}
