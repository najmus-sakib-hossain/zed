//! Onboarding UI for first-time DX CLI setup
//!
//! Provides an interactive configuration wizard using cliclack prompts
//! to guide users through initial setup of DX CLI.

use anyhow::Result;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config::dx_config::{
    BuildConfig, DevConfig, DxConfig, FontToolConfig, IconToolConfig, MediaToolConfig,
    ProjectConfig, RuntimeConfig, StyleToolConfig, ToolsConfig,
};
use crate::{confirm, input, intro, multiselect, outro, select};

/// Integration options for external services
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntegrationConfig {
    pub elevenlabs: Option<String>,
    pub zapier: Option<String>,
    pub n8n: Option<String>,
    #[serde(default)]
    pub browser_control: bool,
    #[serde(default)]
    pub gmail: bool,
    #[serde(default)]
    pub github: bool,
    #[serde(default)]
    pub notion: bool,
    #[serde(default)]
    pub obsidian: bool,
    #[serde(default)]
    pub twitter: bool,
    #[serde(default)]
    pub spotify: bool,
    #[serde(default)]
    pub weather: bool,
}

/// Check if DX CLI is configured
pub fn is_configured() -> bool {
    get_config_path().exists()
}

/// Get the configuration file path
pub fn get_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("dx")
        .join("config.toml")
}

/// Run the onboarding wizard
pub fn run_onboarding() -> Result<(DxConfig, IntegrationConfig)> {
    intro("Dx".bright_green())?;

    // Project Configuration
    let project_name: String = input("What's your default project name?")
        .placeholder("my-dx-project")
        .default_input("my-dx-project")
        .interact()?;

    let project_version: String = input("Default project version?")
        .placeholder("0.1.0")
        .default_input("0.1.0")
        .interact()?;

    // Development Server
    let dev_port: String = input("Development server port?")
        .placeholder("3000")
        .default_input("3000")
        .interact()?;

    let dev_port = dev_port.parse::<u16>().unwrap_or(3000);

    let dev_open = confirm("Auto-open browser on dev server start?")
        .initial_value(false)
        .interact()?;

    // Build Configuration
    let build_target = select("Default build target?")
        .item("browser", "Browser (Web)", "")
        .item("node", "Node.js", "")
        .item("cloudflare", "Cloudflare Workers", "")
        .item("vercel", "Vercel Edge", "")
        .interact()?;

    let minify = confirm("Enable minification by default?").initial_value(true).interact()?;

    let sourcemap = confirm("Generate source maps?").initial_value(false).interact()?;

    // Runtime Configuration
    let typescript = confirm("Use TypeScript?").initial_value(true).interact()?;

    // Tool Configuration
    let configure_tools = confirm("Configure asset tools (style, media, font, icon)?")
        .initial_value(false)
        .interact()?;

    let tools = if configure_tools {
        configure_asset_tools()?
    } else {
        ToolsConfig::default()
    };

    // Integration Configuration
    let configure_integrations =
        confirm("Configure external integrations?").initial_value(false).interact()?;

    let integrations = if configure_integrations {
        configure_integrations_wizard()?
    } else {
        IntegrationConfig::default()
    };

    // Build final config
    let config = DxConfig {
        project: ProjectConfig {
            name: project_name,
            version: project_version,
            description: None,
        },
        build: BuildConfig {
            target: build_target.to_string(),
            minify,
            sourcemap,
            out_dir: "dist".to_string(),
        },
        dev: DevConfig {
            port: dev_port,
            open: dev_open,
            https: false,
        },
        runtime: RuntimeConfig {
            jsx: "dx".to_string(),
            typescript,
        },
        tools,
    };

    outro("Configuration complete!".bright_green())?;

    Ok((config, integrations))
}

/// Configure asset tools
fn configure_asset_tools() -> Result<ToolsConfig> {
    // Style tool
    let style_preprocessor = select("CSS preprocessor?")
        .item("none", "None (plain CSS)", "")
        .item("sass", "Sass/SCSS", "")
        .item("less", "Less", "")
        .item("stylus", "Stylus", "")
        .interact()?;

    let style_modules = confirm("Enable CSS modules?").initial_value(false).interact()?;

    let style = if style_preprocessor != "none" || style_modules {
        Some(StyleToolConfig {
            preprocessor: if style_preprocessor != "none" {
                Some(style_preprocessor.to_string())
            } else {
                None
            },
            modules: style_modules,
            postcss_plugins: vec![],
        })
    } else {
        None
    };

    // Media tool
    let media_quality: String = input("Image quality (1-100)?")
        .placeholder("85")
        .default_input("85")
        .interact()?;

    let media_quality = media_quality.parse::<u8>().unwrap_or(85).clamp(1, 100);

    let media_formats = multiselect("Output image formats?")
        .item("webp", "WebP", "")
        .item("avif", "AVIF", "")
        .item("jpeg", "JPEG", "")
        .item("png", "PNG", "")
        .interact()?;

    let media = if !media_formats.is_empty() {
        Some(MediaToolConfig {
            quality: media_quality,
            formats: media_formats.iter().map(|s: &&str| s.to_string()).collect(),
        })
    } else {
        None
    };

    // Font tool
    let font_subset = confirm("Enable font subsetting?").initial_value(true).interact()?;

    let font = if font_subset {
        Some(FontToolConfig {
            subset: true,
            ranges: vec!["latin".to_string()],
        })
    } else {
        None
    };

    // Icon tool
    let icon_sprite = confirm("Generate icon sprites?").initial_value(true).interact()?;

    let icon = if icon_sprite {
        Some(IconToolConfig {
            sprite: true,
            sizes: vec![16, 24, 32, 48],
        })
    } else {
        None
    };

    Ok(ToolsConfig {
        style,
        media,
        font,
        icon,
    })
}

/// Configure external integrations
fn configure_integrations_wizard() -> Result<IntegrationConfig> {
    let selected_integrations = multiselect("Select integrations to enable:")
        .item("browser", "Browser Control (Chrome/Chromium)", "")
        .item("gmail", "Gmail (Pub/Sub email triggers)", "")
        .item("github", "GitHub (Code, issues, PRs)", "")
        .item("notion", "Notion (Workspace & databases)", "")
        .item("obsidian", "Obsidian (Knowledge graph)", "")
        .item("twitter", "Twitter/X (Tweet, reply, search)", "")
        .item("spotify", "Spotify (Music playback)", "")
        .item("weather", "Weather (Forecasts & conditions)", "")
        .interact()?;

    let mut config = IntegrationConfig::default();

    for integration in selected_integrations {
        match integration {
            "browser" => config.browser_control = true,
            "gmail" => config.gmail = true,
            "github" => config.github = true,
            "notion" => config.notion = true,
            "obsidian" => config.obsidian = true,
            "twitter" => config.twitter = true,
            "spotify" => config.spotify = true,
            "weather" => config.weather = true,
            _ => {}
        }
    }

    // API Keys for specific services
    if confirm("Configure ElevenLabs (Text-to-Speech)?")
        .initial_value(false)
        .interact()?
    {
        let api_key: String = input("ElevenLabs API key:").placeholder("sk_...").interact()?;
        config.elevenlabs = Some(api_key);
    }

    if confirm("Configure Zapier webhooks?").initial_value(false).interact()? {
        let webhook_url: String = input("Zapier webhook URL:")
            .placeholder("https://hooks.zapier.com/...")
            .interact()?;
        config.zapier = Some(webhook_url);
    }

    if confirm("Configure n8n automation?").initial_value(false).interact()? {
        let webhook_url: String = input("n8n webhook URL:")
            .placeholder("https://your-n8n.com/webhook/...")
            .interact()?;
        config.n8n = Some(webhook_url);
    }

    Ok(config)
}

/// Save configuration to disk
pub fn save_config(config: &DxConfig, integrations: &IntegrationConfig) -> Result<()> {
    let config_path = get_config_path();

    // Create config directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Serialize main config
    let config_toml = toml::to_string_pretty(config)?;
    std::fs::write(&config_path, config_toml)?;

    // Save integrations separately
    let integrations_path = config_path.with_file_name("integrations.toml");
    let integrations_toml = toml::to_string_pretty(integrations)?;
    std::fs::write(integrations_path, integrations_toml)?;

    Ok(())
}

/// Load existing configuration
pub fn load_config() -> Result<(DxConfig, IntegrationConfig)> {
    let config_path = get_config_path();
    let config_str = std::fs::read_to_string(&config_path)?;
    let config: DxConfig = toml::from_str(&config_str)?;

    let integrations_path = config_path.with_file_name("integrations.toml");
    let integrations = if integrations_path.exists() {
        let integrations_str = std::fs::read_to_string(integrations_path)?;
        toml::from_str(&integrations_str)?
    } else {
        IntegrationConfig::default()
    };

    Ok((config, integrations))
}
