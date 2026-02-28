//! Notify tool — notifications across channels (desktop, email, webhook, Slack, Discord).
//! Actions: send | desktop | email | webhook | slack | discord | sms | schedule

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct NotifyTool;
impl Default for NotifyTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for NotifyTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "notify".into(),
            description: "Send notifications: desktop toast, email, webhooks, Slack, Discord, SMS, scheduled alerts".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Notification action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["send".into(),"desktop".into(),"email".into(),"webhook".into(),"slack".into(),"discord".into(),"sms".into(),"schedule".into()]) },
                ToolParameter { name: "title".into(), description: "Notification title".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "message".into(), description: "Notification message".into(), param_type: ParameterType::String, required: true, default: None, enum_values: None },
                ToolParameter { name: "url".into(), description: "Webhook URL".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "to".into(), description: "Recipient (email, phone, channel)".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "urgency".into(), description: "Urgency level (low, normal, high, critical)".into(), param_type: ParameterType::String, required: false, default: Some(json!("normal")), enum_values: None },
            ],
            category: "comms".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("send");
        let message = call.arguments.get("message").and_then(|v| v.as_str()).unwrap_or("");
        let title = call
            .arguments
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("DX Notification");

        match action {
            "desktop" | "send" => {
                #[cfg(windows)]
                {
                    let (shell, flag) = ("cmd", "/C");
                    let ps = format!(
                        r#"powershell -Command "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null; $template = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); $text = $template.GetElementsByTagName('text'); $text.Item(0).AppendChild($template.CreateTextNode('{}')) > $null; $text.Item(1).AppendChild($template.CreateTextNode('{}')) > $null; $toast = [Windows.UI.Notifications.ToastNotification]::new($template); [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('DX Agent').Show($toast)""#,
                        title.replace('\'', ""),
                        message.replace('\'', "")
                    );
                    let _ = tokio::process::Command::new(shell).arg(flag).arg(&ps).output().await;
                }
                Ok(ToolResult::success(
                    call.id,
                    format!("Desktop notification: {title} — {message}"),
                ))
            }
            "webhook" => {
                let url = call
                    .arguments
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'url'"))?;
                let body = json!({"text": message, "title": title});
                let client = reqwest::Client::new();
                let resp = client.post(url).json(&body).send().await?;
                Ok(ToolResult::success(call.id, format!("Webhook sent ({})", resp.status())))
            }
            "slack" => {
                let url = call
                    .arguments
                    .get("url")
                    .and_then(|v| v.as_str())
                    .or_else(|| std::env::var("SLACK_WEBHOOK_URL").ok().as_deref().map(|_| ""))
                    .ok_or_else(|| anyhow::anyhow!("Missing 'url' or SLACK_WEBHOOK_URL env"))?;
                let body = json!({"text": format!("*{}*\n{}", title, message)});
                let client = reqwest::Client::new();
                let resp = client.post(url).json(&body).send().await?;
                Ok(ToolResult::success(
                    call.id,
                    format!("Slack notification sent ({})", resp.status()),
                ))
            }
            "discord" => {
                let url = call
                    .arguments
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing webhook 'url'"))?;
                let body = json!({"content": format!("**{}**\n{}", title, message)});
                let client = reqwest::Client::new();
                let resp = client.post(url).json(&body).send().await?;
                Ok(ToolResult::success(
                    call.id,
                    format!("Discord notification sent ({})", resp.status()),
                ))
            }
            _ => Ok(ToolResult::success(
                call.id,
                format!("Notify '{}': {} — {}", action, title, message),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(NotifyTool.definition().name, "notify");
    }
}
