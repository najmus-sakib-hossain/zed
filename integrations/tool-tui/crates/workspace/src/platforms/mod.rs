//! Platform definitions and generators.
//!
//! This module contains all supported IDE/editor platform definitions
//! and their respective configuration generators.

pub mod cloud;
pub mod container;
pub mod desktop;

// Re-export generator traits
pub use cloud::CloudGenerator;
pub use container::ContainerGenerator;
pub use desktop::DesktopGenerator;

// Re-export specific generators
pub use cloud::codesandbox::CodeSandboxGenerator;
pub use cloud::codespaces::CodespacesGenerator;
pub use cloud::firebase_studio::FirebaseStudioGenerator;
pub use cloud::gitpod::GitpodGenerator;
pub use cloud::replit::ReplitGenerator;
pub use cloud::stackblitz::StackBlitzGenerator;
pub use container::docker_compose::DockerComposeGenerator;
pub use container::nix_flakes::NixFlakesGenerator;
pub use desktop::helix::HelixGenerator;
pub use desktop::intellij::IntelliJGenerator;
pub use desktop::neovim::NeovimGenerator;
pub use desktop::sublime::SublimeGenerator;
pub use desktop::vscode::VsCodeGenerator;
pub use desktop::zed::ZedGenerator;

use serde::{Deserialize, Serialize};

/// Supported development environment platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Platform {
    // Desktop Editors
    /// Visual Studio Code / VS Codium.
    VsCode,
    /// Zed editor (Rust-based).
    Zed,
    /// Neovim / Vim.
    Neovim,
    /// IntelliJ IDEA / Fleet.
    IntelliJ,
    /// Helix editor.
    Helix,
    /// Sublime Text.
    SublimeText,

    // Cloud IDEs
    /// GitHub Codespaces.
    Codespaces,
    /// Gitpod.
    Gitpod,
    /// CodeSandbox.
    CodeSandbox,
    /// Firebase Studio (Project IDX).
    FirebaseStudio,
    /// StackBlitz.
    StackBlitz,
    /// Replit.
    Replit,
    /// Glitch.
    Glitch,
    /// CodeAnywhere.
    CodeAnywhere,
    /// AWS Cloud9.
    Cloud9,

    // Container Environments
    /// Dev Containers (devcontainer.json).
    DevContainer,
    /// Docker Compose.
    DockerCompose,
    /// Podman.
    Podman,
    /// Nix Flakes.
    NixFlakes,
}

impl Platform {
    /// Get all desktop editor platforms.
    pub fn desktop_editors() -> &'static [Platform] {
        &[
            Platform::VsCode,
            Platform::Zed,
            Platform::Neovim,
            Platform::IntelliJ,
            Platform::Helix,
            Platform::SublimeText,
        ]
    }

    /// Get all cloud IDE platforms.
    pub fn cloud_ides() -> &'static [Platform] {
        &[
            Platform::Codespaces,
            Platform::Gitpod,
            Platform::CodeSandbox,
            Platform::FirebaseStudio,
            Platform::StackBlitz,
            Platform::Replit,
            Platform::Glitch,
            Platform::CodeAnywhere,
            Platform::Cloud9,
        ]
    }

    /// Get all container environment platforms.
    pub fn container_environments() -> &'static [Platform] {
        &[
            Platform::DevContainer,
            Platform::DockerCompose,
            Platform::Podman,
            Platform::NixFlakes,
        ]
    }

    /// Get all supported platforms.
    pub fn all() -> Vec<Platform> {
        let mut platforms = Vec::new();
        platforms.extend_from_slice(Self::desktop_editors());
        platforms.extend_from_slice(Self::cloud_ides());
        platforms.extend_from_slice(Self::container_environments());
        platforms
    }

    /// Check if platform is a desktop editor.
    pub fn is_desktop(&self) -> bool {
        Self::desktop_editors().contains(self)
    }

    /// Check if platform is a cloud IDE.
    pub fn is_cloud(&self) -> bool {
        Self::cloud_ides().contains(self)
    }

    /// Check if platform is a container environment.
    pub fn is_container(&self) -> bool {
        Self::container_environments().contains(self)
    }

    /// Get the configuration file paths for this platform.
    pub fn config_paths(&self) -> Vec<&'static str> {
        match self {
            Platform::VsCode => vec![
                ".vscode/settings.json",
                ".vscode/tasks.json",
                ".vscode/launch.json",
                ".vscode/extensions.json",
            ],
            Platform::Zed => vec![".zed/settings.json", ".zed/tasks.json"],
            Platform::Neovim => vec![".nvim.lua", ".nvim/init.lua"],
            Platform::IntelliJ => vec![".idea/workspace.xml", ".idea/runConfigurations/"],
            Platform::Helix => vec![".helix/config.toml", ".helix/languages.toml"],
            Platform::SublimeText => vec!["*.sublime-project", "*.sublime-workspace"],
            Platform::Codespaces => vec![".devcontainer/devcontainer.json"],
            Platform::Gitpod => vec![".gitpod.yml", ".gitpod.Dockerfile"],
            Platform::CodeSandbox => vec![".codesandbox/tasks.json", "sandbox.config.json"],
            Platform::FirebaseStudio => vec![".idx/dev.nix"],
            Platform::StackBlitz => vec![".stackblitzrc"],
            Platform::Replit => vec![".replit", "replit.nix"],
            Platform::Glitch => vec!["glitch.json", "watch.json"],
            Platform::CodeAnywhere => vec![".devbox.json"],
            Platform::Cloud9 => vec![".c9/launch.json"],
            Platform::DevContainer => vec![".devcontainer/devcontainer.json"],
            Platform::DockerCompose => vec!["docker-compose.yml", "compose.yaml"],
            Platform::Podman => vec!["podman-compose.yml"],
            Platform::NixFlakes => vec!["flake.nix", ".envrc"],
        }
    }

    /// Get human-readable name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Platform::VsCode => "VS Code",
            Platform::Zed => "Zed",
            Platform::Neovim => "Neovim",
            Platform::IntelliJ => "IntelliJ IDEA",
            Platform::Helix => "Helix",
            Platform::SublimeText => "Sublime Text",
            Platform::Codespaces => "GitHub Codespaces",
            Platform::Gitpod => "Gitpod",
            Platform::CodeSandbox => "CodeSandbox",
            Platform::FirebaseStudio => "Firebase Studio (IDX)",
            Platform::StackBlitz => "StackBlitz",
            Platform::Replit => "Replit",
            Platform::Glitch => "Glitch",
            Platform::CodeAnywhere => "CodeAnywhere",
            Platform::Cloud9 => "AWS Cloud9",
            Platform::DevContainer => "Dev Containers",
            Platform::DockerCompose => "Docker Compose",
            Platform::Podman => "Podman",
            Platform::NixFlakes => "Nix Flakes",
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_categories() {
        assert!(Platform::VsCode.is_desktop());
        assert!(!Platform::VsCode.is_cloud());

        assert!(Platform::Gitpod.is_cloud());
        assert!(!Platform::Gitpod.is_desktop());

        assert!(Platform::DevContainer.is_container());
    }

    #[test]
    fn test_all_platforms() {
        let all = Platform::all();
        assert!(all.contains(&Platform::VsCode));
        assert!(all.contains(&Platform::Gitpod));
        assert!(all.contains(&Platform::NixFlakes));
    }

    #[test]
    fn test_config_paths() {
        let paths = Platform::VsCode.config_paths();
        assert!(paths.contains(&".vscode/settings.json"));
    }
}
