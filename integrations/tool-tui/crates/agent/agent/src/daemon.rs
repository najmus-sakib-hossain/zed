//! # DX Agent Daemon
//!
//! The 24/7 daemon that runs continuously with minimal CPU usage.
//! Executes tasks like checking emails, updating Notion, coding - all in parallel.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::{
    integrations::IntegrationManager, llm::LlmClient, plugins::PluginLoader,
    pr_detector::PrDetector, scheduler::TaskScheduler, skills::SkillRegistry,
    wasm_runtime::WasmRuntime, AgentError, Result,
};

/// Configuration for the Agent Daemon
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Path to the DX configuration directory
    pub dx_path: PathBuf,

    /// Path to store plugins and integrations
    pub plugins_path: PathBuf,

    /// Path to store skills definitions
    pub skills_path: PathBuf,

    /// Whether to run in background mode
    pub background: bool,

    /// Auto-PR detection enabled
    pub auto_pr: bool,

    /// LLM provider configuration
    pub llm_endpoint: String,

    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,

    /// Interval for checking new integrations (seconds)
    pub integration_check_interval: u64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            dx_path: PathBuf::from(".dx"),
            plugins_path: PathBuf::from(".dx/plugins"),
            skills_path: PathBuf::from(".dx/skills"),
            background: false,
            auto_pr: true,
            llm_endpoint: "https://api.anthropic.com/v1/messages".to_string(),
            max_concurrent_tasks: 10,
            integration_check_interval: 300, // 5 minutes
        }
    }
}

/// The main Agent Daemon that runs 24/7
pub struct AgentDaemon {
    config: DaemonConfig,
    integrations: Arc<RwLock<IntegrationManager>>,
    plugins: Arc<RwLock<PluginLoader>>,
    scheduler: Arc<RwLock<TaskScheduler>>,
    skills: Arc<RwLock<SkillRegistry>>,
    wasm_runtime: Arc<WasmRuntime>,
    pr_detector: Arc<PrDetector>,
    llm: Arc<LlmClient>,
    running: Arc<RwLock<bool>>,
}

impl AgentDaemon {
    /// Create a new Agent Daemon
    pub async fn new(config: DaemonConfig) -> Result<Self> {
        info!("Initializing DX Agent Daemon...");

        // Initialize WASM runtime for dynamic plugin execution
        let wasm_runtime = Arc::new(WasmRuntime::new()?);

        // Initialize LLM client with DX Serializer format
        let llm = Arc::new(LlmClient::new(&config.llm_endpoint)?);

        // Initialize plugin loader
        let plugins = Arc::new(RwLock::new(
            PluginLoader::new(&config.plugins_path, wasm_runtime.clone()).await?,
        ));

        // Initialize integration manager
        let integrations = Arc::new(RwLock::new(IntegrationManager::new(&config.dx_path).await?));

        // Initialize skill registry
        let skills = Arc::new(RwLock::new(SkillRegistry::new(&config.skills_path).await?));

        // Initialize task scheduler
        let scheduler = Arc::new(RwLock::new(TaskScheduler::new()?));

        // Initialize PR detector for auto-updates
        let pr_detector = Arc::new(PrDetector::new(&config.dx_path)?);

        Ok(Self {
            config,
            integrations,
            plugins,
            scheduler,
            skills,
            wasm_runtime,
            pr_detector,
            llm,
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start the daemon
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting DX Agent Daemon...");

        {
            let mut running = self.running.write().await;
            *running = true;
        }

        // Load all plugins
        self.load_plugins().await?;

        // Load all integrations
        self.load_integrations().await?;

        // Load all skills
        self.load_skills().await?;

        // Start the scheduler
        self.start_scheduler().await?;

        // Start the main event loop
        self.event_loop().await?;

        Ok(())
    }

    /// Stop the daemon gracefully
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping DX Agent Daemon...");

        {
            let mut running = self.running.write().await;
            *running = false;
        }

        // Stop scheduler
        {
            let scheduler = self.scheduler.write().await;
            scheduler.stop().await?;
        }

        info!("DX Agent Daemon stopped.");
        Ok(())
    }

    /// Load all available plugins
    async fn load_plugins(&self) -> Result<()> {
        info!("Loading plugins...");
        let mut plugins = self.plugins.write().await;
        let count = plugins.load_all().await?;
        info!("Loaded {} plugins", count);
        Ok(())
    }

    /// Load all configured integrations
    async fn load_integrations(&self) -> Result<()> {
        info!("Loading integrations...");
        let mut integrations = self.integrations.write().await;
        let count = integrations.load_all().await?;
        info!("Loaded {} integrations", count);
        Ok(())
    }

    /// Load all skill definitions
    async fn load_skills(&self) -> Result<()> {
        info!("Loading skills...");
        let mut skills = self.skills.write().await;
        let count = skills.load_all().await?;
        info!("Loaded {} skills", count);
        Ok(())
    }

    /// Start the task scheduler
    async fn start_scheduler(&self) -> Result<()> {
        info!("Starting task scheduler...");
        let scheduler = self.scheduler.write().await;
        scheduler.start().await?;
        Ok(())
    }

    /// Main event loop - runs 24/7
    async fn event_loop(&self) -> Result<()> {
        info!("Entering main event loop...");

        loop {
            {
                let running = self.running.read().await;
                if !*running {
                    break;
                }
            }

            // Check for new integrations to create
            if self.config.auto_pr {
                self.check_and_create_pr().await?;
            }

            // Process pending messages from integrations
            self.process_messages().await?;

            // Execute scheduled tasks
            self.execute_tasks().await?;

            // Small sleep to prevent busy loop
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Check for local changes and create PR if needed
    async fn check_and_create_pr(&self) -> Result<()> {
        if let Some(diff) = self.pr_detector.detect_local_changes().await? {
            info!("Detected local changes: {:?}", diff);
            self.pr_detector.create_pr(&diff).await?;
        }
        Ok(())
    }

    /// Process incoming messages from all integrations
    async fn process_messages(&self) -> Result<()> {
        let integrations = self.integrations.read().await;

        for integration in integrations.iter() {
            if let Ok(messages) = integration.poll_messages().await {
                for msg in messages {
                    self.handle_message(integration.name(), msg).await?;
                }
            }
        }

        Ok(())
    }

    /// Handle a single message from an integration
    async fn handle_message(&self, integration: &str, message: String) -> Result<()> {
        info!("Handling message from {}: {}", integration, message);

        // Use LLM with DX Serializer format to process the message
        let response = self.llm.process_message(&message).await?;

        // Execute any skills mentioned in the response
        let skills = self.skills.read().await;
        for skill_name in response.required_skills() {
            if let Some(skill) = skills.get(skill_name) {
                // Convert context HashMap to DX LLM format
                let context_str = response
                    .context()
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join(" ");
                skill.execute(&context_str).await?;
            }
        }

        // Send response back through the integration
        let integrations = self.integrations.read().await;
        if let Some(int) = integrations.get(integration) {
            int.send_message(response.text()).await?;
        }

        Ok(())
    }

    /// Execute scheduled tasks
    async fn execute_tasks(&self) -> Result<()> {
        let scheduler = self.scheduler.read().await;
        let tasks = scheduler.get_due_tasks().await;

        for task in tasks {
            info!("Executing task: {}", task.name());
            if let Err(e) = task.execute().await {
                error!("Task {} failed: {}", task.name(), e);
            }
        }

        Ok(())
    }

    /// Create a new integration dynamically
    /// This is the AGI feature - the agent can create its own integrations!
    pub async fn create_integration(&self, name: &str, code: &str, language: &str) -> Result<()> {
        info!("Creating new integration: {} ({})", name, language);

        // Compile the code to WASM
        let wasm_bytes = self.wasm_runtime.compile(code, language).await?;

        // Create the plugin manifest using DX Serializer format
        let manifest = format!(
            "name={} version=0.0.1 language={} type=integration",
            name, language
        );

        // Load the plugin
        let mut plugins = self.plugins.write().await;
        plugins
            .load_from_bytes(name, &wasm_bytes, &manifest)
            .await?;

        // Register as an integration
        let mut integrations = self.integrations.write().await;
        integrations.register_from_plugin(name).await?;

        info!("✅ Integration {} created successfully!", name);

        // If auto-PR is enabled, prepare a PR
        if self.config.auto_pr {
            self.pr_detector
                .queue_new_integration(name, code, language)
                .await?;
        }

        Ok(())
    }

    /// Add a new skill to the agent
    pub async fn add_skill(&self, definition: &str) -> Result<()> {
        let mut skills = self.skills.write().await;
        skills.add_from_dx_format(definition).await?;
        Ok(())
    }

    /// Execute a skill by name
    pub async fn execute_skill(&self, name: &str, context: &str) -> Result<String> {
        let skills = self.skills.read().await;
        let skill = skills.get(name).ok_or_else(|| {
            AgentError::SkillExecutionFailed(format!("Skill not found: {}", name))
        })?;
        skill.execute(context).await
    }
}
