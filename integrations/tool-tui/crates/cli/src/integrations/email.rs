//! Email integrations

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body: String,
    pub html: bool,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

pub trait EmailProvider {
    async fn send(&self, email: Email) -> Result<String>;
    async fn list_inbox(&self, limit: usize) -> Result<Vec<Email>>;
    async fn read(&self, id: &str) -> Result<Email>;
    async fn delete(&self, id: &str) -> Result<()>;
}

pub struct SmtpProvider {
    host: String,
    port: u16,
    username: String,
    password: String,
}

impl SmtpProvider {
    pub fn new(host: String, port: u16, username: String, password: String) -> Self {
        Self {
            host,
            port,
            username,
            password,
        }
    }
}

pub struct GmailProvider {
    api_key: String,
}

impl GmailProvider {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}
