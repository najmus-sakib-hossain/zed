//! Template Loading and Management
//!
//! High-level interface for loading and working with binary templates.
//! Supports memory-mapped loading for zero-copy access.

use crate::binary::{BinaryTemplate, DxtHeader, HEADER_SIZE};
use crate::error::{GeneratorError, Result};
use memmap2::Mmap;
use std::path::Path;
use std::sync::Arc;

// ============================================================================
// Template Source
// ============================================================================

/// Source of template data.
#[derive(Debug)]
pub enum TemplateSource {
    /// Memory-mapped file (zero-copy)
    Mmap(Mmap),
    /// Owned byte buffer
    Owned(Vec<u8>),
    /// Borrowed byte slice (for embedded templates)
    Static(&'static [u8]),
}

impl AsRef<[u8]> for TemplateSource {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Mmap(mmap) => mmap.as_ref(),
            Self::Owned(vec) => vec.as_ref(),
            Self::Static(slice) => slice,
        }
    }
}

// ============================================================================
// Template
// ============================================================================

/// A loaded template ready for rendering.
///
/// Templates can be loaded from:
/// - Files (memory-mapped for zero-copy)
/// - Byte buffers (for dynamic templates)
/// - Static data (for embedded templates)
///
/// # Example
///
/// ```rust,ignore
/// use dx_generator::Template;
///
/// // Load from file (memory-mapped)
/// let template = Template::load("component.dxt")?;
///
/// // Load from bytes
/// let template = Template::from_bytes(dxt_bytes)?;
///
/// // Check if Micro mode eligible
/// if template.is_micro_eligible() {
///     // Fast path: direct memory copy with patching
/// }
/// ```
#[derive(Debug)]
pub struct Template {
    /// Parsed binary template
    inner: BinaryTemplate,
    /// Original source data (kept for zero-copy string access)
    #[allow(dead_code)]
    source: TemplateSource,
    /// Template ID (for caching)
    id: u32,
}

impl Template {
    /// Load a template from a file path.
    ///
    /// Uses memory mapping for zero-copy access. This is the fastest way
    /// to load templates as it avoids copying data into memory.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file doesn't exist
    /// - The file isn't a valid DXT format
    /// - The checksum doesn't match
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file = std::fs::File::open(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GeneratorError::template_not_found(path.display().to_string())
            } else {
                GeneratorError::Io(e)
            }
        })?;

        // Safety: We're memory-mapping a read-only file
        let mmap = unsafe { Mmap::map(&file)? };

        Self::from_source(TemplateSource::Mmap(mmap))
    }

    /// Create a template from owned bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        Self::from_source(TemplateSource::Owned(bytes))
    }

    /// Create a template from static bytes (for embedded templates).
    pub fn from_static(bytes: &'static [u8]) -> Result<Self> {
        Self::from_source(TemplateSource::Static(bytes))
    }

    /// Create a template from a source.
    fn from_source(source: TemplateSource) -> Result<Self> {
        let bytes = source.as_ref();

        // Validate header
        let header = DxtHeader::from_bytes(bytes)?;

        // Parse the template
        let inner = Self::parse_template(bytes, header)?;

        // Generate template ID from name hash
        let id = xxhash_rust::xxh64::xxh64(inner.name.as_bytes(), 0) as u32;

        Ok(Self { inner, source, id })
    }

    /// Parse a binary template from bytes.
    fn parse_template(bytes: &[u8], header: &DxtHeader) -> Result<BinaryTemplate> {
        let mut offset = HEADER_SIZE;

        // Read string table size
        if bytes.len() < offset + 4 {
            return Err(GeneratorError::invalid_template("Truncated string table size"));
        }
        let strings_size = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        // Read string table
        if bytes.len() < offset + strings_size {
            return Err(GeneratorError::invalid_template("Truncated string table"));
        }
        let strings =
            crate::binary::StringTable::from_bytes(&bytes[offset..offset + strings_size])?;
        offset += strings_size;

        // Read placeholder count
        if bytes.len() < offset + 4 {
            return Err(GeneratorError::invalid_template("Truncated placeholder count"));
        }
        let ph_count = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        // Read placeholders
        let ph_size = std::mem::size_of::<crate::binary::PlaceholderEntry>();
        if bytes.len() < offset + ph_count * ph_size {
            return Err(GeneratorError::invalid_template("Truncated placeholders"));
        }
        let mut placeholders = Vec::with_capacity(ph_count);
        for _ in 0..ph_count {
            // Copy bytes to aligned buffer to avoid alignment issues
            let mut entry_bytes = [0u8; 12]; // PlaceholderEntry is 12 bytes
            entry_bytes.copy_from_slice(&bytes[offset..offset + ph_size]);
            let entry: crate::binary::PlaceholderEntry = *bytemuck::from_bytes(&entry_bytes);
            placeholders.push(entry);
            offset += ph_size;
        }

        // Read instruction size
        if bytes.len() < offset + 4 {
            return Err(GeneratorError::invalid_template("Truncated instruction size"));
        }
        let instr_size = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        // Read instructions
        if bytes.len() < offset + instr_size {
            return Err(GeneratorError::invalid_template("Truncated instructions"));
        }
        let instructions = bytes[offset..offset + instr_size].to_vec();
        offset += instr_size;

        // Read template name
        if bytes.len() < offset + 2 {
            return Err(GeneratorError::invalid_template("Truncated template name length"));
        }
        let name_len = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as usize;
        offset += 2;
        if bytes.len() < offset + name_len {
            return Err(GeneratorError::invalid_template("Truncated template name"));
        }
        let name = String::from_utf8_lossy(&bytes[offset..offset + name_len]).into_owned();
        offset += name_len;

        // Read param names
        if bytes.len() < offset + 2 {
            return Err(GeneratorError::invalid_template("Truncated param count"));
        }
        let param_count = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as usize;
        offset += 2;

        let mut param_names = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            if bytes.len() < offset + 2 {
                return Err(GeneratorError::invalid_template("Truncated param name length"));
            }
            let param_len = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as usize;
            offset += 2;
            if bytes.len() < offset + param_len {
                return Err(GeneratorError::invalid_template("Truncated param name"));
            }
            let param_name =
                String::from_utf8_lossy(&bytes[offset..offset + param_len]).into_owned();
            param_names.push(param_name);
            offset += param_len;
        }

        Ok(BinaryTemplate {
            header: *header,
            strings,
            placeholders,
            instructions,
            name,
            param_names,
        })
    }

    /// Get the template name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// Get the template ID.
    #[must_use]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Check if this template can use Micro mode (static, no control flow).
    #[must_use]
    pub fn is_micro_eligible(&self) -> bool {
        self.inner.is_micro_eligible()
    }

    /// Check if this template is signed.
    #[must_use]
    pub fn is_signed(&self) -> bool {
        self.inner.header.is_signed()
    }

    /// Get the parameter names.
    #[must_use]
    pub fn param_names(&self) -> &[String] {
        &self.inner.param_names
    }

    /// Get the number of placeholders.
    #[must_use]
    pub fn placeholder_count(&self) -> usize {
        self.inner.placeholders.len()
    }

    /// Get the inner binary template.
    #[must_use]
    pub fn inner(&self) -> &BinaryTemplate {
        &self.inner
    }

    /// Convert to an Arc for shared ownership.
    #[must_use]
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

// ============================================================================
// Template Handle
// ============================================================================

/// A lightweight handle to a template in the pool.
///
/// This is Copy and cheap to pass around.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TemplateHandle {
    /// Template ID
    pub id: u32,
    /// Slot in the pool
    pub slot: u16,
    /// Generation (for ABA problem prevention)
    pub generation: u16,
}

impl TemplateHandle {
    /// Create a new template handle.
    #[must_use]
    pub const fn new(id: u32, slot: u16, generation: u16) -> Self {
        Self {
            id,
            slot,
            generation,
        }
    }

    /// Create a null handle.
    #[must_use]
    pub const fn null() -> Self {
        Self {
            id: 0,
            slot: u16::MAX,
            generation: 0,
        }
    }

    /// Check if this is a null handle.
    #[must_use]
    pub const fn is_null(&self) -> bool {
        self.slot == u16::MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary::BinaryTemplate;

    #[test]
    fn test_template_from_bytes() {
        // Build a simple template
        let mut builder = BinaryTemplate::builder("test");
        builder.add_string("Hello!");
        builder.set_static(true);
        let template = builder.build();

        // Serialize and reload
        let bytes = template.to_bytes();
        let loaded = Template::from_bytes(bytes).unwrap();

        assert_eq!(loaded.name(), "test");
        assert!(loaded.is_micro_eligible());
    }

    #[test]
    fn test_template_handle() {
        let handle = TemplateHandle::new(42, 5, 1);
        assert_eq!(handle.id, 42);
        assert_eq!(handle.slot, 5);
        assert!(!handle.is_null());

        let null = TemplateHandle::null();
        assert!(null.is_null());
    }
}
