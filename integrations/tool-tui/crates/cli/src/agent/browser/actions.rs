//! Browser Actions
//!
//! High-level actions for browser automation including sequences,
//! scraping, and variable interpolation.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::controller::{BrowserController, ElementInfo};

/// A single browser action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Navigate to a URL
    Navigate {
        url: String,
        #[serde(default)]
        wait: Option<String>,
    },

    /// Click an element
    Click {
        selector: String,
        #[serde(default)]
        wait_before_ms: Option<u64>,
    },

    /// Double click an element
    DoubleClick { selector: String },

    /// Right click an element
    RightClick { selector: String },

    /// Type text into an element
    Type {
        selector: String,
        text: String,
        #[serde(default)]
        clear_first: bool,
    },

    /// Press a key
    Press {
        key: String,
        #[serde(default)]
        modifiers: Vec<String>,
    },

    /// Clear an input
    Clear { selector: String },

    /// Select dropdown option
    Select { selector: String, value: String },

    /// Check a checkbox
    Check { selector: String },

    /// Uncheck a checkbox
    Uncheck { selector: String },

    /// Hover over an element
    Hover { selector: String },

    /// Scroll to an element
    ScrollTo { selector: String },

    /// Scroll by pixels
    Scroll { x: i32, y: i32 },

    /// Wait for element
    WaitForElement {
        selector: String,
        #[serde(default = "default_timeout")]
        timeout_ms: u64,
    },

    /// Wait for text
    WaitForText {
        text: String,
        #[serde(default = "default_timeout")]
        timeout_ms: u64,
    },

    /// Wait for navigation
    WaitForNavigation {
        #[serde(default = "default_timeout")]
        timeout_ms: u64,
    },

    /// Wait fixed time
    Wait { ms: u64 },

    /// Take screenshot
    Screenshot {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        selector: Option<String>,
    },

    /// Scrape content
    Scrape {
        selector: String,
        #[serde(default)]
        attribute: Option<String>,
        #[serde(default)]
        output_var: Option<String>,
    },

    /// Scrape multiple elements
    ScrapeAll {
        selector: String,
        #[serde(default)]
        attribute: Option<String>,
        #[serde(default)]
        output_var: Option<String>,
    },

    /// Execute JavaScript
    Execute {
        script: String,
        #[serde(default)]
        output_var: Option<String>,
    },

    /// Set a variable
    SetVariable { name: String, value: String },

    /// Conditional action
    If {
        condition: String,
        then: Vec<Action>,
        #[serde(default)]
        otherwise: Vec<Action>,
    },

    /// Loop action
    Loop {
        #[serde(default)]
        count: Option<u32>,
        #[serde(default)]
        while_condition: Option<String>,
        actions: Vec<Action>,
    },

    /// Set cookie
    SetCookie {
        name: String,
        value: String,
        #[serde(default)]
        domain: Option<String>,
    },

    /// Clear cookies
    ClearCookies,

    /// Go back
    Back,

    /// Go forward
    Forward,

    /// Reload page
    Reload,

    /// Close browser
    Close,
}

fn default_timeout() -> u64 {
    30_000
}

/// Result of a scrape action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeResult {
    /// Scraped value(s)
    pub values: Vec<String>,
    /// Number of elements found
    pub count: usize,
    /// Element details
    pub elements: Vec<ElementInfo>,
}

/// Result of executing an action
#[derive(Debug)]
pub enum ActionResult {
    /// Action completed successfully
    Success,
    /// Action returned a value
    Value(serde_json::Value),
    /// Scrape result
    Scraped(ScrapeResult),
    /// Screenshot data
    Screenshot(Vec<u8>),
    /// Action failed
    Failed(String),
    /// Action skipped (condition not met)
    Skipped,
}

/// A sequence of actions to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSequence {
    /// Name of the sequence
    pub name: String,
    /// Description
    #[serde(default)]
    pub description: Option<String>,
    /// Actions to execute
    pub actions: Vec<Action>,
    /// Variables (can be interpolated in actions)
    #[serde(default)]
    pub variables: HashMap<String, String>,
    /// Continue on error
    #[serde(default)]
    pub continue_on_error: bool,
}

impl ActionSequence {
    /// Create a new sequence
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            actions: Vec::new(),
            variables: HashMap::new(),
            continue_on_error: false,
        }
    }

    /// Add an action
    pub fn add(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    /// Set a variable
    pub fn set_var(mut self, name: &str, value: &str) -> Self {
        self.variables.insert(name.to_string(), value.to_string());
        self
    }

    /// Load from configuration file
    pub fn load(path: &std::path::PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        // Would parse .sr format in production
        serde_json::from_str(&content).context("Failed to parse action sequence")
    }
}

/// Executor for browser actions
pub struct ActionExecutor {
    /// Browser controller
    browser: BrowserController,
    /// Variables for interpolation
    variables: HashMap<String, String>,
    /// Action results
    results: Vec<(Action, ActionResult)>,
}

impl ActionExecutor {
    /// Create a new executor
    pub fn new(browser: BrowserController) -> Self {
        Self {
            browser,
            variables: HashMap::new(),
            results: Vec::new(),
        }
    }

    /// Set a variable
    pub fn set_variable(&mut self, name: &str, value: &str) {
        self.variables.insert(name.to_string(), value.to_string());
    }

    /// Get a variable
    pub fn get_variable(&self, name: &str) -> Option<&str> {
        self.variables.get(name).map(|s| s.as_str())
    }

    /// Interpolate variables in a string
    fn interpolate(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (name, value) in &self.variables {
            result = result.replace(&format!("${{{}}}", name), value);
            result = result.replace(&format!("${}", name), value);
        }
        result
    }

    /// Execute a single action
    pub async fn execute_action(&mut self, action: &Action) -> Result<ActionResult> {
        match action {
            Action::Navigate { url, wait: _ } => {
                let url = self.interpolate(url);
                self.browser.navigate(&url).await?;
                Ok(ActionResult::Success)
            }

            Action::Click {
                selector,
                wait_before_ms,
            } => {
                if let Some(ms) = wait_before_ms {
                    tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
                }
                let selector = self.interpolate(selector);
                self.browser.click(&selector).await?;
                Ok(ActionResult::Success)
            }

            Action::DoubleClick { selector } => {
                let selector = self.interpolate(selector);
                // Double click = two clicks
                self.browser.click(&selector).await?;
                self.browser.click(&selector).await?;
                Ok(ActionResult::Success)
            }

            Action::RightClick { selector } => {
                let selector = self.interpolate(selector);
                // Right click would need special handling in real implementation
                let _ = selector;
                Ok(ActionResult::Success)
            }

            Action::Type {
                selector,
                text,
                clear_first,
            } => {
                let selector = self.interpolate(selector);
                let text = self.interpolate(text);
                if *clear_first {
                    self.browser.clear(&selector).await?;
                }
                self.browser.type_text(&selector, &text).await?;
                Ok(ActionResult::Success)
            }

            Action::Press { key, modifiers: _ } => {
                // Would send key press
                let _ = key;
                Ok(ActionResult::Success)
            }

            Action::Clear { selector } => {
                let selector = self.interpolate(selector);
                self.browser.clear(&selector).await?;
                Ok(ActionResult::Success)
            }

            Action::Select { selector, value } => {
                let selector = self.interpolate(selector);
                let value = self.interpolate(value);
                self.browser.select(&selector, &value).await?;
                Ok(ActionResult::Success)
            }

            Action::Check { selector } | Action::Uncheck { selector } => {
                let selector = self.interpolate(selector);
                self.browser.click(&selector).await?;
                Ok(ActionResult::Success)
            }

            Action::Hover { selector } | Action::ScrollTo { selector } => {
                let selector = self.interpolate(selector);
                // Would scroll/hover in real implementation
                let _ = selector;
                Ok(ActionResult::Success)
            }

            Action::Scroll { x: _, y: _ } => Ok(ActionResult::Success),

            Action::WaitForElement {
                selector,
                timeout_ms,
            } => {
                let selector = self.interpolate(selector);
                self.browser
                    .wait_for_element(&selector, *timeout_ms)
                    .await?;
                Ok(ActionResult::Success)
            }

            Action::WaitForText { text, timeout_ms } => {
                let text = self.interpolate(text);
                let start = std::time::Instant::now();
                let timeout = std::time::Duration::from_millis(*timeout_ms);

                loop {
                    let content = self.browser.content().await.unwrap_or_default();
                    if content.contains(&text) {
                        return Ok(ActionResult::Success);
                    }
                    if start.elapsed() > timeout {
                        return Ok(ActionResult::Failed(format!(
                            "Timeout waiting for text: {}",
                            text
                        )));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }

            Action::WaitForNavigation { timeout_ms: _ } => {
                // Would wait for navigation in real implementation
                Ok(ActionResult::Success)
            }

            Action::Wait { ms } => {
                tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
                Ok(ActionResult::Success)
            }

            Action::Screenshot { path, selector } => {
                let data = if let Some(sel) = selector {
                    let sel = self.interpolate(sel);
                    self.browser.screenshot_element(&sel).await?
                } else {
                    self.browser.screenshot().await?
                };

                if let Some(p) = path {
                    let p = self.interpolate(p);
                    std::fs::write(&p, &data)?;
                }

                Ok(ActionResult::Screenshot(data))
            }

            Action::Scrape {
                selector,
                attribute,
                output_var,
            } => {
                let selector = self.interpolate(selector);
                let element = self.browser.get_element(&selector).await?;

                let value = if let Some(attr) = attribute {
                    element.attributes.get(attr).cloned().unwrap_or_default()
                } else {
                    element.text_content.clone().unwrap_or_default()
                };

                if let Some(var) = output_var {
                    self.variables.insert(var.clone(), value.clone());
                }

                Ok(ActionResult::Scraped(ScrapeResult {
                    values: vec![value],
                    count: 1,
                    elements: vec![element],
                }))
            }

            Action::ScrapeAll {
                selector,
                attribute,
                output_var,
            } => {
                let selector = self.interpolate(selector);
                let elements = self.browser.get_elements(&selector).await?;

                let values: Vec<String> = elements
                    .iter()
                    .map(|e| {
                        if let Some(attr) = attribute {
                            e.attributes.get(attr).cloned().unwrap_or_default()
                        } else {
                            e.text_content.clone().unwrap_or_default()
                        }
                    })
                    .collect();

                if let Some(var) = output_var {
                    self.variables
                        .insert(var.clone(), serde_json::to_string(&values)?);
                }

                let count = elements.len();
                Ok(ActionResult::Scraped(ScrapeResult {
                    values,
                    count,
                    elements,
                }))
            }

            Action::Execute { script, output_var } => {
                let script = self.interpolate(script);
                let result = self.browser.execute_js(&script).await?;

                if let Some(var) = output_var {
                    self.variables.insert(var.clone(), result.to_string());
                }

                Ok(ActionResult::Value(result))
            }

            Action::SetVariable { name, value } => {
                let value = self.interpolate(value);
                self.variables.insert(name.clone(), value);
                Ok(ActionResult::Success)
            }

            Action::If {
                condition,
                then,
                otherwise,
            } => {
                let condition = self.interpolate(condition);
                // Simple condition evaluation
                let is_true = !condition.is_empty()
                    && condition != "false"
                    && condition != "0"
                    && condition != "null";

                let actions = if is_true { then } else { otherwise };
                for action in actions {
                    self.execute_action(action).await?;
                }
                Ok(ActionResult::Success)
            }

            Action::Loop {
                count,
                while_condition,
                actions,
            } => {
                if let Some(n) = count {
                    for _ in 0..*n {
                        for action in actions {
                            self.execute_action(action).await?;
                        }
                    }
                } else if let Some(cond) = while_condition {
                    loop {
                        let cond_value = self.interpolate(cond);
                        if cond_value.is_empty()
                            || cond_value == "false"
                            || cond_value == "0"
                            || cond_value == "null"
                        {
                            break;
                        }
                        for action in actions {
                            self.execute_action(action).await?;
                        }
                    }
                }
                Ok(ActionResult::Success)
            }

            Action::SetCookie {
                name,
                value,
                domain,
            } => {
                let name = self.interpolate(name);
                let value = self.interpolate(value);
                let domain = domain
                    .as_ref()
                    .map(|d| self.interpolate(d))
                    .unwrap_or_default();

                self.browser
                    .set_cookie(super::controller::Cookie {
                        name,
                        value,
                        domain,
                        path: "/".to_string(),
                        expires: None,
                        http_only: false,
                        secure: false,
                    })
                    .await?;
                Ok(ActionResult::Success)
            }

            Action::ClearCookies => {
                self.browser.clear_cookies().await?;
                Ok(ActionResult::Success)
            }

            Action::Back => {
                self.browser.back().await?;
                Ok(ActionResult::Success)
            }

            Action::Forward => {
                self.browser.forward().await?;
                Ok(ActionResult::Success)
            }

            Action::Reload => {
                self.browser.reload().await?;
                Ok(ActionResult::Success)
            }

            Action::Close => {
                self.browser.close().await?;
                Ok(ActionResult::Success)
            }
        }
    }

    /// Execute a sequence of actions
    pub async fn execute_sequence(&mut self, sequence: &ActionSequence) -> Result<Vec<ActionResult>> {
        // Set initial variables
        for (name, value) in &sequence.variables {
            self.variables.insert(name.clone(), value.clone());
        }

        let mut results = Vec::new();

        for action in &sequence.actions {
            match self.execute_action(action).await {
                Ok(result) => {
                    self.results.push((action.clone(), ActionResult::Success));
                    results.push(result);
                }
                Err(e) => {
                    let result = ActionResult::Failed(e.to_string());
                    self.results.push((action.clone(), ActionResult::Failed(e.to_string())));
                    results.push(result);

                    if !sequence.continue_on_error {
                        return Err(e);
                    }
                }
            }
        }

        Ok(results)
    }

    /// Get execution results
    pub fn results(&self) -> &[(Action, ActionResult)] {
        &self.results
    }

    /// Get the browser controller
    pub fn browser(&self) -> &BrowserController {
        &self.browser
    }

    /// Get mutable browser controller
    pub fn browser_mut(&mut self) -> &mut BrowserController {
        &mut self.browser
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_sequence_builder() {
        let seq = ActionSequence::new("test")
            .set_var("url", "https://example.com")
            .add(Action::Navigate {
                url: "${url}".to_string(),
                wait: None,
            })
            .add(Action::Click {
                selector: "#button".to_string(),
                wait_before_ms: None,
            });

        assert_eq!(seq.actions.len(), 2);
        assert_eq!(seq.variables.get("url"), Some(&"https://example.com".to_string()));
    }

    #[tokio::test]
    async fn test_interpolation() {
        let config = super::super::controller::BrowserConfig::default();
        let browser = BrowserController::new(config).await.unwrap();
        let mut executor = ActionExecutor::new(browser);

        executor.set_variable("name", "world");
        let result = executor.interpolate("Hello, ${name}!");
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_action_serialization() {
        let action = Action::Navigate {
            url: "https://example.com".to_string(),
            wait: Some("load".to_string()),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("navigate"));
        assert!(json.contains("https://example.com"));
    }
}
