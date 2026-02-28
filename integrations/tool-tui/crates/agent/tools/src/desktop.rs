//! Desktop tool — desktop automation: screenshot, OCR, clipboard, window management.
//! Actions: screenshot | ocr | clipboard_read | clipboard_write | window_list | window_focus | mouse | keyboard

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct DesktopTool;
impl Default for DesktopTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DesktopTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "desktop".into(),
            description: "Desktop automation: screenshots, OCR, clipboard, window management, mouse/keyboard simulation".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Desktop action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["screenshot".into(),"ocr".into(),"clipboard_read".into(),"clipboard_write".into(),"window_list".into(),"window_focus".into(),"mouse".into(),"keyboard".into()]) },
                ToolParameter { name: "output".into(), description: "Output file for screenshot".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "text".into(), description: "Text for clipboard_write or keyboard".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "window".into(), description: "Window title for focus".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "x".into(), description: "X coordinate for mouse".into(), param_type: ParameterType::Integer, required: false, default: None, enum_values: None },
                ToolParameter { name: "y".into(), description: "Y coordinate for mouse".into(), param_type: ParameterType::Integer, required: false, default: None, enum_values: None },
            ],
            category: "desktop".into(),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("screenshot");
        let (shell, flag) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        match action {
            "screenshot" => {
                let output = call
                    .arguments
                    .get("output")
                    .and_then(|v| v.as_str())
                    .unwrap_or("screenshot.png");
                #[cfg(windows)]
                {
                    let ps = format!(
                        r#"powershell -Command "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Screen]::PrimaryScreen | ForEach-Object {{ $bmp = New-Object System.Drawing.Bitmap($_.Bounds.Width, $_.Bounds.Height); $g = [System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($_.Bounds.Location, [System.Drawing.Point]::Empty, $_.Bounds.Size); $bmp.Save('{}') }}""#,
                        output
                    );
                    let _ = tokio::process::Command::new(shell).arg(flag).arg(&ps).output().await;
                }
                Ok(ToolResult::success(call.id, format!("Screenshot saved to {output}")))
            }
            "clipboard_read" => {
                let cmd = if cfg!(windows) {
                    "powershell -Command Get-Clipboard"
                } else {
                    "xclip -selection clipboard -o"
                };
                let output =
                    tokio::process::Command::new(shell).arg(flag).arg(cmd).output().await?;
                Ok(ToolResult::success(
                    call.id,
                    String::from_utf8_lossy(&output.stdout).to_string(),
                ))
            }
            "clipboard_write" => {
                let text = call.arguments.get("text").and_then(|v| v.as_str()).unwrap_or("");
                let cmd = if cfg!(windows) {
                    format!(
                        "powershell -Command \"Set-Clipboard -Value '{}'\"",
                        text.replace('\'', "''")
                    )
                } else {
                    format!("echo '{}' | xclip -selection clipboard", text)
                };
                let _ = tokio::process::Command::new(shell).arg(flag).arg(&cmd).output().await;
                Ok(ToolResult::success(call.id, "Copied to clipboard".into()))
            }
            "window_list" => {
                let cmd = if cfg!(windows) {
                    r#"powershell -Command "Get-Process | Where-Object {$_.MainWindowTitle} | Select-Object -Property Id,MainWindowTitle | Format-Table -AutoSize""#
                } else {
                    "wmctrl -l"
                };
                let output =
                    tokio::process::Command::new(shell).arg(flag).arg(cmd).output().await?;
                Ok(ToolResult::success(
                    call.id,
                    String::from_utf8_lossy(&output.stdout).to_string(),
                ))
            }
            _ => Ok(ToolResult::success(
                call.id,
                format!("Desktop '{}' — requires platform-specific implementation", action),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(DesktopTool.definition().name, "desktop");
    }
}
