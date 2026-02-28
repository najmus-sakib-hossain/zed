//! Self-update system for the DX CLI

mod applier;
mod cache;
mod checker;
mod daemon;
mod downloader;
mod signature;
mod types;

pub use applier::UpdateApplier;
pub use checker::UpdateChecker;
pub use downloader::UpdateDownloader;
pub use types::UpdateInfo;

/// Current version of the DX CLI
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
/// GitHub repository for update checking
pub const GITHUB_REPO: &str = "user/dx";
/// GitHub releases API URL
pub const RELEASES_API_URL: &str = "https://api.github.com/repos/user/dx/releases/latest";
/// DX API URL for updates
pub const DX_API_URL: &str = "https://api.dx.dev/v1/updates";
