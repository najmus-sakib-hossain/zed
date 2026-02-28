//! # Skills System
//!
//! Skills define what the agent can do. They are defined in DX Serializer format
//! and can be dynamically added/updated. The agent uses skills to execute tasks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::Result;

/// A skill definition in DX Serializer format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    /// Unique name of the skill
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Required integrations to execute this skill
    pub required_integrations: Vec<String>,

    /// Input parameters
    pub inputs: Vec<SkillInput>,

    /// Output format
    pub output_format: OutputFormat,

    /// The action to perform (DX Serializer format)
    pub action: String,

    /// Examples for LLM to understand usage
    pub examples: Vec<SkillExample>,
}

/// Input parameter for a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInput {
    pub name: String,
    pub kind: String,
    pub required: bool,
    pub description: String,
}

/// Output format types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Text,
    Json,
    DxLlm,
    DxMachine,
}

/// Example usage of a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExample {
    pub input: String,
    pub output: String,
}

/// A skill that can be executed
pub struct Skill {
    definition: SkillDefinition,
}

impl Skill {
    pub fn new(definition: SkillDefinition) -> Self {
        Self { definition }
    }

    pub fn name(&self) -> &str {
        &self.definition.name
    }

    pub fn description(&self) -> &str {
        &self.definition.description
    }

    /// Execute the skill with the given context
    pub async fn execute(&self, context: &str) -> Result<String> {
        info!("Executing skill: {}", self.definition.name);

        // Parse the context and extract parameters
        // Then execute the action

        // Placeholder - real implementation would parse and execute
        Ok(format!(
            "Skill {} executed with context: {}",
            self.definition.name, context
        ))
    }

    /// Convert skill definition to DX LLM format for token-efficient storage
    pub fn to_dx_llm(&self) -> String {
        format!(
            "skill:1[name={} description={} inputs[{}]={} output={}]",
            self.definition.name,
            self.definition.description.replace(' ', "_"),
            self.definition.inputs.len(),
            self.definition
                .inputs
                .iter()
                .map(|i| format!("{}:{}", i.name, i.kind))
                .collect::<Vec<_>>()
                .join(" "),
            match self.definition.output_format {
                OutputFormat::Text => "text",
                OutputFormat::Json => "json",
                OutputFormat::DxLlm => "dx_llm",
                OutputFormat::DxMachine => "dx_machine",
            }
        )
    }
}

/// Registry of all available skills
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
    skills_path: PathBuf,
}

impl SkillRegistry {
    pub async fn new(skills_path: &Path) -> Result<Self> {
        std::fs::create_dir_all(skills_path)?;

        Ok(Self {
            skills: HashMap::new(),
            skills_path: skills_path.to_path_buf(),
        })
    }

    /// Load all skills from the skills directory
    pub async fn load_all(&mut self) -> Result<usize> {
        let mut count = 0;

        // Load built-in skills
        self.load_builtin_skills().await?;
        count += self.skills.len();

        // Load custom skills from .sr files
        if let Ok(entries) = std::fs::read_dir(&self.skills_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "sr") {
                    if let Ok(skill) = self.load_skill_from_file(&path).await {
                        let name = skill.name().to_string();
                        self.skills.insert(name, skill);
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    /// Load built-in skills
    async fn load_builtin_skills(&mut self) -> Result<()> {
        // Send Message skill
        self.skills.insert(
            "send_message".to_string(),
            Skill::new(SkillDefinition {
                name: "send_message".to_string(),
                description: "Send a message through any messaging integration".to_string(),
                required_integrations: vec!["messaging".to_string()],
                inputs: vec![
                    SkillInput {
                        name: "platform".to_string(),
                        kind: "string".to_string(),
                        required: true,
                        description: "The messaging platform (whatsapp, telegram, discord, etc.)"
                            .to_string(),
                    },
                    SkillInput {
                        name: "recipient".to_string(),
                        kind: "string".to_string(),
                        required: true,
                        description: "The recipient of the message".to_string(),
                    },
                    SkillInput {
                        name: "message".to_string(),
                        kind: "string".to_string(),
                        required: true,
                        description: "The message content".to_string(),
                    },
                ],
                output_format: OutputFormat::DxLlm,
                action: "integration.send_message(platform, recipient, message)".to_string(),
                examples: vec![SkillExample {
                    input: "send_message platform=whatsapp recipient=john message=Hello!"
                        .to_string(),
                    output: "message_sent:1[platform=whatsapp recipient=john status=delivered]"
                        .to_string(),
                }],
            }),
        );

        // Create Todo skill
        self.skills.insert(
            "create_todo".to_string(),
            Skill::new(SkillDefinition {
                name: "create_todo".to_string(),
                description: "Create a todo item in Notion or any productivity tool".to_string(),
                required_integrations: vec!["notion".to_string()],
                inputs: vec![
                    SkillInput {
                        name: "title".to_string(),
                        kind: "string".to_string(),
                        required: true,
                        description: "The todo title".to_string(),
                    },
                    SkillInput {
                        name: "due_date".to_string(),
                        kind: "date".to_string(),
                        required: false,
                        description: "Optional due date".to_string(),
                    },
                ],
                output_format: OutputFormat::DxLlm,
                action: "notion.create_page(database_id, {title, due_date})".to_string(),
                examples: vec![],
            }),
        );

        // Check Email skill
        self.skills.insert(
            "check_email".to_string(),
            Skill::new(SkillDefinition {
                name: "check_email".to_string(),
                description: "Check and summarize recent emails".to_string(),
                required_integrations: vec!["email".to_string()],
                inputs: vec![SkillInput {
                    name: "count".to_string(),
                    kind: "number".to_string(),
                    required: false,
                    description: "Number of emails to check (default: 10)".to_string(),
                }],
                output_format: OutputFormat::DxLlm,
                action: "email.list_recent(count)".to_string(),
                examples: vec![],
            }),
        );

        // Browse Web skill
        self.skills.insert(
            "browse_web".to_string(),
            Skill::new(SkillDefinition {
                name: "browse_web".to_string(),
                description: "Browse a webpage and extract content".to_string(),
                required_integrations: vec!["browser".to_string()],
                inputs: vec![SkillInput {
                    name: "url".to_string(),
                    kind: "string".to_string(),
                    required: true,
                    description: "The URL to browse".to_string(),
                }],
                output_format: OutputFormat::DxLlm,
                action: "browser.navigate(url).get_content()".to_string(),
                examples: vec![],
            }),
        );

        // Run Command skill
        self.skills.insert(
            "run_command".to_string(),
            Skill::new(SkillDefinition {
                name: "run_command".to_string(),
                description: "Run a shell command on the user's machine".to_string(),
                required_integrations: vec![],
                inputs: vec![SkillInput {
                    name: "command".to_string(),
                    kind: "string".to_string(),
                    required: true,
                    description: "The command to run".to_string(),
                }],
                output_format: OutputFormat::Text,
                action: "shell.execute(command)".to_string(),
                examples: vec![],
            }),
        );

        // Create Integration skill (the AGI feature!)
        self.skills.insert("create_integration".to_string(), Skill::new(SkillDefinition {
            name: "create_integration".to_string(),
            description: "Create a new integration by writing code in any language".to_string(),
            required_integrations: vec![],
            inputs: vec![
                SkillInput {
                    name: "name".to_string(),
                    kind: "string".to_string(),
                    required: true,
                    description: "Name of the new integration".to_string(),
                },
                SkillInput {
                    name: "language".to_string(),
                    kind: "string".to_string(),
                    required: true,
                    description: "Programming language (python, javascript, rust, go)".to_string(),
                },
                SkillInput {
                    name: "code".to_string(),
                    kind: "string".to_string(),
                    required: true,
                    description: "The source code for the integration".to_string(),
                },
            ],
            output_format: OutputFormat::DxLlm,
            action: "agent.create_integration(name, code, language)".to_string(),
            examples: vec![
                SkillExample {
                    input: "create_integration name=my_api language=python code=\"def handle(msg): return msg.upper()\"".to_string(),
                    output: "integration_created:1[name=my_api status=ready wasm_size=1024]".to_string(),
                },
            ],
        }));

        // Play Music skill
        self.skills.insert(
            "play_music".to_string(),
            Skill::new(SkillDefinition {
                name: "play_music".to_string(),
                description: "Control music playback on Spotify".to_string(),
                required_integrations: vec!["spotify".to_string()],
                inputs: vec![
                    SkillInput {
                        name: "action".to_string(),
                        kind: "string".to_string(),
                        required: true,
                        description: "Action: play, pause, next, search".to_string(),
                    },
                    SkillInput {
                        name: "query".to_string(),
                        kind: "string".to_string(),
                        required: false,
                        description: "Search query (for search action)".to_string(),
                    },
                ],
                output_format: OutputFormat::DxLlm,
                action: "spotify.execute(action, query)".to_string(),
                examples: vec![],
            }),
        );

        // Create PR skill
        self.skills.insert(
            "create_pr".to_string(),
            Skill::new(SkillDefinition {
                name: "create_pr".to_string(),
                description: "Create a pull request on GitHub".to_string(),
                required_integrations: vec!["github".to_string()],
                inputs: vec![
                    SkillInput {
                        name: "repo".to_string(),
                        kind: "string".to_string(),
                        required: true,
                        description: "Repository in format owner/repo".to_string(),
                    },
                    SkillInput {
                        name: "title".to_string(),
                        kind: "string".to_string(),
                        required: true,
                        description: "PR title".to_string(),
                    },
                    SkillInput {
                        name: "body".to_string(),
                        kind: "string".to_string(),
                        required: true,
                        description: "PR description".to_string(),
                    },
                ],
                output_format: OutputFormat::DxLlm,
                action: "github.create_pr(repo, title, body)".to_string(),
                examples: vec![],
            }),
        );

        Ok(())
    }

    /// Load a skill from a .sr file
    async fn load_skill_from_file(&self, path: &Path) -> Result<Skill> {
        let content = std::fs::read_to_string(path)?;

        // Parse DX Serializer format
        let mut definition = SkillDefinition {
            name: String::new(),
            description: String::new(),
            required_integrations: vec![],
            inputs: vec![],
            output_format: OutputFormat::DxLlm,
            action: String::new(),
            examples: vec![],
        };

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                match key.trim() {
                    "name" => definition.name = value.trim().to_string(),
                    "description" => definition.description = value.trim().to_string(),
                    "action" => definition.action = value.trim().to_string(),
                    _ => {}
                }
            }
        }

        Ok(Skill::new(definition))
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// Add a skill from DX format string
    pub async fn add_from_dx_format(&mut self, dx_format: &str) -> Result<()> {
        // Parse DX format: skill:1[name=x description=y ...]
        let definition = SkillDefinition {
            name: "custom_skill".to_string(),
            description: "Custom skill added from DX format".to_string(),
            required_integrations: vec![],
            inputs: vec![],
            output_format: OutputFormat::DxLlm,
            action: dx_format.to_string(),
            examples: vec![],
        };

        let skill = Skill::new(definition);
        self.skills.insert(skill.name().to_string(), skill);

        Ok(())
    }

    /// List all available skills in DX LLM format (for efficient LLM context)
    pub fn list_as_dx_llm(&self) -> String {
        let skills: Vec<String> = self.skills.values().map(|s| s.to_dx_llm()).collect();

        format!("skills:{}[{}]", skills.len(), skills.join(" "))
    }

    /// Get count of skills
    pub fn count(&self) -> usize {
        self.skills.len()
    }
}
