use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaType {
    Video,
    Image,
    Audio,
    Model3D,
    Code,
    Document,
    Archive,
    Unknown,
}

impl MediaType {
    pub fn from_path(path: &Path) -> Self {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "mp4" | "mkv" | "mov" | "avi" | "webm" | "flv" | "wmv" | "m4v" => MediaType::Video,
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "tiff" | "avif" => {
                MediaType::Image
            }
            "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a" | "opus" | "aiff" => MediaType::Audio,
            "glb" | "gltf" | "fbx" | "obj" | "stl" | "blend" | "usd" | "usda" | "usdc"
            | "usdz" => MediaType::Model3D,
            "rs" | "py" | "js" | "ts" | "go" | "java" | "c" | "cpp" | "h" | "cs" | "rb"
            | "php" | "swift" | "kt" | "dart" | "lua" | "sh" | "bash" | "zsh" | "fish"
            | "toml" | "yaml" | "yml" | "json" | "xml" | "html" | "css" | "scss" | "md"
            | "txt" => MediaType::Code,
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods"
            | "odp" => MediaType::Document,
            "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => MediaType::Archive,
            _ => MediaType::Unknown,
        }
    }
}
