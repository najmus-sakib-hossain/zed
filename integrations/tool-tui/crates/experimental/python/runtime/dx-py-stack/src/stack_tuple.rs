//! Stack-allocated tuple implementation

use std::mem::MaybeUninit;

/// A stack-allocated tuple with compile-time known size
#[repr(C)]
pub struct StackTuple<const N: usize> {
    /// Number of elements actually used
    len: usize,
    /// Elements stored inline
    elements: [MaybeUninit<u64>; N],
}

impl<const N: usize> StackTuple<N> {
    /// Create a new empty stack tuple
    pub const fn new() -> Self {
        Self {
            len: 0,
            // Safety: MaybeUninit doesn't require initialization
            elements: unsafe { MaybeUninit::uninit().assume_init() },
        }
    }

    /// Create a stack tuple from an array
    pub fn from_array(values: [u64; N]) -> Self {
        let mut tuple = Self::new();
        for (i, value) in values.into_iter().enumerate() {
            tuple.elements[i] = MaybeUninit::new(value);
        }
        tuple.len = N;
        tuple
    }

    /// Get the number of elements
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the capacity
    #[inline]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Get an element by index
    #[inline]
    pub fn get(&self, index: usize) -> Option<u64> {
        if index < self.len {
            // Safety: index is within initialized range
            Some(unsafe { self.elements[index].assume_init() })
        } else {
            None
        }
    }

    /// Get an element by index (unchecked)
    ///
    /// # Safety
    /// Caller must ensure index < len
    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> u64 {
        self.elements[index].assume_init()
    }

    /// Set an element by index
    #[inline]
    pub fn set(&mut self, index: usize, value: u64) -> bool {
        if index < N {
            self.elements[index] = MaybeUninit::new(value);
            if index >= self.len {
                self.len = index + 1;
            }
            true
        } else {
            false
        }
    }

    /// Push an element (if there's capacity)
    #[inline]
    pub fn push(&mut self, value: u64) -> bool {
        if self.len < N {
            self.elements[self.len] = MaybeUninit::new(value);
            self.len += 1;
            true
        } else {
            false
        }
    }

    /// Iterate over elements
    pub fn iter(&self) -> impl Iterator<Item = u64> + '_ {
        (0..self.len).map(|i| unsafe { self.elements[i].assume_init() })
    }

    /// Convert to a Vec (for heap fallback)
    pub fn to_vec(&self) -> Vec<u64> {
        self.iter().collect()
    }
}

impl<const N: usize> Default for StackTuple<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Clone for StackTuple<N> {
    fn clone(&self) -> Self {
        let mut new = Self::new();
        for i in 0..self.len {
            new.elements[i] = self.elements[i];
        }
        new.len = self.len;
        new
    }
}

/// Common tuple sizes
pub type Tuple2 = StackTuple<2>;
pub type Tuple3 = StackTuple<3>;
pub type Tuple4 = StackTuple<4>;
pub type Tuple8 = StackTuple<8>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tuple() {
        let tuple: StackTuple<4> = StackTuple::new();
        assert!(tuple.is_empty());
        assert_eq!(tuple.capacity(), 4);
    }

    #[test]
    fn test_from_array() {
        let tuple = StackTuple::from_array([1, 2, 3]);
        assert_eq!(tuple.len(), 3);
        assert_eq!(tuple.get(0), Some(1));
        assert_eq!(tuple.get(1), Some(2));
        assert_eq!(tuple.get(2), Some(3));
    }

    #[test]
    fn test_push() {
        let mut tuple: StackTuple<3> = StackTuple::new();
        assert!(tuple.push(10));
        assert!(tuple.push(20));
        assert!(tuple.push(30));
        assert!(!tuple.push(40)); // Full

        assert_eq!(tuple.len(), 3);
        assert_eq!(tuple.get(0), Some(10));
    }

    #[test]
    fn test_iter() {
        let tuple = StackTuple::from_array([1, 2, 3, 4]);
        let sum: u64 = tuple.iter().sum();
        assert_eq!(sum, 10);
    }

    #[test]
    fn test_to_vec() {
        let tuple = StackTuple::from_array([5, 6, 7]);
        let vec = tuple.to_vec();
        assert_eq!(vec, vec![5, 6, 7]);
    }
}
