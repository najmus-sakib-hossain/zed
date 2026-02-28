//! Binary Dawn CSS Runtime Loader
//!
//! Zero-copy CSS loader for WASM client runtime.
//! Decodes Binary Dawn CSS format and applies styles to DOM.

use core::slice;

// ============================================================================
// Binary Dawn Format Constants
// ============================================================================

const MAGIC: [u8; 4] = [0x44, 0x58, 0x42, 0x44];
const VERSION: u8 = 1;
const HEADER_SIZE: usize = 12;
const MAX_ENTRIES: usize = 1024;

// ============================================================================
// FFI: JavaScript Host Functions
// ============================================================================

unsafe extern "C" {
    fn host_inject_css(css_ptr: *const u8, css_len: u32);
    fn host_apply_class(node_id: u32, class_id: u16);
    fn host_clear_styles();
}

// ============================================================================
// Binary Dawn Header
// ============================================================================

#[repr(C)]
struct Header {
    magic: [u8; 4],
    version: u8,
    flags: u8,
    entry_count: u16,
    checksum: u32,
}

impl Header {
    fn from_bytes(data: &[u8]) -> Result<Self, u8> {
        if data.len() < HEADER_SIZE {
            return Err(1);
        }

        let header = Self {
            magic: [data[0], data[1], data[2], data[3]],
            version: data[4],
            flags: data[5],
            entry_count: u16::from_le_bytes([data[6], data[7]]),
            checksum: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
        };

        if header.magic != MAGIC || header.version != VERSION {
            return Err(2);
        }

        Ok(header)
    }
}

// ============================================================================
// Style Entry
// ============================================================================

#[repr(C)]
#[derive(Clone, Copy)]
struct Entry {
    id: u16,
    css_offset: u32,
    css_len: u16,
}

// ============================================================================
// Style Loader
// ============================================================================

pub struct StyleLoader<'a> {
    data: &'a [u8],
    entries: [Entry; MAX_ENTRIES],
    entry_count: usize,
    string_table_start: usize,
}

impl<'a> StyleLoader<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, u8> {
        let header = Header::from_bytes(data)?;

        if header.entry_count as usize > MAX_ENTRIES {
            return Err(3);
        }

        let mut entries = [Entry {
            id: 0,
            css_offset: 0,
            css_len: 0,
        }; MAX_ENTRIES];
        let mut pos = HEADER_SIZE;
        let entry_count = header.entry_count as usize;

        // Parse entries
        for (_i, entry) in entries.iter_mut().enumerate().take(entry_count) {
            if pos >= data.len() {
                return Err(4);
            }

            // Decode varint ID
            let (id, consumed) = decode_varint(&data[pos..]).ok_or(5)?;
            pos += consumed;

            // Read offset (4 bytes)
            if pos + 6 > data.len() {
                return Err(6);
            }
            let offset =
                u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
            pos += 4;

            // Read length (2 bytes)
            let len = u16::from_le_bytes([data[pos], data[pos + 1]]);
            pos += 2;

            *entry = Entry {
                id,
                css_offset: offset,
                css_len: len,
            };
        }

        let string_table_start = pos;

        // Validate checksum
        let string_table = &data[string_table_start..];
        let computed = seahash(string_table);
        if computed != header.checksum {
            return Err(7);
        }

        Ok(Self {
            data,
            entries,
            entry_count,
            string_table_start,
        })
    }

    pub fn get_css(&self, id: u16) -> Option<&'a str> {
        // Binary search
        let mut left = 0;
        let mut right = self.entry_count;

        while left < right {
            let mid = (left + right) / 2;
            let entry = &self.entries[mid];

            if entry.id == id {
                return self.get_css_by_index(mid);
            } else if entry.id < id {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        None
    }

    fn get_css_by_index(&self, index: usize) -> Option<&'a str> {
        if index >= self.entry_count {
            return None;
        }

        let entry = &self.entries[index];
        let start = self.string_table_start + entry.css_offset as usize;
        let end = start + entry.css_len as usize;

        if end > self.data.len() {
            return None;
        }

        core::str::from_utf8(&self.data[start..end]).ok()
    }

    pub fn apply_all(&self) {
        for i in 0..self.entry_count {
            if let Some(css) = self.get_css_by_index(i) {
                // SAFETY: FFI call to inject CSS
                unsafe {
                    host_inject_css(css.as_ptr(), css.len() as u32);
                }
            }
        }
    }

    pub fn apply_style(&self, node_id: u32, class_id: u16) {
        // SAFETY: FFI call to apply class
        unsafe {
            host_apply_class(node_id, class_id);
        }
    }

    pub fn entry_count(&self) -> usize {
        self.entry_count
    }
}

// ============================================================================
// Hot Reload Support
// ============================================================================

pub struct HotReloadManager {
    current_version: u32,
}

impl Default for HotReloadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HotReloadManager {
    pub const fn new() -> Self {
        Self { current_version: 0 }
    }

    pub fn reload(&mut self, data: &[u8]) -> Result<(), u8> {
        // SAFETY: Clear existing styles
        unsafe {
            host_clear_styles();
        }

        let loader = StyleLoader::new(data)?;
        loader.apply_all();

        self.current_version += 1;
        Ok(())
    }

    pub fn version(&self) -> u32 {
        self.current_version
    }
}

// ============================================================================
// Varint Decoding
// ============================================================================

fn decode_varint(data: &[u8]) -> Option<(u16, usize)> {
    if data.is_empty() {
        return None;
    }

    let first = data[0];

    if first < 128 {
        return Some((first as u16, 1));
    }

    if data.len() < 2 {
        return None;
    }

    let value = ((first & 0x7F) as u16) | ((data[1] as u16) << 7);
    Some((value, 2))
}

// ============================================================================
// Seahash
// ============================================================================

fn seahash(data: &[u8]) -> u32 {
    const K0: u64 = 0x16f11fe89b0d677c;
    const K1: u64 = 0xb480a793d8e6c86c;

    let mut hash = K0;

    for chunk in data.chunks(8) {
        let mut val = 0u64;
        for (i, &byte) in chunk.iter().enumerate() {
            val |= (byte as u64) << (i * 8);
        }
        hash = hash.wrapping_mul(K1).wrapping_add(val);
    }

    (hash ^ (hash >> 32)) as u32
}

// ============================================================================
// WASM Exports
// ============================================================================

static mut LOADER: Option<StyleLoader<'static>> = None;
static mut HOT_RELOAD: HotReloadManager = HotReloadManager { current_version: 0 };

/// Load Binary Dawn CSS from WASM memory
///
/// # Safety
/// Caller must ensure `ptr` points to valid memory of at least `len` bytes
/// that remains valid for the lifetime of the program.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_styles(ptr: *const u8, len: u32) -> u32 {
    if ptr.is_null() || len < HEADER_SIZE as u32 {
        return 1;
    }

    // SAFETY: Caller guarantees valid pointer and length
    let data = unsafe { slice::from_raw_parts(ptr, len as usize) };

    match StyleLoader::new(data) {
        Ok(loader) => {
            loader.apply_all();
            // SAFETY: Store loader in static, transmute extends lifetime
            unsafe {
                LOADER = Some(core::mem::transmute::<StyleLoader<'_>, StyleLoader<'_>>(loader));
            }
            0
        }
        Err(_) => 1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_style(node_id: u32, class_id: u16) -> u32 {
    // SAFETY: Access static loader
    unsafe {
        if let Some(ref loader) = LOADER {
            loader.apply_style(node_id, class_id);
            0
        } else {
            1
        }
    }
}

/// Hot reload styles in dev mode
///
/// # Safety
/// Caller must ensure `ptr` points to valid memory of at least `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_reload_styles(ptr: *const u8, len: u32) -> u32 {
    if ptr.is_null() || len < HEADER_SIZE as u32 {
        return 1;
    }

    // SAFETY: Caller guarantees valid pointer and length
    let data = unsafe { slice::from_raw_parts(ptr, len as usize) };

    // SAFETY: Access static hot reload manager, single-threaded WASM
    unsafe {
        let manager = core::ptr::addr_of_mut!(HOT_RELOAD);
        match (*manager).reload(data) {
            Ok(_) => 0,
            Err(_) => 1,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_style_count() -> u32 {
    // SAFETY: Access static loader, single-threaded WASM
    unsafe {
        let loader_ptr = core::ptr::addr_of!(LOADER);
        (*loader_ptr).as_ref().map(|l| l.entry_count() as u32).unwrap_or(0)
    }
}
