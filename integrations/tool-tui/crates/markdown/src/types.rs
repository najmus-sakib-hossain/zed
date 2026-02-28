//! Core AST types for DX Markdown documents.
//!
//! This module defines the internal representation (AST) that serves as the hub
//! for all format conversions.

use std::collections::HashMap;

/// The internal document representation (AST hub).
///
/// All DXM formats (LLM, Human, Machine) convert through this common representation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DxmDocument {
    /// Document metadata (Brain Header)
    pub meta: DxmMeta,
    /// Reference definitions (key -> value)
    pub refs: HashMap<String, String>,
    /// Document content nodes
    pub nodes: Vec<DxmNode>,
}

/// Document metadata for AI query planning.
///
/// The "Brain Header" provides topology information that allows LLMs to plan
/// their reading strategy before loading full content.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DxmMeta {
    /// DXM format version (e.g., "1.0")
    pub version: String,
    /// Total token count (LLM format)
    pub token_count: usize,
    /// Section hierarchy with byte offsets
    pub sections: Vec<SectionInfo>,
    /// Priority distribution
    pub priorities: PriorityDistribution,
}

/// Information about a document section for navigation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SectionInfo {
    /// Section title
    pub title: String,
    /// Header level (1-6)
    pub level: u8,
    /// Byte offset in the document
    pub offset: usize,
    /// Token count for this section
    pub token_count: usize,
}

/// Distribution of priority markers in the document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PriorityDistribution {
    /// Count of critical priority sections (!!!)
    pub critical: usize,
    /// Count of important priority sections (!!)
    pub important: usize,
    /// Count of low priority sections (!)
    pub low: usize,
}

/// A single node in the document tree.
#[derive(Debug, Clone, PartialEq)]
pub enum DxmNode {
    /// Header with level 1-6
    Header(HeaderNode),
    /// Paragraph with inline content
    Paragraph(Vec<InlineNode>),
    /// Code block with language
    CodeBlock(CodeBlockNode),
    /// Table with schema
    Table(TableNode),
    /// List (ordered or unordered)
    List(ListNode),
    /// Semantic block (warning, FAQ, quote, etc.)
    SemanticBlock(SemanticBlockNode),
    /// Horizontal rule
    HorizontalRule,
}

/// Header node with level and priority.
#[derive(Debug, Clone, PartialEq)]
pub struct HeaderNode {
    /// Header level (1-6)
    pub level: u8,
    /// Header content (inline nodes)
    pub content: Vec<InlineNode>,
    /// Optional priority marker
    pub priority: Option<Priority>,
}

impl Default for HeaderNode {
    fn default() -> Self {
        Self {
            level: 1,
            content: Vec::new(),
            priority: None,
        }
    }
}

/// Inline content types.
#[derive(Debug, Clone, PartialEq)]
pub enum InlineNode {
    /// Plain text
    Text(String),
    /// Bold text (!)
    Bold(Vec<InlineNode>),
    /// Italic text (/)
    Italic(Vec<InlineNode>),
    /// Strikethrough (~)
    Strikethrough(Vec<InlineNode>),
    /// Inline code (@)
    Code(String),
    /// Reference usage (^key)
    Reference(String),
    /// Link with optional title
    Link {
        text: Vec<InlineNode>,
        url: String,
        title: Option<String>,
    },
    /// Image
    Image {
        alt: String,
        url: String,
        title: Option<String>,
    },
}

/// Code block with language annotation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CodeBlockNode {
    /// Programming language (e.g., "rust", "python")
    pub language: Option<String>,
    /// Code content
    pub content: String,
    /// Optional priority marker
    pub priority: Option<Priority>,
}

/// Table with typed schema.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TableNode {
    /// Column definitions
    pub schema: Vec<ColumnDef>,
    /// Table rows (each row is a vector of cell values)
    pub rows: Vec<Vec<CellValue>>,
}

/// Column definition for tables.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColumnDef {
    /// Column name
    pub name: String,
    /// Optional type hint for the column
    pub type_hint: Option<TypeHint>,
}

/// Type hints for table columns.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeHint {
    /// String/text type
    String,
    /// Integer type
    Integer,
    /// Float type
    Float,
    /// Boolean type
    Boolean,
    /// Date type
    Date,
}

/// Cell value in a table.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum CellValue {
    /// Text value
    Text(String),
    /// Integer value
    Integer(i64),
    /// Float value
    Float(f64),
    /// Boolean value
    Boolean(bool),
    /// Null/empty value
    #[default]
    Null,
}

/// List node (ordered or unordered).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ListNode {
    /// Whether the list is ordered (numbered)
    pub ordered: bool,
    /// List items
    pub items: Vec<ListItem>,
}

/// A single list item.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ListItem {
    /// Item content (inline nodes)
    pub content: Vec<InlineNode>,
    /// Nested list (for hierarchical lists)
    pub nested: Option<Box<ListNode>>,
}

/// Semantic block node with typed content.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticBlockNode {
    /// Type of semantic block
    pub block_type: SemanticBlockType,
    /// Block content (inline nodes)
    pub content: Vec<InlineNode>,
    /// Optional priority marker
    pub priority: Option<Priority>,
}

impl Default for SemanticBlockNode {
    fn default() -> Self {
        Self {
            block_type: SemanticBlockType::Info,
            content: Vec::new(),
            priority: None,
        }
    }
}

/// Semantic block types.
///
/// These provide semantic meaning to content blocks, allowing AI systems
/// to understand the purpose of each section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[allow(clippy::upper_case_acronyms)]
pub enum SemanticBlockType {
    /// Warning block (#!)
    Warning,
    /// FAQ block (#?)
    FAQ,
    /// Quote block (#>)
    Quote,
    /// Info block (#i)
    #[default]
    Info,
    /// Example block (#x)
    Example,
}

/// Priority levels for content filtering.
///
/// Priority markers allow AI systems to filter content based on importance,
/// loading high-priority content first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    /// Low priority (!)
    Low,
    /// Important priority (!!)
    Important,
    /// Critical priority (!!!)
    Critical,
}

impl Priority {
    /// Parse priority from marker string.
    pub fn from_marker(marker: &str) -> Option<Self> {
        match marker {
            "!!!" => Some(Self::Critical),
            "!!" => Some(Self::Important),
            "!" => Some(Self::Low),
            _ => None,
        }
    }

    /// Convert priority to marker string.
    pub fn to_marker(self) -> &'static str {
        match self {
            Self::Critical => "!!!",
            Self::Important => "!!",
            Self::Low => "!",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_from_marker() {
        assert_eq!(Priority::from_marker("!!!"), Some(Priority::Critical));
        assert_eq!(Priority::from_marker("!!"), Some(Priority::Important));
        assert_eq!(Priority::from_marker("!"), Some(Priority::Low));
        assert_eq!(Priority::from_marker(""), None);
        assert_eq!(Priority::from_marker("!!!!"), None);
    }

    #[test]
    fn test_priority_to_marker() {
        assert_eq!(Priority::Critical.to_marker(), "!!!");
        assert_eq!(Priority::Important.to_marker(), "!!");
        assert_eq!(Priority::Low.to_marker(), "!");
    }

    #[test]
    fn test_default_document() {
        let doc = DxmDocument::default();
        assert!(doc.nodes.is_empty());
        assert!(doc.refs.is_empty());
        assert_eq!(doc.meta.version, "");
    }

    #[test]
    fn test_header_node_default() {
        let header = HeaderNode::default();
        assert_eq!(header.level, 1);
        assert!(header.content.is_empty());
        assert!(header.priority.is_none());
    }
}

// =============================================================================
// Context Compiler Types (DX Markdown as Context Compiler)
// =============================================================================

/// Compiler configuration for token optimization.
#[derive(Debug, Clone, PartialEq)]
pub struct CompilerConfig {
    /// Optimization mode
    pub mode: CompilerMode,
    /// Strip URLs from links
    pub strip_urls: bool,
    /// Strip images entirely
    pub strip_images: bool,
    /// Strip badge patterns
    pub strip_badges: bool,
    /// Convert tables to TSV and diagrams to DX format
    pub tables_to_tsv: bool,
    /// Enable semantic deduplication (dictionary)
    pub dictionary: bool,
    /// Minify code blocks
    pub minify_code: bool,
    /// Collapse whitespace
    pub collapse_whitespace: bool,
    /// Strip filler phrases
    pub strip_filler: bool,
    /// Use LLM-optimized header format (N|Header instead of ### Header)
    pub llm_headers: bool,
    /// Tokenizer type for counting
    pub tokenizer: TokenizerType,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            mode: CompilerMode::Full,
            strip_urls: true,
            strip_images: true,
            strip_badges: true,
            tables_to_tsv: true,
            dictionary: true,
            minify_code: true,
            collapse_whitespace: true,
            strip_filler: true,
            llm_headers: true,
            tokenizer: TokenizerType::Cl100k,
        }
    }
}

impl CompilerConfig {
    /// Create config for code-focused optimization.
    pub fn code() -> Self {
        Self {
            mode: CompilerMode::Code,
            strip_urls: true,
            strip_images: true,
            strip_badges: true,
            tables_to_tsv: true,
            dictionary: true,
            minify_code: false, // Keep code readable
            collapse_whitespace: true,
            strip_filler: true,
            llm_headers: true,
            tokenizer: TokenizerType::Cl100k,
        }
    }

    /// Create config for docs-focused optimization.
    pub fn docs() -> Self {
        Self {
            mode: CompilerMode::Docs,
            strip_urls: true,
            strip_images: true,
            strip_badges: true,
            tables_to_tsv: true,
            dictionary: true,
            minify_code: true,
            collapse_whitespace: true,
            strip_filler: false, // Keep explanatory text
            llm_headers: true,
            tokenizer: TokenizerType::Cl100k,
        }
    }

    /// Create config for data-focused optimization.
    pub fn data() -> Self {
        Self {
            mode: CompilerMode::Data,
            strip_urls: true,
            strip_images: true,
            strip_badges: true,
            tables_to_tsv: true,
            dictionary: true,
            minify_code: true,
            collapse_whitespace: true,
            strip_filler: true,
            llm_headers: true,
            tokenizer: TokenizerType::Cl100k,
        }
    }

    /// Create config for aggressive optimization.
    pub fn aggressive() -> Self {
        Self {
            mode: CompilerMode::Aggressive,
            strip_urls: true,
            strip_images: true,
            strip_badges: true,
            tables_to_tsv: true,
            dictionary: true,
            minify_code: true,
            collapse_whitespace: true,
            strip_filler: true,
            llm_headers: true,
            tokenizer: TokenizerType::Cl100k,
        }
    }
}

/// Optimization modes for the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CompilerMode {
    /// Strip obvious waste (badges, images, URLs) - default
    #[default]
    Full,
    /// Keep code, minimal prose
    Code,
    /// Keep explanations, minimal code
    Docs,
    /// Keep tables/lists, strip narrative
    Data,
    /// Maximum compression
    Aggressive,
}

impl CompilerMode {
    /// Parse mode from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "full" => Some(Self::Full),
            "code" => Some(Self::Code),
            "docs" => Some(Self::Docs),
            "data" => Some(Self::Data),
            "aggressive" => Some(Self::Aggressive),
            _ => None,
        }
    }

    /// Convert mode to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Code => "code",
            Self::Docs => "docs",
            Self::Data => "data",
            Self::Aggressive => "aggressive",
        }
    }
}

impl std::str::FromStr for CompilerMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid compiler mode: {}", s))
    }
}

/// Tokenizer types for token counting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TokenizerType {
    /// GPT-4, Claude (cl100k_base) - default
    #[default]
    Cl100k,
    /// GPT-4o, GPT-5 (o200k_base)
    O200k,
    /// GPT-3.5 (p50k_base)
    P50k,
}

impl TokenizerType {
    /// Parse tokenizer type from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "cl100k" | "cl100k_base" => Some(Self::Cl100k),
            "o200k" | "o200k_base" => Some(Self::O200k),
            "p50k" | "p50k_base" => Some(Self::P50k),
            _ => None,
        }
    }

    /// Convert tokenizer type to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cl100k => "cl100k_base",
            Self::O200k => "o200k_base",
            Self::P50k => "p50k_base",
        }
    }
}

impl std::str::FromStr for TokenizerType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid tokenizer type: {}", s))
    }
}

/// Compilation result with statistics.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompileResult {
    /// Optimized output
    pub output: String,
    /// Token count before optimization
    pub tokens_before: usize,
    /// Token count after optimization
    pub tokens_after: usize,
    /// Savings breakdown by optimization type
    pub breakdown: SavingsBreakdown,
}

impl CompileResult {
    /// Calculate savings percentage.
    pub fn savings_percent(&self) -> f64 {
        if self.tokens_before == 0 {
            return 0.0;
        }
        let saved = self.tokens_before.saturating_sub(self.tokens_after);
        (saved as f64 / self.tokens_before as f64) * 100.0
    }

    /// Get total tokens saved.
    pub fn tokens_saved(&self) -> usize {
        self.tokens_before.saturating_sub(self.tokens_after)
    }
}

/// Breakdown of token savings by optimization type.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SavingsBreakdown {
    /// Tokens saved from table conversion
    pub tables: usize,
    /// Tokens saved from URL stripping
    pub urls: usize,
    /// Tokens saved from image/badge removal
    pub images: usize,
    /// Tokens saved from dictionary deduplication
    pub dictionary: usize,
    /// Tokens saved from code minification
    pub code: usize,
    /// Tokens saved from whitespace collapse
    pub whitespace: usize,
    /// Tokens saved from filler removal
    pub filler: usize,
}

impl SavingsBreakdown {
    /// Get total tokens saved across all optimizations.
    pub fn total(&self) -> usize {
        self.tables
            + self.urls
            + self.images
            + self.dictionary
            + self.code
            + self.whitespace
            + self.filler
    }
}

/// Information about a detected table.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TableInfo {
    /// Column headers
    pub headers: Vec<String>,
    /// Table rows (each row is a vector of cell strings)
    pub rows: Vec<Vec<String>>,
    /// Start line in source
    pub start_line: usize,
    /// End line in source
    pub end_line: usize,
    /// Original markdown text
    pub original: String,
}

/// Information about a detected code block.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CodeBlockInfo {
    /// Programming language annotation
    pub language: Option<String>,
    /// Code content
    pub content: String,
    /// Start line in source
    pub start_line: usize,
    /// End line in source
    pub end_line: usize,
}

/// Information about a detected badge/image.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BadgeInfo {
    /// Alt text
    pub alt: String,
    /// URL
    pub url: String,
    /// Whether this is a badge (linked image)
    pub is_badge: bool,
    /// Line number in source
    pub line: usize,
}

/// Analysis result from the first compiler pass.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnalysisResult {
    /// Word/phrase frequency map
    pub frequencies: std::collections::HashMap<String, usize>,
    /// Detected tables
    pub tables: Vec<TableInfo>,
    /// Detected code blocks
    pub code_blocks: Vec<CodeBlockInfo>,
    /// Detected badges/images
    pub badges: Vec<BadgeInfo>,
    /// Total token count of input
    pub token_count: usize,
    /// Detected URLs
    pub urls: Vec<UrlInfo>,
}

/// Information about a detected URL.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UrlInfo {
    /// Link text
    pub text: String,
    /// URL
    pub url: String,
    /// Line number in source
    pub line: usize,
}

#[cfg(test)]
mod compiler_type_tests {
    use super::*;

    #[test]
    fn test_compiler_config_default() {
        let config = CompilerConfig::default();
        assert_eq!(config.mode, CompilerMode::Full);
        assert!(config.strip_urls);
        assert!(config.dictionary);
    }

    #[test]
    fn test_compiler_mode_from_str() {
        assert_eq!(CompilerMode::parse("full"), Some(CompilerMode::Full));
        assert_eq!(CompilerMode::parse("CODE"), Some(CompilerMode::Code));
        assert_eq!(CompilerMode::parse("aggressive"), Some(CompilerMode::Aggressive));
        assert_eq!(CompilerMode::parse("invalid"), None);
    }

    #[test]
    fn test_tokenizer_type_from_str() {
        assert_eq!(TokenizerType::parse("cl100k"), Some(TokenizerType::Cl100k));
        assert_eq!(TokenizerType::parse("o200k_base"), Some(TokenizerType::O200k));
        assert_eq!(TokenizerType::parse("invalid"), None);
    }

    #[test]
    fn test_compile_result_savings() {
        let result = CompileResult {
            output: String::new(),
            tokens_before: 100,
            tokens_after: 60,
            breakdown: SavingsBreakdown::default(),
        };
        assert_eq!(result.tokens_saved(), 40);
        assert!((result.savings_percent() - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_savings_breakdown_total() {
        let breakdown = SavingsBreakdown {
            tables: 10,
            urls: 20,
            images: 5,
            dictionary: 15,
            code: 8,
            whitespace: 3,
            filler: 4,
        };
        assert_eq!(breakdown.total(), 65);
    }
}
