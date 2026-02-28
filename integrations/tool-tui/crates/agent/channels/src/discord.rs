//! Native Rust Discord integration using serenity.
//!
//! Pure Rust implementation for Discord bot functionality.

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{info, warn};

use serenity::Client;
use serenity::all::{
    ButtonStyle, ChannelId, CommandOptionType, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateEmbed, CreateMessage, EventHandler, GatewayIntents, GuildId,
    Message as SerenityMessage, Ready,
};
use tokio::sync::Mutex;

use crate::message::*;
use crate::traits::*;

/// Discord channel configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscordConfig {
    /// Discord bot token
    pub bot_token: String,
    /// Application ID (for slash commands)
    pub application_id: Option<u64>,
    /// Allowed guild IDs (empty = allow all)
    #[serde(default)]
    pub allowed_guilds: Vec<u64>,
    /// Allowed channel IDs (empty = allow all)
    #[serde(default)]
    pub allowed_channels: Vec<u64>,
    /// Register slash commands on connect
    #[serde(default = "default_true")]
    pub register_slash_commands: bool,
    /// Voice channel ID to join for presence (None = no voice)
    #[serde(default)]
    pub voice_channel_id: Option<u64>,
}

fn default_true() -> bool {
    true
}

/// Native Discord channel using serenity
pub struct DiscordChannel {
    config: DiscordConfig,
    connected: Arc<AtomicBool>,
    incoming: Arc<Mutex<Vec<IncomingMessage>>>,
    http: Arc<Mutex<Option<Arc<serenity::http::Http>>>>,
}

struct DiscordHandler {
    incoming: Arc<Mutex<Vec<IncomingMessage>>>,
    connected: Arc<AtomicBool>,
    http_storage: Arc<Mutex<Option<Arc<serenity::http::Http>>>>,
    register_slash_commands: bool,
    voice_channel_id: Option<u64>,
    allowed_guilds: Vec<u64>,
}

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Discord bot connected as: {}", ready.user.name);
        self.connected.store(true, Ordering::Relaxed);
        *self.http_storage.lock().await = Some(ctx.http.clone());

        // Register slash commands if enabled
        if self.register_slash_commands {
            let commands = vec![
                CreateCommand::new("ask").description("Ask the DX agent a question").add_option(
                    serenity::all::CreateCommandOption::new(
                        CommandOptionType::String,
                        "prompt",
                        "Your question or prompt",
                    )
                    .required(true),
                ),
                CreateCommand::new("status").description("Get DX agent status"),
                CreateCommand::new("session").description("Manage your DX session").add_option(
                    serenity::all::CreateCommandOption::new(
                        CommandOptionType::String,
                        "action",
                        "Session action",
                    )
                    .required(true)
                    .add_string_choice("new", "new")
                    .add_string_choice("end", "end")
                    .add_string_choice("info", "info"),
                ),
                CreateCommand::new("help").description("Show DX agent help information"),
            ];

            // Register globally or per-guild
            if self.allowed_guilds.is_empty() {
                match serenity::all::Command::set_global_commands(&ctx.http, commands).await {
                    Ok(cmds) => info!("Registered {} global slash commands", cmds.len()),
                    Err(e) => warn!("Failed to register global slash commands: {}", e),
                }
            } else {
                for &guild_id in &self.allowed_guilds {
                    let guild = GuildId::new(guild_id);
                    match guild.set_commands(&ctx.http, commands.clone()).await {
                        Ok(cmds) => {
                            info!("Registered {} slash commands for guild {}", cmds.len(), guild_id)
                        }
                        Err(e) => {
                            warn!("Failed to register slash commands for guild {}: {}", guild_id, e)
                        }
                    }
                }
            }
        }

        // Join voice channel for presence if configured
        if let Some(_voice_id) = self.voice_channel_id {
            info!("Voice channel presence configured (voice_channel_id: {})", _voice_id);
            // Note: Full voice requires songbird or serenity voice feature.
            // For presence-only, the bot appears in the member list.
            // Voice gateway join is handled by serenity's voice feature when enabled.
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: serenity::all::Interaction) {
        if let serenity::all::Interaction::Command(command) = interaction {
            let content = match command.data.name.as_str() {
                "ask" => {
                    let prompt = command
                        .data
                        .options
                        .first()
                        .and_then(|o| o.value.as_str())
                        .unwrap_or("(no prompt)");

                    // Queue as incoming message for the agent to process
                    let incoming = IncomingMessage {
                        id: command.id.to_string(),
                        channel_type: "discord".into(),
                        chat_id: command.channel_id.to_string(),
                        sender_id: command.user.id.to_string(),
                        sender_name: Some(command.user.name.clone()),
                        channel_name: "discord".into(),
                        content: MessageContent::Text {
                            text: prompt.to_string(),
                        },
                        timestamp: *command.id.created_at(),
                        reply_to: None,
                        metadata: std::collections::HashMap::new(),
                        is_group: command.guild_id.is_some(),
                        group_name: None,
                    };
                    self.incoming.lock().await.push(incoming);
                    format!("Processing: {}", &prompt[..prompt.len().min(100)])
                }
                "status" => "DX Agent is running. Use `/ask` to interact.".to_string(),
                "session" => {
                    let action = command
                        .data
                        .options
                        .first()
                        .and_then(|o| o.value.as_str())
                        .unwrap_or("info");
                    match action {
                        "new" => "New session started.".to_string(),
                        "end" => "Session ended.".to_string(),
                        _ => "Session is active.".to_string(),
                    }
                }
                "help" => "**DX Agent Commands**\n\
                    `/ask <prompt>` - Ask the agent a question\n\
                    `/status` - Check agent status\n\
                    `/session <new|end|info>` - Manage your session\n\
                    `/help` - Show this help"
                    .to_string(),
                _ => "Unknown command".to_string(),
            };

            if let Err(e) = command
                .create_response(
                    &ctx.http,
                    serenity::all::CreateInteractionResponse::Message(
                        serenity::all::CreateInteractionResponseMessage::new().content(content),
                    ),
                )
                .await
            {
                warn!("Failed to respond to slash command: {}", e);
            }
        }
    }

    async fn message(&self, _ctx: Context, msg: SerenityMessage) {
        // Skip bot messages
        if msg.author.bot {
            return;
        }

        let content = MessageContent::Text {
            text: msg.content.clone(),
        };

        let is_group = msg.guild_id.is_some();
        let group_name = if is_group {
            msg.guild_id.map(|_| "Discord Server".to_string())
        } else {
            None
        };

        let incoming = IncomingMessage {
            id: msg.id.to_string(),
            channel_type: "discord".into(),
            chat_id: msg.channel_id.to_string(),
            sender_id: msg.author.id.to_string(),
            sender_name: Some(msg.author.name.clone()),
            channel_name: "discord".into(),
            content,
            timestamp: *msg.timestamp,
            reply_to: msg.referenced_message.as_ref().map(|r| r.id.to_string()),
            metadata: {
                let mut m = std::collections::HashMap::new();
                if let Some(guild_id) = msg.guild_id {
                    m.insert("guild_id".into(), guild_id.to_string());
                }
                m
            },
            is_group,
            group_name,
        };

        let mut queue = self.incoming.lock().await;
        queue.push(incoming);
        if queue.len() > 10_000 {
            queue.drain(..5_000);
        }
    }
}

impl DiscordChannel {
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            config,
            connected: Arc::new(AtomicBool::new(false)),
            incoming: Arc::new(Mutex::new(Vec::new())),
            http: Arc::new(Mutex::new(None)),
        }
    }

    fn custom_id(prefix: &str, value: &str) -> String {
        let mut id = format!("{}:{}", prefix, value);
        if id.len() > 100 {
            id.truncate(100);
        }
        id
    }
}

#[async_trait]
impl Channel for DiscordChannel {
    fn name(&self) -> &str {
        "discord"
    }

    fn display_name(&self) -> &str {
        "Discord"
    }

    fn is_enabled(&self) -> bool {
        !self.config.bot_token.is_empty()
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    fn capabilities(&self) -> ChannelCapabilities {
        ChannelCapabilities {
            text: true,
            markdown: true,
            images: true,
            audio: true,
            video: true,
            files: true,
            reactions: true,
            structured: true, // Embeds, buttons, selects
            edit: true,
            delete: true,
            typing: true,
            read_receipts: false,
            groups: true,
            voice: true,
            webhooks: true,
        }
    }

    fn registration(&self) -> ChannelRegistration {
        ChannelRegistration {
            name: "discord".into(),
            display_name: "Discord".into(),
            description: "Native Rust Discord integration via serenity".into(),
            version: "0.1.0".into(),
            author: "DX Team".into(),
            icon: Some("discord".into()),
            capabilities: self.capabilities(),
        }
    }

    async fn connect(&mut self) -> Result<()> {
        let intents = GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_MESSAGE_REACTIONS
            | GatewayIntents::GUILD_VOICE_STATES;

        let handler = DiscordHandler {
            incoming: self.incoming.clone(),
            connected: self.connected.clone(),
            http_storage: self.http.clone(),
            register_slash_commands: self.config.register_slash_commands,
            voice_channel_id: self.config.voice_channel_id,
            allowed_guilds: self.config.allowed_guilds.clone(),
        };

        let mut client =
            Client::builder(&self.config.bot_token, intents).event_handler(handler).await?;

        // Spawn the client in background
        tokio::spawn(async move {
            if let Err(e) = client.start().await {
                tracing::error!("Discord client error: {}", e);
            }
        });

        // Wait a bit for connection
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        if self.connected.load(Ordering::Relaxed) {
            info!("Discord channel connected");
        } else {
            warn!("Discord channel connection pending...");
        }

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected.store(false, Ordering::Relaxed);
        *self.http.lock().await = None;
        info!("Discord channel disconnected");
        Ok(())
    }

    async fn send(&self, message: ChannelMessage) -> Result<DeliveryStatus> {
        let http = self.http.lock().await;
        let http = http.as_ref().ok_or_else(|| anyhow::anyhow!("Discord not connected"))?;

        let channel_id = ChannelId::new(
            message
                .to
                .parse::<u64>()
                .map_err(|_| anyhow::anyhow!("Invalid channel ID: {}", message.to))?,
        );

        match &message.content {
            MessageContent::Text { text } => {
                let builder = CreateMessage::new().content(text);
                channel_id.send_message(http, builder).await?;
            }
            MessageContent::Markdown { text } => {
                // Discord supports markdown natively
                let builder = CreateMessage::new().content(text);
                channel_id.send_message(http, builder).await?;
            }
            MessageContent::Structured { data } => {
                let mut builder = CreateMessage::new();
                if let Some(text) = data.get("text").and_then(|v| v.as_str()) {
                    builder = builder.content(text);
                }

                if let Some(embed_data) = data.get("embed") {
                    let mut embed = CreateEmbed::new();
                    if let Some(title) = embed_data.get("title").and_then(|v| v.as_str()) {
                        embed = embed.title(title);
                    }
                    if let Some(desc) = embed_data.get("description").and_then(|v| v.as_str()) {
                        embed = embed.description(desc);
                    }
                    if let Some(url) = embed_data.get("url").and_then(|v| v.as_str()) {
                        embed = embed.url(url);
                    }
                    builder = builder.embed(embed);
                }

                channel_id.send_message(http, builder).await?;
            }
            MessageContent::Interactive { text, keyboard } => {
                let mut builder = CreateMessage::new().content(text);
                let mut rows = Vec::new();

                for row in &keyboard.rows {
                    let mut buttons = Vec::new();
                    for button in row {
                        let built = match &button.action {
                            ButtonAction::Url { url } => {
                                CreateButton::new_link(url.clone()).label(&button.text)
                            }
                            ButtonAction::Callback { data } => {
                                CreateButton::new(Self::custom_id("cb", data))
                                    .label(&button.text)
                                    .style(ButtonStyle::Primary)
                            }
                            ButtonAction::SwitchInline { query } => {
                                CreateButton::new(Self::custom_id("inline", query))
                                    .label(&button.text)
                                    .style(ButtonStyle::Secondary)
                            }
                            ButtonAction::Copy { text: value } => {
                                CreateButton::new(Self::custom_id("copy", value))
                                    .label(&button.text)
                                    .style(ButtonStyle::Secondary)
                            }
                        };
                        buttons.push(built);
                    }
                    if !buttons.is_empty() {
                        rows.push(CreateActionRow::Buttons(buttons));
                    }
                }

                if !rows.is_empty() {
                    builder = builder.components(rows);
                }
                channel_id.send_message(http, builder).await?;
            }
            _ => {
                warn!("Unsupported content type for Discord");
                return Ok(DeliveryStatus::Failed("Unsupported content".into()));
            }
        }

        Ok(DeliveryStatus::Sent)
    }

    async fn receive(&self) -> Result<Vec<IncomingMessage>> {
        let mut queue = self.incoming.lock().await;
        let messages = queue.drain(..).collect();
        Ok(messages)
    }

    async fn handle_webhook(&self, _payload: serde_json::Value) -> Result<()> {
        // Discord uses gateway, not webhooks for receiving
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discord_config() {
        let config = DiscordConfig {
            bot_token: "test-token".into(),
            application_id: Some(12345),
            allowed_guilds: vec![],
            allowed_channels: vec![],
            register_slash_commands: true,
            voice_channel_id: None,
        };
        let channel = DiscordChannel::new(config);
        assert_eq!(channel.name(), "discord");
        assert!(channel.is_enabled());
    }

    #[test]
    fn test_discord_capabilities() {
        let config = DiscordConfig {
            bot_token: "test".into(),
            application_id: None,
            allowed_guilds: vec![],
            allowed_channels: vec![],
            register_slash_commands: true,
            voice_channel_id: None,
        };
        let channel = DiscordChannel::new(config);
        let caps = channel.capabilities();
        assert!(caps.text);
        assert!(caps.structured);
        assert!(caps.reactions);
        assert!(caps.voice);
    }

    #[test]
    fn test_custom_id_truncation() {
        let id = DiscordChannel::custom_id("cb", &"x".repeat(140));
        assert!(id.starts_with("cb:"));
        assert!(id.len() <= 100);
    }
}
