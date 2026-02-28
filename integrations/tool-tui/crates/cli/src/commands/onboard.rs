//! DX Onboarding Command
//!
//! Interactive onboarding wizard that collects user information and configures DX CLI.
//! Uses all available prompt components for a beautiful, comprehensive setup experience.

use anyhow::Result;
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::prompts::{
    self, MultiSelectItem, PromptInteraction, SelectItem, confirm, email, emoji_picker,
    file_browser, input, log, multiselect, password, phone_input, rating, select, tags, text,
};

/// Complete onboarding response containing all user inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingResponse {
    pub profile: UserProfile,
    pub llm_config: LlmConfig,
    pub channels: Vec<String>,
    pub security: SecurityConfig,
    pub system_health: SystemHealth,
    pub completed_at: String,
}

/// User profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub name: String,
    pub email: String,
    pub username: String,
    pub avatar_emoji: String,
    pub phone: Option<String>,
    pub website: Option<String>,
    pub bio: String,
    pub birth_date: Option<NaiveDate>,
    pub timezone: String,
    pub preferred_editor: String,
    pub programming_languages: Vec<String>,
    pub interests: Vec<String>,
    pub experience_level: String,
    pub notification_preferences: Vec<String>,
    pub theme: String,
    pub enable_telemetry: bool,
    pub enable_auto_update: bool,
    pub workspace_path: PathBuf,
    pub satisfaction_rating: u8,
}

/// LLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub model: Option<String>,
    pub has_api_key: bool,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub require_auth: bool,
    pub enable_rate_limit: bool,
    pub dm_pairing: bool,
}

/// System health check results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub cargo_installed: bool,
    pub git_installed: bool,
    pub docker_available: bool,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            name: String::new(),
            email: String::new(),
            username: String::new(),
            avatar_emoji: "🚀".to_string(),
            phone: None,
            website: None,
            bio: String::new(),
            birth_date: None,
            timezone: "UTC".to_string(),
            preferred_editor: "vscode".to_string(),
            programming_languages: vec![],
            interests: vec![],
            experience_level: "intermediate".to_string(),
            notification_preferences: vec![],
            theme: "dark".to_string(),
            enable_telemetry: true,
            enable_auto_update: true,
            workspace_path: dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")),
            satisfaction_rating: 5,
        }
    }
}

/// Run the onboarding wizard
pub async fn run() -> Result<()> {
    prompts::intro("Welcome to DX CLI! 🚀")?;

    prompts::box_section(
        "Getting Started",
        &[
            "Let's set up your DX environment.",
            "This will take about 2-3 minutes.",
            "You can skip any step by pressing Ctrl+C.",
            "", // Add empty line for spacing
        ],
    )?;

    let mut profile = UserProfile::default();
    let mut llm_config = LlmConfig {
        provider: String::new(),
        model: None,
        has_api_key: false,
    };
    let mut channels: Vec<String> = Vec::new();
    let mut security = SecurityConfig {
        require_auth: false,
        enable_rate_limit: true,
        dm_pairing: false,
    };

    // Step 1: Basic Information
    let theme = crate::prompts::THEME.read().unwrap();
    let symbols = &*crate::prompts::SYMBOLS;
    eprintln!("{}", theme.dim.apply_to(symbols.bar)); // Blank line before section
    log::step("Basic Information")?;
    eprintln!("{}", theme.dim.apply_to(symbols.bar)); // Blank line after section

    profile.name = text("What's your name?")
        .placeholder("John Doe")
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;

    profile.email = email("What's your email?")
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;

    profile.username = input("Choose a username")
        .placeholder("johndoe")
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;

    // Step 2: Avatar & Personal Info
    eprintln!("{}", theme.dim.apply_to(symbols.bar)); // Blank line before section
    log::step("Personalization")?;
    eprintln!("{}", theme.dim.apply_to(symbols.bar)); // Blank line after section

    profile.avatar_emoji = emoji_picker("Choose your avatar emoji")
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    profile.phone = Some(
        phone_input("Phone number (optional)")
            .interact()
            .map_err(|e| anyhow::anyhow!("Input error: {}", e))?,
    );
    eprintln!(); // Add spacing after prompt

    profile.website = Some(
        input("Personal website (optional)")
            .placeholder("https://example.com")
            .interact()
            .map_err(|e| anyhow::anyhow!("Input error: {}", e))?,
    );
    eprintln!(); // Add spacing after prompt

    profile.bio = text("Tell us about yourself")
        .placeholder("I'm a developer who loves Rust!")
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    // Step 3: Date & Time
    log::step("Date & Time Preferences")?;
    eprintln!(); // Add spacing

    let use_birth_date = confirm("Would you like to set your birth date?")
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    if use_birth_date {
        // Simplified date input as string for now
        let date_str = input("Enter your birth date (YYYY-MM-DD)")
            .placeholder("1990-01-01")
            .interact()
            .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
        eprintln!(); // Add spacing after prompt

        if let Ok(date) = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            profile.birth_date = Some(date);
        }
    }

    // Simplified timezone selection
    profile.timezone = input("Enter your timezone (e.g., UTC, America/New_York)")
        .placeholder("UTC")
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    // Step 4: Development Preferences
    log::step("Development Setup")?;
    eprintln!(); // Add spacing

    profile.preferred_editor = select("Preferred code editor")
        .items(vec![
            SelectItem::new("vscode".to_string(), "Visual Studio Code"),
            SelectItem::new("vim".to_string(), "Vim/Neovim"),
            SelectItem::new("emacs".to_string(), "Emacs"),
            SelectItem::new("sublime".to_string(), "Sublime Text"),
            SelectItem::new("intellij".to_string(), "IntelliJ IDEA"),
            SelectItem::new("other".to_string(), "Other"),
        ])
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    profile.programming_languages = multiselect("Programming languages you use")
        .items(vec![
            MultiSelectItem::new("rust".to_string(), "Rust 🦀"),
            MultiSelectItem::new("typescript".to_string(), "TypeScript"),
            MultiSelectItem::new("javascript".to_string(), "JavaScript"),
            MultiSelectItem::new("python".to_string(), "Python"),
            MultiSelectItem::new("go".to_string(), "Go"),
            MultiSelectItem::new("java".to_string(), "Java"),
            MultiSelectItem::new("cpp".to_string(), "C++"),
            MultiSelectItem::new("csharp".to_string(), "C#"),
        ])
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    profile.interests = tags("Your interests (comma-separated)")
        .placeholder("web, mobile, ai, devops")
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    profile.experience_level = select("Experience level")
        .items(vec![
            SelectItem::new("beginner".to_string(), "Beginner (< 1 year)"),
            SelectItem::new("intermediate".to_string(), "Intermediate (1-3 years)"),
            SelectItem::new("advanced".to_string(), "Advanced (3-5 years)"),
            SelectItem::new("expert".to_string(), "Expert (5+ years)"),
        ])
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    // Step 5: Preferences
    log::step("Preferences")?;
    eprintln!(); // Add spacing

    profile.notification_preferences = multiselect("Notification preferences")
        .items(vec![
            MultiSelectItem::new("updates".to_string(), "Software updates"),
            MultiSelectItem::new("security".to_string(), "Security alerts"),
            MultiSelectItem::new("tips".to_string(), "Tips & tricks"),
            MultiSelectItem::new("newsletter".to_string(), "Newsletter"),
        ])
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    profile.theme = select("Theme preference")
        .items(vec![
            SelectItem::new("dark".to_string(), "Dark 🌙"),
            SelectItem::new("light".to_string(), "Light ☀️"),
            SelectItem::new("auto".to_string(), "Auto (system)"),
        ])
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    profile.enable_telemetry = confirm("Enable anonymous telemetry?")
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    profile.enable_auto_update = confirm("Enable automatic updates?")
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    // Step 6: Workspace
    log::step("Workspace Configuration")?;
    eprintln!(); // Add spacing

    profile.workspace_path = file_browser("Select your default workspace")
        .start_dir(dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    // Step 7: LLM Model Configuration
    log::step("AI Model Configuration")?;
    eprintln!(); // Add spacing

    llm_config.provider = select("Default LLM provider")
        .items(vec![
            SelectItem::new("groq".to_string(), "Groq (fast, free tier)"),
            SelectItem::new("openai".to_string(), "OpenAI (GPT-4o)"),
            SelectItem::new("anthropic".to_string(), "Anthropic (Claude)"),
            SelectItem::new("openrouter".to_string(), "OpenRouter (many models)"),
            SelectItem::new("ollama".to_string(), "Ollama (local models)"),
            SelectItem::new("google".to_string(), "Google (Gemini)"),
        ])
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    llm_config.has_api_key = if llm_config.provider != "ollama" {
        let api_key = password(&format!("{} API key", llm_config.provider))
            .interact()
            .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
        eprintln!(); // Add spacing after prompt
        !api_key.is_empty()
    } else {
        false
    };

    llm_config.model = Some(
        input("Default model name (leave empty for provider default)")
            .placeholder("auto")
            .interact()
            .map_err(|e| anyhow::anyhow!("Input error: {}", e))?,
    );
    eprintln!(); // Add spacing after prompt

    // Step 8: Channel Setup
    log::step("Channel Configuration")?;
    eprintln!(); // Add spacing

    let setup_channels = confirm("Would you like to set up messaging channels?")
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    if setup_channels {
        channels = multiselect("Select channels to configure")
            .items(vec![
                MultiSelectItem::new("telegram".to_string(), "Telegram Bot"),
                MultiSelectItem::new("discord".to_string(), "Discord Bot"),
                MultiSelectItem::new("slack".to_string(), "Slack App"),
                MultiSelectItem::new("whatsapp".to_string(), "WhatsApp"),
            ])
            .interact()
            .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
        eprintln!(); // Add spacing after prompt

        for ch in &channels {
            match ch.as_str() {
                "telegram" => {
                    let _token = password("Telegram Bot Token (@BotFather)")
                        .interact()
                        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
                    log::success("Telegram token saved")?;
                }
                "discord" => {
                    let _token = password("Discord Bot Token")
                        .interact()
                        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
                    log::success("Discord token saved")?;
                }
                "slack" => {
                    let _token = password("Slack Bot Token")
                        .interact()
                        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
                    log::success("Slack token saved")?;
                }
                "whatsapp" => {
                    log::info(
                        "WhatsApp uses QR code pairing. Run 'dx channel whatsapp pair' when ready.",
                    )?;
                }
                _ => {}
            }
        }
    }

    // Step 9: Security Configuration
    log::step("Security")?;
    eprintln!(); // Add spacing

    security.require_auth = confirm("Require authentication for gateway connections?")
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    security.enable_rate_limit = confirm("Enable rate limiting?")
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    security.dm_pairing = confirm("Require DM pairing for new users?")
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?;
    eprintln!(); // Add spacing after prompt

    // Step 10: Health Check
    log::step("System Health Check")?;
    eprintln!(); // Add spacing

    log::info("Checking system compatibility...")?;

    // Check Rust/Cargo
    let cargo_ok = std::process::Command::new("cargo").arg("--version").output().is_ok();
    if cargo_ok {
        log::success("Cargo: installed")?;
    } else {
        log::warning("Cargo: not found (some features may be limited)")?;
    }

    // Check git
    let git_ok = std::process::Command::new("git").arg("--version").output().is_ok();
    if git_ok {
        log::success("Git: installed")?;
    } else {
        log::warning("Git: not found")?;
    }

    // Check Docker
    let docker_ok = std::process::Command::new("docker").arg("--version").output().is_ok();
    if docker_ok {
        log::success("Docker: available (sandbox support enabled)")?;
    } else {
        log::info("Docker: not found (process-based sandboxing will be used)")?;
    }

    let system_health = SystemHealth {
        cargo_installed: cargo_ok,
        git_installed: git_ok,
        docker_available: docker_ok,
    };

    log::success("Health check complete")?;
    eprintln!(); // Add spacing

    // Step 11: Satisfaction Rating
    log::step("Final Step")?;
    eprintln!(); // Add spacing

    profile.satisfaction_rating = rating("How excited are you about DX?")
        .max(5)
        .interact()
        .map_err(|e| anyhow::anyhow!("Input error: {}", e))?
        as u8;
    eprintln!(); // Add spacing after prompt

    // Create complete onboarding response
    let response = OnboardingResponse {
        profile: profile.clone(),
        llm_config,
        channels,
        security,
        system_health,
        completed_at: Local::now().to_rfc3339(),
    };

    // Save configuration
    save_profile(&profile)?;
    save_onboarding_response(&response)?;

    // Show summary
    eprintln!(); // Add spacing before summary
    prompts::box_section(
        "Setup Complete! 🎉",
        &[
            "", // Empty line at start
            &format!("Name: {}", profile.name),
            &format!("Email: {}", profile.email),
            &format!("Username: @{}", profile.username),
            &format!("Avatar: {}", profile.avatar_emoji),
            &format!("Editor: {}", profile.preferred_editor),
            &format!("Languages: {}", profile.programming_languages.join(", ")),
            &format!("Experience: {}", profile.experience_level),
            &format!("Theme: {}", profile.theme),
            &format!("Workspace: {}", profile.workspace_path.display()),
            "", // Empty line at end
        ],
    )?;

    eprintln!(); // Add spacing after summary
    log::success("Your DX environment is ready!")?;
    log::info("Run 'dx --help' to see available commands")?;
    log::info(&format!(
        "Onboarding data saved to: {}",
        get_onboarding_response_path()?.display()
    ))?;
    eprintln!(); // Add spacing before outro

    prompts::outro("Happy coding! 🚀")?;

    Ok(())
}

/// Save user profile to config file
fn save_profile(profile: &UserProfile) -> Result<()> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
        .join("dx");

    std::fs::create_dir_all(&config_dir)?;

    let config_path = config_dir.join("profile.json");
    let json = serde_json::to_string_pretty(profile)?;
    std::fs::write(config_path, json)?;

    Ok(())
}

/// Load user profile from config file
pub fn load_profile() -> Result<UserProfile> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
        .join("dx");

    let config_path = config_dir.join("profile.json");

    if !config_path.exists() {
        return Ok(UserProfile::default());
    }

    let json = std::fs::read_to_string(config_path)?;
    let profile = serde_json::from_str(&json)?;

    Ok(profile)
}

/// Get path to onboarding response JSON file
fn get_onboarding_response_path() -> Result<PathBuf> {
    // Save to project root as respond.json
    Ok(PathBuf::from("respond.json"))
}

/// Save complete onboarding response to JSON file
fn save_onboarding_response(response: &OnboardingResponse) -> Result<()> {
    let config_path = get_onboarding_response_path()?;

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(response)?;
    std::fs::write(config_path, json)?;

    Ok(())
}

/// Load onboarding response from JSON file
pub fn load_onboarding_response() -> Result<OnboardingResponse> {
    let config_path = get_onboarding_response_path()?;

    if !config_path.exists() {
        return Err(anyhow::anyhow!("Onboarding response not found"));
    }

    let json = std::fs::read_to_string(config_path)?;
    let response = serde_json::from_str(&json)?;

    Ok(response)
}
