use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use dx_agent_hooks::{HookEvent, HookEventType, HookRegistry};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Args)]
pub struct HooksArgs {
    #[command(subcommand)]
    pub command: HooksCommand,
}

#[derive(Debug, Subcommand)]
pub enum HooksCommand {
    /// List available hooks
    List {
        /// Optional hooks directory (default: .dx/hooks)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Create a new hook template file
    Init {
        /// Hook name (example: on_message)
        name: String,
        /// Optional hooks directory (default: .dx/hooks)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Enable a disabled hook (renames *.lua.disabled -> *.lua)
    Enable {
        name: String,
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Disable a hook (renames *.lua -> *.lua.disabled)
    Disable {
        name: String,
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Remove a hook file
    Remove {
        name: String,
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Trigger hook(s) with an event for local testing
    Run {
        /// Event type: message|session_start|session_end|file|command|tool|error|scheduled
        #[arg(long)]
        event: String,
        /// Event source label (example: cli)
        #[arg(long, default_value = "cli")]
        source: String,
        /// JSON object payload for event data
        #[arg(long)]
        data: Option<String>,
        /// Optional hooks directory (default: .dx/hooks)
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

pub async fn run(args: HooksArgs) -> Result<()> {
    match args.command {
        HooksCommand::List { path } => list_hooks(path),
        HooksCommand::Init { name, path } => init_hook(&name, path),
        HooksCommand::Enable { name, path } => enable_hook(&name, path),
        HooksCommand::Disable { name, path } => disable_hook(&name, path),
        HooksCommand::Remove { name, path } => remove_hook(&name, path),
        HooksCommand::Run {
            event,
            source,
            data,
            path,
        } => run_hooks(&event, &source, data.as_deref(), path),
    }
}

fn hooks_dir(path: Option<PathBuf>) -> PathBuf {
    path.unwrap_or_else(|| PathBuf::from(".dx").join("hooks"))
}

fn normalize_name(name: &str) -> String {
    name.trim_end_matches(".lua").trim_end_matches(".disabled").to_string()
}

fn list_hooks(path: Option<PathBuf>) -> Result<()> {
    let dir = hooks_dir(path);
    std::fs::create_dir_all(&dir)?;

    let mut enabled = Vec::new();
    let mut disabled = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".lua") {
            enabled.push(name);
        } else if name.ends_with(".lua.disabled") {
            disabled.push(name);
        }
    }

    enabled.sort();
    disabled.sort();

    println!("Hooks directory: {}", dir.display());
    if enabled.is_empty() && disabled.is_empty() {
        println!("No hooks found. Use `dx hooks init <name>` to create one.");
        return Ok(());
    }

    println!("\nEnabled:");
    for hook in enabled {
        println!("  - {}", hook);
    }

    println!("\nDisabled:");
    for hook in disabled {
        println!("  - {}", hook);
    }

    Ok(())
}

fn init_hook(name: &str, path: Option<PathBuf>) -> Result<()> {
    let dir = hooks_dir(path);
    std::fs::create_dir_all(&dir)?;

    let base = normalize_name(name);
    let file = dir.join(format!("{}.lua", base));
    if file.exists() {
        return Err(anyhow::anyhow!("Hook already exists: {}", file.display()));
    }

    let template = format!(
        "-- {base}.lua\nfunction {base}(event)\n    log(\"hook fired: \" .. (event.type or \"unknown\"))\n    return true\nend\n"
    );
    std::fs::write(&file, template)?;
    println!("Created hook: {}", file.display());
    Ok(())
}

fn enable_hook(name: &str, path: Option<PathBuf>) -> Result<()> {
    let dir = hooks_dir(path);
    let base = normalize_name(name);
    let from = dir.join(format!("{}.lua.disabled", base));
    let to = dir.join(format!("{}.lua", base));
    if !from.exists() {
        return Err(anyhow::anyhow!("Disabled hook not found: {}", from.display()));
    }
    std::fs::rename(&from, &to)?;
    println!("Enabled hook: {}", base);
    Ok(())
}

fn disable_hook(name: &str, path: Option<PathBuf>) -> Result<()> {
    let dir = hooks_dir(path);
    let base = normalize_name(name);
    let from = dir.join(format!("{}.lua", base));
    let to = dir.join(format!("{}.lua.disabled", base));
    if !from.exists() {
        return Err(anyhow::anyhow!("Hook not found: {}", from.display()));
    }
    std::fs::rename(&from, &to)?;
    println!("Disabled hook: {}", base);
    Ok(())
}

fn remove_hook(name: &str, path: Option<PathBuf>) -> Result<()> {
    let dir = hooks_dir(path);
    let base = normalize_name(name);
    let enabled = dir.join(format!("{}.lua", base));
    let disabled = dir.join(format!("{}.lua.disabled", base));

    let target = if enabled.exists() {
        enabled
    } else if disabled.exists() {
        disabled
    } else {
        return Err(anyhow::anyhow!("Hook not found: {}", base));
    };

    std::fs::remove_file(&target)?;
    println!("Removed hook: {}", target.display());
    Ok(())
}

fn parse_event_type(raw: &str) -> HookEventType {
    match raw.to_ascii_lowercase().as_str() {
        "message" | "message_received" => HookEventType::MessageReceived,
        "message_sent" => HookEventType::MessageSent,
        "session_start" => HookEventType::SessionStart,
        "session_end" => HookEventType::SessionEnd,
        "file" | "file_changed" => HookEventType::FileChanged,
        "command" | "command_executed" => HookEventType::CommandExecuted,
        "tool" | "tool_invoked" => HookEventType::ToolInvoked,
        "error" => HookEventType::Error,
        "scheduled" => HookEventType::Scheduled,
        other => HookEventType::Custom(other.to_string()),
    }
}

fn apply_event_data(event: HookEvent, raw_data: Option<&str>) -> Result<HookEvent> {
    let Some(raw_data) = raw_data else {
        return Ok(event);
    };
    let parsed: Value =
        serde_json::from_str(raw_data).with_context(|| "--data must be a valid JSON object")?;
    let map = parsed
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("--data must be a JSON object"))?;

    let mut out = event;
    for (k, v) in map {
        out = out.with_data(k.clone(), v.clone());
    }
    Ok(out)
}

fn run_hooks(event: &str, source: &str, data: Option<&str>, path: Option<PathBuf>) -> Result<()> {
    let dir = hooks_dir(path);
    std::fs::create_dir_all(&dir)?;

    let registry = HookRegistry::new(dir.clone())
        .map_err(|e| anyhow::anyhow!("Failed to create hook registry: {}", e))?;

    let hook_event = apply_event_data(HookEvent::new(parse_event_type(event), source), data)?;
    let results = registry.trigger(&hook_event);

    println!("Triggered {} hook(s)", results.len());
    for (index, result) in results.iter().enumerate() {
        println!("\n#{} propagate={} logs={}", index + 1, result.propagate, result.logs.len());
        for line in &result.logs {
            println!("  - {}", line);
        }
    }

    Ok(())
}

#[allow(dead_code)]
fn _is_lua_file(path: &Path) -> bool {
    path.extension().and_then(|x| x.to_str()) == Some("lua")
}
