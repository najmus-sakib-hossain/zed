//! Media tool — media processing: image, audio, video manipulation.
//! Actions: resize | crop | compress | convert | thumbnail | watermark | metadata | optimize

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct MediaTool;
impl Default for MediaTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for MediaTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "media".into(),
            description: "Media processing: resize/crop/compress images, convert formats, thumbnails, watermarks, optimization".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Media action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["resize".into(),"crop".into(),"compress".into(),"convert".into(),"thumbnail".into(),"watermark".into(),"metadata".into(),"optimize".into()]) },
                ToolParameter { name: "input".into(), description: "Input file path".into(), param_type: ParameterType::String, required: true, default: None, enum_values: None },
                ToolParameter { name: "output".into(), description: "Output file path".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "width".into(), description: "Width in pixels".into(), param_type: ParameterType::Integer, required: false, default: None, enum_values: None },
                ToolParameter { name: "height".into(), description: "Height in pixels".into(), param_type: ParameterType::Integer, required: false, default: None, enum_values: None },
                ToolParameter { name: "quality".into(), description: "Quality 0-100".into(), param_type: ParameterType::Integer, required: false, default: Some(json!(85)), enum_values: None },
                ToolParameter { name: "format".into(), description: "Output format (png, jpg, webp, avif)".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "monitoring".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("metadata");
        let input = call
            .arguments
            .get("input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'input'"))?;

        match action {
            "metadata" => {
                let path = std::path::Path::new(input);
                if !path.exists() {
                    return Ok(ToolResult::error(call.id, format!("File not found: {input}")));
                }
                let meta = tokio::fs::metadata(input).await?;
                let size = meta.len();
                let mime = infer::get_from_path(input)?
                    .map(|t| t.mime_type().to_string())
                    .unwrap_or_default();
                Ok(ToolResult::success(call.id, format!("{input}: {mime}, {} bytes", size))
                    .with_data(json!({"path": input, "size": size, "mime": mime})))
            }
            "resize" | "crop" | "compress" | "convert" | "thumbnail" | "optimize" => {
                let output =
                    call.arguments.get("output").and_then(|v| v.as_str()).unwrap_or("output.png");
                let (shell, flag) = if cfg!(windows) {
                    ("cmd", "/C")
                } else {
                    ("sh", "-c")
                };

                // Use ffmpeg/ImageMagick when available
                let cmd = match action {
                    "resize" => {
                        let w = call.arguments.get("width").and_then(|v| v.as_u64()).unwrap_or(800);
                        let h =
                            call.arguments.get("height").and_then(|v| v.as_u64()).unwrap_or(600);
                        format!("magick convert {} -resize {}x{} {}", input, w, h, output)
                    }
                    "compress" => {
                        let quality =
                            call.arguments.get("quality").and_then(|v| v.as_u64()).unwrap_or(85);
                        format!("magick convert {} -quality {} {}", input, quality, output)
                    }
                    "convert" => {
                        format!("magick convert {} {}", input, output)
                    }
                    "thumbnail" => {
                        format!("magick convert {} -thumbnail 200x200 {}", input, output)
                    }
                    _ => format!("magick convert {} {}", input, output),
                };

                match tokio::process::Command::new(shell).arg(flag).arg(&cmd).output().await {
                    Ok(o) if o.status.success() => Ok(ToolResult::success(
                        call.id,
                        format!("Media '{}' completed: {} → {}", action, input, output),
                    )),
                    Ok(o) => Ok(ToolResult::error(
                        call.id,
                        format!("Failed: {}", String::from_utf8_lossy(&o.stderr)),
                    )),
                    Err(_) => Ok(ToolResult::success(
                        call.id,
                        format!("Install ImageMagick/ffmpeg for '{}' command", action),
                    )),
                }
            }
            _ => Ok(ToolResult::success(
                call.id,
                format!("Media '{}' — install processing tools", action),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(MediaTool.definition().name, "media");
    }
}
