//! Context-aware writing profiles — auto-detects application context (Part 19).
//!
//! Detects the active application and applies the appropriate writing profile:
//! - Email client → High grammar, Professional tone
//! - Slack/Discord → Low grammar, Casual tone
//! - Code editor → Grammar off for code / High for comments
//! - Terminal → Grammar off, no predictions
//! - Document editor → Maximum grammar, paragraph continuations
//! - Social media → Medium grammar, short-form optimized

use serde::{Deserialize, Serialize};

/// Application category detected from the active window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppCategory {
    /// Email clients (Outlook, Gmail web, Thunderbird, Apple Mail).
    Email,
    /// Chat/messaging (Slack, Discord, Teams, WhatsApp, Telegram).
    Chat,
    /// Code editor (VS Code, Zed, IntelliJ, Vim, Emacs).
    CodeEditor,
    /// Terminal/Shell (iTerm, Windows Terminal, Alacritty).
    Terminal,
    /// Document editor (Google Docs, Word, Notion, Obsidian).
    DocumentEditor,
    /// Social media (Twitter/X, LinkedIn, Reddit, Facebook).
    SocialMedia,
    /// Web browser (general browsing, not a specific web app).
    WebBrowser,
    /// Unknown application.
    Unknown,
}

impl AppCategory {
    /// Display name for the category.
    pub fn display_name(&self) -> &'static str {
        match self {
            AppCategory::Email => "Email",
            AppCategory::Chat => "Chat / Messaging",
            AppCategory::CodeEditor => "Code Editor",
            AppCategory::Terminal => "Terminal",
            AppCategory::DocumentEditor => "Document Editor",
            AppCategory::SocialMedia => "Social Media",
            AppCategory::WebBrowser => "Web Browser",
            AppCategory::Unknown => "Unknown",
        }
    }
}

/// Writing context profile for a detected application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppWritingProfile {
    /// Detected application category.
    pub category: AppCategory,
    /// Grammar strictness (0.0 = off, 1.0 = maximum).
    pub grammar_strictness: f32,
    /// Tone target.
    pub tone: Tone,
    /// Whether text prediction should be active.
    pub prediction_enabled: bool,
    /// Prediction style (full sentence, short phrase, code completion).
    pub prediction_style: PredictionStyle,
    /// Maximum suggestion length.
    pub max_suggestion_words: usize,
}

/// Tone classification for writing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tone {
    Formal,
    Professional,
    Casual,
    Technical,
    Creative,
}

impl Tone {
    /// Return a human-readable string for the tone.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Formal => "Formal",
            Self::Professional => "Professional",
            Self::Casual => "Casual",
            Self::Technical => "Technical",
            Self::Creative => "Creative",
        }
    }
}

/// Style of text prediction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictionStyle {
    /// Full sentence completion.
    FullSentence,
    /// Short phrase (2-5 words).
    ShortPhrase,
    /// Single word.
    SingleWord,
    /// Code completion (Zeta-style).
    CodeCompletion,
    /// No prediction.
    Disabled,
}

/// Get the writing profile for a detected application category.
pub fn profile_for_category(category: AppCategory) -> AppWritingProfile {
    match category {
        AppCategory::Email => AppWritingProfile {
            category,
            grammar_strictness: 0.9,
            tone: Tone::Professional,
            prediction_enabled: true,
            prediction_style: PredictionStyle::FullSentence,
            max_suggestion_words: 20,
        },
        AppCategory::Chat => AppWritingProfile {
            category,
            grammar_strictness: 0.3,
            tone: Tone::Casual,
            prediction_enabled: true,
            prediction_style: PredictionStyle::ShortPhrase,
            max_suggestion_words: 8,
        },
        AppCategory::CodeEditor => AppWritingProfile {
            category,
            grammar_strictness: 0.0, // Grammar off for code
            tone: Tone::Technical,
            prediction_enabled: true,
            prediction_style: PredictionStyle::CodeCompletion,
            max_suggestion_words: 0,
        },
        AppCategory::Terminal => AppWritingProfile {
            category,
            grammar_strictness: 0.0,
            tone: Tone::Technical,
            prediction_enabled: false,
            prediction_style: PredictionStyle::Disabled,
            max_suggestion_words: 0,
        },
        AppCategory::DocumentEditor => AppWritingProfile {
            category,
            grammar_strictness: 1.0,
            tone: Tone::Formal,
            prediction_enabled: true,
            prediction_style: PredictionStyle::FullSentence,
            max_suggestion_words: 30,
        },
        AppCategory::SocialMedia => AppWritingProfile {
            category,
            grammar_strictness: 0.5,
            tone: Tone::Casual,
            prediction_enabled: true,
            prediction_style: PredictionStyle::ShortPhrase,
            max_suggestion_words: 10,
        },
        AppCategory::WebBrowser => AppWritingProfile {
            category,
            grammar_strictness: 0.5,
            tone: Tone::Professional,
            prediction_enabled: true,
            prediction_style: PredictionStyle::ShortPhrase,
            max_suggestion_words: 12,
        },
        AppCategory::Unknown => AppWritingProfile {
            category,
            grammar_strictness: 0.5,
            tone: Tone::Professional,
            prediction_enabled: true,
            prediction_style: PredictionStyle::ShortPhrase,
            max_suggestion_words: 12,
        },
    }
}

/// Detect the application category from a window title and process name.
///
/// Uses heuristic matching against known application names and window titles.
pub fn detect_category(process_name: &str, window_title: &str) -> AppCategory {
    let proc_lower = process_name.to_lowercase();
    let title_lower = window_title.to_lowercase();

    // Email clients
    if proc_lower.contains("outlook")
        || proc_lower.contains("thunderbird")
        || proc_lower.contains("mail")
        || title_lower.contains("gmail")
        || title_lower.contains("compose")
        || title_lower.contains("inbox")
    {
        return AppCategory::Email;
    }

    // Chat / Messaging
    if proc_lower.contains("slack")
        || proc_lower.contains("discord")
        || proc_lower.contains("teams")
        || proc_lower.contains("telegram")
        || proc_lower.contains("whatsapp")
        || proc_lower.contains("signal")
        || proc_lower.contains("element")
    {
        return AppCategory::Chat;
    }

    // Code editors
    if proc_lower.contains("zed")
        || proc_lower.contains("code")
        || proc_lower.contains("vscode")
        || proc_lower.contains("idea")
        || proc_lower.contains("pycharm")
        || proc_lower.contains("webstorm")
        || proc_lower.contains("vim")
        || proc_lower.contains("nvim")
        || proc_lower.contains("emacs")
        || proc_lower.contains("sublime")
        || proc_lower.contains("atom")
    {
        return AppCategory::CodeEditor;
    }

    // Terminals
    if proc_lower.contains("terminal")
        || proc_lower.contains("iterm")
        || proc_lower.contains("alacritty")
        || proc_lower.contains("kitty")
        || proc_lower.contains("wezterm")
        || proc_lower.contains("cmd")
        || proc_lower.contains("powershell")
        || proc_lower.contains("bash")
        || proc_lower.contains("zsh")
    {
        return AppCategory::Terminal;
    }

    // Document editors
    if proc_lower.contains("word")
        || proc_lower.contains("pages")
        || proc_lower.contains("notion")
        || proc_lower.contains("obsidian")
        || title_lower.contains("google docs")
        || title_lower.contains("document")
        || proc_lower.contains("libreoffice")
    {
        return AppCategory::DocumentEditor;
    }

    // Social media (web-based — detected from browser titles)
    if title_lower.contains("twitter")
        || title_lower.contains("x.com")
        || title_lower.contains("linkedin")
        || title_lower.contains("reddit")
        || title_lower.contains("facebook")
        || title_lower.contains("instagram")
        || title_lower.contains("mastodon")
    {
        return AppCategory::SocialMedia;
    }

    // Web browsers (generic)
    if proc_lower.contains("chrome")
        || proc_lower.contains("firefox")
        || proc_lower.contains("safari")
        || proc_lower.contains("edge")
        || proc_lower.contains("brave")
        || proc_lower.contains("arc")
    {
        return AppCategory::WebBrowser;
    }

    AppCategory::Unknown
}
