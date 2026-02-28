//! Lock and sync (convenience command)

use dx_py_core::Result;

use super::{lock, sync};

/// Run the install command (lock + sync)
pub fn run(dev: bool, extras: &[String], verbose: bool) -> Result<()> {
    // First, lock dependencies
    lock::run(false)?;

    println!();

    // Then, sync to virtual environment
    sync::run(dev, extras, verbose)?;

    Ok(())
}
