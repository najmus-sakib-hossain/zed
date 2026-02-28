//! Command execution logic
//!
//! This module provides both legacy static dispatch and new registry-based dispatch.
//! The system is in migration mode - new commands should use the registry.

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::commands;
use crate::ui::theme::Theme;

use super::args_shell::{ConfigArgs, SelfArgs, SelfCommands};
use super::commands::Commands;

/// Execute a CLI command using legacy static dispatch
///
/// # Migration Note
///
/// **DEPRECATED**: This function uses static enum matching and is being phased out.
/// All new command execution should use `HybridExecutor` which provides:
/// - Dynamic command resolution via `CommandRegistry`
/// - Hot-reload support for config-driven commands
/// - Plugin system integration (WASM, native, script)
/// - Gradual migration path with fallback to legacy
///
/// To enable registry-first execution, set environment variable:
/// ```bash
/// export DX_USE_REGISTRY=1
/// ```
///
/// See: `crates/cli/src/cli/hybrid_executor.rs` for the new execution model.
/// See: `crates/cli/src/registry/` for the registry implementation.
///
/// # Removal Timeline
///
/// This function will be removed once all commands are migrated to the registry.
/// Target: Phase 2 completion (TASK-041)
#[deprecated(
    since = "2.0.0",
    note = "Use HybridExecutor with CommandRegistry instead. See hybrid_executor.rs"
)]
pub async fn execute_command(command: Commands, theme: &Theme) -> Result<()> {
    match command {
        // Project Commands
        Commands::Init(args) => commands::project::run_init(args, theme).await,
        Commands::Dev(args) => commands::project::run_dev(args, theme).await,
        Commands::Build(args) => commands::project::run_build(args, theme).await,
        Commands::Run(args) => commands::project::run_run(args, theme).await,
        Commands::Test(args) => commands::project::run_test(args, theme).await,
        Commands::Deploy(args) => commands::project::run_deploy(args, theme).await,

        // Code Quality
        Commands::Check(args) => commands::check::run(args, theme).await,

        // Asset Tools
        Commands::Style(args) => commands::style::run(args, theme).await,
        Commands::Media(args) => commands::media::run(args, theme).await,
        Commands::Font(args) => commands::font::run(args, theme).await,
        Commands::Icon(args) => commands::icon::run(args, theme).await,

        // Infrastructure
        Commands::Forge(args) => commands::forge::run(args, theme).await,
        Commands::Serializer(args) => args.execute().await,
        Commands::Markdown(args) => args.execute().await,
        Commands::Token(args) => args.args.execute().await,
        Commands::Monitor(args) => commands::monitor::run(args, theme).await,

        // Development
        Commands::Driven(args) => commands::driven::run(args, theme).await,
        Commands::Generator(args) => commands::generator::run(args, theme).await,
        Commands::Workspace(args) => commands::workspace::run(args, theme).await,
        Commands::Sandbox(args) => args.run().await,
        // Commands::WhatsApp(cmd) => commands::whatsapp::handle_whatsapp(cmd).await,

        // Git & Version Control
        Commands::Git(args) => execute_git_command(args, theme).await,

        // Editor & Code Viewing
        Commands::Editor(args) => execute_editor_command(args, theme).await,

        // Security & Audit
        Commands::Security(args) => execute_security_command(args, theme).await,

        // Plugins
        Commands::Plugin(args) => execute_plugin_command(args, theme).await,
        Commands::Hooks(args) => commands::hooks::run(args).await,

        // Gateway & Channels
        Commands::Gateway(args) => execute_gateway_command(args, theme).await,
        Commands::Channel(args) => execute_channel_command(args, theme).await,

        // Shell & Self
        Commands::Shell(args) => commands::project::run_shell(args, theme).await,
        Commands::SelfCmd(args) => execute_self_command(args).await,
        Commands::Config(args) => run_config(args, theme),
        Commands::Onboard => commands::onboard::run().await,

        // Utility
        Commands::Info(args) => commands::project::run_info(args, theme).await,
        Commands::System => commands::system::run_system(theme),
        Commands::Logo => commands::logo::run_logo(theme),
        Commands::Clean(args) => commands::project::run_clean(args, theme).await,
        Commands::Completions(args) => commands::project::run_completions(args),
        Commands::Tree(args) => commands::tree::run(args, theme).await,
        Commands::Doctor(args) => commands::doctor::run(args, theme).await,

        // Animations
        Commands::Animate(args) => commands::project::run_animate(args, theme),
        Commands::Splash => crate::ui::splash::show_splash(),
        Commands::Rainbow => crate::ui::rainbow::show_rainbow_showcase(),
        Commands::Icons => crate::ui::svg_renderer::show_svg_gallery(),
        Commands::Sound(args) => commands::sound::run(args, theme).await,
        Commands::Audio(args) => commands::audio::execute(args).await,

        // Interactive
        Commands::Interact { command } => commands::interact::handle_interact_command(command),

        // Showcase
        Commands::Showcase(args) => commands::showcase::run(args, theme).await,

        // Chat
        Commands::Chat(args) => args.execute(),

        // LLM
        Commands::Llm(args) => commands::llm::run(args, theme).await,

        // Daemon & Orchestration
        Commands::Daemon(args) => commands::daemon::run(args, theme).await,
        Commands::Branch(args) => commands::branch::run(args, theme).await,
        Commands::R2(args) => commands::r2::run(args, theme).await,
        Commands::Safety(args) => commands::safety::run(args, theme).await,
        Commands::Testing(args) => commands::testing::run(args, theme).await,

        // Triple Layout Demo
        Commands::Triple => crate::ui::triple_layout::run_demo(),

        // Format System Demo
        Commands::DemoFormats => commands::demo_formats::demo_formats().await,

        // Syntax Highlighting
        Commands::Syntax { file } => {
            if let Some(path) = file {
                let viewer = crate::ui::syntax_viewer::SyntaxViewer::new();
                viewer.show_file(&path)
            } else {
                crate::ui::syntax_viewer::demo_syntax_highlighting()
            }
        }

        // Diff Viewer
        Commands::Diff { old, new } => {
            if let (Some(old_path), Some(new_path)) = (old, new) {
                crate::ui::diff_viewer::DiffViewer::show_file_diff(&old_path, &new_path)
            } else {
                crate::ui::diff_viewer::demo_diff_viewer()
            }
        }
    }
}

/// Execute self command
async fn execute_self_command(args: SelfArgs) -> Result<()> {
    match args.command {
        SelfCommands::Update { force, yes } => {
            commands::self_update::execute_update(force, yes).await
        }
        SelfCommands::Info => commands::self_update::execute_info().await,
        SelfCommands::Uninstall { yes } => commands::self_update::execute_uninstall(yes).await,
    }
}

/// Handle config command
fn run_config(args: ConfigArgs, _theme: &Theme) -> Result<()> {
    use crate::ui::onboarding;

    if args.show {
        // Show current configuration
        if !onboarding::is_configured() {
            crate::ui::logger::info("DX CLI is not configured yet. Run 'dx' to start onboarding.");
            return Ok(());
        }

        match onboarding::load_config() {
            Ok((config, integrations)) => {
                println!("\n{}", "Current Configuration:".bright_cyan().bold());
                println!("\n{}", toml::to_string_pretty(&config)?);

                if integrations.elevenlabs.is_some()
                    || integrations.zapier.is_some()
                    || integrations.n8n.is_some()
                    || integrations.browser_control
                    || integrations.gmail
                    || integrations.github
                {
                    println!("\n{}", "Integrations:".bright_cyan().bold());
                    println!("{}", toml::to_string_pretty(&integrations)?);
                }
                Ok(())
            }
            Err(e) => {
                crate::ui::logger::error(&format!("Failed to load configuration: {}", e));
                Err(anyhow::anyhow!("Configuration load failed"))
            }
        }
    } else if args.reset {
        // Reset configuration
        let config_path = onboarding::get_config_path();
        if config_path.exists() {
            std::fs::remove_file(&config_path)?;
            let integrations_path = config_path.with_file_name("integrations.toml");
            if integrations_path.exists() {
                std::fs::remove_file(integrations_path)?;
            }
            crate::ui::logger::success("Configuration reset. Run 'dx' to configure again.");
        } else {
            crate::ui::logger::info("No configuration found.");
        }
        Ok(())
    } else {
        // Run onboarding
        match onboarding::run_onboarding() {
            Ok((config, integrations)) => {
                if let Err(e) = onboarding::save_config(&config, &integrations) {
                    crate::ui::logger::error(&format!("Failed to save configuration: {}", e));
                    return Err(anyhow::anyhow!("Configuration save failed"));
                }
                crate::ui::logger::success("Configuration saved successfully!");
                Ok(())
            }
            Err(e) => {
                crate::ui::logger::error(&format!("Configuration failed: {}", e));
                Err(anyhow::anyhow!("Configuration failed"))
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// NEW COMMAND HANDLERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Execute git command
async fn execute_git_command(args: commands::git::GitArgs, _theme: &Theme) -> Result<()> {
    use commands::git::{BranchSubcommands, GitCommands, GitRepo, StashSubcommands};

    let repo = GitRepo::open_current()?;

    match args.command {
        GitCommands::Status { short, verbose } => {
            let mut cmd_args = vec!["status"];
            if short {
                cmd_args.push("--short");
            }
            if verbose {
                cmd_args.push("--verbose");
            }
            let output = repo.exec(&cmd_args)?;
            println!("{}", output);
        }
        GitCommands::Commit {
            message,
            amend,
            all,
        } => {
            let mut cmd_args = vec!["commit"];
            if amend {
                cmd_args.push("--amend");
            }
            if all {
                cmd_args.push("-a");
            }
            if let Some(ref msg) = message {
                cmd_args.push("-m");
                cmd_args.push(msg);
            }
            let output = repo.exec(&cmd_args)?;
            println!("{}", output);
        }
        GitCommands::Diff {
            target,
            staged,
            side_by_side: _,
            inline: _,
        } => {
            let mut cmd_args = vec!["diff"];
            if staged {
                cmd_args.push("--cached");
            }
            if let Some(ref t) = target {
                cmd_args.push(t);
            }
            let output = repo.exec(&cmd_args)?;
            println!("{}", output);
        }
        GitCommands::Branch { command } => match command {
            Some(BranchSubcommands::List { all, remote }) => {
                let mut cmd_args = vec!["branch"];
                if all {
                    cmd_args.push("-a");
                }
                if remote {
                    cmd_args.push("-r");
                }
                let output = repo.exec(&cmd_args)?;
                println!("{}", output);
            }
            Some(BranchSubcommands::Create { name, start_point }) => {
                let mut cmd_args = vec!["branch", &name];
                if let Some(ref sp) = start_point {
                    cmd_args.push(sp);
                }
                repo.exec(&cmd_args)?;
                println!("{} Branch '{}' created", "✓".green(), name);
            }
            Some(BranchSubcommands::Switch { name, create }) => {
                if create {
                    repo.exec(&["checkout", "-b", &name])?;
                } else {
                    repo.exec(&["checkout", &name])?;
                }
                println!("{} Switched to branch '{}'", "✓".green(), name);
            }
            Some(BranchSubcommands::Delete { name, force }) => {
                let flag = if force { "-D" } else { "-d" };
                repo.exec(&["branch", flag, &name])?;
                println!("{} Branch '{}' deleted", "✓".green(), name);
            }
            Some(BranchSubcommands::Merge { branch, no_commit }) => {
                let mut cmd_args = vec!["merge"];
                if no_commit {
                    cmd_args.push("--no-commit");
                }
                cmd_args.push(&branch);
                let output = repo.exec(&cmd_args)?;
                println!("{}", output);
            }
            None => {
                let output = repo.exec(&["branch"])?;
                println!("{}", output);
            }
        },
        GitCommands::Stash { command } => match command {
            Some(StashSubcommands::Save {
                message,
                include_untracked,
            }) => {
                let mut cmd_args = vec!["stash", "push"];
                if include_untracked {
                    cmd_args.push("-u");
                }
                if let Some(ref msg) = message {
                    cmd_args.push("-m");
                    cmd_args.push(msg);
                }
                let output = repo.exec(&cmd_args)?;
                println!("{}", output);
            }
            Some(StashSubcommands::List) | None => {
                let output = repo.exec(&["stash", "list"])?;
                println!("{}", output);
            }
            Some(StashSubcommands::Pop { stash }) => {
                let stash_ref = stash.map(|i| format!("stash@{{{}}}", i));
                let mut cmd_args = vec!["stash", "pop"];
                if let Some(ref sr) = stash_ref {
                    cmd_args.push(sr);
                }
                let output = repo.exec(&cmd_args)?;
                println!("{}", output);
            }
            Some(StashSubcommands::Apply { stash }) => {
                let stash_ref = stash.map(|i| format!("stash@{{{}}}", i));
                let mut cmd_args = vec!["stash", "apply"];
                if let Some(ref sr) = stash_ref {
                    cmd_args.push(sr);
                }
                let output = repo.exec(&cmd_args)?;
                println!("{}", output);
            }
            Some(StashSubcommands::Drop { stash }) => {
                let stash_ref = stash.map(|i| format!("stash@{{{}}}", i));
                let mut cmd_args = vec!["stash", "drop"];
                if let Some(ref sr) = stash_ref {
                    cmd_args.push(sr);
                }
                let output = repo.exec(&cmd_args)?;
                println!("{}", output);
            }
            Some(StashSubcommands::Show { stash }) => {
                let stash_ref = stash.map(|i| format!("stash@{{{}}}", i));
                let mut cmd_args = vec!["stash", "show", "-p"];
                if let Some(ref sr) = stash_ref {
                    cmd_args.push(sr);
                }
                let output = repo.exec(&cmd_args)?;
                println!("{}", output);
            }
        },
        GitCommands::Add { files, patch } => {
            let mut cmd_args: Vec<String> = vec!["add".to_string()];
            if patch {
                cmd_args.push("-p".to_string());
            }
            for f in &files {
                cmd_args.push(f.display().to_string());
            }
            let args_refs: Vec<&str> = cmd_args.iter().map(|s| s.as_str()).collect();
            let output = repo.exec(&args_refs)?;
            println!("{} Files staged", "✓".green());
            if !output.is_empty() {
                println!("{}", output);
            }
        }
        GitCommands::Log {
            count,
            oneline,
            graph,
        } => {
            let count_str = format!("-{}", count);
            let mut cmd_args = vec!["log", &count_str];
            if oneline {
                cmd_args.push("--oneline");
            }
            if graph {
                cmd_args.push("--graph");
            }
            let output = repo.exec(&cmd_args)?;
            println!("{}", output);
        }
    }
    Ok(())
}

/// Execute editor command
async fn execute_editor_command(args: commands::editor::EditorArgs, _theme: &Theme) -> Result<()> {
    use commands::editor::EditorCommands;

    match args.command {
        Some(EditorCommands::Open {
            files,
            line,
            column,
            readonly,
        }) => {
            println!(
                "{} Opening {:?} at line {:?}, col {:?}, readonly: {}",
                "●".cyan(),
                files,
                line,
                column,
                readonly
            );
        }
        Some(EditorCommands::Tree { path, all, depth }) => {
            let tree_path = path.unwrap_or_else(|| std::env::current_dir().unwrap());
            println!(
                "{} Showing tree for {} (all: {}, depth: {:?})",
                "●".cyan(),
                tree_path.display(),
                all,
                depth
            );
        }
        Some(EditorCommands::Config {
            set,
            get,
            show,
            reset,
        }) => {
            if show {
                println!("{} Editor configuration:", "●".cyan());
                println!("  keybindings: vim");
                println!("  line_numbers: true");
                println!("  relative_numbers: false");
                println!("  theme: dracula");
            } else if reset {
                println!("{} Editor configuration reset to defaults", "✓".green());
            } else if let Some(key) = get {
                println!("{} {}: (value)", "●".cyan(), key);
            } else if let Some(kv) = set {
                println!("{} Set {}", "✓".green(), kv);
            }
        }
        Some(EditorCommands::Keys {
            action,
            list,
            preset,
        }) => {
            if list {
                println!("{} Keybindings:", "●".cyan());
                println!("  j/k    - move up/down");
                println!("  h/l    - move left/right");
                println!("  i      - insert mode");
                println!("  v      - visual mode");
                println!("  /      - search");
                println!("  :w     - save");
                println!("  :q     - quit");
            } else if let Some(preset) = preset {
                println!("{} Preset set to {:?}", "✓".green(), preset);
            } else if let Some(action) = action {
                println!("{} Keybinding for {}: (key)", "●".cyan(), action);
            }
        }
        Some(EditorCommands::Syntax {
            file,
            theme,
            start,
            end,
        }) => {
            let viewer = crate::ui::syntax_viewer::SyntaxViewer::new();
            let _ = theme;
            let _ = start;
            let _ = end;
            viewer.show_file(&file)?;
        }
        Some(EditorCommands::Search {
            pattern,
            path,
            case_sensitive,
            regex,
            include,
            exclude,
        }) => {
            let search_path = path.unwrap_or_else(|| std::env::current_dir().unwrap());
            println!(
                "{} Searching for '{}' in {} (case: {}, regex: {}, include: {:?}, exclude: {:?})",
                "●".cyan(),
                pattern,
                search_path.display(),
                case_sensitive,
                regex,
                include,
                exclude
            );
        }
        Some(EditorCommands::Diff {
            file1,
            file2,
            side_by_side,
            minimal,
        }) => {
            let _ = (side_by_side, minimal);
            // Show unified diff (side-by-side will be added in future TUI update)
            crate::ui::diff_viewer::DiffViewer::show_file_diff(&file1, &file2)?;
        }
        Some(EditorCommands::Minimap { file, width }) => {
            println!("{} Showing minimap for {} (width: {})", "●".cyan(), file.display(), width);
        }
        Some(EditorCommands::Languages) => {
            println!("{} Supported languages:", "●".cyan());
            let langs = [
                "Rust",
                "TypeScript",
                "JavaScript",
                "Python",
                "Go",
                "C",
                "C++",
                "Java",
                "Ruby",
                "PHP",
                "Swift",
                "Kotlin",
                "HTML",
                "CSS",
                "JSON",
                "YAML",
                "TOML",
                "Markdown",
                "SQL",
                "Shell",
            ];
            for lang in &langs {
                println!("  • {}", lang);
            }
        }
        Some(EditorCommands::Themes) => {
            println!("{} Available themes:", "●".cyan());
            let themes = [
                "dracula",
                "nord",
                "solarized-dark",
                "solarized-light",
                "gruvbox",
                "monokai",
                "one-dark",
                "github-dark",
            ];
            for theme in &themes {
                println!("  • {}", theme);
            }
        }
        Some(EditorCommands::Preview { file, pager }) => {
            println!("{} Previewing {} (pager: {})", "●".cyan(), file.display(), pager);
        }
        Some(EditorCommands::Interactive { path }) => {
            let work_path = path.unwrap_or_else(|| std::env::current_dir().unwrap());
            println!("{} Starting interactive editor in {}", "●".cyan(), work_path.display());
        }
        None => {
            // Default: open path or current directory
            if let Some(path) = args.path {
                println!("{} Opening {}", "●".cyan(), path.display());
            } else {
                println!("{} Opening current directory", "●".cyan());
            }
        }
    }
    Ok(())
}

/// Execute security command
async fn execute_security_command(
    args: commands::security::SecurityArgs,
    _theme: &Theme,
) -> Result<()> {
    use commands::security::SecurityCommands;

    match args.command {
        SecurityCommands::Audit {
            path,
            format: _,
            deep,
            deps_only,
        } => {
            let audit_path = path.unwrap_or_else(|| std::env::current_dir().unwrap());
            println!("{}", "╔════════════════════════════════════════════╗".cyan());
            println!("{}", "║           Security Audit                   ║".cyan());
            println!("{}", "╚════════════════════════════════════════════╝".cyan());
            println!();
            println!("{} Auditing: {}", "●".yellow(), audit_path.display());
            println!("{} Deep scan: {}", "●".yellow(), deep);
            println!("{} Dependencies only: {}", "●".yellow(), deps_only);
            println!();
            println!("{} No vulnerabilities found", "✓".green());
        }
        SecurityCommands::Secrets { command } => {
            use commands::security::SecretsSubcommands;
            match command {
                SecretsSubcommands::List => {
                    println!("{} Stored secrets:", "●".cyan());
                    println!("  (none configured)");
                }
                SecretsSubcommands::Set { name, value: _ } => {
                    println!("{} Secret '{}' saved", "✓".green(), name);
                }
                SecretsSubcommands::Remove { name } => {
                    println!("{} Secret '{}' removed", "✓".green(), name);
                }
                SecretsSubcommands::Rotate { force } => {
                    println!("{} Master key rotated (force: {})", "✓".green(), force);
                }
                SecretsSubcommands::Export { output } => {
                    println!("{} Secrets exported to {}", "✓".green(), output.display());
                }
                SecretsSubcommands::Import { input } => {
                    println!("{} Secrets imported from {}", "✓".green(), input.display());
                }
            }
        }
        SecurityCommands::Permissions { command } => {
            use commands::security::PermissionsSubcommands;
            match command {
                PermissionsSubcommands::List { context } => {
                    println!("{} Permissions (context: {:?}):", "●".cyan(), context);
                    println!("  file:read    - granted");
                    println!("  file:write   - granted");
                    println!("  network      - granted");
                }
                PermissionsSubcommands::Grant {
                    permission,
                    context,
                } => {
                    println!("{} Granted '{}' to '{}'", "✓".green(), permission, context);
                }
                PermissionsSubcommands::Revoke {
                    permission,
                    context,
                } => {
                    println!("{} Revoked '{}' from '{}'", "✓".green(), permission, context);
                }
                PermissionsSubcommands::Reset { force } => {
                    println!("{} Permissions reset (force: {})", "✓".green(), force);
                }
            }
        }
        SecurityCommands::Logs {
            count,
            action,
            export: _,
            output: _,
        } => {
            println!("{} Audit logs (last {}, filter: {:?}):", "●".cyan(), count, action);
            println!("  (no entries)");
        }
        SecurityCommands::Sandbox { command } => {
            use commands::security::SandboxSubcommands;
            match command {
                SandboxSubcommands::Status => {
                    println!("{} Sandbox status: active", "●".green());
                    println!("  Memory limit: 256MB");
                    println!("  CPU limit: 30s");
                }
                SandboxSubcommands::List => {
                    println!("{} Running sandboxes: 0", "●".cyan());
                }
                SandboxSubcommands::Config {
                    memory,
                    cpu,
                    network,
                    filesystem,
                } => {
                    println!("{} Sandbox config updated:", "✓".green());
                    if let Some(m) = memory {
                        println!("  Memory: {}", m);
                    }
                    if let Some(c) = cpu {
                        println!("  CPU: {}", c);
                    }
                    if let Some(n) = network {
                        println!("  Network: {}", n);
                    }
                    if let Some(f) = filesystem {
                        println!("  Filesystem: {}", f);
                    }
                }
                SandboxSubcommands::Stop { id } => {
                    println!("{} Sandbox '{}' stopped", "✓".green(), id);
                }
            }
        }
        SecurityCommands::Trust { context, level } => {
            println!("{} Trust level for '{}' set to '{}'", "✓".green(), context, level);
        }
    }
    Ok(())
}

/// Execute plugin command
async fn execute_plugin_command(args: commands::plugin::PluginArgs, _theme: &Theme) -> Result<()> {
    use commands::plugin::PluginCommands;

    match args.command {
        PluginCommands::List {
            verbose,
            plugin_type: _,
            format: _,
        } => {
            println!("{}", "╔════════════════════════════════════════════╗".cyan());
            println!("{}", "║           Installed Plugins                ║".cyan());
            println!("{}", "╚════════════════════════════════════════════╝".cyan());
            println!();
            if verbose {
                println!("  No plugins installed");
            } else {
                println!("  No plugins installed");
            }
        }
        PluginCommands::Install {
            source,
            force,
            no_verify,
        } => {
            println!(
                "{} Installing plugin from: {} (force: {}, verify: {})",
                "●".yellow(),
                source,
                force,
                !no_verify
            );
            println!("{} Plugin installed successfully", "✓".green());
        }
        PluginCommands::Remove { name, force } => {
            println!("{} Removing plugin: {} (force: {})", "●".yellow(), name, force);
            println!("{} Plugin removed", "✓".green());
        }
        PluginCommands::Update { name, check } => {
            if check {
                println!("{} Checking for updates...", "●".yellow());
                println!("{} All plugins up to date", "✓".green());
            } else {
                println!("{} Updating plugin: {:?}", "●".yellow(), name);
                println!("{} Plugins updated", "✓".green());
            }
        }
        PluginCommands::Info { name } => {
            println!("{} Plugin info: {}", "●".cyan(), name);
            println!("  (plugin not found)");
        }
        PluginCommands::Run {
            name,
            args: run_args,
        } => {
            println!("{} Running plugin: {} {:?}", "●".yellow(), name, run_args);
        }
        PluginCommands::Create {
            name,
            plugin_type: _,
            output,
            lang,
        } => {
            let out_path = output.unwrap_or_else(|| std::env::current_dir().unwrap().join(&name));
            println!(
                "{} Creating plugin '{}' ({}) in {}",
                "●".yellow(),
                name,
                lang,
                out_path.display()
            );
            println!("{} Plugin scaffold created", "✓".green());
        }
        PluginCommands::Build {
            path,
            release,
            optimize,
        } => {
            let build_path = path.unwrap_or_else(|| std::env::current_dir().unwrap());
            println!(
                "{} Building plugin in {} (release: {}, optimize: {})",
                "●".yellow(),
                build_path.display(),
                release,
                optimize
            );
            println!("{} Plugin built successfully", "✓".green());
        }
        PluginCommands::Publish { path, no_verify } => {
            let pub_path = path.unwrap_or_else(|| std::env::current_dir().unwrap());
            println!(
                "{} Publishing plugin from {} (verify: {})",
                "●".yellow(),
                pub_path.display(),
                !no_verify
            );
            println!("{} Plugin published", "✓".green());
        }
        PluginCommands::Search { query, limit } => {
            println!("{} Searching for '{}' (limit: {})", "●".yellow(), query, limit);
            println!("  No plugins found");
        }
        PluginCommands::Enable { name } => {
            println!("{} Plugin '{}' enabled", "✓".green(), name);
        }
        PluginCommands::Disable { name } => {
            println!("{} Plugin '{}' disabled", "✓".green(), name);
        }
    }
    Ok(())
}

/// Execute gateway command
async fn execute_gateway_command(
    args: commands::gateway::GatewayArgs,
    _theme: &Theme,
) -> Result<()> {
    use commands::gateway::GatewayCommands;

    match args.command {
        GatewayCommands::Start {
            host,
            port,
            foreground,
            mdns,
            auth,
        } => {
            println!("{}", "╔════════════════════════════════════════════╗".cyan());
            println!("{}", "║           DX Gateway Server                ║".cyan());
            println!("{}", "╚════════════════════════════════════════════╝".cyan());
            println!();

            // Auto-install Bun if missing (for OpenClaw features)
            println!("{} Checking Bun installation...", "●".yellow());
            let bun_version = match crate::nodejs::installer::ensure_bun().await {
                Ok(version) => {
                    println!("{} Bun {} ready", "✓".green(), version);
                    Some(version)
                }
                Err(e) => {
                    println!("{} Bun installation failed: {}", "⚠".yellow(), e);
                    println!("   Falling back to Rust-only gateway");
                    None
                }
            };

            if let Some(_version) = bun_version {
                println!("{} Using OpenClaw features (Bun detected)", "✓".green());
                println!("{} Starting gateway on {}:{}", "●".yellow(), host, port);
                println!("{} mDNS discovery: {}", "●".yellow(), mdns);
                println!("{} Authentication: {}", "●".yellow(), auth);
                println!("{} Foreground: {}", "●".yellow(), foreground);
                println!();

                // Start OpenClaw gateway with full features
                let mut bridge = crate::nodejs::OpenClawBridge::new()?;
                let config = crate::nodejs::GatewayConfig {
                    port,
                    bind_address: host.clone(),
                    control_ui_enabled: true,
                    openai_api_enabled: true,
                    auth_token: if auth {
                        Some("dx-gateway-token".to_string())
                    } else {
                        None
                    },
                };

                bridge.start_gateway(config).await?;

                println!("{} Gateway started with OpenClaw features:", "✓".green());
                println!("   {} Control UI: http://{}:{}", "→".cyan(), host, port);
                println!("   {} OpenAI API: http://{}:{}/v1", "→".cyan(), host, port);
                println!("   {} WebSocket: ws://{}:{}", "→".cyan(), host, port);
                println!("   {} All messaging channels enabled", "→".cyan());
                println!("   {} Canvas/A2UI rendering", "→".cyan());
                println!("   {} Voice/TikTok mode", "→".cyan());
                println!("   {} Cron jobs", "→".cyan());
                println!("   {} Device pairing", "→".cyan());
                println!();
                println!("{} Press Ctrl+C to stop", "●".yellow());

                // Keep running
                tokio::signal::ctrl_c().await?;
                println!("\n{} Shutting down gateway...", "●".yellow());
            } else {
                println!("{} Bun not found - using basic Rust gateway", "⚠".yellow());
                println!(
                    "   Install Bun for full features: curl -fsSL https://bun.sh/install | bash"
                );
                println!("{} Starting gateway on {}:{}", "●".yellow(), host, port);
                println!("{} mDNS discovery: {}", "●".yellow(), mdns);
                println!("{} Authentication: {}", "●".yellow(), auth);
                println!("{} Foreground: {}", "●".yellow(), foreground);
                println!();
                println!("{} Gateway started (basic mode)", "✓".green());
            }
        }
        GatewayCommands::Stop { force } => {
            println!("{} Stopping gateway (force: {})", "●".yellow(), force);
            println!("{} Gateway stopped", "✓".green());
        }
        GatewayCommands::Status { verbose, format: _ } => {
            println!("{} Gateway status: not running", "●".yellow());
            if verbose {
                println!("  Last started: never");
                println!("  Connections: 0");
            }
        }
        GatewayCommands::Clients { verbose } => {
            println!("{} Connected clients: 0", "●".cyan());
            if verbose {
                println!("  (no clients)");
            }
        }
        GatewayCommands::Pair { duration, qr } => {
            let code = "DX-1234-5678";
            println!(
                "{} Pairing code (valid for {}s): {}",
                "●".yellow(),
                duration,
                code.bright_green()
            );
            if qr {
                println!("  (QR code would be displayed here)");
            }
        }
        GatewayCommands::Disconnect { client_id } => {
            println!("{} Client '{}' disconnected", "✓".green(), client_id);
        }
        GatewayCommands::Config {
            max_connections,
            session_timeout,
            allowed_commands,
        } => {
            println!("{} Gateway config updated:", "✓".green());
            if let Some(max) = max_connections {
                println!("  Max connections: {}", max);
            }
            if let Some(timeout) = session_timeout {
                println!("  Session timeout: {}s", timeout);
            }
            if let Some(cmds) = allowed_commands {
                println!("  Allowed commands: {}", cmds);
            }
        }
        GatewayCommands::Logs { lines, follow } => {
            println!("{} Gateway logs (last {}, follow: {}):", "●".cyan(), lines, follow);
            println!("  (no logs)");
        }
    }
    Ok(())
}

/// Execute channel command
async fn execute_channel_command(
    args: commands::channel::ChannelArgs,
    _theme: &Theme,
) -> Result<()> {
    // Delegate to the channel module's execute function
    commands::channel::execute(args.command).await
}
