use anyhow::{Context, Result, anyhow};
use copilot_sdk_supercharged::CopilotClient;
use copilot_sdk_supercharged::types::CopilotClientOptions;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::process::Command;

const KEYRING_SERVICE: &str = "dx-github-copilot";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CopilotAccountId(pub String);

impl CopilotAccountId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("account id cannot be empty"));
        }
        Ok(Self(trimmed.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotAccountProfile {
    pub id: CopilotAccountId,
    pub label: String,
    /// Directory used to isolate Copilot CLI auth/cache for this account.
    ///
    /// DX passes this via `COPILOT_CONFIG_DIR` to the Copilot CLI process.
    pub copilot_config_dir: PathBuf,
    #[serde(default)]
    pub last_used_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CopilotAccountFile {
    version: u32,
    profiles: Vec<CopilotAccountProfile>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis() as u64
}

fn dx_config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("dx")
}

fn default_accounts_path() -> PathBuf {
    dx_config_dir().join("credentials").join("github-copilot.accounts.json")
}

fn default_profile_id(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    for ch in label.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            out.push('-');
        } else if ch.is_whitespace() {
            out.push('-');
        }
    }
    let out = out.trim_matches('-');
    if out.is_empty() {
        "copilot".to_string()
    } else {
        out.to_string()
    }
}

async fn command_works(
    cmd: &str,
    args: &[&str],
    env_overrides: Option<&HashMap<String, String>>,
) -> bool {
    let mut command = Command::new(cmd);
    command.args(args);
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());
    if let Some(env) = env_overrides {
        command.envs(env);
    }
    command.status().await.map(|status| status.success()).unwrap_or(false)
}

/// Stores and manages multiple GitHub Copilot accounts for DX.
///
/// The key idea is **auth isolation**: each account gets its own Copilot CLI config directory.
/// When DX spawns `copilot` via `copilot-sdk-supercharged`, it passes:
/// - `COPILOT_CONFIG_DIR=<per-account-dir>`
///
/// This allows multiple `CopilotClient` instances to run concurrently, each with independent auth.
#[derive(Debug, Clone)]
pub struct CopilotAccountManager {
    accounts_path: PathBuf,
}

impl Default for CopilotAccountManager {
    fn default() -> Self {
        Self::new(default_accounts_path())
    }
}

impl CopilotAccountManager {
    pub fn new(accounts_path: PathBuf) -> Self {
        Self { accounts_path }
    }

    pub fn accounts_path(&self) -> &Path {
        &self.accounts_path
    }

    pub fn default_profile_config_dir(id: &CopilotAccountId) -> PathBuf {
        dx_config_dir().join("credentials").join("copilot").join(&id.0)
    }

    pub fn load(&self) -> Result<Vec<CopilotAccountProfile>> {
        if !self.accounts_path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&self.accounts_path)
            .with_context(|| format!("failed to read {}", self.accounts_path.display()))?;
        let parsed = serde_json::from_str::<CopilotAccountFile>(&content)
            .context("failed to parse github-copilot accounts file")?;
        Ok(parsed.profiles)
    }

    pub fn save(&self, profiles: &[CopilotAccountProfile]) -> Result<()> {
        if let Some(parent) = self.accounts_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let payload = CopilotAccountFile {
            version: 1,
            profiles: profiles.to_vec(),
        };
        let content = serde_json::to_string_pretty(&payload)
            .context("failed to serialize github-copilot accounts file")?;
        std::fs::write(&self.accounts_path, content)
            .with_context(|| format!("failed to write {}", self.accounts_path.display()))?;
        Ok(())
    }

    pub fn upsert_profile(
        &self,
        mut profile: CopilotAccountProfile,
    ) -> Result<CopilotAccountProfile> {
        if profile.label.trim().is_empty() {
            return Err(anyhow!("profile label cannot be empty"));
        }
        if profile.copilot_config_dir.as_os_str().is_empty() {
            return Err(anyhow!("copilot_config_dir cannot be empty"));
        }

        let mut profiles = self.load()?;
        let now = now_ms();
        profile.last_used_at_ms = Some(now);

        let mut replaced = false;
        for existing in &mut profiles {
            if existing.id == profile.id {
                *existing = profile.clone();
                replaced = true;
                break;
            }
        }
        if !replaced {
            profiles.push(profile.clone());
        }
        self.save(&profiles)?;
        Ok(profile)
    }

    pub fn create_profile(&self, label: impl Into<String>) -> Result<CopilotAccountProfile> {
        let label = label.into();
        let id = CopilotAccountId::new(default_profile_id(&label))?;
        let profile = CopilotAccountProfile {
            id: id.clone(),
            label,
            copilot_config_dir: Self::default_profile_config_dir(&id),
            last_used_at_ms: Some(now_ms()),
        };
        self.upsert_profile(profile)
    }

    pub fn delete_profile(&self, id: &CopilotAccountId) -> Result<()> {
        let mut profiles = self.load()?;
        profiles.retain(|p| &p.id != id);
        self.save(&profiles)?;

        let _ = self.delete_github_token(id);
        Ok(())
    }

    pub fn set_github_token(&self, id: &CopilotAccountId, token: &str) -> Result<()> {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("token cannot be empty"));
        }
        let entry = keyring::Entry::new(KEYRING_SERVICE, &id.0)
            .context("failed to open OS keyring entry")?;
        entry.set_password(trimmed).context("failed to store token in OS keyring")?;
        Ok(())
    }

    pub fn get_github_token(&self, id: &CopilotAccountId) -> Result<Option<String>> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, &id.0)
            .context("failed to open OS keyring entry")?;
        match entry.get_password() {
            Ok(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(trimmed.to_string()))
                }
            }
            Err(err) => {
                // Normalize "missing" into None.
                if err.to_string().to_ascii_lowercase().contains("not found") {
                    return Ok(None);
                }
                Err(err).context("failed to read token from OS keyring")
            }
        }
    }

    pub fn delete_github_token(&self, id: &CopilotAccountId) -> Result<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, &id.0)
            .context("failed to open OS keyring entry")?;
        let _ = entry.delete_credential();
        Ok(())
    }

    pub fn options_for_profile(
        profile: &CopilotAccountProfile,
        github_token: Option<String>,
    ) -> CopilotClientOptions {
        let mut env = HashMap::new();
        env.insert(
            "COPILOT_CONFIG_DIR".to_string(),
            profile.copilot_config_dir.to_string_lossy().to_string(),
        );

        CopilotClientOptions {
            cli_path: Some("copilot".to_string()),
            env: Some(env),
            github_token,
            auto_start: false,
            auto_restart: false,
            ..Default::default()
        }
    }

    pub async fn start_client(&self, profile: &CopilotAccountProfile) -> Result<CopilotClient> {
        let github_token = self.get_github_token(&profile.id).unwrap_or(None);
        let client = CopilotClient::new(Self::options_for_profile(profile, github_token));
        client.start().await.map_err(|err| anyhow!("copilot sdk start failed: {err}"))?;
        Ok(client)
    }

    pub async fn start_clients_parallel(
        &self,
        profiles: &[CopilotAccountProfile],
    ) -> Result<Vec<(CopilotAccountId, CopilotClient)>> {
        let mut tasks = Vec::with_capacity(profiles.len());
        for profile in profiles {
            let manager = self.clone();
            let profile = profile.clone();
            tasks.push(tokio::spawn(async move {
                let client = manager.start_client(&profile).await?;
                Ok::<_, anyhow::Error>((profile.id, client))
            }));
        }

        let mut out = Vec::with_capacity(tasks.len());
        for task in tasks {
            let item = task.await.context("failed to join copilot client task")??;
            out.push(item);
        }
        Ok(out)
    }

    pub async fn is_cli_logged_in(&self, profile: &CopilotAccountProfile) -> Result<bool> {
        let mut env = HashMap::new();
        env.insert(
            "COPILOT_CONFIG_DIR".to_string(),
            profile.copilot_config_dir.to_string_lossy().to_string(),
        );

        // `copilot auth status` exists in newer CLIs; fall back to `copilot whoami`.
        if command_works("copilot", &["auth", "status"], Some(&env)).await {
            return Ok(true);
        }

        Ok(command_works("copilot", &["whoami"], Some(&env)).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_accounts_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("dx-onboard-copilot-accounts-{}-{}.json", std::process::id(), now_ms()));
        path
    }

    #[test]
    fn round_trip_accounts_file() {
        let path = temp_accounts_path();
        let manager = CopilotAccountManager::new(path.clone());

        let p1 = CopilotAccountProfile {
            id: CopilotAccountId("work".to_string()),
            label: "Work".to_string(),
            copilot_config_dir: PathBuf::from("/tmp/copilot-work"),
            last_used_at_ms: None,
        };
        let p2 = CopilotAccountProfile {
            id: CopilotAccountId("personal".to_string()),
            label: "Personal".to_string(),
            copilot_config_dir: PathBuf::from("/tmp/copilot-personal"),
            last_used_at_ms: Some(123),
        };

        manager.save(&[p1.clone(), p2.clone()]).expect("save");
        let loaded = manager.load().expect("load");
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id.0, "work");
        assert_eq!(loaded[1].id.0, "personal");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn default_profile_id_sanitizes() {
        assert_eq!(default_profile_id("Work Account"), "work-account");
        assert_eq!(default_profile_id("  "), "copilot");
        assert_eq!(default_profile_id("Personal_1"), "personal-1");
    }
}
