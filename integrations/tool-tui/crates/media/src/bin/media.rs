//! Unified Media CLI - Search and download media, icons, and fonts
//!
//! Usage:
//!   media search "sunset" --type image
//!   media icon search "home" --limit 10
//!   media font search "roboto"
//!   media tools image convert input.png output.jpg

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match dx_media::cli_unified::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");

            // Print chain of errors
            let mut source = e.source();
            while let Some(cause) = source {
                eprintln!("  Caused by: {cause}");
                source = cause.source();
            }

            ExitCode::FAILURE
        }
    }
}
