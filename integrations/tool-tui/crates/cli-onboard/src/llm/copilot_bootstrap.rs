use anyhow::{Context, Result, anyhow};
use copilot_sdk_supercharged::CopilotClient;
use copilot_sdk_supercharged::types::CopilotClientOptions;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Password};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::process::Command;

const COPILOT_TOKEN_URL: &str = "https://api.github.com/copilot_internal/v2/token";
const DEFAULT_COPILOT_API_BASE_URL: &str = "https://api.individual.githubcopilot.com";

const DX_COPILOT_USE_TOKEN_CACHE_ENV: &str = "DX_COPILOT_USE_TOKEN_CACHE";
const DX_COPILOT_TRUST_ENV_TOKEN_ENV: &str = "DX_COPILOT_TRUST_ENV_TOKEN";

#[derive(Debug, Clone)]
pub struct CopilotBootstrapResult {
    pub token: String,
    pub expires_at_ms: u64,
    pub source: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedCopilotToken {
    token: String,
    /// milliseconds since epoch
    #[serde(rename = "expiresAt")]
    expires_at_ms: u64,
    /// milliseconds since epoch
    #[serde(rename = "updatedAt")]
    updated_at_ms: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis() as u64
}

fn is_token_usable(cache: &CachedCopilotToken, now: u64) -> bool {
    // Keep a small safety margin when checking expiry.
    cache.expires_at_ms.saturating_sub(now) > 5 * 60 * 1000
}

fn derive_copilot_api_base_url_from_token(token: &str) -> Option<String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Token is semicolon-delimited key/value pairs. Look for `proxy-ep=...`.
    let mut proxy_ep: Option<&str> = None;
    for part in trimmed.split(';') {
        let part = part.trim();
        if let Some(rest) =
            part.strip_prefix("proxy-ep=").or_else(|| part.strip_prefix("proxy-ep ="))
        {
            proxy_ep = Some(rest.trim());
            break;
        }

        // Case-insensitive fallback.
        let lower = part.to_ascii_lowercase();
        if lower.starts_with("proxy-ep=") {
            proxy_ep = Some(part.split_once('=').map(|(_, v)| v.trim()).unwrap_or(""));
            break;
        }
    }

    let proxy_ep = proxy_ep?.trim();
    if proxy_ep.is_empty() {
        return None;
    }

    let host = proxy_ep.trim_start_matches("https://").trim_start_matches("http://").trim();
    if host.is_empty() {
        return None;
    }

    // Align with upstream logic: proxy.* -> api.*
    let host = if host.to_ascii_lowercase().starts_with("proxy.") {
        format!("api.{}", &host["proxy.".len()..])
    } else {
        host.to_string()
    };

    Some(format!("https://{}", host))
}

fn dx_config_dir() -> PathBuf {
    // Uses standard OS config location.
    // Windows: %APPDATA%\dx
    // macOS: ~/Library/Application Support/dx
    // Linux: ~/.config/dx
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("dx")
}

fn copilot_token_cache_path() -> PathBuf {
    dx_config_dir().join("credentials").join("github-copilot.token.json")
}

fn should_use_token_cache() -> bool {
    match env::var(DX_COPILOT_USE_TOKEN_CACHE_ENV) {
        Ok(value) => {
            let v = value.trim().to_ascii_lowercase();
            matches!(v.as_str(), "1" | "true" | "yes" | "on")
        }
        Err(_) => false,
    }
}

fn should_trust_env_token() -> bool {
    match env::var(DX_COPILOT_TRUST_ENV_TOKEN_ENV) {
        Ok(value) => {
            let v = value.trim().to_ascii_lowercase();
            matches!(v.as_str(), "1" | "true" | "yes" | "on")
        }
        Err(_) => false,
    }
}

async fn run_status_inherit(cmd: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .with_context(|| format!("failed to spawn {cmd}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("{cmd} failed with status {status}"))
    }
}

async fn run_capture_stdout(cmd: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .await
        .with_context(|| format!("failed to spawn {cmd}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("{cmd} failed with status {}: {}", output.status, stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn command_works(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|status| status.success())
        .unwrap_or(false)
}

async fn ensure_copilot_cli_installed_interactive() -> Result<()> {
    if command_works("copilot", &["version"]).await
        || command_works("copilot", &["--version"]).await
    {
        return Ok(());
    }

    eprintln!("{} Copilot CLI not found in PATH.", "DX".cyan().bold());
    let install = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Install GitHub Copilot CLI (copilot) automatically now?")
        .default(true)
        .interact()
        .context("failed to read confirmation")?;
    if !install {
        return Err(anyhow!("copilot CLI missing"));
    }

    // Install order mirrors Copilot CLI README (2026-02): winget -> brew -> npm -> script.
    if cfg!(target_os = "windows") {
        if command_works("winget", &["--version"]).await {
            run_status_inherit("winget", &["install", "GitHub.Copilot"])
                .await
                .context("winget install GitHub.Copilot failed")?;
        } else if command_works("npm", &["--version"]).await {
            run_status_inherit("npm", &["install", "-g", "@github/copilot"])
                .await
                .context("npm install -g @github/copilot failed")?;
        } else {
            return Err(anyhow!(
                "cannot auto-install Copilot CLI: need winget or npm (see https://github.com/github/copilot-cli)"
            ));
        }
    } else {
        if command_works("brew", &["--version"]).await {
            run_status_inherit("brew", &["install", "copilot-cli"])
                .await
                .context("brew install copilot-cli failed")?;
        } else if command_works("npm", &["--version"]).await {
            run_status_inherit("npm", &["install", "-g", "@github/copilot"])
                .await
                .context("npm install -g @github/copilot failed")?;
        } else if command_works("bash", &["--version"]).await
            && command_works("curl", &["--version"]).await
        {
            // Last resort: official install script.
            // Note: uses a shell pipeline because that's how the official script is distributed.
            run_status_inherit("bash", &["-lc", "curl -fsSL https://gh.io/copilot-install | bash"])
                .await
                .context("copilot install script failed")?;
        } else {
            return Err(anyhow!(
                "cannot auto-install Copilot CLI: need brew, npm, or bash+curl (see https://github.com/github/copilot-cli)"
            ));
        }
    }

    if command_works("copilot", &["version"]).await
        || command_works("copilot", &["--version"]).await
    {
        Ok(())
    } else {
        Err(anyhow!("copilot CLI install completed but `copilot` is still not in PATH"))
    }
}

async fn ensure_gh_installed_interactive() -> Result<()> {
    if command_works("gh", &["--version"]).await {
        return Ok(());
    }

    if !cfg!(target_os = "windows") {
        // Keep scope tight: we only auto-install gh on Windows for now.
        return Err(anyhow!("GitHub CLI (gh) not found"));
    }

    eprintln!(
        "{} GitHub CLI (gh) not found; DX can install it to enable seamless Copilot token setup.",
        "DX".cyan().bold()
    );
    let install = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Install GitHub CLI (gh) automatically now? (recommended)")
        .default(true)
        .interact()
        .context("failed to read confirmation")?;
    if !install {
        return Err(anyhow!("gh CLI missing"));
    }

    if command_works("winget", &["--version"]).await {
        run_status_inherit("winget", &["install", "GitHub.cli"])
            .await
            .context("winget install GitHub.cli failed")?;
    } else {
        return Err(anyhow!("cannot auto-install gh: winget not found"));
    }

    if command_works("gh", &["--version"]).await {
        Ok(())
    } else {
        Err(anyhow!("gh install completed but `gh` is still not in PATH"))
    }
}

async fn ensure_github_token_interactive() -> Result<(String, String)> {
    // Prefer Copilot CLI’s documented token env var, then GH_TOKEN.
    //
    // Note: we intentionally do NOT prefer `GITHUB_TOKEN` here.
    // In many developer setups (especially VS Code / extensions), `GITHUB_TOKEN` may be present
    // but not suitable for Copilot token exchange (wrong scopes, stale, or not a real user token).
    // DX defaults to an HTTPS-backed GitHub auth source (`gh auth token` / `gh auth login`).
    if let Ok(value) = env::var("COPILOT_GITHUB_TOKEN") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok((trimmed.to_string(), "env:COPILOT_GITHUB_TOKEN".to_string()));
        }
    }
    if let Ok(value) = env::var("GH_TOKEN") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok((trimmed.to_string(), "env:GH_TOKEN".to_string()));
        }
    }

    // Best UX: use GitHub CLI device/web login if available.
    if ensure_gh_installed_interactive().await.is_ok() {
        if let Ok(token) = run_capture_stdout("gh", &["auth", "token", "-h", "github.com"]).await {
            if !token.trim().is_empty() {
                return Ok((token, "gh:auth-token".to_string()));
            }
        }

        eprintln!("{} Opening GitHub login flow (one-time)…", "DX".cyan().bold());
        // Use `--web` so gh opens the browser automatically.
        // Inherit stdio so the user can interact normally.
        run_status_inherit(
            "gh",
            &["auth", "login", "--web", "-h", "github.com", "--git-protocol", "https"],
        )
            .await
            .context("gh auth login failed")?;

        let token = run_capture_stdout("gh", &["auth", "token", "-h", "github.com"])
            .await
            .context("gh auth token failed after login")?;
        if !token.trim().is_empty() {
            return Ok((token, "gh:auth-token".to_string()));
        }
    }

    // Last-resort env fallback.
    // If the user explicitly exported a token, we still allow it here.
    if let Ok(value) = env::var("GITHUB_TOKEN") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok((trimmed.to_string(), "env:GITHUB_TOKEN".to_string()));
        }
    }

    // Fallback: prompt user for a fine-grained PAT.
    eprintln!(
        "{} Paste a fine-grained GitHub PAT with the 'Copilot Requests' permission.",
        "DX".cyan().bold()
    );
    eprintln!("│ Create one at: https://github.com/settings/personal-access-tokens/new");
    let token = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("GitHub token")
        .allow_empty_password(false)
        .interact()
        .context("failed to read token input")?;
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("missing GitHub token"));
    }
    Ok((trimmed.to_string(), "prompt:pat".to_string()))
}

async fn sdk_is_authenticated(github_token: Option<String>) -> Result<bool> {
    // The SDK talks to Copilot CLI in server mode via JSON-RPC.
    // We set `cli_path` to `copilot` (must be in PATH).
    let client = CopilotClient::new(CopilotClientOptions {
        cli_path: Some("copilot".to_string()),
        github_token,
        auto_start: false,
        auto_restart: false,
        ..Default::default()
    });

    client.start().await.map_err(|err| anyhow!("copilot sdk start failed: {err}"))?;
    let status = client
        .get_auth_status()
        .await
        .map_err(|err| anyhow!("copilot sdk get_auth_status failed: {err}"))?;
    let _ = client.stop().await;
    Ok(status.is_authenticated)
}

#[derive(Debug, Deserialize)]
struct CopilotTokenResponse {
    token: Option<String>,
    expires_at: Option<serde_json::Value>,
}

fn parse_expires_at_ms(value: &serde_json::Value) -> Option<u64> {
    match value {
        serde_json::Value::Number(num) => {
            num.as_u64().map(|n| if n > 10_000_000_000 { n } else { n * 1000 })
        }
        serde_json::Value::String(s) => s
            .trim()
            .parse::<u64>()
            .ok()
            .map(|n| if n > 10_000_000_000 { n } else { n * 1000 }),
        _ => None,
    }
}

async fn fetch_copilot_service_token(github_token: &str) -> Result<(String, u64)> {
    let client = reqwest::Client::new();
    let resp = client
        .get(COPILOT_TOKEN_URL)
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {github_token}"))
        .send()
        .await
        .context("failed to call Copilot token endpoint")?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!(CopilotTokenExchangeError { status, body }));
    }

    let payload: CopilotTokenResponse =
        resp.json().await.context("failed to parse Copilot token response")?;
    let token = payload
        .token
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| anyhow!("Copilot token response missing token"))?;
    let expires_raw = payload
        .expires_at
        .ok_or_else(|| anyhow!("Copilot token response missing expires_at"))?;
    let expires_at_ms = parse_expires_at_ms(&expires_raw)
        .ok_or_else(|| anyhow!("Copilot token response has invalid expires_at"))?;
    Ok((token, expires_at_ms))
}

#[derive(Debug)]
struct CopilotTokenExchangeError {
    status: u16,
    body: String,
}

impl std::fmt::Display for CopilotTokenExchangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Copilot token exchange failed (HTTP {}): {}", self.status, self.body.trim())
    }
}

impl std::error::Error for CopilotTokenExchangeError {}

fn read_cached_token(path: &Path) -> Option<CachedCopilotToken> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<CachedCopilotToken>(&content).ok()
}

fn write_cached_token(path: &Path, token: &CachedCopilotToken) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create cache dir {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(token).context("failed to serialize token cache")?;
    std::fs::write(path, content)
        .with_context(|| format!("failed to write token cache {}", path.display()))?;
    Ok(())
}

/// Ensures GitHub Copilot is usable for DX's Copilot provider.
///
/// What it does:
/// - Ensures `copilot` is installed (winget/brew/npm/script)
/// - Ensures a GitHub auth token is available (env or `gh auth login`)
/// - Exchanges the GitHub token for a Copilot service token (`/copilot_internal/v2/token`)
/// - Sets `GITHUB_COPILOT_TOKEN` + `GITHUB_COPILOT_BASE_URL`
///
/// Note: Copilot service tokens are short-lived. DX does **not** reuse a cached token by
/// default (to avoid “works once, fails next run” access denied issues). If you want
/// to enable caching anyway, set `DX_COPILOT_USE_TOKEN_CACHE=1`.
pub async fn ensure_github_copilot_ready_interactive() -> Result<CopilotBootstrapResult> {
    // If a Copilot *service token* is already present in the environment, prefer it.
    // This covers cases where the user has authenticated via HTTPS elsewhere and DX
    // should not force an extra `gh auth login` flow.
    if let Ok(existing) = env::var("GITHUB_COPILOT_TOKEN") {
        let trimmed = existing.trim();
        // Heuristic: Copilot service tokens are semicolon-delimited key/value pairs.
        let looks_like_service_token = trimmed.contains(';') && trimmed.len() > 80;
        if looks_like_service_token {
            let base_url = env::var("GITHUB_COPILOT_BASE_URL").ok().filter(|v| !v.trim().is_empty()).or_else(|| {
                derive_copilot_api_base_url_from_token(trimmed)
            }).unwrap_or_else(|| DEFAULT_COPILOT_API_BASE_URL.to_string());
            unsafe {
                env::set_var("GITHUB_COPILOT_BASE_URL", &base_url);
            }

            return Ok(CopilotBootstrapResult {
                token: trimmed.to_string(),
                expires_at_ms: 0,
                source: "env:GITHUB_COPILOT_TOKEN".to_string(),
                base_url,
            });
        }
    }

    // By default we always refresh, because Copilot service tokens are short-lived.
    // Opt-in behavior to trust an existing env token (useful for debugging):
    //   DX_COPILOT_TRUST_ENV_TOKEN=1
    if should_trust_env_token() {
        if let Ok(existing) = env::var("GITHUB_COPILOT_TOKEN") {
            let trimmed = existing.trim();
            if !trimmed.is_empty() {
                let base_url = env::var("GITHUB_COPILOT_BASE_URL")
                    .unwrap_or_else(|_| DEFAULT_COPILOT_API_BASE_URL.to_string());
                return Ok(CopilotBootstrapResult {
                    token: trimmed.to_string(),
                    expires_at_ms: 0,
                    source: "env:GITHUB_COPILOT_TOKEN".to_string(),
                    base_url,
                });
            }
        }
    }

    ensure_copilot_cli_installed_interactive().await?;

    // Optional cache (disabled by default).
    let cache_path = copilot_token_cache_path();
    if should_use_token_cache() {
        if let Some(cache) = read_cached_token(&cache_path) {
            let now = now_ms();
            if !cache.token.trim().is_empty() && is_token_usable(&cache, now) {
                let base_url = derive_copilot_api_base_url_from_token(&cache.token)
                    .unwrap_or_else(|| DEFAULT_COPILOT_API_BASE_URL.to_string());
                unsafe {
                    env::set_var("GITHUB_COPILOT_TOKEN", cache.token.trim());
                    env::set_var("GITHUB_COPILOT_BASE_URL", &base_url);
                }
                return Ok(CopilotBootstrapResult {
                    token: cache.token,
                    expires_at_ms: cache.expires_at_ms,
                    source: format!("cache:{}", cache_path.display()),
                    base_url,
                });
            }
        }
    }

    let (github_token, github_token_source) = ensure_github_token_interactive().await?;

    // Check auth status via SDK; if not logged in, drive `copilot login`.
    // This matches the “DX just enabled Copilot for me” UX.
    if command_works("copilot", &["login", "--help"]).await {
        let authenticated_before =
            sdk_is_authenticated(Some(github_token.clone())).await.unwrap_or(false);
        if !authenticated_before {
            eprintln!("{} Copilot CLI not logged in; starting login flow…", "DX".cyan().bold());
            let _ = run_status_inherit("copilot", &["login"]).await;
        }

        // Best-effort re-check (don’t hard-fail here; token exchange below is authoritative).
        let _ = sdk_is_authenticated(Some(github_token.clone())).await;
    }

    let (service_token, expires_at_ms) = match fetch_copilot_service_token(&github_token).await {
        Ok(value) => value,
        Err(err) => {
            let status = err.downcast_ref::<CopilotTokenExchangeError>().map(|value| value.status);

            // Common “Access denied” case: GH token is stale / missing Copilot permissions.
            // If we can, drive an interactive re-login and retry once.
            if matches!(status, Some(401 | 403)) && ensure_gh_installed_interactive().await.is_ok()
            {
                eprintln!(
                    "{} Copilot token exchange was denied (HTTP {}). Re-authenticating GitHub and retrying…",
                    "DX".cyan().bold(),
                    status.unwrap_or_default()
                );
                let _ = run_status_inherit(
                    "gh",
                    &["auth", "login", "--web", "-h", "github.com", "--git-protocol", "https"],
                )
                .await;
                if let Ok(new_token) =
                    run_capture_stdout("gh", &["auth", "token", "-h", "github.com"]).await
                {
                    if !new_token.trim().is_empty() {
                        fetch_copilot_service_token(&new_token).await.with_context(|| {
                            format!("token exchange via {COPILOT_TOKEN_URL} failed after re-login")
                        })?
                    } else {
                        return Err(err).with_context(|| {
                            format!("token exchange via {COPILOT_TOKEN_URL} failed")
                        });
                    }
                } else {
                    return Err(err)
                        .with_context(|| format!("token exchange via {COPILOT_TOKEN_URL} failed"));
                }
            } else {
                return Err(err)
                    .with_context(|| format!("token exchange via {COPILOT_TOKEN_URL} failed"));
            }
        }
    };

    let base_url = derive_copilot_api_base_url_from_token(&service_token)
        .unwrap_or_else(|| DEFAULT_COPILOT_API_BASE_URL.to_string());

    if should_use_token_cache() {
        let cache = CachedCopilotToken {
            token: service_token.clone(),
            expires_at_ms,
            updated_at_ms: now_ms(),
        };
        let _ = write_cached_token(&cache_path, &cache);
    }

    unsafe {
        env::set_var("GITHUB_COPILOT_TOKEN", service_token.trim());
        env::set_var("GITHUB_COPILOT_BASE_URL", &base_url);
    }

    Ok(CopilotBootstrapResult {
        token: service_token,
        expires_at_ms,
        source: format!("fetched:{COPILOT_TOKEN_URL} (github_token={github_token_source})"),
        base_url,
    })
}
