use anyhow::Result;
use clap::{Parser, Subcommand};
use forge::cli;
use mimalloc::MiMalloc;
use tracing_subscriber::EnvFilter;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Parser, Debug)]
#[command(name = "forge", version, about = "Blazing-fast version control for media assets")]
struct Args {
    #[arg(long, global = true)]
    verbose: bool,

    #[arg(long, global = true)]
    repo_dir: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Init {
        #[arg(default_value = ".")]
        path: String,
    },
    Add {
        paths: Vec<String>,
        #[arg(long)]
        force: bool,
    },
    Commit {
        #[arg(short = 'm', long)]
        message: String,
    },
    Status,
    Log {
        #[arg(short = 'n', long, default_value_t = 20)]
        count: usize,
    },
    Diff {
        path: Option<String>,
        #[arg(long)]
        commit1: Option<String>,
        #[arg(long)]
        commit2: Option<String>,
    },
    Checkout {
        commit_id: String,
    },
    Push {
        #[arg(default_value = "origin")]
        remote: String,
        /// Mirror targets: all-free | youtube | pinterest | soundcloud | sketchfab | github | gdrive | dropbox | mega | r2
        #[arg(long)]
        mirror: Option<String>,
        /// Enable pro paid backends (R2, B2, GCS)
        #[arg(long)]
        pro: bool,
    },
    /// Authenticate a mirror backend and save credentials
    Auth {
        /// Backend: youtube | pinterest | soundcloud | sketchfab | github | gdrive | dropbox | mega | r2 | all-free
        backend: String,
        /// Provide token directly (skip interactive prompt)
        #[arg(long)]
        token: Option<String>,
    },
    /// Create a demo project and print push instructions
    #[command(name = "vibe-demo")]
    VibeDemo,
    Pull {
        #[arg(default_value = "origin")]
        remote: String,
    },
    #[command(name = "train-dict")]
    TrainDict {
        #[arg(long)]
        file_type: String,
        #[arg(long)]
        samples: String,
        #[arg(long)]
        output: String,
    },
}

fn init_tracing(verbose: bool) {
    let default_level = if verbose { "debug" } else { "info" };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("forge={default_level}")));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

fn main() -> Result<()> {
    let args = Args::parse();
    init_tracing(args.verbose);

    if let Some(repo_dir) = &args.repo_dir {
        std::env::set_current_dir(repo_dir)?;
    }

    match args.command {
        Command::Init { path } => cli::init::run(&path),
        Command::Add { mut paths, force } => {
            if paths.is_empty() {
                paths.push(".".to_string());
            }
            cli::add::run(&paths, force)
        }
        Command::Commit { message } => cli::commit::run(&message),
        Command::Status => cli::status::run(),
        Command::Log { count } => cli::log::run(count),
        Command::Diff {
            path,
            commit1,
            commit2,
        } => cli::diff::run(path.as_deref(), commit1.as_deref(), commit2.as_deref()),
        Command::Checkout { commit_id } => cli::checkout::run(&commit_id),
        Command::Push { remote, mirror, pro } => cli::push::run(&remote, mirror.as_deref(), pro),
        Command::Pull { remote } => cli::pull::run(&remote),
        Command::Auth { backend, token } => cli::auth::run(&backend, token.as_deref()),
        Command::VibeDemo => cli::vibe_demo::run(),
        Command::TrainDict {
            file_type,
            samples,
            output,
        } => cli::train_dict::run(&file_type, &samples, &output),
    }
}
