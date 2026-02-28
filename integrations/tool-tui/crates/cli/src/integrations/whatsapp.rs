//! WhatsApp integration using Bun runtime
//!
//! Uses whatsapp-web.js via Bun for personal WhatsApp accounts

use crate::runtime::{RuntimeManager, RuntimePriority};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
// Unused: use serde_json::json;

/// WhatsApp client using Bun runtime
pub struct WhatsAppClient {
    runtime: RuntimeManager,
    initialized: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WhatsAppMessage {
    pub to: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WhatsAppMediaMessage {
    pub to: String,
    pub media_url: String,
    pub caption: Option<String>,
}

impl WhatsAppClient {
    /// Create new WhatsApp client
    pub fn new() -> Result<Self> {
        let runtime = RuntimeManager::with_priority(RuntimePriority::Bun)
            .context("Failed to initialize Bun runtime for WhatsApp")?;

        Ok(Self {
            runtime,
            initialized: false,
        })
    }

    /// Initialize WhatsApp client
    pub async fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        // Initialize whatsapp-web.js
        let init_code = r#"
            const { Client, LocalAuth } = require('whatsapp-web.js');
            
            globalThis.whatsappClient = new Client({
                authStrategy: new LocalAuth({
                    dataPath: './.dx/whatsapp-session'
                }),
                puppeteer: {
                    headless: true,
                    args: ['--no-sandbox', '--disable-setuid-sandbox']
                }
            });
            
            globalThis.whatsappReady = false;
            
            whatsappClient.on('ready', () => {
                console.log('WhatsApp client is ready!');
                globalThis.whatsappReady = true;
            });
            
            whatsappClient.on('qr', (qr) => {
                console.log('QR Code:', qr);
            });
            
            whatsappClient.on('authenticated', () => {
                console.log('WhatsApp authenticated');
            });
            
            whatsappClient.on('auth_failure', (msg) => {
                console.error('Authentication failure:', msg);
            });
            
            await whatsappClient.initialize();
            
            return { initialized: true };
        "#;

        let result = self
            .runtime
            .eval(init_code)
            .await
            .context("Failed to initialize WhatsApp client")?;

        if !result.success {
            return Err(anyhow::anyhow!("WhatsApp initialization failed: {:?}", result.error));
        }

        self.initialized = true;
        tracing::info!("WhatsApp client initialized successfully");

        Ok(())
    }

    /// Send text message
    pub async fn send_message(&mut self, to: &str, message: &str) -> Result<()> {
        self.ensure_initialized().await?;

        let code = format!(
            r#"
            if (!globalThis.whatsappReady) {{
                throw new Error('WhatsApp client not ready');
            }}
            
            const chatId = '{}@c.us';
            await whatsappClient.sendMessage(chatId, '{}');
            
            return {{ sent: true }};
            "#,
            to, message
        );

        let result = self.runtime.eval(&code).await.context("Failed to send WhatsApp message")?;

        if !result.success {
            return Err(anyhow::anyhow!("Failed to send message: {:?}", result.error));
        }

        tracing::info!("WhatsApp message sent to {}", to);
        Ok(())
    }

    /// Send media message
    pub async fn send_media(
        &mut self,
        to: &str,
        media_url: &str,
        caption: Option<&str>,
    ) -> Result<()> {
        self.ensure_initialized().await?;

        let caption_str = caption.unwrap_or("");
        let code = format!(
            r#"
            if (!globalThis.whatsappReady) {{
                throw new Error('WhatsApp client not ready');
            }}
            
            const {{ MessageMedia }} = require('whatsapp-web.js');
            const chatId = '{}@c.us';
            
            const media = await MessageMedia.fromUrl('{}');
            await whatsappClient.sendMessage(chatId, media, {{ caption: '{}' }});
            
            return {{ sent: true }};
            "#,
            to, media_url, caption_str
        );

        let result = self.runtime.eval(&code).await.context("Failed to send WhatsApp media")?;

        if !result.success {
            return Err(anyhow::anyhow!("Failed to send media: {:?}", result.error));
        }

        tracing::info!("WhatsApp media sent to {}", to);
        Ok(())
    }

    /// Get QR code for authentication
    pub async fn get_qr_code(&mut self) -> Result<String> {
        let code = r#"
            return new Promise((resolve) => {
                whatsappClient.once('qr', (qr) => {
                    resolve(qr);
                });
            });
        "#;

        let result = self.runtime.eval(code).await.context("Failed to get QR code")?;

        if !result.success {
            return Err(anyhow::anyhow!("Failed to get QR code: {:?}", result.error));
        }

        Ok(result.data.as_str().unwrap_or("").to_string())
    }

    /// Check if client is ready
    pub async fn is_ready(&mut self) -> Result<bool> {
        let code = "return globalThis.whatsappReady || false";

        let result = self.runtime.eval(code).await?;
        Ok(result.data.as_bool().unwrap_or(false))
    }

    async fn ensure_initialized(&mut self) -> Result<()> {
        if !self.initialized {
            self.initialize().await?;
        }
        Ok(())
    }
}

impl Drop for WhatsAppClient {
    fn drop(&mut self) {
        // Cleanup will be handled by runtime drop
    }
}
