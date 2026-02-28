#[repr(C, packed)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DxpHeader {
    pub magic: [u8; 4],       // 4 bytes
    pub version: u16,         // 2 bytes
    pub flags: u16,           // 2 bytes
    pub name_hash: u64,       // 8 bytes
    pub version_num: u64,     // 8 bytes
    pub total_size: u64,      // 8 bytes
    pub index_offset: u64,    // 8 bytes
    pub file_count: u32,      // 4 bytes
    pub metadata_size: u32,   // 4 bytes
    pub metadata_offset: u64, // 8 bytes
    pub deps_offset: u64,     // 8 bytes
    pub deps_count: u16,      // 2 bytes
    pub _pad: [u8; 6],        // 6 bytes
    pub content_hash: u128,   // 16 bytes
    pub timestamp: u64,       // 8 bytes
    pub reserved: [u8; 32],   // 32 bytes (was 16, now 32 to reach 128 total)
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct DxlHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    pub package_count: u64,
    pub table_size: u32,
    pub log_count: u32,
    pub metadata_offset: u64,
    pub graph_offset: u64,
    pub edge_count: u64,
    pub log_offset: u64,
    pub content_hash: u128,
    pub timestamp: u64,
    pub node_version: u64,
    pub platform: u32,
    pub reserved: [u8; 20],
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct DxrpRequestHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub request_type: u8,
    pub flags: u8,
    pub package_count: u16,
    pub _pad: u16,
    pub cache_timestamp: u64,
    pub platform: u32,
    pub reserved: u32,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct DxrpResponseHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub response_type: u8,
    pub status: u8,
    pub package_count: u32,
    pub total_size: u64,
    pub timestamp: u64,
    pub reserved: u32,
    pub _pad: u32,
}
