//! Core types for DX JS Bundler - Binary-first, zero-copy representations

use bytemuck::{Pod, Zeroable};

/// Module identifier (hash-based for O(1) lookup)
pub type ModuleId = u64;

/// Chunk identifier
pub type ChunkId = u32;

/// String index into string table (no string allocations!)
pub type StringIdx = u32;

/// Source position
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct SourcePos {
    pub offset: u32,
    pub line: u32,
    pub column: u32,
    _pad: u32,
}

/// Source span
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct SourceSpan {
    pub start: u32,
    pub end: u32,
}

impl SourceSpan {
    #[inline(always)]
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    #[inline(always)]
    pub fn len(&self) -> u32 {
        self.end - self.start
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// Token types we care about (single byte for cache efficiency)
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    // Module structure
    Import = 0,
    Export = 1,
    ExportDefault = 2,
    ExportNamed = 3,

    // JSX
    JsxOpen = 10,
    JsxClose = 11,
    JsxSelfClose = 12,
    JsxFragment = 13,

    // TypeScript (to strip)
    Interface = 20,
    TypeAlias = 21,
    TypeAnnotation = 22,
    AsExpression = 23,
    GenericParams = 24,
    EnumDecl = 25,

    // ES6+ features
    ArrowFunction = 30,
    ConstDecl = 31,
    LetDecl = 32,
    TemplateString = 33,
    Destructuring = 34,
    SpreadOperator = 35,
    AsyncAwait = 36,

    // Literals
    StringLiteral = 40,
    NumericLiteral = 41,
    RegexLiteral = 42,

    // Comments (to strip in minify)
    LineComment = 50,
    BlockComment = 51,

    // Identifiers
    Identifier = 60,

    // Other
    Code = 255,
}

/// Compact token (12 bytes, fits in cache line)
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Token {
    pub kind: u8,
    pub flags: u8,
    _pad: u16,
    pub start: u32,
    pub end: u32,
}

impl Token {
    #[inline(always)]
    pub fn new(kind: TokenKind, start: u32, end: u32) -> Self {
        Self {
            kind: kind as u8,
            flags: 0,
            _pad: 0,
            start,
            end,
        }
    }

    #[inline(always)]
    pub fn span(&self) -> SourceSpan {
        SourceSpan::new(self.start, self.end)
    }

    #[inline(always)]
    pub fn len(&self) -> u32 {
        self.end - self.start
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// Import statement data
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ImportData {
    /// Position in source
    pub span: SourceSpan,
    /// Module path string index
    pub path_idx: StringIdx,
    /// Import specifiers start index
    pub specifiers_start: u32,
    /// Number of specifiers
    pub specifier_count: u16,
    /// Flags (is_dynamic, is_type_only, etc.)
    pub flags: u16,
}

/// Export statement data
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ExportData {
    /// Position in source
    pub span: SourceSpan,
    /// Exported name string index
    pub name_idx: StringIdx,
    /// Local name string index (for `export { local as name }`)
    pub local_idx: StringIdx,
    /// Flags (is_default, is_type_only, is_reexport)
    pub flags: u16,
    _pad: u16,
}

/// Module metadata (binary header for cached modules)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ModuleHeader {
    /// Magic bytes "DXMD"
    pub magic: [u8; 4],
    /// Module ID (path hash)
    pub id: ModuleId,
    /// Content hash (for cache invalidation)
    pub content_hash: [u8; 16],
    /// Source size
    pub source_size: u32,
    /// Transformed size
    pub transformed_size: u32,
    /// Import count
    pub import_count: u16,
    /// Export count
    pub export_count: u16,
    /// Flags
    pub flags: u32,
}

impl ModuleHeader {
    pub const MAGIC: [u8; 4] = *b"DXMD";

    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
    }
}

/// Module flags
pub mod module_flags {
    pub const HAS_JSX: u32 = 1 << 0;
    pub const HAS_TYPESCRIPT: u32 = 1 << 1;
    pub const HAS_DYNAMIC_IMPORT: u32 = 1 << 2;
    pub const HAS_EXPORTS: u32 = 1 << 3;
    pub const IS_ENTRY: u32 = 1 << 4;
    pub const IS_ASYNC: u32 = 1 << 5;
    pub const HAS_SIDE_EFFECTS: u32 = 1 << 6;
}

/// Resolved module (runtime representation)
#[derive(Clone)]
pub struct ResolvedModule {
    pub id: ModuleId,
    pub path: std::path::PathBuf,
    pub source: Vec<u8>,
    pub imports: Vec<ModuleId>,
    pub exports: Vec<StringIdx>,
    pub flags: u32,
}

/// Transformed module
#[derive(Clone)]
pub struct TransformedModule {
    pub id: ModuleId,
    pub content: Vec<u8>,
    pub source_map: Option<Vec<u8>>,
    pub imports: Vec<ModuleId>,
}

/// Bundle chunk
#[derive(Clone)]
pub struct Chunk {
    pub id: ChunkId,
    pub modules: Vec<ModuleId>,
    pub content: Vec<u8>,
    pub hash: [u8; 16],
    pub is_entry: bool,
}

/// Scan result from SIMD scanner
#[derive(Clone, Default)]
pub struct ScanResult {
    pub imports: Vec<u32>,
    pub exports: Vec<u32>,
    pub jsx_elements: Vec<u32>,
    pub typescript_patterns: Vec<(u32, TypeScriptPattern)>,
    pub strings: Vec<SourceSpan>,
    pub comments: Vec<SourceSpan>,
}

/// TypeScript pattern types
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeScriptPattern {
    Interface = 0,
    TypeAlias = 1,
    TypeAnnotation = 2,
    GenericParams = 3,
    AsExpression = 4,
    EnumDecl = 5,
}

/// Import map for module resolution
#[derive(Default)]
pub struct ImportMap {
    /// Path hash → Module ID
    map: dashmap::DashMap<u64, ModuleId>,
}

impl ImportMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve path to module ID
    pub fn resolve(&self, path: &[u8]) -> ModuleId {
        let hash = xxhash_rust::xxh64::xxh64(path, 0);
        *self.map.entry(hash).or_insert(hash)
    }

    /// Register a module
    pub fn register(&self, path: &[u8], id: ModuleId) {
        let hash = xxhash_rust::xxh64::xxh64(path, 0);
        self.map.insert(hash, id);
    }

    /// Get module ID if registered
    pub fn get(&self, path: &[u8]) -> Option<ModuleId> {
        let hash = xxhash_rust::xxh64::xxh64(path, 0);
        self.map.get(&hash).map(|v| *v)
    }
}

/// String table for interned strings (no allocations during bundling)
pub struct StringTable {
    /// All strings concatenated
    data: Vec<u8>,
    /// String offsets and lengths
    entries: Vec<(u32, u32)>,
    /// Hash → index lookup
    index: dashmap::DashMap<u64, StringIdx>,
}

impl StringTable {
    pub fn new() -> Self {
        Self {
            data: Vec::with_capacity(64 * 1024), // 64KB initial
            entries: Vec::with_capacity(1024),
            index: dashmap::DashMap::new(),
        }
    }

    /// Intern a string, returning its index
    pub fn intern(&mut self, s: &[u8]) -> StringIdx {
        let hash = xxhash_rust::xxh64::xxh64(s, 0);

        if let Some(idx) = self.index.get(&hash) {
            return *idx;
        }

        let offset = self.data.len() as u32;
        self.data.extend_from_slice(s);
        let idx = self.entries.len() as StringIdx;
        self.entries.push((offset, s.len() as u32));
        self.index.insert(hash, idx);

        idx
    }

    /// Get string by index
    pub fn get(&self, idx: StringIdx) -> Option<&[u8]> {
        self.entries
            .get(idx as usize)
            .map(|&(offset, len)| &self.data[offset as usize..(offset + len) as usize])
    }
}

impl Default for StringTable {
    fn default() -> Self {
        Self::new()
    }
}
