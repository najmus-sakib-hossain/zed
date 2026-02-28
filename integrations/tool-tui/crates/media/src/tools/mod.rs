//! Media processing tools module.
//!
//! This module provides 60 media processing tools organized into 6 categories:
//! - **Image Tools**: Format conversion, resizing, compression, watermarking, etc.
//! - **Video Tools**: Transcoding, trimming, GIF creation, thumbnail extraction, etc.
//! - **Audio Tools**: Conversion, tag editing, normalization, waveform visualization, etc.
//! - **Document Tools**: PDF manipulation, Markdown conversion, CSV/JSON conversion, etc.
//! - **Archive Tools**: Compression, extraction, integrity checking, etc.
//! - **Utility Tools**: File management, hashing, encoding, clipboard operations, etc.

pub mod archive;
pub mod audio;
pub mod document;
pub mod image;
pub mod utility;
pub mod video;

// Re-export commonly used items
pub use archive::ArchiveTools;
pub use audio::AudioTools;
pub use document::DocumentTools;
pub use image::ImageTools;
pub use utility::UtilityTools;
pub use video::VideoTools;

use std::path::Path;

/// Trait for all tool operations.
pub trait Tool {
    /// Returns the name of the tool.
    fn name(&self) -> &'static str;

    /// Returns a description of the tool.
    fn description(&self) -> &'static str;

    /// Returns the category of the tool.
    fn category(&self) -> ToolCategory;
}

/// Tool categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    /// Image processing tools.
    Image,
    /// Video processing tools.
    Video,
    /// Audio processing tools.
    Audio,
    /// Document processing tools.
    Document,
    /// Archive/compression tools.
    Archive,
    /// System/utility tools.
    Utility,
}

impl ToolCategory {
    /// Returns all tool categories.
    pub fn all() -> &'static [ToolCategory] {
        &[
            Self::Image,
            Self::Video,
            Self::Audio,
            Self::Document,
            Self::Archive,
            Self::Utility,
        ]
    }

    /// Returns the category name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Document => "document",
            Self::Archive => "archive",
            Self::Utility => "utility",
        }
    }
}

/// Tool operation result with detailed output.
#[derive(Debug, Clone)]
pub struct ToolOutput {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Output message.
    pub message: String,
    /// Output file path(s) if any.
    pub output_paths: Vec<std::path::PathBuf>,
    /// Additional metadata.
    pub metadata: std::collections::HashMap<String, String>,
}

impl ToolOutput {
    /// Create a successful output.
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            output_paths: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a successful output with output path.
    pub fn success_with_path(message: impl Into<String>, path: impl AsRef<Path>) -> Self {
        Self {
            success: true,
            message: message.into(),
            output_paths: vec![path.as_ref().to_path_buf()],
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a failed output.
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            output_paths: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add a metadata entry.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Add output paths.
    pub fn with_paths(mut self, paths: Vec<std::path::PathBuf>) -> Self {
        self.output_paths = paths;
        self
    }
}
