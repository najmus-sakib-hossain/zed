//! Mood/Media toggle system — determines which media actions are available.
//!
//! Each mood corresponds to a different set of input action buttons and
//! media generation capabilities in the DX panel.

use serde::{Deserialize, Serialize};

/// The seven media moods that control the DX input panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mood {
    /// Text/code generation using LLMs (default).
    Text,
    /// Image generation (DALL-E, Flux, SDXL, etc.).
    Image,
    /// Audio/sound/music generation.
    Audio,
    /// Video generation (Runway, Kling, Sora, etc.).
    Video,
    /// Live streaming/real-time.
    Live,
    /// 3D model generation (Meshy, TripoSR, etc.).
    ThreeD,
    /// PDF/document/chart generation.
    Pdf,
}

impl Mood {
    /// All available moods in display order.
    pub fn all() -> &'static [Mood] {
        &[
            Mood::Text,
            Mood::Image,
            Mood::Audio,
            Mood::Video,
            Mood::Live,
            Mood::ThreeD,
            Mood::Pdf,
        ]
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Mood::Text => "Text",
            Mood::Image => "Image",
            Mood::Audio => "Audio",
            Mood::Video => "Video",
            Mood::Live => "Live",
            Mood::ThreeD => "3D",
            Mood::Pdf => "PDF",
        }
    }

    /// Icon name for the mood (using Zed icon system).
    pub fn icon_name(&self) -> &'static str {
        match self {
            Mood::Text => "text",
            Mood::Image => "image",
            Mood::Audio => "audio",
            Mood::Video => "video",
            Mood::Live => "live",
            Mood::ThreeD => "cube",
            Mood::Pdf => "file-pdf",
        }
    }

    /// Label for the send button in this mood.
    pub fn send_button_label(&self) -> &'static str {
        match self {
            Mood::Text => "Send",
            Mood::Image => "Generate Image",
            Mood::Audio => "Generate Audio",
            Mood::Video => "Generate Video",
            Mood::Live => "Go Live",
            Mood::ThreeD => "Generate 3D",
            Mood::Pdf => "Generate PDF",
        }
    }

    /// Placeholder text for the input in this mood.
    pub fn input_placeholder(&self) -> &'static str {
        match self {
            Mood::Text => "Ask anything...",
            Mood::Image => "Describe the image you want to create...",
            Mood::Audio => "Describe the sound or music...",
            Mood::Video => "Describe the video you want to generate...",
            Mood::Live => "Start a live session...",
            Mood::ThreeD => "Describe the 3D model to generate...",
            Mood::Pdf => "Describe the document to create...",
        }
    }
}

/// Actions available in each mood's action bar.
#[derive(Debug, Clone)]
pub struct MoodActionSet {
    pub mood: Mood,
    pub actions: Vec<MoodAction>,
}

/// An individual action button in the mood action bar.
#[derive(Debug, Clone)]
pub struct MoodAction {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub tooltip: &'static str,
}

/// Get the action set for a given mood.
pub fn actions_for_mood(mood: Mood) -> MoodActionSet {
    match mood {
        Mood::Text => MoodActionSet {
            mood,
            actions: vec![
                MoodAction { id: "attach-file", label: "File", icon: "paperclip", tooltip: "Attach a file" },
                MoodAction { id: "attach-image", label: "Image", icon: "image", tooltip: "Attach an image" },
                MoodAction { id: "voice-input", label: "Voice", icon: "mic", tooltip: "Voice input" },
                MoodAction { id: "code-block", label: "Code", icon: "code", tooltip: "Insert code block" },
            ],
        },
        Mood::Image => MoodActionSet {
            mood,
            actions: vec![
                MoodAction { id: "upload-reference", label: "Reference", icon: "image", tooltip: "Upload reference image" },
                MoodAction { id: "style-preset", label: "Style", icon: "palette", tooltip: "Choose style preset" },
                MoodAction { id: "aspect-ratio", label: "Size", icon: "maximize", tooltip: "Set dimensions" },
                MoodAction { id: "provider-select", label: "Provider", icon: "server", tooltip: "Choose image provider" },
            ],
        },
        Mood::Audio => MoodActionSet {
            mood,
            actions: vec![
                MoodAction { id: "voice-select", label: "Voice", icon: "mic", tooltip: "Choose voice" },
                MoodAction { id: "music-genre", label: "Genre", icon: "music", tooltip: "Select music genre" },
                MoodAction { id: "duration", label: "Duration", icon: "clock", tooltip: "Set duration" },
                MoodAction { id: "provider-select", label: "Provider", icon: "server", tooltip: "Choose audio provider" },
            ],
        },
        Mood::Video => MoodActionSet {
            mood,
            actions: vec![
                MoodAction { id: "upload-reference", label: "Reference", icon: "image", tooltip: "Upload reference frame" },
                MoodAction { id: "aspect-ratio", label: "Size", icon: "maximize", tooltip: "Set resolution" },
                MoodAction { id: "duration", label: "Duration", icon: "clock", tooltip: "Set video length" },
                MoodAction { id: "provider-select", label: "Provider", icon: "server", tooltip: "Choose video provider" },
            ],
        },
        Mood::Live => MoodActionSet {
            mood,
            actions: vec![
                MoodAction { id: "camera-select", label: "Camera", icon: "video", tooltip: "Select camera" },
                MoodAction { id: "mic-select", label: "Mic", icon: "mic", tooltip: "Select microphone" },
                MoodAction { id: "screen-share", label: "Screen", icon: "monitor", tooltip: "Share screen" },
            ],
        },
        Mood::ThreeD => MoodActionSet {
            mood,
            actions: vec![
                MoodAction { id: "upload-reference", label: "Reference", icon: "image", tooltip: "Upload reference image" },
                MoodAction { id: "format-select", label: "Format", icon: "cube", tooltip: "Output format (glTF, OBJ, STL)" },
                MoodAction { id: "texture", label: "Textures", icon: "palette", tooltip: "PBR texture options" },
                MoodAction { id: "provider-select", label: "Provider", icon: "server", tooltip: "Choose 3D provider" },
            ],
        },
        Mood::Pdf => MoodActionSet {
            mood,
            actions: vec![
                MoodAction { id: "template", label: "Template", icon: "file-text", tooltip: "Choose document template" },
                MoodAction { id: "format-select", label: "Format", icon: "file", tooltip: "Output format (PDF, XLSX, HTML)" },
                MoodAction { id: "chart", label: "Chart", icon: "bar-chart", tooltip: "Add data visualization" },
                MoodAction { id: "attach-data", label: "Data", icon: "database", tooltip: "Attach data source" },
            ],
        },
    }
}
