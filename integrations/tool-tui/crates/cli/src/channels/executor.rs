//! Channel executor using Bun runtime for Node.js messaging code

use crate::nodejs::{BunConfig, BunRuntime};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelType {
    WhatsApp,
    Telegram,
    Discord,
    Signal,
    Slack,
    IMessage,
}

impl ChannelType {
    pub fn runner_script(&self) -> &str {
        match self {
            Self::WhatsApp => "whatsapp-runner.mjs",
            Self::Telegram => "telegram-runner.mjs",
            Self::Discord => "discord-runner.mjs",
            Self::Signal => "signal-runner.mjs",
            Self::Slack => "slack-runner.mjs",
            Self::IMessage => "imessage-runner.mjs",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub channel_type: ChannelType,
    pub credentials: HashMap<String, String>,
    pub enabled: bool,
}

pub struct ChannelExecutor {
    runtime: BunRuntime,
    bridge_dir: PathBuf,
}

impl ChannelExecutor {
    pub fn new(bridge_dir: PathBuf) -> Self {
        let config = BunConfig {
            working_dir: bridge_dir.clone(),
            ..Default::default()
        };

        Self {
            runtime: BunRuntime::new(config),
            bridge_dir,
        }
    }

    /// Start a messaging channel
    pub async fn start_channel(&mut self, config: &ChannelConfig) -> Result<()> {
        if !config.enabled {
            return Ok(());
        }

        let script_path =
            self.bridge_dir.join("../bridge").join(config.channel_type.runner_script());

        if !script_path.exists() {
            anyhow::bail!("Channel runner script not found: {:?}", script_path);
        }

        self.runtime
            .spawn_worker(&script_path)
            .await
            .context("Failed to start channel worker")?;

        // Send credentials
        let creds_json = serde_json::to_string(&config.credentials)?;
        self.runtime.send_message(&creds_json).await?;

        Ok(())
    }

    /// Send a message through the channel
    pub async fn send_message(&mut self, recipient: &str, message: &str) -> Result<()> {
        let payload = serde_json::json!({
            "action": "send",
            "recipient": recipient,
            "message": message,
        });

        self.runtime.send_message(&payload.to_string()).await
    }

    /// Stop the channel
    pub async fn stop(&mut self) -> Result<()> {
        self.runtime.stop().await
    }
}

/// Manager for multiple messaging channels
pub struct ChannelManager {
    executors: HashMap<String, ChannelExecutor>,
    bridge_dir: PathBuf,
}

impl ChannelManager {
    pub fn new(bridge_dir: PathBuf) -> Self {
        Self {
            executors: HashMap::new(),
            bridge_dir,
        }
    }

    pub async fn start_channel(&mut self, name: String, config: ChannelConfig) -> Result<()> {
        let mut executor = ChannelExecutor::new(self.bridge_dir.clone());
        executor.start_channel(&config).await?;
        self.executors.insert(name, executor);
        Ok(())
    }

    pub async fn send_message(
        &mut self,
        channel: &str,
        recipient: &str,
        message: &str,
    ) -> Result<()> {
        let executor = self.executors.get_mut(channel).context("Channel not found")?;
        executor.send_message(recipient, message).await
    }

    pub async fn stop_all(&mut self) -> Result<()> {
        for (_, executor) in self.executors.iter_mut() {
            let _ = executor.stop().await;
        }
        self.executors.clear();
        Ok(())
    }
}
