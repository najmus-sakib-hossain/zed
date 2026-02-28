//! Tagged value representation for small integers and pointers

/// Tagged value that can hold either a small integer or a pointer
///
/// Uses the lowest bit as a tag:
/// - 0: Pointer (aligned to at least 2 bytes)
/// - 1: Small integer (shifted left by 1)
///
/// This allows integers in the range -2^62 to 2^62-1 to be stored
/// without heap allocation.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct TaggedValue(u64);

impl TaggedValue {
    /// Tag bit for small integers
    const INT_TAG: u64 = 1;

    /// Maximum small integer value (2^62 - 1)
    pub const MAX_SMALL_INT: i64 = (1i64 << 62) - 1;

    /// Minimum small integer value (-2^62)
    pub const MIN_SMALL_INT: i64 = -(1i64 << 62);

    /// Create a tagged value from a small integer
    #[inline]
    pub fn from_small_int(value: i64) -> Option<Self> {
        if (Self::MIN_SMALL_INT..=Self::MAX_SMALL_INT).contains(&value) {
            // Shift left by 1 and set the tag bit
            let tagged = ((value as u64) << 1) | Self::INT_TAG;
            Some(Self(tagged))
        } else {
            None
        }
    }

    /// Create a tagged value from a small integer (unchecked)
    ///
    /// # Safety
    /// Caller must ensure value is in range [MIN_SMALL_INT, MAX_SMALL_INT]
    #[inline]
    pub unsafe fn from_small_int_unchecked(value: i64) -> Self {
        Self(((value as u64) << 1) | Self::INT_TAG)
    }

    /// Create a tagged value from a pointer
    #[inline]
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        debug_assert!(ptr as u64 & Self::INT_TAG == 0, "Pointer must be aligned");
        Self(ptr as u64)
    }

    /// Create a null tagged value
    #[inline]
    pub const fn null() -> Self {
        Self(0)
    }

    /// Check if this is a small integer
    #[inline]
    pub fn is_small_int(&self) -> bool {
        self.0 & Self::INT_TAG != 0
    }

    /// Check if this is a pointer
    #[inline]
    pub fn is_ptr(&self) -> bool {
        self.0 & Self::INT_TAG == 0
    }

    /// Check if this is null
    #[inline]
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Get the small integer value
    #[inline]
    pub fn as_small_int(&self) -> Option<i64> {
        if self.is_small_int() {
            // Arithmetic right shift to preserve sign
            Some((self.0 as i64) >> 1)
        } else {
            None
        }
    }

    /// Get the small integer value (unchecked)
    ///
    /// # Safety
    /// Caller must ensure this is a small integer
    #[inline]
    pub unsafe fn as_small_int_unchecked(&self) -> i64 {
        (self.0 as i64) >> 1
    }

    /// Get the pointer value
    #[inline]
    pub fn as_ptr<T>(&self) -> Option<*const T> {
        if self.is_ptr() && !self.is_null() {
            Some(self.0 as *const T)
        } else {
            None
        }
    }

    /// Get the pointer value (unchecked)
    ///
    /// # Safety
    /// Caller must ensure this is a pointer
    #[inline]
    pub unsafe fn as_ptr_unchecked<T>(&self) -> *const T {
        self.0 as *const T
    }

    /// Get the mutable pointer value
    #[inline]
    pub fn as_mut_ptr<T>(&self) -> Option<*mut T> {
        if self.is_ptr() && !self.is_null() {
            Some(self.0 as *mut T)
        } else {
            None
        }
    }

    /// Get the raw bits
    #[inline]
    pub fn raw(&self) -> u64 {
        self.0
    }

    /// Create from raw bits
    #[inline]
    pub const fn from_raw(bits: u64) -> Self {
        Self(bits)
    }
}

impl std::fmt::Debug for TaggedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_null() {
            write!(f, "TaggedValue(null)")
        } else if self.is_small_int() {
            write!(f, "TaggedValue(int: {})", self.as_small_int().unwrap())
        } else {
            write!(f, "TaggedValue(ptr: 0x{:x})", self.0)
        }
    }
}

impl Default for TaggedValue {
    fn default() -> Self {
        Self::null()
    }
}

impl From<i64> for TaggedValue {
    fn from(value: i64) -> Self {
        Self::from_small_int(value)
            .unwrap_or_else(|| panic!("Integer {} out of range for TaggedValue", value))
    }
}

impl From<i32> for TaggedValue {
    fn from(value: i32) -> Self {
        // i32 always fits in small int range
        unsafe { Self::from_small_int_unchecked(value as i64) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_int() {
        let v = TaggedValue::from_small_int(42).unwrap();
        assert!(v.is_small_int());
        assert!(!v.is_ptr());
        assert_eq!(v.as_small_int(), Some(42));
    }

    #[test]
    fn test_negative_int() {
        let v = TaggedValue::from_small_int(-100).unwrap();
        assert!(v.is_small_int());
        assert_eq!(v.as_small_int(), Some(-100));
    }

    #[test]
    fn test_zero() {
        let v = TaggedValue::from_small_int(0).unwrap();
        assert!(v.is_small_int());
        assert!(!v.is_null());
        assert_eq!(v.as_small_int(), Some(0));
    }

    #[test]
    fn test_null() {
        let v = TaggedValue::null();
        assert!(v.is_null());
        assert!(v.is_ptr());
        assert!(!v.is_small_int());
    }

    #[test]
    fn test_pointer() {
        let data: u64 = 12345678;
        let ptr = &data as *const u64;
        let v = TaggedValue::from_ptr(ptr);

        assert!(v.is_ptr());
        assert!(!v.is_small_int());
        assert_eq!(v.as_ptr::<u64>(), Some(ptr));
    }

    #[test]
    fn test_max_min_int() {
        let max = TaggedValue::from_small_int(TaggedValue::MAX_SMALL_INT).unwrap();
        assert_eq!(max.as_small_int(), Some(TaggedValue::MAX_SMALL_INT));

        let min = TaggedValue::from_small_int(TaggedValue::MIN_SMALL_INT).unwrap();
        assert_eq!(min.as_small_int(), Some(TaggedValue::MIN_SMALL_INT));
    }

    #[test]
    fn test_out_of_range() {
        assert!(TaggedValue::from_small_int(TaggedValue::MAX_SMALL_INT + 1).is_none());
        assert!(TaggedValue::from_small_int(TaggedValue::MIN_SMALL_INT - 1).is_none());
    }

    #[test]
    fn test_from_i32() {
        let v: TaggedValue = 42i32.into();
        assert_eq!(v.as_small_int(), Some(42));

        let v: TaggedValue = (-1000i32).into();
        assert_eq!(v.as_small_int(), Some(-1000));
    }
}
