//! LLM management commands

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::llm::{InferenceEngine, InferenceRequest, LlmConfig, ModelManager};
use crate::ui::theme::Theme;

#[derive(Debug, Args)]
pub struct LlmArgs {
    #[command(subcommand)]
    pub command: LlmCommand,
}

#[derive(Debug, Subcommand)]
pub enum LlmCommand {
    /// Initialize LLM configuration
    Init {
        /// Backend to use (ollama, khroma)
        #[arg(short, long, default_value = "ollama")]
        backend: String,

        /// Model to download
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Download a model from Hugging Face
    Download {
        /// Model ID (e.g., google/gemma-2-2b-it)
        model: String,

        /// Model revision
        #[arg(short, long, default_value = "main")]
        revision: String,
    },

    /// List downloaded models
    List,

    /// Test LLM inference
    Test {
        /// Prompt to test
        #[arg(short, long, default_value = "Hello, how are you?")]
        prompt: String,

        /// Enable streaming output
        #[arg(short, long)]
        stream: bool,
    },

    /// Show LLM configuration
    Config,
}

pub async fn run(args: LlmArgs, theme: &Theme) -> Result<()> {
    match args.command {
        LlmCommand::Init { backend, model } => {
            let mut config = LlmConfig::default();
            config.default_backend = backend;

            if let Some(model_id) = model {
                config.huggingface.default_model = model_id;
            }

            let config_path = LlmConfig::default_path();
            config.save(&config_path)?;

            theme.print_success(&format!("LLM config saved to: {}", config_path.display()));
            Ok(())
        }

        LlmCommand::Download { model, revision } => {
            let config = LlmConfig::load(&LlmConfig::default_path()).unwrap_or_default();

            let manager = ModelManager::new(config.cache_dir)?;

            theme.print_info("Download", &format!("Downloading model: {}", model));
            let path = manager.download_model(&model, Some(&revision))?;

            theme.print_success(&format!("Model downloaded to: {}", path.display()));
            Ok(())
        }

        LlmCommand::List => {
            let config = LlmConfig::load(&LlmConfig::default_path()).unwrap_or_default();

            let manager = ModelManager::new(config.cache_dir)?;
            let models = manager.list_models()?;

            if models.is_empty() {
                println!("No models downloaded yet");
            } else {
                theme.print_info("Models", "Downloaded models:");
                for model in models {
                    println!("  • {}", model);
                }
            }

            Ok(())
        }

        LlmCommand::Test { prompt, stream } => {
            let config = LlmConfig::load(&LlmConfig::default_path())
                .context("LLM not configured. Run 'dx llm init' first")?;

            theme.print_info(
                "Backend",
                &format!("Initializing {} backend...", config.default_backend),
            );
            let engine = InferenceEngine::new(config.clone()).await?;

            theme.print_info("Status", "Generating response...");

            if stream {
                let request = InferenceRequest {
                    prompt: prompt.clone(),
                    max_tokens: config.inference.max_tokens,
                    temperature: config.inference.temperature,
                    stream: true,
                };

                print!("\n");
                engine
                    .generate_stream(request, |token| {
                        print!("{}", token);
                        use std::io::Write;
                        std::io::stdout().flush().ok();
                    })
                    .await?;
                println!("\n");
            } else {
                let request = InferenceRequest {
                    prompt: prompt.clone(),
                    max_tokens: config.inference.max_tokens,
                    temperature: config.inference.temperature,
                    stream: false,
                };

                let response = engine.generate(request).await?;
                println!("\n{}\n", response.text);
            }

            theme.print_success("Test complete");
            Ok(())
        }

        LlmCommand::Config => {
            let config = LlmConfig::load(&LlmConfig::default_path()).unwrap_or_default();

            println!("LLM Configuration:");
            println!("  Backend: {}", config.default_backend);
            println!("  Cache Dir: {}", config.cache_dir.display());
            println!("\nHugging Face:");
            println!("  Model: {}", config.huggingface.default_model);
            println!("  Revision: {}", config.huggingface.revision);
            println!("\nOllama:");
            println!("  URL: {}", config.ollama.url);
            println!("  Model: {}", config.ollama.default_model);
            if let Some(dir) = &config.ollama.models_dir {
                println!("  Models Dir: {}", dir);
            }
            println!("  Remote Enabled: {}", config.ollama.enable_remote);
            if config.ollama.enable_remote {
                if let Some(model) = &config.ollama.remote_model {
                    println!("  Remote Model: {}", model);
                }
                println!(
                    "  API Key: {}",
                    if config.ollama.remote_api_key.is_some() {
                        "Set"
                    } else {
                        "Not set"
                    }
                );
            }
            println!("\nInference:");
            println!("  Max Tokens: {}", config.inference.max_tokens);
            println!("  Temperature: {}", config.inference.temperature);
            println!("  Top-P: {}", config.inference.top_p);

            Ok(())
        }
    }
}
