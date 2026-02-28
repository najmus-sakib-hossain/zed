//! Logging system for DX CLI

use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn init_logger() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry().with(filter).with(fmt::layer()).init();
}

pub fn init_file_logger(log_path: PathBuf) -> anyhow::Result<()> {
    let file = std::fs::OpenOptions::new().create(true).append(true).open(log_path)?;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(file))
        .init();

    Ok(())
}
