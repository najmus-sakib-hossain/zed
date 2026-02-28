//! Command definitions

use clap::Subcommand;

// use crate::commands;  // Not needed for minimal build

#[derive(Subcommand)]
pub enum Commands {
    /// Interactive onboarding wizard for first-time setup
    #[command(visible_alias = "setup")]
    Onboard,
}

// All other commands commented out for minimal onboarding build
/*
#[derive(Subcommand)]
pub enum Commands {
    // ═══════════════════════════════════════════════════════════════════
    //  PROJECT COMMANDS
    // ═══════════════════════════════════════════════════════════════════
    /// Initialize a new DX project
    #[command(visible_alias = "i")]
    Init(super::args_project::InitArgs),

    /// Start development server with hot reload
    #[command(visible_alias = "d")]
    Dev(super::args_project::DevArgs),

    /// Build project for production
    #[command(visible_alias = "b")]
    Build(super::args_project::BuildArgs),

    /// Run the project
    #[command(visible_alias = "r")]
    Run(super::args_project::RunArgs),

    /// Run tests
    #[command(visible_alias = "t")]
    Test(super::args_project::TestArgs),

    /// Deploy to production
    Deploy(super::args_project::DeployArgs),

    // ═══════════════════════════════════════════════════════════════════
    //  CODE QUALITY
    // ═══════════════════════════════════════════════════════════════════
    /// Check code quality (format, lint, score, test, coverage) - 100-200x faster than ESLint
    #[command(visible_alias = "chk")]
    Check(commands::check::CheckArgs),

    // ═══════════════════════════════════════════════════════════════════
    //  ASSET TOOLS
    // ═══════════════════════════════════════════════════════════════════
    /// Binary CSS (B-CSS) compiler - 98% smaller, 80x faster
    #[command(visible_alias = "css")]
    Style(commands::style::StyleArgs),

    /// Image/video optimization - WebP, AVIF, responsive srcsets
    #[command(visible_alias = "img")]
    Media(commands::media::MediaArgs),

    /// Font subsetting and WOFF2 optimization
    Font(commands::font::FontArgs),

    /// SVG icon system with binary encoding and sprite generation
    Icon(commands::icon::IconArgs),

    // ═══════════════════════════════════════════════════════════════════
    //  INFRASTRUCTURE
    // ═══════════════════════════════════════════════════════════════════
    /// Package manager + orchestrator for all dx-* crates
    #[command(visible_alias = "f")]
    Forge(commands::forge::ForgeArgs),

    /// World-record data format (DX ∞) - 73% smaller, 4x faster
    #[command(visible_alias = "ser", visible_alias = "data")]
    Serializer(commands::serializer::SerializerArgs),

    /// Markdown beautifier - human-readable + LLM-optimized
    #[command(visible_alias = "md")]
    Markdown(commands::markdown::MarkdownCommand),

    /// Token analysis and efficiency metrics
    #[command(visible_alias = "tok")]
    Token(super::args_animation::TokenArgs),

    /// Resource monitoring (CPU, memory, I/O, network)
    #[command(visible_alias = "mon")]
    Monitor(commands::monitor::MonitorArgs),

    // ═══════════════════════════════════════════════════════════════════
    //  DEVELOPMENT
    // ═══════════════════════════════════════════════════════════════════
    /// AI agents control - review, refactor, test generation
    #[command(visible_alias = "ai")]
    Driven(commands::driven::DrivenArgs),

    /// Code generation tools - components, APIs, forms, CRUD
    #[command(visible_alias = "gen", visible_alias = "g")]
    Generator(commands::generator::GeneratorArgs),

    /// Code editors + preinstall and setup
    #[command(visible_alias = "ws", visible_alias = "ide")]
    Workspace(commands::workspace::WorkspaceArgs),

    /// Sandbox management - isolated execution environments
    #[command(visible_alias = "sb")]
    Sandbox(commands::sandbox::SandboxCommand),

    // Temporarily disabled - WhatsApp integration
    // /// WhatsApp messaging - send messages from CLI
    // #[command(visible_alias = "wa")]
    // WhatsApp(commands::whatsapp::WhatsAppCommand),

    // ═══════════════════════════════════════════════════════════════════
    //  GIT & VERSION CONTROL
    // ═══════════════════════════════════════════════════════════════════
    /// Git integration - status, commit, diff, branch, stash
    Git(commands::git::GitArgs),

    // ═══════════════════════════════════════════════════════════════════
    //  EDITOR & CODE VIEWING
    // ═══════════════════════════════════════════════════════════════════
    /// Built-in code editor with syntax highlighting, vim keys, minimap
    #[command(visible_alias = "ed", visible_alias = "vi")]
    Editor(commands::editor::EditorArgs),

    // ═══════════════════════════════════════════════════════════════════
    //  SECURITY & AUDIT
    // ═══════════════════════════════════════════════════════════════════
    /// Security audit, secrets management, permissions
    #[command(visible_alias = "sec")]
    Security(commands::security::SecurityArgs),

    // ═══════════════════════════════════════════════════════════════════
    //  PLUGINS
    // ═══════════════════════════════════════════════════════════════════
    /// Plugin management - install, remove, update, create
    #[command(visible_alias = "plug")]
    Plugin(commands::plugin::PluginArgs),

    /// Lua hooks management - list, init, enable/disable, run
    #[command(visible_alias = "hk")]
    Hooks(commands::hooks::HooksArgs),

    // ═══════════════════════════════════════════════════════════════════
    //  GATEWAY & CHANNELS
    // ═══════════════════════════════════════════════════════════════════
    /// Gateway server for platform apps (iOS, Android, macOS)
    #[command(visible_alias = "gw")]
    Gateway(commands::gateway::GatewayArgs),

    /// Messaging channels - WhatsApp, Telegram, Discord, Slack, Signal, iMessage
    #[command(visible_alias = "ch", visible_alias = "msg")]
    Channel(commands::channel::ChannelArgs),

    // ═══════════════════════════════════════════════════════════════════
    //  SHELL & SELF
    // ═══════════════════════════════════════════════════════════════════
    /// Shell integration and completions
    Shell(super::args_shell::ShellArgs),

    /// Self-management commands (update, info)
    #[command(name = "self")]
    SelfCmd(super::args_shell::SelfArgs),

    /// Configure or reconfigure DX CLI
    Config(super::args_shell::ConfigArgs),

    /// Interactive onboarding wizard for first-time setup
    #[command(visible_alias = "setup")]
    Onboard,

    // ═══════════════════════════════════════════════════════════════════
    //  UTILITY
    // ═══════════════════════════════════════════════════════════════════
    /// Show project and environment information
    Info(super::args_utility::InfoArgs),

    /// Display system information with ASCII art logo
    #[command(visible_alias = "sys")]
    System,

    /// Display ASCII art logo gallery from programming languages and tools
    Logo,

    /// Clean build artifacts and caches
    Clean(super::args_utility::CleanArgs),

    /// Generate shell completions
    Completions(super::args_utility::CompletionsArgs),

    /// Display directory tree with file sizes and code statistics
    Tree(super::args_utility::TreeArgs),

    /// Run diagnostics and health checks
    #[command(visible_alias = "doc")]
    Doctor(commands::doctor::DoctorArgs),

    // ═══════════════════════════════════════════════════════════════════
    //  ANIMATIONS & EASTER EGGS
    // ═══════════════════════════════════════════════════════════════════
    /// Show epic CLI animations (Matrix, train, confetti, etc.)
    #[command(visible_alias = "anim")]
    Animate(super::args_animation::AnimateArgs),

    /// Animated splash screen with 400+ figlet fonts
    Splash,

    /// Rainbow animated text, boxes, and gradients
    Rainbow,

    /// Show icon gallery with all available CLI icons
    Icons,

    /// Play audio with terminal visualizer
    #[command(visible_alias = "play")]
    Sound(commands::sound::SoundArgs),

    /// Transcribe audio to text using Google Gemini or Whisper
    #[command(visible_alias = "transcribe")]
    Audio(commands::audio::AudioArgs),

    /// Interactive terminal features (click, drag-drop, keyboard shortcuts)
    #[command(visible_alias = "int")]
    Interact {
        #[command(subcommand)]
        command: commands::interact::InteractCommand,
    },

    /// Showcase new animation features (toasts, typing effects)
    Showcase(commands::showcase::ShowcaseArgs),

    /// AI chat interface with Agent, Plan, and Ask modes
    #[command(visible_alias = "c")]
    Chat(commands::chat::ChatCommand),

    /// Local LLM management and inference
    Llm(commands::llm::LlmArgs),

    // ═══════════════════════════════════════════════════════════════════
    //  DAEMON & ORCHESTRATION
    // ═══════════════════════════════════════════════════════════════════
    /// Daemon management (Agent 24/7 + Project on-demand)
    #[command(visible_alias = "dm")]
    Daemon(commands::daemon::DaemonArgs),

    /// Traffic branching system with AI bot routing
    Branch(commands::branch::BranchArgs),

    /// Cloudflare R2 storage integration
    R2(commands::r2::R2Args),

    /// AI safety and compliance
    #[command(visible_alias = "safe")]
    Safety(commands::safety::SafetyArgs),

    /// Testing infrastructure
    #[command(visible_alias = "tst")]
    Testing(commands::testing::TestArgs),

    // ═══════════════════════════════════════════════════════════════════
    //  DEMOS
    // ═══════════════════════════════════════════════════════════════════
    /// Demo triple layout (left, center, right in one box)
    Triple,

    /// Demo the 3-format system (human, llm, machine)
    #[command(visible_alias = "formats")]
    DemoFormats,

    /// Show syntax-highlighted code
    Syntax {
        /// File path to display
        file: Option<std::path::PathBuf>,
    },

    /// Show diff between files or demo
    Diff {
        /// Old file path
        old: Option<std::path::PathBuf>,
        /// New file path
        new: Option<std::path::PathBuf>,
    },
}
*/
