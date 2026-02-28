use crate::llm::error::ProviderError;
use crate::llm::presets::openai_compatible_provider_presets;
use crate::llm::types::ProviderMetadata;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProviderConfigFile {
    pub default_provider: Option<String>,
    pub providers: BTreeMap<String, ProviderConfigEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProviderConfigEntry {
    pub enabled: bool,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub custom_headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ProviderMetadata>,
    #[serde(default)]
    pub active_profile: Option<String>,
    #[serde(default)]
    pub profiles: BTreeMap<String, ProviderProfileEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProviderProfileEntry {
    pub enabled: bool,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub custom_headers: BTreeMap<String, String>,
}

impl ProviderConfigFile {
    pub fn default_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".dx").join("config").join("providers.toml")
    }

    pub fn legacy_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".dx").join("providers.toml")
    }

    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    pub fn to_toml_pretty(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn from_presets(default_provider: Option<String>) -> Self {
        let mut providers = BTreeMap::new();

        for preset in openai_compatible_provider_presets() {
            providers.insert(
                preset.id.to_string(),
                ProviderConfigEntry {
                    enabled: true,
                    base_url: preset.base_url.to_string(),
                    api_key_env: if preset.api_key_env.is_empty() {
                        None
                    } else {
                        Some(preset.api_key_env.to_string())
                    },
                    custom_headers: BTreeMap::new(),
                    metadata: Some(preset.metadata()),
                    active_profile: Some("default".to_string()),
                    profiles: {
                        let mut profiles = BTreeMap::new();
                        profiles.insert(
                            "default".to_string(),
                            ProviderProfileEntry {
                                enabled: true,
                                base_url: preset.base_url.to_string(),
                                api_key_env: if preset.api_key_env.is_empty() {
                                    None
                                } else {
                                    Some(preset.api_key_env.to_string())
                                },
                                custom_headers: BTreeMap::new(),
                            },
                        );
                        profiles
                    },
                },
            );
        }

        Self {
            default_provider,
            providers,
        }
    }

    pub fn load_with_migration(default_provider: Option<String>) -> Result<Self, ProviderError> {
        let default_path = Self::default_path();
        if default_path.exists() {
            return Self::load_from_path(default_path);
        }

        let legacy_path = Self::legacy_path();
        if legacy_path.exists() {
            let migrated = Self::load_from_path(legacy_path)?;
            let _ = migrated.save_to_default_path();
            return Ok(migrated);
        }

        Ok(Self::from_presets(default_provider))
    }

    pub fn load_from_path(path: PathBuf) -> Result<Self, ProviderError> {
        let content = fs::read_to_string(&path).map_err(|err| ProviderError::InvalidConfig {
            provider: "config".to_string(),
            detail: format!("failed reading {}: {err}", path.display()),
        })?;

        let parsed: ProviderConfigFile =
            toml::from_str(&content).map_err(|err| ProviderError::InvalidConfig {
                provider: "config".to_string(),
                detail: format!("failed parsing {}: {err}", path.display()),
            })?;

        Ok(parsed)
    }

    pub fn save_to_default_path(&self) -> Result<PathBuf, ProviderError> {
        let path = Self::default_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| ProviderError::InvalidConfig {
                provider: "config".to_string(),
                detail: format!("failed creating {}: {err}", parent.display()),
            })?;
        }

        let content = self.to_toml_pretty().map_err(|err| ProviderError::InvalidConfig {
            provider: "config".to_string(),
            detail: format!("failed serializing provider config: {err}"),
        })?;

        fs::write(&path, content).map_err(|err| ProviderError::InvalidConfig {
            provider: "config".to_string(),
            detail: format!("failed writing {}: {err}", path.display()),
        })?;

        Ok(path)
    }

    pub fn validate(&self) -> Result<(), ProviderError> {
        if let Some(default_provider) = &self.default_provider
            && !self.providers.contains_key(default_provider)
        {
            return Err(ProviderError::InvalidConfig {
                provider: "config".to_string(),
                detail: format!("default provider `{default_provider}` does not exist"),
            });
        }

        for (provider_id, entry) in &self.providers {
            if !entry.enabled {
                continue;
            }

            if entry.base_url.trim().is_empty() {
                return Err(ProviderError::InvalidConfig {
                    provider: provider_id.clone(),
                    detail: "enabled provider is missing base_url".to_string(),
                });
            }

            let normalized = entry.base_url.to_ascii_lowercase();
            if !normalized.starts_with("http://") && !normalized.starts_with("https://") {
                return Err(ProviderError::InvalidConfig {
                    provider: provider_id.clone(),
                    detail: "base_url must start with http:// or https://".to_string(),
                });
            }

            if let Some(api_key_env) = &entry.api_key_env {
                let value = api_key_env.trim();
                if value.is_empty() {
                    return Err(ProviderError::InvalidConfig {
                        provider: provider_id.clone(),
                        detail: "api_key_env cannot be empty when set".to_string(),
                    });
                }

                if !value
                    .chars()
                    .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
                {
                    return Err(ProviderError::InvalidConfig {
                        provider: provider_id.clone(),
                        detail: "api_key_env must be uppercase snake case".to_string(),
                    });
                }
            }

            if let Some(active_profile) = &entry.active_profile
                && !entry.profiles.is_empty()
                && !entry.profiles.contains_key(active_profile)
            {
                return Err(ProviderError::InvalidConfig {
                    provider: provider_id.clone(),
                    detail: format!("active profile `{active_profile}` does not exist"),
                });
            }

            for (profile_name, profile) in &entry.profiles {
                if !profile.enabled {
                    continue;
                }

                if profile.base_url.trim().is_empty() {
                    return Err(ProviderError::InvalidConfig {
                        provider: provider_id.clone(),
                        detail: format!("profile `{profile_name}` is missing base_url"),
                    });
                }

                let normalized = profile.base_url.to_ascii_lowercase();
                if !normalized.starts_with("http://") && !normalized.starts_with("https://") {
                    return Err(ProviderError::InvalidConfig {
                        provider: provider_id.clone(),
                        detail: format!(
                            "profile `{profile_name}` base_url must start with http:// or https://"
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    pub fn apply_provider_selection(
        &mut self,
        selected_provider_ids: &[String],
        default_provider: Option<String>,
    ) {
        for entry in self.providers.values_mut() {
            entry.enabled = false;
        }

        for provider_id in selected_provider_ids {
            let generated_env = format!("{}_API_KEY", provider_id.to_ascii_uppercase());
            let guessed_base_url = format!(
                "https://api.{}.com/v1",
                provider_id.replace('_', "-").to_ascii_lowercase()
            );

            let entry =
                self.providers
                    .entry(provider_id.clone())
                    .or_insert_with(|| ProviderConfigEntry {
                        enabled: true,
                        base_url: guessed_base_url,
                        api_key_env: Some(generated_env),
                        custom_headers: BTreeMap::new(),
                        metadata: None,
                        active_profile: Some("default".to_string()),
                        profiles: BTreeMap::new(),
                    });

            entry.enabled = true;
            if entry.base_url.trim().is_empty() {
                entry.base_url = format!(
                    "https://api.{}.com/v1",
                    provider_id.replace('_', "-").to_ascii_lowercase()
                );
            }

            let default_profile =
                entry.profiles.entry("default".to_string()).or_insert_with(|| {
                    ProviderProfileEntry {
                        enabled: true,
                        base_url: entry.base_url.clone(),
                        api_key_env: entry.api_key_env.clone(),
                        custom_headers: entry.custom_headers.clone(),
                    }
                });

            default_profile.enabled = true;
            if default_profile.base_url.trim().is_empty() {
                default_profile.base_url = entry.base_url.clone();
            }
            if default_profile.api_key_env.is_none() {
                default_profile.api_key_env = entry.api_key_env.clone();
            }

            if entry.active_profile.is_none() {
                entry.active_profile = Some("default".to_string());
            }
        }

        if let Some(provider) = default_provider {
            self.default_provider = Some(provider);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrip_toml() {
        let mut providers = BTreeMap::new();
        providers.insert(
            "groq".to_string(),
            ProviderConfigEntry {
                enabled: true,
                base_url: "https://api.groq.com/openai".to_string(),
                api_key_env: Some("GROQ_API_KEY".to_string()),
                custom_headers: BTreeMap::new(),
                metadata: None,
                active_profile: Some("default".to_string()),
                profiles: {
                    let mut profiles = BTreeMap::new();
                    profiles.insert(
                        "default".to_string(),
                        ProviderProfileEntry {
                            enabled: true,
                            base_url: "https://api.groq.com/openai".to_string(),
                            api_key_env: Some("GROQ_API_KEY".to_string()),
                            custom_headers: BTreeMap::new(),
                        },
                    );
                    profiles
                },
            },
        );

        let cfg = ProviderConfigFile {
            default_provider: Some("groq".to_string()),
            providers,
        };

        let toml = cfg.to_toml_pretty().expect("serialize to toml");
        let parsed = ProviderConfigFile::from_toml(&toml).expect("deserialize from toml");

        assert_eq!(parsed.default_provider.as_deref(), Some("groq"));
        let entry = parsed.providers.get("groq").expect("groq entry");
        assert_eq!(entry.base_url, "https://api.groq.com/openai");
        assert_eq!(entry.api_key_env.as_deref(), Some("GROQ_API_KEY"));
    }

    #[test]
    fn builds_from_presets() {
        let config = ProviderConfigFile::from_presets(Some("openai".to_string()));
        assert!(config.providers.len() >= 35);
        assert!(config.providers.contains_key("openai"));
        assert!(config.providers.contains_key("openrouter"));
    }

    #[test]
    fn validates_enabled_provider_base_url() {
        let mut config = ProviderConfigFile::from_presets(Some("openai".to_string()));
        let openai = config.providers.get_mut("openai").expect("openai provider");
        openai.base_url = "not-a-url".to_string();

        let validation = config.validate();
        assert!(validation.is_err());
    }

    #[test]
    fn apply_provider_selection_enables_only_selected() {
        let mut config = ProviderConfigFile::from_presets(Some("openai".to_string()));
        config.apply_provider_selection(&["groq".to_string()], Some("groq".to_string()));

        let openai_enabled =
            config.providers.get("openai").map(|entry| entry.enabled).unwrap_or(true);
        let groq_enabled = config.providers.get("groq").map(|entry| entry.enabled).unwrap_or(false);

        assert!(!openai_enabled);
        assert!(groq_enabled);
        assert_eq!(config.default_provider.as_deref(), Some("groq"));

        let groq = config.providers.get("groq").expect("groq provider");
        assert_eq!(groq.active_profile.as_deref(), Some("default"));
        assert!(groq.profiles.contains_key("default"));
    }
}
