//! Channel Creator
//!
//! Self-expanding system for creating new messaging channel adapters.
//! Detects capability gaps and generates adapter code from SDK documentation.

use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;

use super::{
    EnvironmentConfig, EnvironmentError, EnvironmentResult, Runtime,
    compiler::{CompilationConfig, CompilationPipeline, CompilationResult},
};

/// Specification for a channel adapter
#[derive(Debug, Clone)]
pub struct AdapterSpec {
    /// Unique identifier for the adapter
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Target messaging platform (e.g., "mattermost", "teams", "slack")
    pub platform: String,
    /// SDK documentation URL
    pub sdk_docs_url: Option<String>,
    /// API base URL
    pub api_base_url: String,
    /// Authentication type
    pub auth_type: AuthType,
    /// Required capabilities
    pub capabilities: Vec<AdapterCapability>,
    /// Runtime to use for the adapter
    pub runtime: Runtime,
}

/// Authentication types for adapters
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthType {
    /// No authentication
    None,
    /// API key in header
    ApiKey { header: String },
    /// Bearer token
    Bearer,
    /// OAuth2 flow
    OAuth2 {
        auth_url: String,
        token_url: String,
        scopes: Vec<String>,
    },
    /// Webhook signature verification
    WebhookSecret,
    /// Custom authentication
    Custom { description: String },
}

/// Capabilities that an adapter can provide
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AdapterCapability {
    /// Send messages
    SendMessage = 0,
    /// Receive messages (webhook)
    ReceiveMessage = 1,
    /// Edit messages
    EditMessage = 2,
    /// Delete messages
    DeleteMessage = 3,
    /// React to messages
    AddReaction = 4,
    /// Upload files
    UploadFile = 5,
    /// Download files
    DownloadFile = 6,
    /// Create channels
    CreateChannel = 7,
    /// List channels
    ListChannels = 8,
    /// Get user info
    GetUser = 9,
    /// List users
    ListUsers = 10,
    /// Typing indicator
    TypingIndicator = 11,
    /// Thread replies
    ThreadReply = 12,
    /// Mentions
    Mention = 13,
}

impl std::fmt::Display for AdapterCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterCapability::SendMessage => write!(f, "send_message"),
            AdapterCapability::ReceiveMessage => write!(f, "receive_message"),
            AdapterCapability::EditMessage => write!(f, "edit_message"),
            AdapterCapability::DeleteMessage => write!(f, "delete_message"),
            AdapterCapability::AddReaction => write!(f, "add_reaction"),
            AdapterCapability::UploadFile => write!(f, "upload_file"),
            AdapterCapability::DownloadFile => write!(f, "download_file"),
            AdapterCapability::CreateChannel => write!(f, "create_channel"),
            AdapterCapability::ListChannels => write!(f, "list_channels"),
            AdapterCapability::GetUser => write!(f, "get_user"),
            AdapterCapability::ListUsers => write!(f, "list_users"),
            AdapterCapability::TypingIndicator => write!(f, "typing_indicator"),
            AdapterCapability::ThreadReply => write!(f, "thread_reply"),
            AdapterCapability::Mention => write!(f, "mention"),
        }
    }
}

/// Result of channel generation
#[derive(Debug, Clone)]
pub struct GeneratedChannel {
    /// Adapter specification
    pub spec: AdapterSpec,
    /// Generated source code path
    pub source_path: PathBuf,
    /// Compiled WASM path (if compiled)
    pub wasm_path: Option<PathBuf>,
    /// Generated code language
    pub language: String,
    /// Code generation timestamp
    pub generated_at: u64,
    /// Compilation result (if compiled)
    pub compilation: Option<CompilationResult>,
}

/// Capability gap detection result
#[derive(Debug, Clone)]
pub struct CapabilityGap {
    /// Requested capability
    pub capability: AdapterCapability,
    /// Platform that's missing it
    pub platform: String,
    /// Suggested solution
    pub suggestion: String,
}

/// Channel creator for self-expanding adapter system
pub struct ChannelCreator {
    config: EnvironmentConfig,
    pipeline: CompilationPipeline,
    installed_adapters: RwLock<HashMap<String, GeneratedChannel>>,
    capability_cache: RwLock<HashMap<String, Vec<AdapterCapability>>>,
}

impl ChannelCreator {
    /// Create a new channel creator
    pub fn new(config: EnvironmentConfig) -> Self {
        let pipeline = CompilationPipeline::new(config.clone());
        Self {
            config,
            pipeline,
            installed_adapters: RwLock::new(HashMap::new()),
            capability_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Detect capability gaps for a platform
    pub async fn detect_gaps(
        &self,
        platform: &str,
        required: &[AdapterCapability],
    ) -> Vec<CapabilityGap> {
        let cache = self.capability_cache.read().await;
        let available = cache.get(platform).cloned().unwrap_or_default();

        required
            .iter()
            .filter(|cap| !available.contains(cap))
            .map(|cap| CapabilityGap {
                capability: *cap,
                platform: platform.to_string(),
                suggestion: self.suggest_solution(*cap, platform),
            })
            .collect()
    }

    /// Suggest a solution for a capability gap
    fn suggest_solution(&self, cap: AdapterCapability, platform: &str) -> String {
        match cap {
            AdapterCapability::SendMessage => {
                format!("Implement POST /{}/messages endpoint", platform)
            }
            AdapterCapability::ReceiveMessage => {
                format!("Set up webhook endpoint for {} events", platform)
            }
            AdapterCapability::EditMessage => {
                format!("Implement PATCH /{}/messages/{{id}} endpoint", platform)
            }
            AdapterCapability::UploadFile => {
                format!("Implement multipart file upload for {}", platform)
            }
            _ => format!("Implement {} capability for {}", cap, platform),
        }
    }

    /// Generate adapter code from specification
    pub async fn generate_adapter(
        &self,
        spec: &AdapterSpec,
    ) -> EnvironmentResult<GeneratedChannel> {
        let adapters_dir = self.config.dx_root.join("adapters");
        tokio::fs::create_dir_all(&adapters_dir).await?;

        let source_path =
            adapters_dir.join(format!("{}.{}", spec.id, self.get_extension(spec.runtime)));

        // Generate the adapter code
        let code = self.generate_code(spec).await?;

        // Write to file
        tokio::fs::write(&source_path, &code).await?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let channel = GeneratedChannel {
            spec: spec.clone(),
            source_path,
            wasm_path: None,
            language: self.get_language(spec.runtime),
            generated_at: now,
            compilation: None,
        };

        // Cache the adapter
        let mut adapters = self.installed_adapters.write().await;
        adapters.insert(spec.id.clone(), channel.clone());

        // Update capability cache
        let mut cap_cache = self.capability_cache.write().await;
        cap_cache.insert(spec.platform.clone(), spec.capabilities.clone());

        Ok(channel)
    }

    /// Generate adapter code
    async fn generate_code(&self, spec: &AdapterSpec) -> EnvironmentResult<String> {
        match spec.runtime {
            Runtime::NodeJs | Runtime::Bun | Runtime::Deno => {
                self.generate_javascript_adapter(spec).await
            }
            Runtime::Python => self.generate_python_adapter(spec).await,
            Runtime::Go => self.generate_go_adapter(spec).await,
            Runtime::Rust => self.generate_rust_adapter(spec).await,
        }
    }

    /// Generate JavaScript adapter
    async fn generate_javascript_adapter(&self, spec: &AdapterSpec) -> EnvironmentResult<String> {
        let auth_code = match &spec.auth_type {
            AuthType::None => "// No authentication required".to_string(),
            AuthType::ApiKey { header } => format!(r#"headers['{header}'] = config.apiKey;"#),
            AuthType::Bearer => {
                r#"headers['Authorization'] = `Bearer ${config.token}`;"#.to_string()
            }
            AuthType::OAuth2 { .. } => r#"
// OAuth2 authentication
const token = await refreshOAuthToken(config);
headers['Authorization'] = `Bearer ${token}`;
"#
            .to_string(),
            AuthType::WebhookSecret => "// Webhook signature verification".to_string(),
            AuthType::Custom { description } => format!("// Custom auth: {}", description),
        };

        let capabilities_code: Vec<String> = spec.capabilities.iter().map(|cap| {
            match cap {
                AdapterCapability::SendMessage => format!(r#"
  async sendMessage(channelId, content) {{
    const response = await fetch(`${{this.baseUrl}}/channels/${{channelId}}/messages`, {{
      method: 'POST',
      headers: this.headers,
      body: JSON.stringify({{ content }}),
    }});
    return response.json();
  }}"#),
                AdapterCapability::ReceiveMessage => r#"
  setupWebhook(path, handler) {
    // Webhook setup for receiving messages
    this.webhookHandler = handler;
  }"#.to_string(),
                AdapterCapability::EditMessage => format!(r#"
  async editMessage(channelId, messageId, content) {{
    const response = await fetch(`${{this.baseUrl}}/channels/${{channelId}}/messages/${{messageId}}`, {{
      method: 'PATCH',
      headers: this.headers,
      body: JSON.stringify({{ content }}),
    }});
    return response.json();
  }}"#),
                AdapterCapability::DeleteMessage => format!(r#"
  async deleteMessage(channelId, messageId) {{
    await fetch(`${{this.baseUrl}}/channels/${{channelId}}/messages/${{messageId}}`, {{
      method: 'DELETE',
      headers: this.headers,
    }});
  }}"#),
                AdapterCapability::ListChannels => format!(r#"
  async listChannels() {{
    const response = await fetch(`${{this.baseUrl}}/channels`, {{
      headers: this.headers,
    }});
    return response.json();
  }}"#),
                _ => format!("  // TODO: Implement {} capability", cap),
            }
        }).collect();

        let code = format!(
            r#"/**
 * {name} Adapter for DX
 * Platform: {platform}
 * Generated by DX Channel Creator
 */

class {class_name}Adapter {{
  constructor(config) {{
    this.baseUrl = '{api_base_url}';
    this.config = config;
    this.headers = {{
      'Content-Type': 'application/json',
    }};
    {auth_code}
  }}
{capabilities}
}}

// Export for WASM
export {{ {class_name}Adapter }};
"#,
            name = spec.name,
            platform = spec.platform,
            class_name = to_pascal_case(&spec.id),
            api_base_url = spec.api_base_url,
            auth_code = auth_code,
            capabilities = capabilities_code.join("\n"),
        );

        Ok(code)
    }

    /// Generate Python adapter
    async fn generate_python_adapter(&self, spec: &AdapterSpec) -> EnvironmentResult<String> {
        let auth_code = match &spec.auth_type {
            AuthType::None => "# No authentication required".to_string(),
            AuthType::ApiKey { header } => {
                format!(r#"self.headers['{header}'] = config['api_key']"#)
            }
            AuthType::Bearer => {
                r#"self.headers['Authorization'] = f'Bearer {config["token"]}'"#.to_string()
            }
            _ => "# Custom authentication".to_string(),
        };

        let capabilities_code: Vec<String> = spec
            .capabilities
            .iter()
            .map(|cap| match cap {
                AdapterCapability::SendMessage => r#"
    async def send_message(self, channel_id: str, content: str) -> dict:
        """Send a message to a channel."""
        async with aiohttp.ClientSession() as session:
            async with session.post(
                f'{self.base_url}/channels/{channel_id}/messages',
                headers=self.headers,
                json={'content': content}
            ) as response:
                return await response.json()"#
                    .to_string(),
                AdapterCapability::ListChannels => r#"
    async def list_channels(self) -> list:
        """List all available channels."""
        async with aiohttp.ClientSession() as session:
            async with session.get(
                f'{self.base_url}/channels',
                headers=self.headers
            ) as response:
                return await response.json()"#
                    .to_string(),
                _ => format!("    # TODO: Implement {} capability", cap),
            })
            .collect();

        let code = format!(
            r#"""\"
{name} Adapter for DX
Platform: {platform}
Generated by DX Channel Creator
\"""

import aiohttp
from typing import Optional

class {class_name}Adapter:
    def __init__(self, config: dict):
        self.base_url = '{api_base_url}'
        self.config = config
        self.headers = {{
            'Content-Type': 'application/json',
        }}
        {auth_code}
{capabilities}

# Export for componentize-py
__all__ = ['{class_name}Adapter']
"#,
            name = spec.name,
            platform = spec.platform,
            class_name = to_pascal_case(&spec.id),
            api_base_url = spec.api_base_url,
            auth_code = auth_code,
            capabilities = capabilities_code.join("\n"),
        );

        Ok(code)
    }

    /// Generate Go adapter
    async fn generate_go_adapter(&self, spec: &AdapterSpec) -> EnvironmentResult<String> {
        let code = format!(
            r#"// {name} Adapter for DX
// Platform: {platform}
// Generated by DX Channel Creator

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
)

type {class_name}Adapter struct {{
	baseURL string
	headers map[string]string
	client  *http.Client
}}

func New{class_name}Adapter(config map[string]string) *{class_name}Adapter {{
	return &{class_name}Adapter{{
		baseURL: "{api_base_url}",
		headers: map[string]string{{
			"Content-Type": "application/json",
		}},
		client: &http.Client{{}},
	}}
}}

func (a *{class_name}Adapter) SendMessage(channelID, content string) (map[string]interface{{}}, error) {{
	payload := map[string]string{{"content": content}}
	body, _ := json.Marshal(payload)
	
	req, err := http.NewRequest("POST", 
		fmt.Sprintf("%s/channels/%s/messages", a.baseURL, channelID),
		bytes.NewBuffer(body))
	if err != nil {{
		return nil, err
	}}
	
	for k, v := range a.headers {{
		req.Header.Set(k, v)
	}}
	
	resp, err := a.client.Do(req)
	if err != nil {{
		return nil, err
	}}
	defer resp.Body.Close()
	
	var result map[string]interface{{}}
	json.NewDecoder(resp.Body).Decode(&result)
	return result, nil
}}

func main() {{}}
"#,
            name = spec.name,
            platform = spec.platform,
            class_name = to_pascal_case(&spec.id),
            api_base_url = spec.api_base_url,
        );

        Ok(code)
    }

    /// Generate Rust adapter
    async fn generate_rust_adapter(&self, spec: &AdapterSpec) -> EnvironmentResult<String> {
        let code = format!(
            r#"//! {name} Adapter for DX
//! Platform: {platform}
//! Generated by DX Channel Creator

use serde::{{Deserialize, Serialize}};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct {class_name}Adapter {{
    base_url: String,
    headers: HashMap<String, String>,
    client: reqwest::Client,
}}

#[derive(Debug, Serialize)]
struct SendMessagePayload {{
    content: String,
}}

#[derive(Debug, Deserialize)]
pub struct Message {{
    pub id: String,
    pub content: String,
    pub channel_id: String,
}}

impl {class_name}Adapter {{
    pub fn new(config: HashMap<String, String>) -> Self {{
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        
        Self {{
            base_url: "{api_base_url}".to_string(),
            headers,
            client: reqwest::Client::new(),
        }}
    }}

    pub async fn send_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<Message, reqwest::Error> {{
        let url = format!("{{}}/channels/{{}}/messages", self.base_url, channel_id);
        
        let payload = SendMessagePayload {{
            content: content.to_string(),
        }};

        self.client
            .post(&url)
            .headers(self.build_headers())
            .json(&payload)
            .send()
            .await?
            .json()
            .await
    }}

    fn build_headers(&self) -> reqwest::header::HeaderMap {{
        let mut headers = reqwest::header::HeaderMap::new();
        for (k, v) in &self.headers {{
            if let Ok(name) = reqwest::header::HeaderName::try_from(k.as_str()) {{
                if let Ok(value) = reqwest::header::HeaderValue::from_str(v) {{
                    headers.insert(name, value);
                }}
            }}
        }}
        headers
    }}
}}
"#,
            name = spec.name,
            platform = spec.platform,
            class_name = to_pascal_case(&spec.id),
            api_base_url = spec.api_base_url,
        );

        Ok(code)
    }

    /// Compile a generated adapter to WASM
    pub async fn compile_adapter(
        &mut self,
        adapter_id: &str,
    ) -> EnvironmentResult<CompilationResult> {
        let adapters = self.installed_adapters.read().await;
        let channel =
            adapters
                .get(adapter_id)
                .ok_or_else(|| EnvironmentError::ChannelCreationFailed {
                    reason: format!("Adapter {} not found", adapter_id),
                })?;

        let source_path = channel.source_path.clone();
        let runtime = channel.spec.runtime;
        drop(adapters);

        let config = CompilationConfig::default();
        let result = self.pipeline.compile(&source_path, runtime, config, None).await?;

        // Update the channel with compilation result
        let mut adapters = self.installed_adapters.write().await;
        if let Some(channel) = adapters.get_mut(adapter_id) {
            channel.wasm_path = Some(result.wasm_path.clone());
            channel.compilation = Some(result.clone());
        }

        Ok(result)
    }

    /// Install a compiled adapter as a new channel
    pub async fn install_adapter(&self, adapter_id: &str) -> EnvironmentResult<PathBuf> {
        let adapters = self.installed_adapters.read().await;
        let channel =
            adapters
                .get(adapter_id)
                .ok_or_else(|| EnvironmentError::ChannelCreationFailed {
                    reason: format!("Adapter {} not found", adapter_id),
                })?;

        let wasm_path =
            channel
                .wasm_path
                .as_ref()
                .ok_or_else(|| EnvironmentError::ChannelCreationFailed {
                    reason: "Adapter not compiled".into(),
                })?;

        // Copy to plugins directory
        let plugins_dir = self.config.dx_root.join("plugins").join("channels");
        tokio::fs::create_dir_all(&plugins_dir).await?;

        let install_path = plugins_dir.join(format!("{}.wasm", adapter_id));
        tokio::fs::copy(wasm_path, &install_path).await?;

        Ok(install_path)
    }

    /// Get file extension for runtime
    fn get_extension(&self, runtime: Runtime) -> &'static str {
        match runtime {
            Runtime::NodeJs | Runtime::Bun => "js",
            Runtime::Python => "py",
            Runtime::Go => "go",
            Runtime::Rust => "rs",
            Runtime::Deno => "ts",
        }
    }

    /// Get language name for runtime
    fn get_language(&self, runtime: Runtime) -> String {
        match runtime {
            Runtime::NodeJs | Runtime::Bun => "JavaScript".to_string(),
            Runtime::Python => "Python".to_string(),
            Runtime::Go => "Go".to_string(),
            Runtime::Rust => "Rust".to_string(),
            Runtime::Deno => "TypeScript".to_string(),
        }
    }

    /// Create a Mattermost adapter example
    pub fn mattermost_spec() -> AdapterSpec {
        AdapterSpec {
            id: "mattermost".to_string(),
            name: "Mattermost".to_string(),
            platform: "mattermost".to_string(),
            sdk_docs_url: Some("https://api.mattermost.com/".to_string()),
            api_base_url: "https://your-mattermost-server.com/api/v4".to_string(),
            auth_type: AuthType::Bearer,
            capabilities: vec![
                AdapterCapability::SendMessage,
                AdapterCapability::ReceiveMessage,
                AdapterCapability::EditMessage,
                AdapterCapability::DeleteMessage,
                AdapterCapability::AddReaction,
                AdapterCapability::ListChannels,
                AdapterCapability::GetUser,
                AdapterCapability::ThreadReply,
            ],
            runtime: Runtime::NodeJs,
        }
    }

    /// Create a Microsoft Teams adapter example
    pub fn teams_spec() -> AdapterSpec {
        AdapterSpec {
            id: "teams".to_string(),
            name: "Microsoft Teams".to_string(),
            platform: "teams".to_string(),
            sdk_docs_url: Some(
                "https://docs.microsoft.com/en-us/graph/api/resources/teams-api-overview"
                    .to_string(),
            ),
            api_base_url: "https://graph.microsoft.com/v1.0".to_string(),
            auth_type: AuthType::OAuth2 {
                auth_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
                    .to_string(),
                token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string(),
                scopes: vec![
                    "ChannelMessage.Send".to_string(),
                    "ChannelMessage.Read.All".to_string(),
                ],
            },
            capabilities: vec![
                AdapterCapability::SendMessage,
                AdapterCapability::ReceiveMessage,
                AdapterCapability::ListChannels,
                AdapterCapability::GetUser,
                AdapterCapability::ThreadReply,
                AdapterCapability::Mention,
            ],
            runtime: Runtime::NodeJs,
        }
    }
}

/// Convert string to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| c == '-' || c == '_' || c == ' ')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("hello-world"), "HelloWorld");
        assert_eq!(to_pascal_case("mattermost"), "Mattermost");
        assert_eq!(to_pascal_case("microsoft_teams"), "MicrosoftTeams");
    }

    #[test]
    fn test_mattermost_spec() {
        let spec = ChannelCreator::mattermost_spec();
        assert_eq!(spec.id, "mattermost");
        assert_eq!(spec.platform, "mattermost");
        assert!(matches!(spec.auth_type, AuthType::Bearer));
    }

    #[test]
    fn test_teams_spec() {
        let spec = ChannelCreator::teams_spec();
        assert_eq!(spec.id, "teams");
        assert!(matches!(spec.auth_type, AuthType::OAuth2 { .. }));
    }

    #[test]
    fn test_capability_display() {
        assert_eq!(AdapterCapability::SendMessage.to_string(), "send_message");
        assert_eq!(AdapterCapability::ThreadReply.to_string(), "thread_reply");
    }
}
