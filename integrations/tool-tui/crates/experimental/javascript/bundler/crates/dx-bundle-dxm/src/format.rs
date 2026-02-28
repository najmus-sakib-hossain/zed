//! DX Module Binary Format (.dxm)
//!
//! Pre-compiled binary representation of JavaScript/TypeScript modules.
//! These are "lego bricks" ready to snap together without parsing.
//!
//! Layout:
//! ┌──────────────────────────────────────┐
//! │ Header (32 bytes)                    │
//! ├──────────────────────────────────────┤
//! │ Export Table (N * 16 bytes)          │
//! ├──────────────────────────────────────┤
//! │ Import Patch Table (M * 8 bytes)     │
//! ├──────────────────────────────────────┤
//! │ Body (raw optimized JS bytes)        │
//! └──────────────────────────────────────┘

/// Magic bytes: "DXM\0"
pub const DXM_MAGIC: [u8; 4] = [0x44, 0x58, 0x4D, 0x00];

/// Version 1.0
pub const DXM_VERSION: u16 = 0x0100;

/// DXM File Header (32 bytes, fixed size)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DxmHeader {
    /// Magic bytes: "DXM\0"
    pub magic: [u8; 4],
    /// Format version (major.minor)
    pub version: u16,
    /// Flags (reserved)
    pub flags: u16,
    /// Number of exports
    pub export_count: u32,
    /// Number of import patch slots
    pub import_patch_count: u32,
    /// Offset to body from start of file
    pub body_offset: u32,
    /// Length of body in bytes
    pub body_len: u32,
    /// Original source hash (for cache invalidation)
    pub source_hash: u64,
}

impl DxmHeader {
    pub const SIZE: usize = 32;

    pub fn new(
        export_count: u32,
        import_patch_count: u32,
        body_len: u32,
        source_hash: u64,
    ) -> Self {
        let export_table_size = export_count * ExportEntry::SIZE as u32;
        let import_table_size = import_patch_count * ImportPatchSlot::SIZE as u32;
        let body_offset = Self::SIZE as u32 + export_table_size + import_table_size;

        Self {
            magic: DXM_MAGIC,
            version: DXM_VERSION,
            flags: 0,
            export_count,
            import_patch_count,
            body_offset,
            body_len,
            source_hash,
        }
    }

    /// Safe accessor for export_count (avoids unaligned reference)
    #[inline]
    pub fn export_count(&self) -> u32 {
        self.export_count
    }

    /// Safe accessor for import_patch_count (avoids unaligned reference)
    #[inline]
    pub fn import_patch_count(&self) -> u32 {
        self.import_patch_count
    }

    /// Safe accessor for body_len (avoids unaligned reference)
    #[inline]
    pub fn body_len(&self) -> u32 {
        self.body_len
    }

    /// Safe accessor for source_hash (avoids unaligned reference)
    #[inline]
    pub fn source_hash(&self) -> u64 {
        self.source_hash
    }

    /// Safe accessor for body_offset (avoids unaligned reference)
    #[inline]
    pub fn body_offset(&self) -> u32 {
        self.body_offset
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0..4].copy_from_slice(&self.magic);
        bytes[4..6].copy_from_slice(&self.version.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.flags.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.export_count.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.import_patch_count.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.body_offset.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.body_len.to_le_bytes());
        bytes[24..32].copy_from_slice(&self.source_hash.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 32 {
            return Err("Buffer too small for header");
        }

        let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
        if magic != DXM_MAGIC {
            return Err("Invalid DXM magic bytes");
        }

        Ok(Self {
            magic,
            version: u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
            flags: u16::from_le_bytes(bytes[6..8].try_into().unwrap()),
            export_count: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            import_patch_count: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
            body_offset: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
            body_len: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
            source_hash: u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
        })
    }
}

/// Export Entry (16 bytes)
/// Maps export name hash to offset in body
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ExportEntry {
    /// FNV-1a hash of export name
    pub name_hash: u64,
    /// Offset in body where export is defined
    pub offset: u32,
    /// Length of export definition
    pub length: u32,
}

impl ExportEntry {
    pub const SIZE: usize = 16;

    pub fn new(name: &str, offset: u32, length: u32) -> Self {
        Self {
            name_hash: fnv1a_hash(name),
            offset,
            length,
        }
    }

    /// Safe accessor for name_hash (avoids unaligned reference)
    #[inline]
    pub fn name_hash(&self) -> u64 {
        self.name_hash
    }

    /// Safe accessor for offset (avoids unaligned reference)
    #[inline]
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// Safe accessor for length (avoids unaligned reference)
    #[inline]
    pub fn length(&self) -> u32 {
        self.length
    }

    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..8].copy_from_slice(&self.name_hash.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.offset.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.length.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            name_hash: u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
            offset: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            length: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
        }
    }
}

/// Import Patch Slot (8 bytes)
/// Marks locations in body where imports need to be patched during fusion
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ImportPatchSlot {
    /// Offset in body where import reference exists
    pub offset: u32,
    /// Length of the import reference
    pub length: u16,
    /// Index into import table (which module)
    pub module_index: u16,
}

impl ImportPatchSlot {
    pub const SIZE: usize = 8;

    pub fn new(offset: u32, length: u16, module_index: u16) -> Self {
        Self {
            offset,
            length,
            module_index,
        }
    }

    /// Safe accessor for offset (avoids unaligned reference)
    #[inline]
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// Safe accessor for length (avoids unaligned reference)
    #[inline]
    pub fn length(&self) -> u16 {
        self.length
    }

    /// Safe accessor for module_index (avoids unaligned reference)
    #[inline]
    pub fn module_index(&self) -> u16 {
        self.module_index
    }

    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0..4].copy_from_slice(&self.offset.to_le_bytes());
        bytes[4..6].copy_from_slice(&self.length.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.module_index.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            offset: u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
            length: u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
            module_index: u16::from_le_bytes(bytes[6..8].try_into().unwrap()),
        }
    }
}

/// Complete DXM Module (in-memory representation)
#[derive(Debug, Clone)]
pub struct DxmModule {
    pub header: DxmHeader,
    pub exports: Vec<ExportEntry>,
    pub import_patches: Vec<ImportPatchSlot>,
    pub body: Vec<u8>,
}

impl DxmModule {
    /// Create a new DXM module
    pub fn new(source_hash: u64) -> Self {
        Self {
            header: DxmHeader::new(0, 0, 0, source_hash),
            exports: Vec::new(),
            import_patches: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Add an export
    pub fn add_export(&mut self, name: &str, offset: u32, length: u32) {
        self.exports.push(ExportEntry::new(name, offset, length));
    }

    /// Add an import patch slot
    pub fn add_import_patch(&mut self, offset: u32, length: u16, module_index: u16) {
        self.import_patches.push(ImportPatchSlot::new(offset, length, module_index));
    }

    /// Set the body (optimized JS bytes)
    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = body;
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let header = DxmHeader::new(
            self.exports.len() as u32,
            self.import_patches.len() as u32,
            self.body.len() as u32,
            self.header.source_hash,
        );

        let total_size = DxmHeader::SIZE
            + self.exports.len() * ExportEntry::SIZE
            + self.import_patches.len() * ImportPatchSlot::SIZE
            + self.body.len();

        let mut bytes = Vec::with_capacity(total_size);

        // Write header
        bytes.extend_from_slice(&header.to_bytes());

        // Write export table
        for export in &self.exports {
            bytes.extend_from_slice(&export.to_bytes());
        }

        // Write import patch table
        for patch in &self.import_patches {
            bytes.extend_from_slice(&patch.to_bytes());
        }

        // Write body
        bytes.extend_from_slice(&self.body);

        bytes
    }

    /// Deserialize from bytes (zero-copy friendly)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < DxmHeader::SIZE {
            return Err("Buffer too small");
        }

        let header = DxmHeader::from_bytes(bytes)?;

        let export_table_start = DxmHeader::SIZE;
        let export_table_end =
            export_table_start + (header.export_count as usize * ExportEntry::SIZE);

        let import_table_start = export_table_end;
        let _import_table_end =
            import_table_start + (header.import_patch_count as usize * ImportPatchSlot::SIZE);

        let body_start = header.body_offset as usize;
        let body_end = body_start + header.body_len as usize;

        if bytes.len() < body_end {
            return Err("Buffer too small for body");
        }

        // Parse exports
        let mut exports = Vec::with_capacity(header.export_count as usize);
        for i in 0..header.export_count as usize {
            let offset = export_table_start + i * ExportEntry::SIZE;
            exports.push(ExportEntry::from_bytes(&bytes[offset..offset + ExportEntry::SIZE]));
        }

        // Parse import patches
        let mut import_patches = Vec::with_capacity(header.import_patch_count as usize);
        for i in 0..header.import_patch_count as usize {
            let offset = import_table_start + i * ImportPatchSlot::SIZE;
            import_patches
                .push(ImportPatchSlot::from_bytes(&bytes[offset..offset + ImportPatchSlot::SIZE]));
        }

        // Copy body
        let body = bytes[body_start..body_end].to_vec();

        Ok(Self {
            header,
            exports,
            import_patches,
            body,
        })
    }

    /// Get body as a slice (for zero-copy fusion)
    pub fn body_slice(&self) -> &[u8] {
        &self.body
    }

    /// Find export by name hash
    pub fn find_export(&self, name: &str) -> Option<&ExportEntry> {
        let hash = fnv1a_hash(name);
        self.exports.iter().find(|e| e.name_hash == hash)
    }

    /// Total size in bytes when serialized
    pub fn total_size(&self) -> usize {
        DxmHeader::SIZE
            + self.exports.len() * ExportEntry::SIZE
            + self.import_patches.len() * ImportPatchSlot::SIZE
            + self.body.len()
    }
}

/// FNV-1a hash function for export name lookup
#[inline]
pub fn fnv1a_hash(s: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_header_roundtrip() {
        let header = DxmHeader::new(5, 3, 1000, 0xDEADBEEF);
        let bytes = header.to_bytes();
        let parsed = DxmHeader::from_bytes(&bytes).unwrap();

        // Use accessor methods to avoid unaligned references
        assert_eq!(parsed.export_count(), 5);
        assert_eq!(parsed.import_patch_count(), 3);
        assert_eq!(parsed.body_len(), 1000);
        assert_eq!(parsed.source_hash(), 0xDEADBEEF);
    }

    #[test]
    fn test_module_roundtrip() {
        let mut module = DxmModule::new(12345);
        module.add_export("useState", 0, 100);
        module.add_export("useEffect", 100, 150);
        module.add_import_patch(50, 10, 0);
        module.set_body(b"function useState(){} function useEffect(){}".to_vec());

        let bytes = module.to_bytes();
        let parsed = DxmModule::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.exports.len(), 2);
        assert_eq!(parsed.import_patches.len(), 1);
        assert_eq!(parsed.body, module.body);
    }

    #[test]
    fn test_export_lookup() {
        let mut module = DxmModule::new(0);
        module.add_export("useState", 0, 100);
        module.add_export("useEffect", 100, 150);

        let found = module.find_export("useState").unwrap();
        // Use accessor methods to avoid unaligned references
        assert_eq!(found.offset(), 0);
        assert_eq!(found.length(), 100);

        assert!(module.find_export("nonexistent").is_none());
    }

    // =========================================================================
    // Property 3: Packed Struct Round-Trip
    // Validates: Requirements 3.3
    // For any DxmHeader, ExportEntry, or ImportPatchSlot struct, serializing
    // to bytes and deserializing back SHALL produce an equivalent struct with
    // all field values preserved.
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 3: DxmHeader round-trip preserves all fields
        #[test]
        fn prop_header_roundtrip(
            export_count in 0u32..1000,
            import_patch_count in 0u32..1000,
            body_len in 0u32..1_000_000,
            source_hash in any::<u64>()
        ) {
            let header = DxmHeader::new(export_count, import_patch_count, body_len, source_hash);
            let bytes = header.to_bytes();
            let parsed = DxmHeader::from_bytes(&bytes).unwrap();

            prop_assert_eq!(parsed.export_count(), export_count);
            prop_assert_eq!(parsed.import_patch_count(), import_patch_count);
            prop_assert_eq!(parsed.body_len(), body_len);
            prop_assert_eq!(parsed.source_hash(), source_hash);
        }

        /// Property 3: ExportEntry round-trip preserves all fields
        #[test]
        fn prop_export_entry_roundtrip(
            name_hash in any::<u64>(),
            offset in any::<u32>(),
            length in any::<u32>()
        ) {
            let entry = ExportEntry {
                name_hash,
                offset,
                length,
            };
            let bytes = entry.to_bytes();
            let parsed = ExportEntry::from_bytes(&bytes);

            prop_assert_eq!(parsed.name_hash(), name_hash);
            prop_assert_eq!(parsed.offset(), offset);
            prop_assert_eq!(parsed.length(), length);
        }

        /// Property 3: ImportPatchSlot round-trip preserves all fields
        #[test]
        fn prop_import_patch_roundtrip(
            offset in any::<u32>(),
            length in any::<u16>(),
            module_index in any::<u16>()
        ) {
            let slot = ImportPatchSlot::new(offset, length, module_index);
            let bytes = slot.to_bytes();
            let parsed = ImportPatchSlot::from_bytes(&bytes);

            prop_assert_eq!(parsed.offset(), offset);
            prop_assert_eq!(parsed.length(), length);
            prop_assert_eq!(parsed.module_index(), module_index);
        }

        /// Property 3: DxmModule round-trip preserves all data
        #[test]
        fn prop_module_roundtrip(
            source_hash in any::<u64>(),
            body in prop::collection::vec(any::<u8>(), 0..1000),
            export_count in 0usize..10,
            import_count in 0usize..10
        ) {
            let mut module = DxmModule::new(source_hash);

            // Add exports with sequential offsets
            for i in 0..export_count {
                let name = format!("export_{}", i);
                module.add_export(&name, (i * 100) as u32, 50);
            }

            // Add import patches
            for i in 0..import_count {
                module.add_import_patch((i * 10) as u32, 5, i as u16);
            }

            module.set_body(body.clone());

            let bytes = module.to_bytes();
            let parsed = DxmModule::from_bytes(&bytes).unwrap();

            prop_assert_eq!(parsed.exports.len(), export_count);
            prop_assert_eq!(parsed.import_patches.len(), import_count);
            prop_assert_eq!(parsed.body, body);
            prop_assert_eq!(parsed.header.source_hash(), source_hash);
        }
    }
}
