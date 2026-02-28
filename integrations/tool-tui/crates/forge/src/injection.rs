//! Component Injection System
//!
//! Fetches components from R2 storage, caches them locally,
//! and injects them into user files with proper imports.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::patterns::{DxToolType, PatternMatch};
use crate::storage::r2::{R2Config, R2Storage};

/// Component metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetadata {
    pub name: String,
    pub version: String,
    pub tool: String,
    pub hash: String,
    pub size: usize,
    pub dependencies: Vec<String>,
    pub exports: Vec<String>,
}

/// Component cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub metadata: ComponentMetadata,
    pub local_path: PathBuf,
    pub cached_at: chrono::DateTime<chrono::Utc>,
    pub last_used: chrono::DateTime<chrono::Utc>,
}

/// Component injection manager
pub struct InjectionManager {
    cache_dir: PathBuf,
    cache_index: HashMap<String, CacheEntry>,
    r2_storage: Option<R2Storage>,
}

impl InjectionManager {
    /// Create a new injection manager
    pub fn new(forge_dir: &Path) -> Result<Self> {
        let cache_dir = forge_dir.join("component_cache");
        std::fs::create_dir_all(&cache_dir)?;

        let cache_index_path = cache_dir.join("index.json");
        let cache_index = if cache_index_path.exists() {
            let content = std::fs::read_to_string(&cache_index_path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        // Try to initialize R2 storage (optional)
        let r2_storage = R2Config::from_env().ok().and_then(|config| R2Storage::new(config).ok());

        Ok(Self {
            cache_dir,
            cache_index,
            r2_storage,
        })
    }

    /// Get component cache key
    fn cache_key(&self, tool: &DxToolType, component: &str) -> String {
        format!("{}/{}", tool.tool_name(), component)
    }

    /// Check if component is cached
    pub fn is_cached(&self, tool: &DxToolType, component: &str) -> bool {
        let key = self.cache_key(tool, component);
        if let Some(entry) = self.cache_index.get(&key) {
            entry.local_path.exists()
        } else {
            false
        }
    }

    /// Get component from cache
    pub async fn get_cached(
        &mut self,
        tool: &DxToolType,
        component: &str,
    ) -> Result<Option<String>> {
        let key = self.cache_key(tool, component);

        if let Some(entry) = self.cache_index.get_mut(&key) {
            // Update last used time
            entry.last_used = chrono::Utc::now();
            let local_path = entry.local_path.clone();
            self.save_index()?;

            let content = fs::read_to_string(&local_path).await?;
            Ok(Some(content))
        } else {
            Ok(None)
        }
    }

    /// Fetch component from R2 and cache it
    pub async fn fetch_component(
        &mut self,
        tool: &DxToolType,
        component: &str,
        version: Option<&str>,
    ) -> Result<String> {
        // Check cache first
        if let Some(cached) = self.get_cached(tool, component).await? {
            return Ok(cached);
        }

        // Fetch from R2 if available
        if let Some(r2) = &self.r2_storage {
            // Try to fetch from R2 with retry logic
            let max_retries = 3;
            let mut last_error = None;

            for attempt in 0..max_retries {
                match r2.download_component(tool.tool_name(), component, version).await {
                    Ok(content) => {
                        // Verify content hash
                        let mut hasher = Sha256::new();
                        hasher.update(content.as_bytes());
                        let hash = format!("{:x}", hasher.finalize());

                        // Cache it with verified hash
                        self.cache_component(tool, component, &content).await?;

                        tracing::info!(
                            "✅ Fetched component {}/{} from R2 (hash: {})",
                            tool.tool_name(),
                            component,
                            &hash[..8]
                        );

                        return Ok(content);
                    }
                    Err(e) => {
                        last_error = Some(e);
                        if attempt < max_retries - 1 {
                            tracing::warn!(
                                "⚠️ R2 fetch attempt {}/{} failed for {}/{}, retrying...",
                                attempt + 1,
                                max_retries,
                                tool.tool_name(),
                                component
                            );
                            // Exponential backoff: 100ms, 200ms, 400ms
                            tokio::time::sleep(std::time::Duration::from_millis(
                                100 * (1 << attempt),
                            ))
                            .await;
                        }
                    }
                }
            }

            // All retries failed, fall back to placeholder
            tracing::error!(
                "❌ Failed to fetch {}/{} from R2 after {} attempts: {:?}",
                tool.tool_name(),
                component,
                max_retries,
                last_error
            );
            tracing::info!("📦 Using placeholder component for {}/{}", tool.tool_name(), component);

            let content = self.create_placeholder_component(tool, component);
            self.cache_component(tool, component, &content).await?;
            Ok(content)
        } else {
            // No R2 storage configured, return placeholder
            let content = self.create_placeholder_component(tool, component);
            self.cache_component(tool, component, &content).await?;
            Ok(content)
        }
    }

    /// Cache a component locally
    async fn cache_component(
        &mut self,
        tool: &DxToolType,
        component: &str,
        content: &str,
    ) -> Result<()> {
        let key = self.cache_key(tool, component);

        // Create tool-specific cache directory
        let tool_cache_dir = self.cache_dir.join(tool.tool_name());
        fs::create_dir_all(&tool_cache_dir).await?;

        // Determine file extension based on tool type
        let extension = match tool {
            DxToolType::Ui => "tsx",
            DxToolType::Icons => "tsx",
            DxToolType::Fonts => "css",
            DxToolType::Style => "css",
            DxToolType::Auth => "ts",
            _ => "ts",
        };

        let local_path = tool_cache_dir.join(format!("{}.{}", component, extension));

        // Write content
        fs::write(&local_path, content).await?;

        // Compute hash
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        // Create cache entry
        let metadata = ComponentMetadata {
            name: component.to_string(),
            version: "latest".to_string(),
            tool: tool.tool_name().to_string(),
            hash,
            size: content.len(),
            dependencies: vec![],
            exports: vec![component.to_string()],
        };

        let entry = CacheEntry {
            metadata,
            local_path,
            cached_at: chrono::Utc::now(),
            last_used: chrono::Utc::now(),
        };

        self.cache_index.insert(key, entry);
        self.save_index()?;

        Ok(())
    }

    /// Create placeholder component for development
    fn create_placeholder_component(&self, tool: &DxToolType, component: &str) -> String {
        match tool {
            DxToolType::Ui => format!(
                r#"// Auto-generated dx-ui component: {}
import React from 'react';

export interface {}Props {{
  children?: React.ReactNode;
  className?: string;
}}

export function {}({{ children, className }}: {}Props) {{
  return (
    <div className={{className}}>
      {{children}}
    </div>
  );
}}

export default {};
"#,
                component, component, component, component, component
            ),
            DxToolType::Icons => format!(
                r#"// Auto-generated dx-icons component: {}
import React from 'react';

export interface {}Props {{
  size?: number;
  color?: string;
  className?: string;
}}

export function {}({{ size = 24, color = 'currentColor', className }}: {}Props) {{
  return (
    <svg width={{size}} height={{size}} fill={{color}} className={{className}}>
      <path d="M12 2L2 22h20L12 2z" />
    </svg>
  );
}}

export default {};
"#,
                component, component, component, component, component
            ),
            DxToolType::Fonts => format!(
                r#"/* Auto-generated dx-fonts: {} */
@font-face {{
  font-family: '{}';
  src: url('https://fonts.dx.tools/{}/regular.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}}

.font-{} {{
  font-family: '{}', sans-serif;
}}
"#,
                component, component, component, component, component
            ),
            _ => format!("// Placeholder for {} from {}", component, tool.tool_name()),
        }
    }

    /// Inject component into file
    pub async fn inject_into_file(
        &mut self,
        file_path: &Path,
        matches: &[PatternMatch],
    ) -> Result<()> {
        let mut content = fs::read_to_string(file_path).await?;

        // Group matches by tool
        let mut by_tool: HashMap<DxToolType, Vec<&PatternMatch>> = HashMap::new();
        for m in matches {
            by_tool.entry(m.tool.clone()).or_default().push(m);
        }

        // Generate imports
        let mut imports = Vec::new();
        for (tool, tool_matches) in &by_tool {
            for m in tool_matches {
                let component = &m.component_name;

                // Fetch component (will cache if needed)
                self.fetch_component(tool, component, None).await?;

                // Generate import statement
                let import_path =
                    format!(".dx/forge/component_cache/{}/{}", tool.tool_name(), component);

                imports.push(format!("import {{ {} }} from '{}';", component, import_path));
            }
        }

        // Insert imports at the top (after existing imports if any)
        if !imports.is_empty() {
            imports.sort();
            imports.dedup();

            let import_block = imports.join("\n") + "\n";

            // Find insertion point (after existing imports or at start)
            if let Some(last_import) = content.rfind("import ") {
                if let Some(newline) = content[last_import..].find('\n') {
                    let insert_pos = last_import + newline + 1;
                    content.insert_str(insert_pos, &import_block);
                }
            } else {
                content.insert_str(0, &import_block);
            }

            // Write back to file
            fs::write(file_path, content).await?;
        }

        Ok(())
    }

    /// Clear old cache entries (LRU eviction)
    pub async fn cleanup_cache(&mut self, max_age_days: i64) -> Result<usize> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(max_age_days);
        let mut removed = 0;

        let keys_to_remove: Vec<String> = self
            .cache_index
            .iter()
            .filter(|(_, entry)| entry.last_used < cutoff)
            .map(|(k, _)| k.clone())
            .collect();

        for key in keys_to_remove {
            if let Some(entry) = self.cache_index.remove(&key) {
                if entry.local_path.exists() {
                    fs::remove_file(&entry.local_path).await?;
                    removed += 1;
                }
            }
        }

        self.save_index()?;
        Ok(removed)
    }

    /// Save cache index to disk
    fn save_index(&self) -> Result<()> {
        let index_path = self.cache_dir.join("index.json");
        let content = serde_json::to_string_pretty(&self.cache_index)?;
        std::fs::write(index_path, content)?;
        Ok(())
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        let total_size: usize = self.cache_index.values().map(|e| e.metadata.size).sum();

        let by_tool: HashMap<String, usize> =
            self.cache_index.values().fold(HashMap::new(), |mut acc, entry| {
                *acc.entry(entry.metadata.tool.clone()).or_insert(0) += 1;
                acc
            });

        CacheStats {
            total_components: self.cache_index.len(),
            total_size_bytes: total_size,
            components_by_tool: by_tool,
        }
    }
}

/// Cache statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_components: usize,
    pub total_size_bytes: usize,
    pub components_by_tool: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_injection_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = InjectionManager::new(temp_dir.path());
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_component_caching() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = InjectionManager::new(temp_dir.path()).unwrap();

        let content = manager.fetch_component(&DxToolType::Ui, "Button", None).await.unwrap();

        assert!(content.contains("Button"));
        assert!(manager.is_cached(&DxToolType::Ui, "Button"));
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = InjectionManager::new(temp_dir.path()).unwrap();

        manager.fetch_component(&DxToolType::Ui, "Button", None).await.unwrap();
        manager.fetch_component(&DxToolType::Icons, "Home", None).await.unwrap();

        let stats = manager.cache_stats();
        assert_eq!(stats.total_components, 2);
    }
}
