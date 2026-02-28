use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::time::Instant;

#[derive(Parser, Debug)]
pub struct ChatCommand {
    /// Google AI Studio API Key (or set GOOGLE_AI_STUDIO_KEY env var)
    #[arg(short, long)]
    api_key: Option<String>,

    /// Model to use for chat
    #[arg(short, long, default_value = "gemini-2.5-flash")]
    model: String,

    /// List all available models
    #[arg(short, long)]
    list_models: bool,

    /// Enable interactive chat mode (ongoing conversation)
    #[arg(short = 'i', long)]
    interactive: bool,

    /// Initial prompt/message
    #[arg(trailing_var_arg = true)]
    prompt: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    contents: Vec<Content>,
}

#[derive(Debug, Serialize, Clone)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize, Clone)]
struct Part {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ContentResponse,
}

#[derive(Debug, Deserialize)]
struct ContentResponse {
    parts: Vec<PartResponse>,
}

#[derive(Debug, Deserialize)]
struct PartResponse {
    text: String,
}

impl ChatCommand {
    pub async fn execute(&self) -> Result<()> {
        let api_key = self.get_api_key()?;

        if self.list_models {
            self.list_available_models();
            return Ok(());
        }

        // Interactive mode
        if self.interactive || self.prompt.is_empty() {
            self.run_interactive_chat(&api_key).await?;
            return Ok(());
        }

        // Single message mode
        let prompt = self.prompt.join(" ");
        if prompt.trim().is_empty() {
            anyhow::bail!("No prompt provided. Use --help for usage information.");
        }

        println!("🤖 Sending to {}...\n", self.model);
        let mut history = vec![];
        self.send_chat_message(&api_key, &prompt, &mut history)
            .await?;

        Ok(())
    }

    async fn run_interactive_chat(&self, api_key: &str) -> Result<()> {
        println!(
            "\n{}",
            "╔════════════════════════════════════════════════════════════╗".bright_cyan()
        );
        println!(
            "{}",
            "║          🤖 DX Interactive Chat - Google AI Studio         ║".bright_cyan()
        );
        println!(
            "{}",
            "╚════════════════════════════════════════════════════════════╝".bright_cyan()
        );
        println!(
            "\n{}: {}",
            "Model".bright_green(),
            self.model.bright_yellow()
        );
        println!(
            "{}",
            "Type 'exit', 'quit', or press Ctrl+C to end the conversation".bright_black()
        );
        println!(
            "{}",
            "Type 'clear' to clear conversation history".bright_black()
        );
        println!("{}", "Type 'help' for available commands\n".bright_black());

        let mut history: Vec<Content> = vec![];
        let stdin = io::stdin();

        loop {
            print!("{} ", "You:".bright_green().bold());
            io::stdout().flush()?;

            let mut input = String::new();
            stdin.read_line(&mut input)?;
            let input = input.trim();

            // Handle commands
            match input.to_lowercase().as_str() {
                "exit" | "quit" => {
                    println!("\n{}", "👋 Goodbye! Thanks for chatting!".bright_cyan());
                    break;
                }
                "clear" => {
                    history.clear();
                    println!("{}", "✨ Conversation history cleared!".bright_yellow());
                    continue;
                }
                "help" => {
                    self.print_help();
                    continue;
                }
                "" => continue,
                _ => {}
            }

            // Send message and get response
            print!("\n{} ", "AI:".bright_blue().bold());
            io::stdout().flush()?;

            match self.send_chat_message(api_key, input, &mut history).await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("{} {}", "❌ Error:".bright_red(), e);
                    println!("{}", "Continuing conversation...".bright_yellow());
                }
            }

            println!();
        }

        Ok(())
    }

    fn print_help(&self) {
        println!("\n{}", "Available Commands:".bright_cyan().bold());
        println!("  {} - Exit the chat", "exit/quit".bright_yellow());
        println!("  {} - Clear conversation history", "clear".bright_yellow());
        println!("  {} - Show this help message", "help".bright_yellow());
        println!();
    }

    fn get_api_key(&self) -> Result<String> {
        // Check command-line argument first
        if let Some(key) = &self.api_key {
            return Ok(key.clone());
        }

        // Check environment variable
        if let Ok(key) = std::env::var("GOOGLE_AI_STUDIO_KEY") {
            if !key.trim().is_empty() {
                return Ok(key);
            }
        }

        // Prompt user for API key
        print!("Enter your Google AI Studio API Key: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let key = input.trim().to_string();

        if key.is_empty() {
            anyhow::bail!("API key is required. Get one from https://aistudio.google.com/apikey");
        }

        Ok(key)
    }

    async fn send_chat_message(
        &self,
        api_key: &str,
        prompt: &str,
        history: &mut Vec<Content>,
    ) -> Result<()> {
        // Add user message to history
        history.push(Content {
            parts: vec![Part {
                text: prompt.to_string(),
            }],
        });

        // Use streaming endpoint
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            self.model, api_key
        );

        let request = ChatRequest {
            contents: history.clone(),
        };

        // Create client with longer timeout
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()?;

        let start_time = Instant::now();

        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Google AI Studio. Check your internet connection and API key.")?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await?;
            eprintln!(
                "\n{} ({}): {}",
                "❌ API Error".bright_red(),
                status,
                error_text
            );
            history.pop();
            anyhow::bail!(
                "API request failed with status: {}. Check your API key and model name.",
                status
            );
        }

        // Stream the response
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut full_response = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    buffer.push_str(&text);

                    // Process complete lines
                    while let Some(newline_pos) = buffer.find('\n') {
                        let line = buffer[..newline_pos].to_string();
                        buffer = buffer[newline_pos + 1..].to_string();

                        if line.starts_with("data: ") {
                            let json_str = line[6..].trim();

                            if json_str == "[DONE]" {
                                break;
                            }

                            if let Ok(chunk_response) =
                                serde_json::from_str::<ChatResponse>(json_str)
                            {
                                if let Some(candidate) = chunk_response.candidates.first() {
                                    if let Some(part) = candidate.content.parts.first() {
                                        print!("{}", part.text);
                                        io::stdout().flush()?;
                                        full_response.push_str(&part.text);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("\n{} {}", "❌ Stream error:".bright_red(), e);
                    break;
                }
            }
        }

        let elapsed = start_time.elapsed();
        println!("\n");
        println!(
            "{} {:.2}s",
            "⏱️  Response time:".bright_black(),
            elapsed.as_secs_f64()
        );

        if !full_response.is_empty() {
            // Add AI response to history
            history.push(Content {
                parts: vec![Part {
                    text: full_response,
                }],
            });
        } else {
            history.pop();
            anyhow::bail!("No response received from the model");
        }

        Ok(())
    }

    fn list_available_models(&self) {
        println!("Available Google AI Studio Models:\n");

        println!("=== GEMINI 2.5 MODELS (Fastest - Recommended) ===");
        println!(
            "  gemini-2.5-flash                  - ⚡ FASTEST! Best price-performance (1M context)"
        );
        println!(
            "  gemini-2.5-flash-lite             - ⚡ Ultra fast, cost-efficient (1M context)"
        );
        println!("  gemini-2.5-pro                    - Advanced reasoning model (1M context)");
        println!("  gemini-2.5-flash-preview-09-2025  - Latest flash preview with thinking");

        println!("\n=== GEMINI 3 MODELS (Latest - November 2025) ===");
        println!(
            "  gemini-3-pro-preview              - Most intelligent multimodal model (1M context)"
        );
        println!("  gemini-3-pro-image-preview        - Image generation + understanding");
        println!(
            "  gemini-3-flash-preview            - Balanced speed and intelligence (1M context)"
        );

        println!("\n=== GEMMA 3 MODELS (Open Source - Slower but Free) ===");
        println!("  ⚠️  Note: Gemma models are slower than Gemini models");
        println!("  Instruction-Tuned (Recommended for Chat):");
        println!("    gemma-3-1b-it                   - 1B params, text-only, 32K context");
        println!("    gemma-3-4b-it                   - 4B params, multimodal (text+image), 128K context");
        println!("    gemma-3-12b-it                  - 12B params, multimodal, 128K context");
        println!("    gemma-3-27b-it                  - 27B params, multimodal, 128K context (supports function calling)");

        println!("\n  Base Models (Pre-trained, for fine-tuning):");
        println!("    gemma-3-1b, gemma-3-4b, gemma-3-12b, gemma-3-27b");

        println!("\n=== FUNCTION CALLING SUPPORT ===");
        println!("  ✅ All Gemini models support native function calling");
        println!("  ✅ Gemma 3 27B supports function calling via prompt engineering");
        println!("  ✅ FunctionGemma 270M - Specialized for function calling");

        println!("\n=== SPEED COMPARISON ===");
        println!("  🚀 Fastest:  gemini-2.5-flash-lite (< 1 second)");
        println!("  ⚡ Fast:     gemini-2.5-flash (1-2 seconds)");
        println!("  🏃 Medium:   gemini-2.5-pro (2-4 seconds)");
        println!("  🐢 Slower:   gemma-3-27b-it (5-15 seconds)");

        println!("\n=== USAGE EXAMPLES ===");
        println!("  # Fast chat (default)");
        println!("  dx chat \"Hello\"");
        println!();
        println!("  # Function calling demo");
        println!("  cargo run --example function_calling_demo");
        println!();
        println!("  # Use Gemma 3 27B (slower but supports function calling)");
        println!("  dx chat -m gemma-3-27b-it \"Explain Rust\"");

        println!("\n📝 Get your API key: https://aistudio.google.com/apikey");
        println!("📚 Documentation: https://ai.google.dev/gemini-api/docs");
        println!(
            "🔧 Function Calling: https://ai.google.dev/gemma/docs/capabilities/function-calling"
        );
    }
}
