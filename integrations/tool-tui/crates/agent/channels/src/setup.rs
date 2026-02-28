//! Setup wizards for channel configuration.
//!
//! Step-by-step interactive flows that guide users through
//! channel setup (API keys, tokens, webhook URLs, etc.).

use anyhow::{Result, bail};

/// Type alias for step validator functions.
pub type StepValidator = Box<dyn Fn(&str) -> Result<()> + Send + Sync>;

/// A single step in a setup wizard.
pub struct SetupStep {
    /// Step name / identifier.
    pub name: String,
    /// Prompt text shown to the user.
    pub prompt: String,
    /// Help text with additional guidance.
    pub help: Option<String>,
    /// Whether the step is optional.
    pub optional: bool,
    /// Validation function for user input.
    validator: StepValidator,
}

impl std::fmt::Debug for SetupStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SetupStep")
            .field("name", &self.name)
            .field("prompt", &self.prompt)
            .field("optional", &self.optional)
            .finish()
    }
}

impl SetupStep {
    /// Create a new required step with a validator.
    pub fn new(
        name: impl Into<String>,
        prompt: impl Into<String>,
        validator: impl Fn(&str) -> Result<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            prompt: prompt.into(),
            help: None,
            optional: false,
            validator: Box::new(validator),
        }
    }

    /// Create a step that accepts any non-empty input.
    pub fn required(name: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self::new(name, prompt, |input| {
            if input.trim().is_empty() {
                bail!("This field is required");
            }
            Ok(())
        })
    }

    /// Create an optional step (any input accepted).
    pub fn optional(name: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            prompt: prompt.into(),
            help: None,
            optional: true,
            validator: Box::new(|_| Ok(())),
        }
    }

    /// Set help text.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Validate user input against this step's rules.
    pub fn validate(&self, input: &str) -> Result<()> {
        if self.optional && input.trim().is_empty() {
            return Ok(());
        }
        (self.validator)(input)
    }
}

/// An interactive setup wizard that walks through steps.
pub struct SetupWizard {
    /// Wizard display name.
    pub name: String,
    /// Description of what this wizard sets up.
    pub description: String,
    /// Ordered steps.
    pub steps: Vec<SetupStep>,
    /// Current step index.
    current_step: usize,
    /// Collected answers keyed by step name.
    answers: std::collections::HashMap<String, String>,
}

impl SetupWizard {
    /// Create a new wizard.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        steps: Vec<SetupStep>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            steps,
            current_step: 0,
            answers: std::collections::HashMap::new(),
        }
    }

    /// Get the current step (if not finished).
    pub fn current(&self) -> Option<&SetupStep> {
        self.steps.get(self.current_step)
    }

    /// Get the current step index.
    pub fn current_index(&self) -> usize {
        self.current_step
    }

    /// Total number of steps.
    pub fn total_steps(&self) -> usize {
        self.steps.len()
    }

    /// Whether all steps have been completed.
    pub fn is_complete(&self) -> bool {
        self.current_step >= self.steps.len()
    }

    /// Progress as a fraction (0.0 to 1.0).
    pub fn progress(&self) -> f64 {
        if self.steps.is_empty() {
            return 1.0;
        }
        self.current_step as f64 / self.steps.len() as f64
    }

    /// Submit an answer for the current step.
    ///
    /// Validates the input and advances to the next step
    /// if validation passes.
    pub fn submit(&mut self, input: &str) -> Result<()> {
        let step = self
            .steps
            .get(self.current_step)
            .ok_or_else(|| anyhow::anyhow!("Wizard is already complete"))?;

        step.validate(input)?;

        self.answers.insert(step.name.clone(), input.to_string());
        self.current_step += 1;
        Ok(())
    }

    /// Skip the current step (only if optional).
    pub fn skip(&mut self) -> Result<()> {
        let step = self
            .steps
            .get(self.current_step)
            .ok_or_else(|| anyhow::anyhow!("Wizard is already complete"))?;

        if !step.optional {
            bail!("Step '{}' is required and cannot be skipped", step.name);
        }

        self.current_step += 1;
        Ok(())
    }

    /// Go back to the previous step.
    pub fn back(&mut self) -> bool {
        if self.current_step > 0 {
            self.current_step -= 1;
            true
        } else {
            false
        }
    }

    /// Get the answer for a specific step.
    pub fn get_answer(&self, step_name: &str) -> Option<&str> {
        self.answers.get(step_name).map(|s| s.as_str())
    }

    /// Get all collected answers.
    pub fn answers(&self) -> &std::collections::HashMap<String, String> {
        &self.answers
    }

    /// Reset the wizard to the first step.
    pub fn reset(&mut self) {
        self.current_step = 0;
        self.answers.clear();
    }
}

/// Preset wizard for Telegram bot setup.
pub fn telegram_setup() -> SetupWizard {
    SetupWizard::new(
        "Telegram Bot Setup",
        "Configure your Telegram bot integration",
        vec![
            SetupStep::new(
                "bot_token",
                "Enter your Telegram Bot Token (from @BotFather):",
                |input| {
                    if !input.contains(':') || input.len() < 20 {
                        bail!("Invalid bot token format. Get one from @BotFather.");
                    }
                    Ok(())
                },
            )
            .with_help("Create a bot via @BotFather on Telegram and paste the token here."),
            SetupStep::optional("webhook_url", "Webhook URL (leave empty for polling mode):")
                .with_help(
                    "If you have a public HTTPS URL, webhooks are more efficient than polling.",
                ),
            SetupStep::optional(
                "allowed_users",
                "Allowed user IDs (comma-separated, empty = allow all):",
            ),
        ],
    )
}

/// Preset wizard for Discord bot setup.
pub fn discord_setup() -> SetupWizard {
    SetupWizard::new(
        "Discord Bot Setup",
        "Configure your Discord bot integration",
        vec![
            SetupStep::new("bot_token", "Enter your Discord Bot Token:", |input| {
                if input.trim().len() < 30 {
                    bail!("Discord tokens are typically longer. Check the Developer Portal.");
                }
                Ok(())
            }),
            SetupStep::optional("guild_id", "Guild (server) ID (optional, empty = all guilds):"),
        ],
    )
}

/// Preset wizard for Slack app setup.
pub fn slack_setup() -> SetupWizard {
    SetupWizard::new(
        "Slack App Setup",
        "Configure your Slack app integration",
        vec![
            SetupStep::new("bot_token", "Enter your Slack Bot Token (xoxb-...):", |input| {
                if !input.starts_with("xoxb-") {
                    bail!("Slack bot tokens start with 'xoxb-'");
                }
                Ok(())
            }),
            SetupStep::new(
                "app_token",
                "Enter your Slack App Token (xapp-...) for Socket Mode:",
                |input| {
                    if !input.starts_with("xapp-") {
                        bail!("Slack app tokens start with 'xapp-'");
                    }
                    Ok(())
                },
            ),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_wizard() -> SetupWizard {
        SetupWizard::new(
            "Test",
            "Test wizard",
            vec![
                SetupStep::required("name", "Your name:"),
                SetupStep::optional("nickname", "Nickname:"),
                SetupStep::new("age", "Age:", |input| {
                    input.parse::<u32>().map_err(|_| anyhow::anyhow!("Must be a number"))?;
                    Ok(())
                }),
            ],
        )
    }

    #[test]
    fn test_wizard_flow() {
        let mut w = simple_wizard();
        assert!(!w.is_complete());
        assert_eq!(w.total_steps(), 3);
        assert_eq!(w.current_index(), 0);

        w.submit("Alice").expect("valid name");
        w.submit("ali").expect("optional ok");
        w.submit("30").expect("valid age");

        assert!(w.is_complete());
        assert_eq!(w.get_answer("name"), Some("Alice"));
        assert_eq!(w.get_answer("age"), Some("30"));
    }

    #[test]
    fn test_required_step_rejects_empty() {
        let mut w = simple_wizard();
        let result = w.submit("");
        assert!(result.is_err());
    }

    #[test]
    fn test_optional_step_accepts_empty() {
        let mut w = simple_wizard();
        w.submit("Alice").expect("name");
        w.submit("").expect("optional accepts empty");
    }

    #[test]
    fn test_skip_optional() {
        let mut w = simple_wizard();
        w.submit("Alice").expect("name");
        w.skip().expect("skip optional");
        assert_eq!(w.current_index(), 2);
    }

    #[test]
    fn test_skip_required_fails() {
        let mut w = simple_wizard();
        let result = w.skip();
        assert!(result.is_err());
    }

    #[test]
    fn test_back() {
        let mut w = simple_wizard();
        w.submit("Alice").expect("name");
        assert!(w.back());
        assert_eq!(w.current_index(), 0);
        assert!(!w.back()); // can't go before 0
    }

    #[test]
    fn test_validation() {
        let mut w = simple_wizard();
        w.submit("Alice").expect("name");
        w.skip().expect("optional");
        let result = w.submit("not-a-number");
        assert!(result.is_err());
    }

    #[test]
    fn test_progress() {
        let mut w = simple_wizard();
        assert!((w.progress() - 0.0).abs() < f64::EPSILON);
        w.submit("A").expect("ok");
        assert!((w.progress() - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_reset() {
        let mut w = simple_wizard();
        w.submit("Alice").expect("ok");
        w.reset();
        assert_eq!(w.current_index(), 0);
        assert!(w.answers().is_empty());
    }

    #[test]
    fn test_telegram_setup_token_validation() {
        let mut w = telegram_setup();
        let result = w.submit("bad");
        assert!(result.is_err());

        let result = w.submit("123456789:ABCdefGHIjklMNOpqrsTUVwxyz");
        assert!(result.is_ok());
    }

    #[test]
    fn test_slack_setup_token_validation() {
        let mut w = slack_setup();
        let result = w.submit("bad");
        assert!(result.is_err());

        let result = w.submit("xoxb-123456-789012-abcdefghijklmnopqrst");
        assert!(result.is_ok());
    }
}
