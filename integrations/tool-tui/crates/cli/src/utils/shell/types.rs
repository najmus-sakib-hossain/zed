//! Shell type definitions

use std::path::PathBuf;

/// Shell types supported by DX
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Nushell,
}

impl ShellType {
    /// Detect the current shell from environment
    pub fn detect() -> Option<Self> {
        if let Ok(shell) = std::env::var("SHELL") {
            let shell_lower = shell.to_lowercase();
            if shell_lower.contains("bash") {
                return Some(ShellType::Bash);
            }
            if shell_lower.contains("zsh") {
                return Some(ShellType::Zsh);
            }
            if shell_lower.contains("fish") {
                return Some(ShellType::Fish);
            }
        }

        #[cfg(windows)]
        {
            if std::env::var("PSModulePath").is_ok() {
                return Some(ShellType::PowerShell);
            }
        }

        if std::env::var("NU_VERSION").is_ok() {
            return Some(ShellType::Nushell);
        }

        None
    }

    /// Get the config file path for this shell
    pub fn config_path(&self) -> Option<PathBuf> {
        let home = home::home_dir()?;

        Some(match self {
            ShellType::Bash => {
                let bashrc = home.join(".bashrc");
                if bashrc.exists() {
                    bashrc
                } else {
                    home.join(".bash_profile")
                }
            }
            ShellType::Zsh => home.join(".zshrc"),
            ShellType::Fish => home.join(".config/fish/config.fish"),
            ShellType::PowerShell => {
                #[cfg(windows)]
                {
                    home.join("Documents/PowerShell/Microsoft.PowerShell_profile.ps1")
                }
                #[cfg(not(windows))]
                {
                    home.join(".config/powershell/Microsoft.PowerShell_profile.ps1")
                }
            }
            ShellType::Nushell => home.join(".config/nushell/config.nu"),
        })
    }

    /// Get the shell name as a string
    pub fn name(&self) -> &'static str {
        match self {
            ShellType::Bash => "bash",
            ShellType::Zsh => "zsh",
            ShellType::Fish => "fish",
            ShellType::PowerShell => "powershell",
            ShellType::Nushell => "nushell",
        }
    }
}

impl std::fmt::Display for ShellType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
