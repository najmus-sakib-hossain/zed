//! DX WWW Framework - CLI Entry Point
//!
//! This is the main entry point for the `dx-www` CLI binary.

use std::process::ExitCode;

#[cfg(feature = "cli")]
use dx_www::cli::Cli;

fn main() -> ExitCode {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    #[cfg(feature = "cli")]
    {
        match Cli::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{e:?}");
                ExitCode::FAILURE
            }
        }
    }

    #[cfg(not(feature = "cli"))]
    {
        eprintln!("CLI feature is not enabled. Rebuild with --features cli");
        ExitCode::FAILURE
    }
}
