//! Shell integration script generation and installation

use super::types::ShellType;
use crate::utils::error::DxError;

const INTEGRATION_MARKER: &str = "# DX Shell Integration";

pub fn generate_integration(shell: ShellType) -> String {
    match shell {
        ShellType::Bash => generate_bash(),
        ShellType::Zsh => generate_zsh(),
        ShellType::Fish => generate_fish(),
        ShellType::PowerShell => generate_powershell(),
        ShellType::Nushell => generate_nushell(),
    }
}

fn generate_bash() -> String {
    format!(
        r#"{marker}
# Smart aliases for DX CLI
alias d='dx'
alias dr='dx run'
alias db='dx build'
alias dd='dx dev'
alias dt='dx test'
alias dg='dx generator'
alias ds='dx style'
alias df='dx forge'

# CD hook for DX project detection
__dx_cd_hook() {{
    if [[ -f "dx.toml" ]]; then
        echo -e "\033[36m→\033[0m DX project detected"
    fi
}}

cd() {{
    builtin cd "$@" && __dx_cd_hook
}}

if command -v dx &> /dev/null; then
    eval "$(dx completions bash 2>/dev/null)"
fi
"#,
        marker = INTEGRATION_MARKER
    )
}

fn generate_zsh() -> String {
    format!(
        r#"{marker}
# Smart aliases for DX CLI
alias d='dx'
alias dr='dx run'
alias db='dx build'
alias dd='dx dev'
alias dt='dx test'
alias dg='dx generator'
alias ds='dx style'
alias df='dx forge'

# CD hook for DX project detection
__dx_chpwd_hook() {{
    if [[ -f "dx.toml" ]]; then
        echo -e "\033[36m→\033[0m DX project detected"
    fi
}}

autoload -Uz add-zsh-hook
add-zsh-hook chpwd __dx_chpwd_hook

if command -v dx &> /dev/null; then
    eval "$(dx completions zsh 2>/dev/null)"
fi
"#,
        marker = INTEGRATION_MARKER
    )
}

fn generate_fish() -> String {
    format!(
        r#"{marker}
# Smart aliases for DX CLI
alias d='dx'
alias dr='dx run'
alias db='dx build'
alias dd='dx dev'
alias dt='dx test'
alias dg='dx generator'
alias ds='dx style'
alias df='dx forge'

# CD hook for DX project detection
function __dx_cd_hook --on-variable PWD
    if test -f "dx.toml"
        echo -e "\033[36m→\033[0m DX project detected"
    end
end

if command -v dx &> /dev/null
    dx completions fish 2>/dev/null | source
end
"#,
        marker = INTEGRATION_MARKER
    )
}

fn generate_powershell() -> String {
    format!(
        r#"{marker}
# Smart aliases for DX CLI
Set-Alias -Name d -Value dx
function dr {{ dx run $args }}
function db {{ dx build $args }}
function dd {{ dx dev $args }}
function dt {{ dx test $args }}
function dg {{ dx generator $args }}
function ds {{ dx style $args }}
function df {{ dx forge $args }}

# CD hook for DX project detection
$__dx_original_prompt = $function:prompt
function prompt {{
    if (Test-Path "dx.toml") {{
        Write-Host "→ DX project detected" -ForegroundColor Cyan
    }}
    & $__dx_original_prompt
}}

if (Get-Command dx -ErrorAction SilentlyContinue) {{
    dx completions powershell 2>$null | Out-String | Invoke-Expression
}}
"#,
        marker = INTEGRATION_MARKER
    )
}

fn generate_nushell() -> String {
    format!(
        r#"{marker}
# Smart aliases for DX CLI
alias d = dx
alias dr = dx run
alias db = dx build
alias dd = dx dev
alias dt = dx test
alias dg = dx generator
alias ds = dx style
alias df = dx forge
"#,
        marker = INTEGRATION_MARKER
    )
}

pub fn is_installed(shell: ShellType) -> Result<bool, DxError> {
    let config_path = shell.config_path().ok_or(DxError::ShellNotDetected)?;

    if !config_path.exists() {
        return Ok(false);
    }

    let content = std::fs::read_to_string(&config_path).map_err(|e| DxError::Io {
        message: format!("Failed to read {}: {}", config_path.display(), e),
    })?;

    Ok(content.contains(INTEGRATION_MARKER))
}

pub fn install(shell: ShellType, force: bool) -> Result<(), DxError> {
    if !force && is_installed(shell)? {
        return Err(DxError::ShellIntegrationExists {
            shell: shell.name().to_string(),
        });
    }

    let config_path = shell.config_path().ok_or(DxError::ShellNotDetected)?;

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DxError::Io {
            message: format!("Failed to create directory {}: {}", parent.display(), e),
        })?;
    }

    let integration = generate_integration(shell);

    let existing = if config_path.exists() {
        std::fs::read_to_string(&config_path).unwrap_or_default()
    } else {
        String::new()
    };

    let cleaned = if force {
        remove_integration(&existing)
    } else {
        existing
    };

    let new_content = if cleaned.is_empty() {
        integration
    } else {
        format!("{}\n\n{}", cleaned.trim_end(), integration)
    };

    std::fs::write(&config_path, &new_content).map_err(|e| DxError::Io {
        message: format!("Failed to write {}: {}", config_path.display(), e),
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o644);
        std::fs::set_permissions(&config_path, perms).map_err(|e| DxError::Io {
            message: format!("Failed to set permissions on {}: {}", config_path.display(), e),
        })?;
    }

    Ok(())
}

pub fn uninstall(shell: ShellType) -> Result<(), DxError> {
    let config_path = shell.config_path().ok_or(DxError::ShellNotDetected)?;

    if !config_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&config_path).map_err(|e| DxError::Io {
        message: format!("Failed to read {}: {}", config_path.display(), e),
    })?;

    let cleaned = remove_integration(&content);

    std::fs::write(&config_path, cleaned).map_err(|e| DxError::Io {
        message: format!("Failed to write {}: {}", config_path.display(), e),
    })?;

    Ok(())
}

fn remove_integration(content: &str) -> String {
    let mut result = String::new();
    let mut in_integration = false;

    for line in content.lines() {
        if line.contains(INTEGRATION_MARKER) {
            in_integration = true;
            continue;
        }

        if in_integration {
            if line.starts_with('#') && !line.contains("DX") && !line.contains("dx") {
                in_integration = false;
            } else if line.trim().is_empty() {
                continue;
            } else if !line.starts_with('#')
                && !line.starts_with("alias")
                && !line.starts_with("function")
                && !line.starts_with("if")
                && !line.starts_with("fi")
                && !line.starts_with("eval")
                && !line.starts_with("autoload")
                && !line.starts_with("add-zsh-hook")
                && !line.starts_with("Set-Alias")
                && !line.starts_with("$")
                && !line.contains("dx")
                && !line.contains("__dx")
            {
                in_integration = false;
            }
        }

        if !in_integration {
            result.push_str(line);
            result.push('\n');
        }
    }

    result.trim_end().to_string() + "\n"
}
