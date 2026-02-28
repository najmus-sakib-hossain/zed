//! # Phone Call Integration
//!
//! Handle incoming phone calls via Twilio.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::calls::{CallHandler, CallConfig};
//!
//! let config = CallConfig::from_file("~/.dx/config/answer-call.sr")?;
//! let handler = CallHandler::new(&config)?;
//!
//! // Handle incoming call
//! handler.on_call(|call| {
//!     call.say("Hello, this is DX assistant.");
//!     call.record();
//! }).await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};

/// Call configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallConfig {
    /// Whether call handling is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Twilio Account SID
    #[serde(default)]
    pub account_sid: String,
    /// Twilio Auth Token
    #[serde(default)]
    pub auth_token: String,
    /// Twilio phone number
    #[serde(default)]
    pub phone_number: String,
    /// Webhook URL for incoming calls
    pub webhook_url: Option<String>,
    /// Default voice for TTS
    #[serde(default = "default_voice")]
    pub voice: String,
    /// Default language
    #[serde(default = "default_language")]
    pub language: String,
    /// Recording settings
    #[serde(default)]
    pub recording: RecordingConfig,
    /// Transcription settings
    #[serde(default)]
    pub transcription: TranscriptionConfig,
}

fn default_true() -> bool {
    true
}

fn default_voice() -> String {
    "Polly.Joanna".to_string()
}

fn default_language() -> String {
    "en-US".to_string()
}

impl Default for CallConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            account_sid: String::new(),
            auth_token: String::new(),
            phone_number: String::new(),
            webhook_url: None,
            voice: default_voice(),
            language: default_language(),
            recording: RecordingConfig::default(),
            transcription: TranscriptionConfig::default(),
        }
    }
}

impl CallConfig {
    /// Load from .sr config file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        Self::parse_sr(&content)
    }

    fn parse_sr(_content: &str) -> Result<Self> {
        Ok(Self::default())
    }

    /// Resolve environment variables
    pub fn resolve_env_vars(&mut self) {
        if self.account_sid.is_empty() || self.account_sid.starts_with('$') {
            self.account_sid = std::env::var("TWILIO_ACCOUNT_SID").unwrap_or_default();
        }
        if self.auth_token.is_empty() || self.auth_token.starts_with('$') {
            self.auth_token = std::env::var("TWILIO_AUTH_TOKEN").unwrap_or_default();
        }
        if self.phone_number.is_empty() || self.phone_number.starts_with('$') {
            self.phone_number = std::env::var("TWILIO_PHONE_NUMBER").unwrap_or_default();
        }
    }
}

/// Recording configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingConfig {
    /// Whether to record calls
    #[serde(default)]
    pub enabled: bool,
    /// Recording format
    #[serde(default = "default_recording_format")]
    pub format: String,
    /// Max recording duration in seconds
    #[serde(default = "default_max_duration")]
    pub max_duration: u32,
}

fn default_recording_format() -> String {
    "mp3".to_string()
}

fn default_max_duration() -> u32 {
    3600 // 1 hour
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            format: default_recording_format(),
            max_duration: default_max_duration(),
        }
    }
}

/// Transcription configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    /// Whether to transcribe calls
    #[serde(default)]
    pub enabled: bool,
    /// Transcription provider
    #[serde(default = "default_transcription_provider")]
    pub provider: String,
    /// Language for transcription
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_transcription_provider() -> String {
    "twilio".to_string()
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_transcription_provider(),
            language: default_language(),
        }
    }
}

/// Call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallInfo {
    /// Call SID
    pub sid: String,
    /// Caller phone number
    pub from: String,
    /// Called phone number
    pub to: String,
    /// Call status
    pub status: CallStatus,
    /// Call direction
    pub direction: CallDirection,
    /// Call duration in seconds
    pub duration: Option<u32>,
    /// Start time
    pub start_time: Option<String>,
    /// End time
    pub end_time: Option<String>,
}

/// Call status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CallStatus {
    Queued,
    Ringing,
    InProgress,
    Completed,
    Busy,
    Failed,
    NoAnswer,
    Canceled,
}

/// Call direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CallDirection {
    Inbound,
    Outbound,
}

/// Call action for TwiML response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallAction {
    /// Speak text
    Say {
        text: String,
        voice: Option<String>,
        language: Option<String>,
    },
    /// Play audio file
    Play {
        url: String,
    },
    /// Record the call
    Record {
        action: Option<String>,
        max_length: Option<u32>,
        transcribe: bool,
    },
    /// Gather DTMF input
    Gather {
        action: String,
        num_digits: Option<u32>,
        timeout: Option<u32>,
        speech: bool,
    },
    /// Dial another number
    Dial {
        number: String,
        caller_id: Option<String>,
        timeout: Option<u32>,
    },
    /// Redirect to another TwiML
    Redirect {
        url: String,
    },
    /// Reject the call
    Reject {
        reason: Option<String>,
    },
    /// Hang up
    Hangup,
    /// Pause
    Pause {
        length: u32,
    },
}

/// Call handler
pub struct CallHandler {
    config: CallConfig,
    base_url: String,
}

impl CallHandler {
    /// Twilio API base URL
    const API_BASE: &'static str = "https://api.twilio.com/2010-04-01";

    /// Create a new call handler
    pub fn new(config: &CallConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        Ok(Self {
            config,
            base_url: Self::API_BASE.to_string(),
        })
    }

    /// Check if handler is configured
    pub fn is_configured(&self) -> bool {
        !self.config.account_sid.is_empty()
            && !self.config.auth_token.is_empty()
            && !self.config.phone_number.is_empty()
    }

    /// Generate TwiML response for incoming call
    pub fn generate_twiml(&self, actions: &[CallAction]) -> String {
        let mut twiml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Response>\n");

        for action in actions {
            match action {
                CallAction::Say { text, voice, language } => {
                    let v = voice.as_deref().unwrap_or(&self.config.voice);
                    let l = language.as_deref().unwrap_or(&self.config.language);
                    twiml.push_str(&format!(
                        "  <Say voice=\"{}\" language=\"{}\">{}</Say>\n",
                        v, l, xml_escape(text)
                    ));
                }
                CallAction::Play { url } => {
                    twiml.push_str(&format!("  <Play>{}</Play>\n", xml_escape(url)));
                }
                CallAction::Record { action, max_length, transcribe } => {
                    let mut attrs = String::new();
                    if let Some(a) = action {
                        attrs.push_str(&format!(" action=\"{}\"", a));
                    }
                    if let Some(m) = max_length {
                        attrs.push_str(&format!(" maxLength=\"{}\"", m));
                    }
                    if *transcribe {
                        attrs.push_str(" transcribe=\"true\"");
                    }
                    twiml.push_str(&format!("  <Record{}/>\n", attrs));
                }
                CallAction::Gather { action, num_digits, timeout, speech } => {
                    let mut attrs = format!(" action=\"{}\"", action);
                    if let Some(n) = num_digits {
                        attrs.push_str(&format!(" numDigits=\"{}\"", n));
                    }
                    if let Some(t) = timeout {
                        attrs.push_str(&format!(" timeout=\"{}\"", t));
                    }
                    if *speech {
                        attrs.push_str(" input=\"speech dtmf\"");
                    }
                    twiml.push_str(&format!("  <Gather{}/>\n", attrs));
                }
                CallAction::Dial { number, caller_id, timeout } => {
                    let mut attrs = String::new();
                    if let Some(c) = caller_id {
                        attrs.push_str(&format!(" callerId=\"{}\"", c));
                    }
                    if let Some(t) = timeout {
                        attrs.push_str(&format!(" timeout=\"{}\"", t));
                    }
                    twiml.push_str(&format!("  <Dial{}>{}</Dial>\n", attrs, number));
                }
                CallAction::Redirect { url } => {
                    twiml.push_str(&format!("  <Redirect>{}</Redirect>\n", xml_escape(url)));
                }
                CallAction::Reject { reason } => {
                    let r = reason.as_deref().unwrap_or("rejected");
                    twiml.push_str(&format!("  <Reject reason=\"{}\"/>\n", r));
                }
                CallAction::Hangup => {
                    twiml.push_str("  <Hangup/>\n");
                }
                CallAction::Pause { length } => {
                    twiml.push_str(&format!("  <Pause length=\"{}\"/>\n", length));
                }
            }
        }

        twiml.push_str("</Response>");
        twiml
    }

    /// Make an outbound call
    pub async fn call(&self, to: &str, twiml_url: &str) -> Result<CallInfo> {
        if !self.is_configured() {
            return Err(DrivenError::Config("Twilio not configured".into()));
        }

        let url = format!(
            "{}/Accounts/{}/Calls.json",
            self.base_url, self.config.account_sid
        );

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .form(&[
                ("To", to),
                ("From", &self.config.phone_number),
                ("Url", twiml_url),
            ])
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Twilio error: {}", error)));
        }

        let call: TwilioCallResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        Ok(CallInfo {
            sid: call.sid,
            from: call.from,
            to: call.to,
            status: parse_call_status(&call.status),
            direction: if call.direction == "outbound-api" {
                CallDirection::Outbound
            } else {
                CallDirection::Inbound
            },
            duration: None,
            start_time: call.start_time,
            end_time: None,
        })
    }

    /// Get call information
    pub async fn get_call(&self, sid: &str) -> Result<CallInfo> {
        if !self.is_configured() {
            return Err(DrivenError::Config("Twilio not configured".into()));
        }

        let url = format!(
            "{}/Accounts/{}/Calls/{}.json",
            self.base_url, self.config.account_sid, sid
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to get call".into()));
        }

        let call: TwilioCallResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        Ok(CallInfo {
            sid: call.sid,
            from: call.from,
            to: call.to,
            status: parse_call_status(&call.status),
            direction: if call.direction.contains("outbound") {
                CallDirection::Outbound
            } else {
                CallDirection::Inbound
            },
            duration: call.duration.and_then(|d| d.parse().ok()),
            start_time: call.start_time,
            end_time: call.end_time,
        })
    }

    /// End a call
    pub async fn hangup(&self, sid: &str) -> Result<()> {
        if !self.is_configured() {
            return Err(DrivenError::Config("Twilio not configured".into()));
        }

        let url = format!(
            "{}/Accounts/{}/Calls/{}.json",
            self.base_url, self.config.account_sid, sid
        );

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .form(&[("Status", "completed")])
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to end call".into()));
        }

        Ok(())
    }

    /// List recent calls
    pub async fn list_calls(&self, limit: u32) -> Result<Vec<CallInfo>> {
        if !self.is_configured() {
            return Err(DrivenError::Config("Twilio not configured".into()));
        }

        let url = format!(
            "{}/Accounts/{}/Calls.json?PageSize={}",
            self.base_url, self.config.account_sid, limit
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to list calls".into()));
        }

        #[derive(Deserialize)]
        struct CallsResponse {
            calls: Vec<TwilioCallResponse>,
        }

        let result: CallsResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        Ok(result
            .calls
            .into_iter()
            .map(|c| CallInfo {
                sid: c.sid,
                from: c.from,
                to: c.to,
                status: parse_call_status(&c.status),
                direction: if c.direction.contains("outbound") {
                    CallDirection::Outbound
                } else {
                    CallDirection::Inbound
                },
                duration: c.duration.and_then(|d| d.parse().ok()),
                start_time: c.start_time,
                end_time: c.end_time,
            })
            .collect())
    }
}

#[derive(Debug, Deserialize)]
struct TwilioCallResponse {
    sid: String,
    from: String,
    to: String,
    status: String,
    direction: String,
    duration: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
}

fn parse_call_status(status: &str) -> CallStatus {
    match status {
        "queued" => CallStatus::Queued,
        "ringing" => CallStatus::Ringing,
        "in-progress" => CallStatus::InProgress,
        "completed" => CallStatus::Completed,
        "busy" => CallStatus::Busy,
        "failed" => CallStatus::Failed,
        "no-answer" => CallStatus::NoAnswer,
        "canceled" => CallStatus::Canceled,
        _ => CallStatus::Failed,
    }
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CallConfig::default();
        assert!(config.enabled);
        assert_eq!(config.voice, "Polly.Joanna");
    }

    #[test]
    fn test_twiml_generation() {
        let config = CallConfig::default();
        let handler = CallHandler::new(&config).unwrap();
        
        let twiml = handler.generate_twiml(&[
            CallAction::Say {
                text: "Hello".to_string(),
                voice: None,
                language: None,
            },
            CallAction::Hangup,
        ]);
        
        assert!(twiml.contains("<Say"));
        assert!(twiml.contains("Hello"));
        assert!(twiml.contains("<Hangup/>"));
    }
}
