//! Bridge between Rust and OpenClaw TypeScript code
//! Executes OpenClaw features via Bun runtime

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

use super::runtime::BunRuntime;

/// Gateway server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub port: u16,
    pub bind_address: String,
    pub control_ui_enabled: bool,
    pub openai_api_enabled: bool,
    pub auth_token: Option<String>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: 31337,
            bind_address: "127.0.0.1".to_string(),
            control_ui_enabled: true,
            openai_api_enabled: true,
            auth_token: None,
        }
    }
}

/// OpenClaw bridge - executes TypeScript code via Bun
pub struct OpenClawBridge {
    runtime: BunRuntime,
    nodejs_dir: PathBuf,
}

impl OpenClawBridge {
    /// Create new bridge
    pub fn new() -> Result<Self> {
        let nodejs_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join("nodejs");

        let mut config = super::runtime::BunConfig::default();
        config.working_dir = nodejs_dir.clone();

        Ok(Self {
            runtime: BunRuntime::new(config),
            nodejs_dir,
        })
    }

    /// Start gateway server
    pub async fn start_gateway(&mut self, config: GatewayConfig) -> Result<()> {
        let script = self.nodejs_dir.join("gateway_entry.ts");

        // Create entry script if it doesn't exist
        if !script.exists() {
            self.create_gateway_entry(&script)?;
        }

        let config_json = serde_json::to_string(&config)?;
        self.runtime.spawn_worker(&script).await?;
        self.runtime.send_message(&config_json).await?;

        Ok(())
    }

    /// Send message via channel
    pub async fn send_message(
        &mut self,
        channel: &str,
        recipient: &str,
        message: &str,
    ) -> Result<()> {
        let cmd = serde_json::json!({
            "action": "send_message",
            "channel": channel,
            "recipient": recipient,
            "message": message,
        });

        self.runtime.send_message(&serde_json::to_string(&cmd)?).await?;

        Ok(())
    }

    /// Execute cron job
    pub async fn execute_cron(&mut self, job_id: &str) -> Result<Value> {
        let cmd = serde_json::json!({
            "action": "execute_cron",
            "job_id": job_id,
        });

        let result = self
            .runtime
            .execute_code(&format!(
                "import {{ executeCronJob }} from './cron/service.ts'; \
                 const result = await executeCronJob({}); \
                 console.log(JSON.stringify(result));",
                serde_json::to_string(&cmd)?
            ))
            .await?;

        Ok(serde_json::from_str(&result)?)
    }

    /// Create gateway entry script
    fn create_gateway_entry(&self, path: &PathBuf) -> Result<()> {
        let script = r#"#!/usr/bin/env bun
// Gateway entry point - bridges Rust to TypeScript

import { startGatewayServer } from './gateway/server.ts';

// Read config from stdin
process.stdin.on('data', async (data) => {
    try {
        const config = JSON.parse(data.toString());
        await startGatewayServer(config);
        console.log('GATEWAY_STARTED');
    } catch (err) {
        console.error('GATEWAY_ERROR:', err.message);
    }
});

// Handle commands
process.stdin.on('data', async (data) => {
    try {
        const cmd = JSON.parse(data.toString());
        
        if (cmd.action === 'send_message') {
            // Import channel and send
            const channel = await import(`./channels/${cmd.channel}.ts`);
            await channel.sendMessage(cmd.recipient, cmd.message);
            console.log('MESSAGE_SENT');
        }
    } catch (err) {
        console.error('COMMAND_ERROR:', err.message);
    }
});
"#;

        std::fs::write(path, script).context("Failed to create gateway entry script")?;
        Ok(())
    }
}

impl Default for OpenClawBridge {
    fn default() -> Self {
        Self::new().expect("Failed to create OpenClaw bridge")
    }
}
