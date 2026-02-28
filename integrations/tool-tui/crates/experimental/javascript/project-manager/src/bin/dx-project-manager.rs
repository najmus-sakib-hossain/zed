//! dx-project-manager CLI binary
//!
//! Binary-first project management system.

use std::process::ExitCode;

fn main() -> ExitCode {
    let exit_code = dx_js_project_manager::cli::main();
    ExitCode::from(exit_code as u8)
}
