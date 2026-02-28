//! # DX CLI - AGI-like AI Agent
//!
//! The main entry point for DX - an AGI-like AI Agent that can:
//! - Connect to ANY app (WhatsApp, Telegram, Discord, GitHub, Notion, Spotify, etc.)
//! - Create its own integrations dynamically via WASM compilation
//! - Auto-update itself by detecting local changes and creating PRs
//! - Run 24/7 as a daemon with minimal CPU usage
//! - Save 70%+ tokens using DX Serializer LLM format
//!
//! ## Quick Start
//!
//! ```bash
//! # Start the agent daemon
//! dx agent start
//!
//! # Connect to an integration
//! dx connect github
//! dx connect telegram
//! dx connect notion
//!
//! # Create a new integration dynamically
//! dx create integration my-api --language python
//!
//! # List available skills
//! dx skills list
//!
//! # Execute a skill
//! dx run "send a message to john on whatsapp saying hello"
//! ```

use clap::{Parser, Subcommand};
use colored::Colorize;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

mod commands;
mod prompts;

/// DX CLI - AGI-like AI Agent
#[derive(Parser)]
#[command(name = "dx")]
#[command(author = "DX Team")]
#[command(version = "0.1.0")]
#[command(about = "🤖 DX - AGI-like AI Agent that connects to any app", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Use JSON output
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start, stop, or manage the agent daemon
    Agent {
        #[command(subcommand)]
        action: AgentCommands,
    },

    /// Connect to an integration (github, telegram, notion, etc.)
    Connect {
        /// Integration name
        integration: String,

        /// API token (optional, will prompt if not provided)
        #[arg(short, long)]
        token: Option<String>,
    },

    /// Disconnect from an integration
    Disconnect {
        /// Integration name
        integration: String,
    },

    /// Create new integrations, skills, or plugins
    Create {
        #[command(subcommand)]
        what: CreateCommands,
    },

    /// List integrations, skills, or tasks
    List {
        #[command(subcommand)]
        what: ListCommands,
    },

    /// Manage skills
    Skills {
        #[command(subcommand)]
        action: SkillsCommands,
    },

    /// Run a natural language command
    Run {
        /// The command to run (natural language)
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },

    /// Schedule tasks
    Schedule {
        #[command(subcommand)]
        action: ScheduleCommands,
    },

    /// Serializer commands (convert to/from DX format)
    Serializer {
        #[command(subcommand)]
        action: SerializerCommands,
    },

    /// Chat with Google AI Studio models (Gemini, Gemma)
    Chat(commands::chat::ChatCommand),

    /// Show status of the agent and integrations
    Status,

    /// Initialize DX in the current directory
    Init,
}

#[derive(Subcommand)]
pub enum AgentCommands {
    /// Start the agent daemon
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,
    },
    /// Stop the agent daemon
    Stop,
    /// Restart the agent daemon
    Restart,
    /// Show agent status
    Status,
    /// View agent logs
    Logs {
        /// Number of lines to show
        #[arg(short, long, default_value = "50")]
        lines: usize,

        /// Follow the log output
        #[arg(short, long)]
        follow: bool,
    },
}

#[derive(Subcommand)]
pub enum CreateCommands {
    /// Create a new integration
    Integration {
        /// Integration name
        name: String,

        /// Programming language (python, javascript, go, rust)
        #[arg(short, long, default_value = "python")]
        language: String,

        /// Source file (optional, will use template if not provided)
        #[arg(short, long)]
        source: Option<String>,
    },
    /// Create a new skill
    Skill {
        /// Skill name
        name: String,

        /// Skill description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Create a new plugin
    Plugin {
        /// Plugin name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum ListCommands {
    /// List available integrations
    Integrations,
    /// List available skills
    Skills,
    /// List scheduled tasks
    Tasks,
    /// List loaded plugins
    Plugins,
}

#[derive(Subcommand)]
pub enum SkillsCommands {
    /// List all skills
    List,
    /// Show skill details
    Show {
        /// Skill name
        name: String,
    },
    /// Add a new skill
    Add {
        /// Path to skill definition (.sr file)
        path: String,
    },
    /// Remove a skill
    Remove {
        /// Skill name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum ScheduleCommands {
    /// Add a scheduled task
    Add {
        /// Task name
        name: String,

        /// Cron expression
        #[arg(short, long)]
        cron: String,

        /// Skill to execute
        #[arg(short, long)]
        skill: String,

        /// Skill context
        #[arg(long)]
        context: Option<String>,
    },
    /// Remove a scheduled task
    Remove {
        /// Task name
        name: String,
    },
    /// List scheduled tasks
    List,
}

#[derive(Subcommand)]
pub enum SerializerCommands {
    /// Convert JSON to DX format
    FromJson {
        /// Input file or stdin
        input: Option<String>,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Convert DX format to JSON
    ToJson {
        /// Input file or stdin
        input: Option<String>,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Process a file (generate .llm and .machine formats)
    Process {
        /// Input file or directory
        path: String,

        /// Recursive processing
        #[arg(short, long)]
        recursive: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Set up logging
    let level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .compact()
        .init();

    // Print banner
    if !cli.json {
        print_banner();
    }

    match cli.command {
        Some(Commands::Agent { action }) => commands::agent::run(action).await?,
        Some(Commands::Connect { integration, token }) => {
            commands::connect::run(&integration, token.as_deref()).await?
        }
        Some(Commands::Disconnect { integration }) => {
            commands::disconnect::run(&integration).await?
        }
        Some(Commands::Create { what }) => commands::create::run(what).await?,
        Some(Commands::List { what }) => commands::list::run(what).await?,
        Some(Commands::Skills { action }) => commands::skills::run(action).await?,
        Some(Commands::Run { command }) => {
            let cmd = command.join(" ");
            commands::run::run(&cmd).await?
        }
        Some(Commands::Schedule { action }) => commands::schedule::run(action).await?,
        Some(Commands::Serializer { action }) => commands::serializer::run(action).await?,
        Some(Commands::Chat(cmd)) => cmd.execute().await?,
        Some(Commands::Status) => commands::status::run().await?,
        Some(Commands::Init) => commands::init::run().await?,
        None => run_onboarding().await?,
    }

    Ok(())
}

async fn run_onboarding() -> anyhow::Result<()> {
    use prompts::{
        autocomplete, box_section, calendar, code_snippet, confirm, date_picker, email,
        emoji_picker, file_browser, intro, json_editor, kanban, list_editor, log, markdown_editor,
        matrix_select, multiselect, number, outro, phone_input, range_slider, rating,
        search_filter, select, slider, table_editor, tags, text, time_picker, toggle, tree_select,
        url, CodeSnippet, KanbanTask, PromptInteraction, TreeNode, Validate,
    };

    intro("Welcome to DX - Your AGI-like AI Agent")?;

    box_section(
        "Interactive Prompt Showcase",
        &[
            "Experience our beautiful CLI prompt system!",
            "We'll demonstrate 15+ different input types with our design system.",
        ],
    )?;

    // Text Input Demo
    let mut name_prompt = text("What's your name?")
        .placeholder("Enter your name")
        .validate(|input: &str| {
            if input.trim().is_empty() {
                Validate::Invalid("Name cannot be empty".to_string())
            } else if input.len() < 2 {
                Validate::Invalid("Name must be at least 2 characters".to_string())
            } else {
                Validate::Valid
            }
        });
    let name = name_prompt.interact()?;

    log::success(format!("Welcome, {}! Let's continue with the setup.", name))?;

    // Rating Demo
    let mut satisfaction_prompt = rating("How satisfied are you with CLI tools?").max(5);
    let _satisfaction = satisfaction_prompt.interact()?;

    // Toggle Demo
    let mut notifications_prompt = toggle("Enable desktop notifications?")
        .labels("Enabled", "Disabled")
        .initial_value(true);
    let _notifications = notifications_prompt.interact()?;

    // Slider Demo
    let mut confidence_prompt = slider("Set AI confidence threshold (0-100)", 0, 100)
        .step(5)
        .initial_value(75);
    let _confidence = confidence_prompt.interact()?;

    log::info("Great choices! Now let's configure your team settings.")?;

    // Number Input Demo
    let mut team_size_prompt = number("How many team members will use DX?")
        .min(1)
        .max(1000);
    let team_size = team_size_prompt.interact()?;

    if team_size > 10 {
        log::info("Great! DX scales perfectly for large teams.")?;
    }

    // Tags Demo
    let mut skills_prompt =
        tags("Enter your team's skills").placeholder("Type a skill and press Enter or comma");
    let _skills = skills_prompt.interact()?;

    // List Editor Demo
    let mut goals_prompt = list_editor("Manage your project goals")
        .initial_items(vec!["Launch MVP".to_string(), "Get 100 users".to_string()]);
    let _goals = goals_prompt.interact()?;

    // Autocomplete Demo
    let mut framework_prompt = autocomplete("Select your primary development framework:")
        .item_with_description("react", "React", "A JavaScript library for building UIs")
        .item_with_description("vue", "Vue.js", "The Progressive JavaScript Framework")
        .item_with_description(
            "angular",
            "Angular",
            "Platform for building mobile and desktop apps",
        )
        .item_with_description("svelte", "Svelte", "Cybernetically enhanced web apps")
        .item_with_description("nextjs", "Next.js", "The React Framework for Production")
        .item_with_description("nuxt", "Nuxt", "The Intuitive Vue Framework")
        .item_with_description("astro", "Astro", "Build faster websites")
        .item_with_description("remix", "Remix", "Full stack web framework")
        .item_with_description("solid", "SolidJS", "Simple and performant reactivity")
        .item_with_description("qwik", "Qwik", "Resumable framework");
    let _framework = framework_prompt.interact()?;

    box_section(
        "Advanced Input Demos",
        &["Let's try some advanced input types!"],
    )?;

    // Email Input Demo
    let mut email_prompt = email("What's your email address?");
    let _user_email = email_prompt.interact()?;

    // URL Input Demo
    let mut url_prompt = url("Enter your project repository URL:").require_https(false);
    let _repo_url = url_prompt.interact()?;

    // Range Slider Demo
    let mut price_range_prompt =
        range_slider("Select your budget range (USD):", 0, 10000).initial_range(1000, 5000);
    let _price_range = price_range_prompt.interact()?;

    // Tree Select Demo
    let mut project_structure_prompt = tree_select("Select a project component:")
        .node(
            TreeNode::new("frontend", "Frontend")
                .child(TreeNode::new("react", "React Components"))
                .child(TreeNode::new("styles", "Styles & Themes"))
                .child(TreeNode::new("assets", "Assets & Media")),
        )
        .node(
            TreeNode::new("backend", "Backend")
                .child(TreeNode::new("api", "API Routes"))
                .child(TreeNode::new("db", "Database Models"))
                .child(TreeNode::new("auth", "Authentication")),
        )
        .node(
            TreeNode::new("infra", "Infrastructure")
                .child(TreeNode::new("docker", "Docker Config"))
                .child(TreeNode::new("ci", "CI/CD Pipelines")),
        );
    let _selected_component = project_structure_prompt.interact()?;

    // File Browser Demo
    let mut config_file_prompt =
        file_browser("Select a configuration file:").allow_directories(false);
    let _config_file = config_file_prompt.interact()?;

    log::success("Advanced inputs completed! Now let's try some more specialized prompts.")?;

    box_section("Specialized Prompts", &["Date/Time and Tables!"])?;

    // Date Picker Demo
    let mut date_prompt = date_picker("Select a project deadline:");
    let _deadline = date_prompt.interact()?;

    // Time Picker Demo
    let mut time_prompt = time_picker("Select meeting time:").format_24h(false);
    let _meeting_time = time_prompt.interact()?;

    // Table Editor Demo
    let mut table_prompt = table_editor("Edit team members:")
        .headers(vec![
            "Name".to_string(),
            "Role".to_string(),
            "Email".to_string(),
        ])
        .add_row(vec![
            "Alice".to_string(),
            "Developer".to_string(),
            "alice@example.com".to_string(),
        ])
        .add_row(vec![
            "Bob".to_string(),
            "Designer".to_string(),
            "bob@example.com".to_string(),
        ]);
    let _team_data = table_prompt.interact()?;

    log::success("All specialized prompts completed! Let's try data entry prompts.")?;

    box_section("Data Entry & Search", &["Phone, JSON, Matrix, and Search!"])?;

    // Phone Input Demo with country selection
    let mut country_prompt = select("Select your country:")
        .item("+93", "Afghanistan", "Asia")
        .item("+355", "Albania", "Europe")
        .item("+213", "Algeria", "Africa")
        .item("+376", "Andorra", "Europe")
        .item("+244", "Angola", "Africa")
        .item("+54", "Argentina", "South America")
        .item("+374", "Armenia", "Asia")
        .item("+61", "Australia", "Oceania")
        .item("+43", "Austria", "Europe")
        .item("+994", "Azerbaijan", "Asia")
        .item("+973", "Bahrain", "Asia")
        .item("+880", "Bangladesh", "Asia")
        .item("+375", "Belarus", "Europe")
        .item("+32", "Belgium", "Europe")
        .item("+501", "Belize", "North America")
        .item("+229", "Benin", "Africa")
        .item("+975", "Bhutan", "Asia")
        .item("+591", "Bolivia", "South America")
        .item("+387", "Bosnia and Herzegovina", "Europe")
        .item("+267", "Botswana", "Africa")
        .item("+55", "Brazil", "South America")
        .item("+673", "Brunei", "Asia")
        .item("+359", "Bulgaria", "Europe")
        .item("+226", "Burkina Faso", "Africa")
        .item("+257", "Burundi", "Africa")
        .item("+855", "Cambodia", "Asia")
        .item("+237", "Cameroon", "Africa")
        .item("+1", "Canada", "North America")
        .item("+238", "Cape Verde", "Africa")
        .item("+236", "Central African Republic", "Africa")
        .item("+235", "Chad", "Africa")
        .item("+56", "Chile", "South America")
        .item("+86", "China", "Asia")
        .item("+57", "Colombia", "South America")
        .item("+269", "Comoros", "Africa")
        .item("+242", "Congo", "Africa")
        .item("+506", "Costa Rica", "North America")
        .item("+385", "Croatia", "Europe")
        .item("+53", "Cuba", "North America")
        .item("+357", "Cyprus", "Europe")
        .item("+420", "Czech Republic", "Europe")
        .item("+45", "Denmark", "Europe")
        .item("+253", "Djibouti", "Africa")
        .item("+593", "Ecuador", "South America")
        .item("+20", "Egypt", "Africa")
        .item("+503", "El Salvador", "North America")
        .item("+372", "Estonia", "Europe")
        .item("+251", "Ethiopia", "Africa")
        .item("+679", "Fiji", "Oceania")
        .item("+358", "Finland", "Europe")
        .item("+33", "France", "Europe")
        .item("+241", "Gabon", "Africa")
        .item("+220", "Gambia", "Africa")
        .item("+995", "Georgia", "Asia")
        .item("+49", "Germany", "Europe")
        .item("+233", "Ghana", "Africa")
        .item("+30", "Greece", "Europe")
        .item("+502", "Guatemala", "North America")
        .item("+224", "Guinea", "Africa")
        .item("+245", "Guinea-Bissau", "Africa")
        .item("+592", "Guyana", "South America")
        .item("+509", "Haiti", "North America")
        .item("+504", "Honduras", "North America")
        .item("+852", "Hong Kong", "Asia")
        .item("+36", "Hungary", "Europe")
        .item("+354", "Iceland", "Europe")
        .item("+91", "India", "Asia")
        .item("+62", "Indonesia", "Asia")
        .item("+98", "Iran", "Asia")
        .item("+964", "Iraq", "Asia")
        .item("+353", "Ireland", "Europe")
        .item("+972", "Israel", "Asia")
        .item("+39", "Italy", "Europe")
        .item("+225", "Ivory Coast", "Africa")
        .item("+81", "Japan", "Asia")
        .item("+962", "Jordan", "Asia")
        .item("+7", "Kazakhstan", "Asia")
        .item("+254", "Kenya", "Africa")
        .item("+965", "Kuwait", "Asia")
        .item("+996", "Kyrgyzstan", "Asia")
        .item("+856", "Laos", "Asia")
        .item("+371", "Latvia", "Europe")
        .item("+961", "Lebanon", "Asia")
        .item("+266", "Lesotho", "Africa")
        .item("+231", "Liberia", "Africa")
        .item("+218", "Libya", "Africa")
        .item("+423", "Liechtenstein", "Europe")
        .item("+370", "Lithuania", "Europe")
        .item("+352", "Luxembourg", "Europe")
        .item("+853", "Macau", "Asia")
        .item("+389", "Macedonia", "Europe")
        .item("+261", "Madagascar", "Africa")
        .item("+265", "Malawi", "Africa")
        .item("+60", "Malaysia", "Asia")
        .item("+960", "Maldives", "Asia")
        .item("+223", "Mali", "Africa")
        .item("+356", "Malta", "Europe")
        .item("+222", "Mauritania", "Africa")
        .item("+230", "Mauritius", "Africa")
        .item("+52", "Mexico", "North America")
        .item("+373", "Moldova", "Europe")
        .item("+377", "Monaco", "Europe")
        .item("+976", "Mongolia", "Asia")
        .item("+382", "Montenegro", "Europe")
        .item("+212", "Morocco", "Africa")
        .item("+258", "Mozambique", "Africa")
        .item("+95", "Myanmar", "Asia")
        .item("+264", "Namibia", "Africa")
        .item("+977", "Nepal", "Asia")
        .item("+31", "Netherlands", "Europe")
        .item("+64", "New Zealand", "Oceania")
        .item("+505", "Nicaragua", "North America")
        .item("+227", "Niger", "Africa")
        .item("+234", "Nigeria", "Africa")
        .item("+850", "North Korea", "Asia")
        .item("+47", "Norway", "Europe")
        .item("+968", "Oman", "Asia")
        .item("+92", "Pakistan", "Asia")
        .item("+970", "Palestine", "Asia")
        .item("+507", "Panama", "North America")
        .item("+675", "Papua New Guinea", "Oceania")
        .item("+595", "Paraguay", "South America")
        .item("+51", "Peru", "South America")
        .item("+63", "Philippines", "Asia")
        .item("+48", "Poland", "Europe")
        .item("+351", "Portugal", "Europe")
        .item("+974", "Qatar", "Asia")
        .item("+40", "Romania", "Europe")
        .item("+7", "Russia", "Europe")
        .item("+250", "Rwanda", "Africa")
        .item("+966", "Saudi Arabia", "Asia")
        .item("+221", "Senegal", "Africa")
        .item("+381", "Serbia", "Europe")
        .item("+248", "Seychelles", "Africa")
        .item("+232", "Sierra Leone", "Africa")
        .item("+65", "Singapore", "Asia")
        .item("+421", "Slovakia", "Europe")
        .item("+386", "Slovenia", "Europe")
        .item("+252", "Somalia", "Africa")
        .item("+27", "South Africa", "Africa")
        .item("+82", "South Korea", "Asia")
        .item("+211", "South Sudan", "Africa")
        .item("+34", "Spain", "Europe")
        .item("+94", "Sri Lanka", "Asia")
        .item("+249", "Sudan", "Africa")
        .item("+597", "Suriname", "South America")
        .item("+268", "Swaziland", "Africa")
        .item("+46", "Sweden", "Europe")
        .item("+41", "Switzerland", "Europe")
        .item("+963", "Syria", "Asia")
        .item("+886", "Taiwan", "Asia")
        .item("+992", "Tajikistan", "Asia")
        .item("+255", "Tanzania", "Africa")
        .item("+66", "Thailand", "Asia")
        .item("+228", "Togo", "Africa")
        .item("+676", "Tonga", "Oceania")
        .item("+216", "Tunisia", "Africa")
        .item("+90", "Turkey", "Asia")
        .item("+993", "Turkmenistan", "Asia")
        .item("+256", "Uganda", "Africa")
        .item("+380", "Ukraine", "Europe")
        .item("+971", "United Arab Emirates", "Asia")
        .item("+44", "United Kingdom", "Europe")
        .item("+1", "United States", "North America")
        .item("+598", "Uruguay", "South America")
        .item("+998", "Uzbekistan", "Asia")
        .item("+678", "Vanuatu", "Oceania")
        .item("+58", "Venezuela", "South America")
        .item("+84", "Vietnam", "Asia")
        .item("+967", "Yemen", "Asia")
        .item("+260", "Zambia", "Africa")
        .item("+263", "Zimbabwe", "Africa");
    let country_code = country_prompt.interact()?;

    let mut phone_prompt = phone_input("Enter your phone number:").country_code(country_code);
    let _phone = phone_prompt.interact()?;

    // JSON Editor Demo
    let mut json_prompt = json_editor("Edit configuration JSON:")
        .initial_json(r#"{"name": "DX", "version": "0.1.0"}"#);
    let _json_config = json_prompt.interact()?;

    // Matrix Select Demo
    let mut features_matrix = matrix_select("Select features to enable:")
        .row(vec![
            ("auth", "Authentication".to_string()),
            ("db", "Database".to_string()),
            ("cache", "Caching".to_string()),
        ])
        .row(vec![
            ("api", "REST API".to_string()),
            ("graphql", "GraphQL".to_string()),
            ("websocket", "WebSocket".to_string()),
        ])
        .row(vec![
            ("logging", "Logging".to_string()),
            ("metrics", "Metrics".to_string()),
            ("tracing", "Tracing".to_string()),
        ]);
    let _selected_features = features_matrix.interact()?;

    // Search with Filters Demo
    let mut search_prompt = search_filter("Search for a package:")
        .item(
            "react",
            "React",
            vec!["frontend".to_string(), "ui".to_string()],
        )
        .item(
            "vue",
            "Vue.js",
            vec!["frontend".to_string(), "ui".to_string()],
        )
        .item(
            "express",
            "Express",
            vec!["backend".to_string(), "api".to_string()],
        )
        .item(
            "fastify",
            "Fastify",
            vec!["backend".to_string(), "api".to_string()],
        )
        .item("postgres", "PostgreSQL", vec!["database".to_string()])
        .item(
            "redis",
            "Redis",
            vec!["database".to_string(), "cache".to_string()],
        )
        .filter("frontend")
        .filter("backend")
        .filter("database")
        .filter("ui")
        .filter("api")
        .filter("cache");
    let _selected_package = search_prompt.interact()?;

    log::success("All payment and data prompts completed! Let's try content creation tools.")?;

    box_section(
        "Content Creation & Management",
        &["Calendar, Markdown, Code, Emoji, and Kanban!"],
    )?;

    // Calendar View Demo
    let mut calendar_prompt = calendar("Select a date from calendar:");
    let _selected_date = calendar_prompt.interact()?;

    // Markdown Editor Demo
    let mut markdown_prompt = markdown_editor("Write a README:")
        .initial_content("# My Project\n\nA cool project description.");
    let _readme_content = markdown_prompt.interact()?;

    // Code Snippet Demo
    let mut snippet_prompt = code_snippet("Select a code template:")
        .snippet(CodeSnippet {
            name: "React Component".to_string(),
            language: "typescript".to_string(),
            code: "export const MyComponent = () => {\n  return <div>Hello</div>;\n};".to_string(),
            description: "Basic React functional component".to_string(),
        })
        .snippet(CodeSnippet {
            name: "Express Route".to_string(),
            language: "javascript".to_string(),
            code: "app.get('/api/users', async (req, res) => {\n  const users = await db.users.findAll();\n  res.json(users);\n});".to_string(),
            description: "Express.js API route handler".to_string(),
        })
        .snippet(CodeSnippet {
            name: "Rust Function".to_string(),
            language: "rust".to_string(),
            code: "pub fn calculate(x: i32, y: i32) -> i32 {\n    x + y\n}".to_string(),
            description: "Simple Rust function".to_string(),
        });
    let _selected_snippet = snippet_prompt.interact()?;

    // Emoji Picker Demo
    let mut emoji_prompt = emoji_picker("Pick an emoji:");
    let _selected_emoji = emoji_prompt.interact()?;

    // Kanban Board Demo
    let mut kanban_prompt = kanban("Manage your tasks:")
        .task(
            0,
            KanbanTask {
                id: "1".to_string(),
                title: "Setup project".to_string(),
                description: "Initialize repository and dependencies".to_string(),
            },
        )
        .task(
            0,
            KanbanTask {
                id: "2".to_string(),
                title: "Design UI".to_string(),
                description: "Create mockups and wireframes".to_string(),
            },
        )
        .task(
            1,
            KanbanTask {
                id: "3".to_string(),
                title: "Implement auth".to_string(),
                description: "Add authentication system".to_string(),
            },
        )
        .task(
            2,
            KanbanTask {
                id: "4".to_string(),
                title: "Write tests".to_string(),
                description: "Unit and integration tests".to_string(),
            },
        );
    let _kanban_state = kanban_prompt.interact()?;

    log::success("All content creation prompts completed! Now let's configure your AI.")?;

    box_section(
        "AI Configuration",
        &["Now let's configure your AI providers and integrations."],
    )?;

    // Choose AI providers
    let mut providers_prompt = multiselect("Select AI providers to configure:")
        .item(
            "openai",
            "OpenAI (GPT-4, GPT-3.5)",
            "Most popular, great for general tasks",
        )
        .item(
            "anthropic",
            "Anthropic (Claude)",
            "Excellent for analysis and writing",
        )
        .item("google", "Google (Gemini)", "Fast and cost-effective")
        .item(
            "ollama",
            "Ollama (Local models)",
            "Run models locally for privacy",
        )
        .item(
            "custom",
            "Custom API endpoint",
            "Connect to any OpenAI-compatible API",
        );
    let providers = providers_prompt.interact()?;

    if providers.is_empty() {
        log::warning(
            "No providers selected. You can configure them later with 'dx connect <provider>'",
        )?;
    }

    // Choose integrations
    let mut integrations_prompt = multiselect("Select integrations to set up:")
        .item("github", "GitHub", "Code repositories and PR management")
        .item("discord", "Discord", "Chat and community management")
        .item("telegram", "Telegram", "Messaging and notifications")
        .item("notion", "Notion", "Document and knowledge management")
        .item("spotify", "Spotify", "Music control and recommendations")
        .item("gmail", "Gmail", "Email processing and automation")
        .item("slack", "Slack", "Team communication")
        .item("twitter", "Twitter/X", "Social media monitoring")
        .item("browser", "Browser automation", "Web scraping and control")
        .item("filesystem", "File system access", "Local file operations");
    let integrations = integrations_prompt.interact()?;

    if integrations.is_empty() {
        log::info(
            "No integrations selected. You can add them later with 'dx connect <integration>'",
        )?;
    }

    // Choose tools/capabilities
    let mut tools_prompt = multiselect("Select AI tools and capabilities:")
        .item(
            "code_generation",
            "Code Generation",
            "Generate, refactor, and explain code",
        )
        .item(
            "data_analysis",
            "Data Analysis",
            "Process and analyze datasets",
        )
        .item(
            "web_search",
            "Web Search",
            "Search and summarize web content",
        )
        .item(
            "image_generation",
            "Image Generation",
            "Create images with AI",
        )
        .item(
            "speech_recognition",
            "Speech Recognition",
            "Transcribe audio to text",
        )
        .item("translation", "Translation", "Translate between languages")
        .item("summarization", "Summarization", "Condense long texts")
        .item("automation", "Task Automation", "Automate repetitive tasks")
        .item(
            "research",
            "Research Assistant",
            "Help with research and analysis",
        );
    let tools = tools_prompt.interact()?;

    if tools.is_empty() {
        log::info("No tools selected. All tools will be available by default.")?;
    }

    // Choose default AI model
    let mut default_model_prompt = select("Choose your default AI model:")
        .item("gpt-4", "GPT-4", "Most capable, best for complex tasks")
        .item("claude-3", "Claude 3", "Excellent for analysis and writing")
        .item("gemini-pro", "Gemini Pro", "Fast and cost-effective")
        .item(
            "llama-3",
            "Llama 3 (Local)",
            "Privacy-focused, runs locally",
        )
        .item("custom", "Custom model", "Specify your own model");
    let _default_model = default_model_prompt.interact()?;

    // Ask about daemon mode
    let mut start_daemon_prompt =
        confirm("Would you like to start the DX agent daemon now?").initial_value(true);
    let start_daemon = start_daemon_prompt.interact()?;

    if start_daemon {
        log::step("Starting DX agent daemon...")?;
        // Here we would start the daemon
        // For now, just show the command
        log::success("Daemon started! Use 'dx status' to check status.")?;
    }

    // Final setup confirmation
    let mut proceed_prompt =
        confirm("Setup complete! Would you like to start chatting with your AI agent?")
            .initial_value(true);
    let proceed = proceed_prompt.interact()?;

    if proceed {
        outro("🎉 Setup complete! Run 'dx run \"hello\"' to start chatting, or use any of the configured integrations.")?;
    } else {
        outro("Setup complete! You can always start later with 'dx run <your message>'")?;
    }

    Ok(())
}

fn print_banner() {
    // Build banner lines and pad them to a consistent visual width using unicode-width
    use unicode_width::UnicodeWidthStr;

    let lines: Vec<&str> = vec![
        "",
        "   ██████╗ ██╗  ██╗     █████╗  ██████╗ ███████╗███╗   ██╗████████╗",
        "   ██╔══██╗╚██╗██╔╝    ██╔══██╗██╔════╝ ██╔════╝████╗  ██║╚══██╔══╝",
        "   ██║  ██║ ╚███╔╝     ███████║██║  ███╗█████╗  ██╔██╗ ██║   ██║",
        "   ██║  ██║ ██╔██╗     ██╔══██║██║   ██║██╔══╝  ██║╚██╗██║   ██║",
        "   ██████╔╝██╔╝ ██╗    ██║  ██║╚██████╔╝███████╗██║ ╚████║   ██║",
        "   ╚═════╝ ╚═╝  ╚═╝    ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═══╝   ╚═╝",
        "",
        "   🤖 AGI-like AI Agent | Connect to ANY app | 70% token savings",
        "",
    ];

    let max_width = lines
        .iter()
        .map(|s| UnicodeWidthStr::width(*s))
        .max()
        .unwrap_or(0);
    let top = format!("╔{}╗", "═".repeat(max_width));
    let bottom = format!("╚{}╝", "═".repeat(max_width));

    println!();
    println!("{}", top.bright_cyan());
    for line in &lines {
        let cur = UnicodeWidthStr::width(*line);
        let mut s = (*line).to_string();
        if cur < max_width {
            s.push_str(&" ".repeat(max_width - cur));
        }
        println!("{}", format!("║{}║", s).bright_cyan());
    }
    println!("{}", bottom.bright_cyan());
    println!();
}
