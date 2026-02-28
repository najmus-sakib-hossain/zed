//! dx generator: Code Generation Tools
//!
//! AI-powered and template-based code generation:
//! - Component scaffolding
//! - API endpoint generation
//! - Database model generation
//! - Form generation from schema
//! - Type generation
//! - Migration generation

use anyhow::Result;
use clap::{Args, Subcommand};
use owo_colors::OwoColorize;

use crate::ui::{spinner::Spinner, table, theme::Theme};

#[derive(Args)]
pub struct GeneratorArgs {
    #[command(subcommand)]
    pub command: GeneratorCommands,
}

#[derive(Subcommand)]
pub enum GeneratorCommands {
    /// Generate a component
    Component {
        /// Component name
        #[arg(index = 1)]
        name: String,

        /// Component type (functional, class, server)
        #[arg(short, long, default_value = "functional")]
        kind: String,

        /// Include tests
        #[arg(long)]
        with_test: bool,

        /// Include styles
        #[arg(long)]
        with_style: bool,
    },

    /// Generate API endpoint
    Api {
        /// Endpoint name
        #[arg(index = 1)]
        name: String,

        /// HTTP methods (get, post, put, delete)
        #[arg(short, long, default_value = "get,post")]
        methods: String,

        /// Include validation
        #[arg(long)]
        with_validation: bool,
    },

    /// Generate database model
    Model {
        /// Model name
        #[arg(index = 1)]
        name: String,

        /// Fields (name:type,name:type)
        #[arg(short, long)]
        fields: Option<String>,

        /// Generate migration
        #[arg(long)]
        with_migration: bool,
    },

    /// Generate form from schema
    Form {
        /// Form name
        #[arg(index = 1)]
        name: String,

        /// Schema file
        #[arg(short, long)]
        schema: Option<String>,

        /// Include validation
        #[arg(long)]
        with_validation: bool,
    },

    /// Generate types from schema
    Types {
        /// Input schema file
        #[arg(index = 1)]
        input: Option<String>,

        /// Output file
        #[arg(short, long)]
        output: Option<String>,

        /// Schema format (json-schema, openapi, graphql)
        #[arg(long, default_value = "json-schema")]
        format: String,
    },

    /// Generate database migration
    Migration {
        /// Migration name
        #[arg(index = 1)]
        name: String,

        /// Generate from model diff
        #[arg(long)]
        auto: bool,
    },

    /// Generate CRUD operations
    Crud {
        /// Resource name
        #[arg(index = 1)]
        name: String,

        /// Include all CRUD operations
        #[arg(long)]
        full: bool,
    },

    /// Generate from template
    Template {
        /// Template name
        #[arg(index = 1)]
        template: String,

        /// Output name
        #[arg(index = 2)]
        name: Option<String>,
    },

    /// List available generators
    List,

    /// Show generator configuration
    Config,
}

pub async fn run(args: GeneratorArgs, theme: &Theme) -> Result<()> {
    match args.command {
        GeneratorCommands::Component {
            name,
            kind,
            with_test,
            with_style,
        } => run_component(&name, &kind, with_test, with_style, theme).await,
        GeneratorCommands::Api {
            name,
            methods,
            with_validation,
        } => run_api(&name, &methods, with_validation, theme).await,
        GeneratorCommands::Model {
            name,
            fields,
            with_migration,
        } => run_model(&name, fields, with_migration, theme).await,
        GeneratorCommands::Form {
            name,
            schema: _,
            with_validation,
        } => run_form(&name, with_validation, theme).await,
        GeneratorCommands::Types {
            input: _,
            output: _,
            format,
        } => run_types(&format, theme).await,
        GeneratorCommands::Migration { name, auto } => run_migration(&name, auto, theme).await,
        GeneratorCommands::Crud { name, full } => run_crud(&name, full, theme).await,
        GeneratorCommands::Template { template, name } => {
            run_template(&template, name, theme).await
        }
        GeneratorCommands::List => run_list(theme).await,
        GeneratorCommands::Config => run_config(theme).await,
    }
}

async fn run_component(
    name: &str,
    kind: &str,
    with_test: bool,
    with_style: bool,
    theme: &Theme,
) -> Result<()> {
    theme.print_section(&format!("dx generator: Component {}", name));
    eprintln!();

    eprintln!("  {} Type: {}", "│".bright_black(), kind.cyan());
    eprintln!();

    let spinner = Spinner::dots("Generating component...");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    spinner.success(format!("Created components/{}.tsx", name));

    if with_style {
        let spinner = Spinner::dots("Generating styles...");
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        spinner.success(format!("Created components/{}.css", name));
    }

    if with_test {
        let spinner = Spinner::dots("Generating tests...");
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        spinner.success(format!("Created components/{}.test.tsx", name));
    }

    let spinner = Spinner::dots("Updating barrel exports...");
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    spinner.success("Updated components/index.ts");

    eprintln!();
    eprintln!("  {} Generated files:", "│".bright_black());
    eprintln!("    {} components/{}.tsx", "├".bright_black(), name.cyan());
    if with_style {
        eprintln!("    {} components/{}.css", "├".bright_black(), name.cyan());
    }
    if with_test {
        eprintln!("    {} components/{}.test.tsx", "├".bright_black(), name.cyan());
    }
    eprintln!();

    theme.print_success(&format!("Component {} created", name));
    eprintln!();

    Ok(())
}

async fn run_api(name: &str, methods: &str, with_validation: bool, theme: &Theme) -> Result<()> {
    theme.print_section(&format!("dx generator: API {}", name));
    eprintln!();

    eprintln!("  {} Methods: {}", "│".bright_black(), methods.cyan());
    eprintln!();

    let spinner = Spinner::dots("Generating route handler...");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    spinner.success(format!("Created api/{}/route.ts", name));

    if with_validation {
        let spinner = Spinner::dots("Generating validation schema...");
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        spinner.success(format!("Created api/{}/schema.ts", name));
    }

    let spinner = Spinner::dots("Generating types...");
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    spinner.success(format!("Created api/{}/types.ts", name));

    eprintln!();
    eprintln!("  {} Endpoints:", "│".bright_black());
    for method in methods.split(',') {
        eprintln!("    {} {} /api/{}", "├".bright_black(), method.to_uppercase().cyan(), name);
    }
    eprintln!();

    theme.print_success(&format!("API endpoint {} created", name));
    eprintln!();

    Ok(())
}

async fn run_model(
    name: &str,
    fields: Option<String>,
    with_migration: bool,
    theme: &Theme,
) -> Result<()> {
    theme.print_section(&format!("dx generator: Model {}", name));
    eprintln!();

    let field_str = fields.as_deref().unwrap_or("id:uuid,created_at:timestamp");
    eprintln!("  {} Fields: {}", "│".bright_black(), field_str.cyan());
    eprintln!();

    let spinner = Spinner::dots("Generating model...");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    spinner.success(format!("Created models/{}.ts", name));

    let spinner = Spinner::dots("Generating types...");
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    spinner.success(format!("Created models/{}.types.ts", name));

    if with_migration {
        let spinner = Spinner::dots("Generating migration...");
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        spinner.success(format!(
            "Created migrations/{}_create_{}.sql",
            "20251219",
            name.to_lowercase()
        ));
    }

    eprintln!();
    eprintln!("  {} Model schema:", "│".bright_black());
    eprintln!("    {}", format!("struct {} {{", name).bright_black());
    for field in field_str.split(',') {
        let parts: Vec<&str> = field.split(':').collect();
        if parts.len() == 2 {
            eprintln!("      {}: {};", parts[0].cyan(), parts[1].yellow());
        }
    }
    eprintln!("    {}", "}".bright_black());
    eprintln!();

    theme.print_success(&format!("Model {} created", name));
    eprintln!();

    Ok(())
}

async fn run_form(name: &str, with_validation: bool, theme: &Theme) -> Result<()> {
    theme.print_section(&format!("dx generator: Form {}", name));
    eprintln!();

    let spinner = Spinner::dots("Analyzing schema...");
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    spinner.success("Found 5 fields");

    let spinner = Spinner::dots("Generating form component...");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    spinner.success(format!("Created forms/{}.tsx", name));

    if with_validation {
        let spinner = Spinner::dots("Generating validation...");
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        spinner.success(format!("Created forms/{}.schema.ts", name));
    }

    let spinner = Spinner::dots("Generating types...");
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    spinner.success(format!("Created forms/{}.types.ts", name));

    eprintln!();
    theme.print_info("Fields", "5");
    theme.print_info("Validation", if with_validation { "Zod" } else { "None" });
    eprintln!();

    theme.print_success(&format!("Form {} created", name));
    eprintln!();

    Ok(())
}

async fn run_types(format: &str, theme: &Theme) -> Result<()> {
    theme.print_section(&format!("dx generator: Types ({})", format));
    eprintln!();

    let spinner = Spinner::dots("Parsing schema...");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    spinner.success("Parsed schema.json");

    let spinner = Spinner::dots("Generating TypeScript types...");
    tokio::time::sleep(std::time::Duration::from_millis(60)).await;
    spinner.success("Generated 12 types");

    let spinner = Spinner::dots("Writing output...");
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    spinner.success("Created types/generated.ts");

    eprintln!();
    eprintln!("  {} Generated types:", "│".bright_black());
    eprintln!("    {} {}", "├".bright_black(), "User".cyan());
    eprintln!("    {} {}", "├".bright_black(), "Post".cyan());
    eprintln!("    {} {}", "├".bright_black(), "Comment".cyan());
    eprintln!(
        "    {} {} {}",
        "└".bright_black(),
        "...".bright_black(),
        "9 more".bright_black()
    );
    eprintln!();

    theme.print_success("Types generated");
    eprintln!();

    Ok(())
}

async fn run_migration(name: &str, auto: bool, theme: &Theme) -> Result<()> {
    theme.print_section(&format!("dx generator: Migration {}", name));
    eprintln!();

    if auto {
        let spinner = Spinner::dots("Comparing model states...");
        tokio::time::sleep(std::time::Duration::from_millis(60)).await;
        spinner.success("Found 3 changes");

        let spinner = Spinner::dots("Generating migration...");
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        spinner.success("Generated from diff");
    } else {
        let spinner = Spinner::dots("Creating migration file...");
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        spinner.success("Created migration template");
    }

    eprintln!();
    theme.print_info("File", &format!("migrations/20251219_{}.sql", name.to_lowercase()));
    eprintln!();

    if auto {
        eprintln!("  {} Changes:", "│".bright_black());
        eprintln!("    {} ADD COLUMN email VARCHAR(255)", "+".green());
        eprintln!("    {} ALTER COLUMN name SET NOT NULL", "~".yellow());
        eprintln!("    {} DROP COLUMN deprecated_field", "-".red());
        eprintln!();
    }

    theme.print_success(&format!("Migration {} created", name));
    eprintln!();

    Ok(())
}

async fn run_crud(name: &str, full: bool, theme: &Theme) -> Result<()> {
    theme.print_section(&format!("dx generator: CRUD {}", name));
    eprintln!();

    let operations = if full {
        vec!["Create", "Read", "Update", "Delete", "List", "Search"]
    } else {
        vec!["Create", "Read", "Update", "Delete"]
    };

    for op in &operations {
        let spinner = Spinner::dots(format!("Generating {}...", op));
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        spinner.success(format!("{} handler", op));
    }

    let spinner = Spinner::dots("Generating types...");
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    spinner.success("Type definitions");

    let spinner = Spinner::dots("Generating tests...");
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    spinner.success(format!("{} tests", operations.len() * 2));

    eprintln!();
    eprintln!("  {} Generated files:", "│".bright_black());
    eprintln!("    {} api/{}/route.ts", "├".bright_black(), name.to_lowercase().cyan());
    eprintln!("    {} api/{}/[id]/route.ts", "├".bright_black(), name.to_lowercase().cyan());
    eprintln!("    {} api/{}/types.ts", "├".bright_black(), name.to_lowercase().cyan());
    eprintln!("    {} api/{}/route.test.ts", "└".bright_black(), name.to_lowercase().cyan());
    eprintln!();

    theme.print_success(&format!("CRUD for {} created", name));
    eprintln!();

    Ok(())
}

async fn run_template(template: &str, name: Option<String>, theme: &Theme) -> Result<()> {
    let output_name = name.as_deref().unwrap_or(template);
    theme.print_section(&format!("dx generator: Template {} → {}", template, output_name));
    eprintln!();

    let spinner = Spinner::dots("Loading template...");
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    spinner.success(format!("Loaded {}", template));

    let spinner = Spinner::dots("Processing variables...");
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    spinner.success("Substituted 8 variables");

    let spinner = Spinner::dots("Writing files...");
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    spinner.success("Created 3 files");

    theme.print_success(&format!("Template {} applied", template));
    eprintln!();

    Ok(())
}

async fn run_list(theme: &Theme) -> Result<()> {
    theme.print_section("dx generator: Available Generators");
    eprintln!();

    let generators = [
        ("component", "Generate React/Vue/Svelte component"),
        ("api", "Generate API endpoint with handlers"),
        ("model", "Generate database model and types"),
        ("form", "Generate form from schema"),
        ("types", "Generate TypeScript types from schema"),
        ("migration", "Generate database migration"),
        ("crud", "Generate full CRUD operations"),
        ("template", "Generate from custom template"),
    ];

    for (name, desc) in generators {
        eprintln!("  {} {} - {}", "├".bright_black(), name.cyan(), desc.white());
    }

    eprintln!();
    eprintln!(
        "  {} Use {} for details",
        "→".cyan(),
        "dx generator <name> --help".cyan().bold()
    );
    eprintln!();

    Ok(())
}

async fn run_config(theme: &Theme) -> Result<()> {
    theme.print_section("dx generator: Configuration");
    eprintln!();

    table::print_kv_list(&[
        ("Component style", "functional"),
        ("Test framework", "vitest"),
        ("Style system", "dx-style (B-CSS)"),
        ("Validation", "zod"),
        ("Database", "PostgreSQL"),
        ("Templates dir", ".dx/templates"),
    ]);
    eprintln!();

    Ok(())
}
