//! GC Object Header
//!
//! Every GC-managed object has a header that stores metadata for garbage collection.

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

/// GC color for tricolor marking
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum GcColor {
    /// White: Not yet visited (will be collected if still white after marking)
    White = 0,
    /// Gray: Visited but children not yet scanned
    Gray = 1,
    /// Black: Visited and all children scanned (will not be collected)
    Black = 2,
}

impl From<u8> for GcColor {
    fn from(value: u8) -> Self {
        match value {
            0 => GcColor::White,
            1 => GcColor::Gray,
            2 => GcColor::Black,
            _ => GcColor::White,
        }
    }
}

/// Object type tag for runtime type identification
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ObjectType {
    String = 0,
    Object = 1,
    Array = 2,
    Function = 3,
    Promise = 4,
    RegExp = 5,
    Date = 6,
    Map = 7,
    Set = 8,
    WeakMap = 9,
    WeakSet = 10,
    ArrayBuffer = 11,
    TypedArray = 12,
    DataView = 13,
    Error = 14,
    BigInt = 15,
    Symbol = 16,
    Closure = 17,
}

impl From<u8> for ObjectType {
    fn from(value: u8) -> Self {
        match value {
            0 => ObjectType::String,
            1 => ObjectType::Object,
            2 => ObjectType::Array,
            3 => ObjectType::Function,
            4 => ObjectType::Promise,
            5 => ObjectType::RegExp,
            6 => ObjectType::Date,
            7 => ObjectType::Map,
            8 => ObjectType::Set,
            9 => ObjectType::WeakMap,
            10 => ObjectType::WeakSet,
            11 => ObjectType::ArrayBuffer,
            12 => ObjectType::TypedArray,
            13 => ObjectType::DataView,
            14 => ObjectType::Error,
            15 => ObjectType::BigInt,
            16 => ObjectType::Symbol,
            17 => ObjectType::Closure,
            _ => ObjectType::Object,
        }
    }
}

/// Header for GC-managed objects
///
/// This header is placed at the beginning of every heap-allocated object.
/// It contains metadata needed for garbage collection.
///
/// Layout (8 bytes):
/// - color: 1 byte (GC marking color)
/// - object_type: 1 byte (runtime type tag)
/// - flags: 1 byte (various flags)
/// - generation: 1 byte (0 = young, 1 = old)
/// - size: 4 bytes (object size in bytes)
#[repr(C)]
pub struct GcHeader {
    /// GC marking color (atomic for concurrent marking)
    color: AtomicU8,
    /// Object type tag
    object_type: AtomicU8,
    /// Flags (pinned, forwarded, etc.)
    flags: AtomicU8,
    /// Generation (0 = young, 1 = old)
    generation: AtomicU8,
    /// Object size in bytes (including header)
    size: AtomicU32,
}

/// Flag bits
pub const FLAG_PINNED: u8 = 0x01; // Object cannot be moved
pub const FLAG_FORWARDED: u8 = 0x02; // Object has been forwarded (copying GC)
/// Reserved for weak reference support
#[allow(dead_code)]
pub const FLAG_WEAK: u8 = 0x04; // Object is weakly referenced
/// Reserved for finalizer support
#[allow(dead_code)]
pub const FLAG_FINALIZED: u8 = 0x08; // Finalizer has been run

impl GcHeader {
    /// Create a new GC header
    pub fn new(object_type: ObjectType, size: u32) -> Self {
        Self {
            color: AtomicU8::new(GcColor::White as u8),
            object_type: AtomicU8::new(object_type as u8),
            flags: AtomicU8::new(0),
            generation: AtomicU8::new(0), // Start in young generation
            size: AtomicU32::new(size),
        }
    }

    /// Get the GC color
    #[inline]
    pub fn color(&self) -> GcColor {
        GcColor::from(self.color.load(Ordering::Acquire))
    }

    /// Set the GC color
    #[inline]
    pub fn set_color(&self, color: GcColor) {
        self.color.store(color as u8, Ordering::Release);
    }

    /// Get the object type
    #[inline]
    pub fn object_type(&self) -> ObjectType {
        ObjectType::from(self.object_type.load(Ordering::Relaxed))
    }

    /// Get the object size
    #[inline]
    pub fn size(&self) -> u32 {
        self.size.load(Ordering::Relaxed)
    }

    /// Get the generation
    #[inline]
    pub fn generation(&self) -> u8 {
        self.generation.load(Ordering::Relaxed)
    }

    /// Promote to old generation
    #[inline]
    pub fn promote(&self) {
        self.generation.store(1, Ordering::Release);
    }

    /// Check if object is in young generation
    #[inline]
    pub fn is_young(&self) -> bool {
        self.generation() == 0
    }

    /// Check if object is pinned
    #[inline]
    pub fn is_pinned(&self) -> bool {
        (self.flags.load(Ordering::Relaxed) & FLAG_PINNED) != 0
    }

    /// Pin the object (prevent moving)
    #[inline]
    pub fn pin(&self) {
        self.flags.fetch_or(FLAG_PINNED, Ordering::Release);
    }

    /// Check if object has been forwarded
    #[inline]
    pub fn is_forwarded(&self) -> bool {
        (self.flags.load(Ordering::Relaxed) & FLAG_FORWARDED) != 0
    }

    /// Mark as forwarded
    #[inline]
    pub fn set_forwarded(&self) {
        self.flags.fetch_or(FLAG_FORWARDED, Ordering::Release);
    }

    /// Get the header size
    #[inline]
    pub const fn header_size() -> usize {
        std::mem::size_of::<GcHeader>()
    }
}

impl Default for GcHeader {
    fn default() -> Self {
        Self::new(ObjectType::Object, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        // Header should be 8 bytes for efficient alignment
        assert_eq!(GcHeader::header_size(), 8);
    }

    #[test]
    fn test_header_color() {
        let header = GcHeader::new(ObjectType::Object, 64);
        assert_eq!(header.color(), GcColor::White);

        header.set_color(GcColor::Gray);
        assert_eq!(header.color(), GcColor::Gray);

        header.set_color(GcColor::Black);
        assert_eq!(header.color(), GcColor::Black);
    }

    #[test]
    fn test_header_generation() {
        let header = GcHeader::new(ObjectType::Object, 64);
        assert!(header.is_young());
        assert_eq!(header.generation(), 0);

        header.promote();
        assert!(!header.is_young());
        assert_eq!(header.generation(), 1);
    }

    #[test]
    fn test_header_pinning() {
        let header = GcHeader::new(ObjectType::Object, 64);
        assert!(!header.is_pinned());

        header.pin();
        assert!(header.is_pinned());
    }
}
