//! Example Browser Automation Script for DX
//!
//! This demonstrates how to use DX's browser automation capabilities
//! for web scraping, testing, and automated interactions.
//!
//! # Features
//!
//! - Headless/headed browser control
//! - Page navigation and waiting
//! - Element interaction (click, type, select)
//! - Screenshot and PDF generation
//! - Cookie and session management
//! - Config-driven automation via browser.sr

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// Browser Automation Types
// ============================================================================

/// Browser configuration
#[derive(Debug, Clone)]
pub struct BrowserConfig {
    /// Run in headless mode (no visible window)
    pub headless: bool,
    /// Browser executable path (auto-detect if None)
    pub executable_path: Option<PathBuf>,
    /// User agent string
    pub user_agent: Option<String>,
    /// Viewport width
    pub viewport_width: u32,
    /// Viewport height
    pub viewport_height: u32,
    /// Default timeout for operations
    pub timeout: Duration,
    /// Enable JavaScript
    pub javascript: bool,
    /// Accept insecure certificates
    pub accept_insecure_certs: bool,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            executable_path: None,
            user_agent: None,
            viewport_width: 1920,
            viewport_height: 1080,
            timeout: Duration::from_secs(30),
            javascript: true,
            accept_insecure_certs: false,
        }
    }
}

/// Browser action that can be executed
#[derive(Debug, Clone)]
pub enum BrowserAction {
    /// Navigate to a URL
    Navigate { url: String },
    /// Wait for a selector to appear
    WaitFor { selector: String, timeout_ms: u64 },
    /// Click an element
    Click { selector: String },
    /// Type text into an element
    Type { selector: String, text: String },
    /// Select an option from a dropdown
    Select { selector: String, value: String },
    /// Take a screenshot
    Screenshot { path: PathBuf, full_page: bool },
    /// Generate PDF
    Pdf { path: PathBuf },
    /// Execute JavaScript
    Evaluate { script: String },
    /// Wait for navigation to complete
    WaitNavigation,
    /// Wait for a fixed duration
    Sleep { duration_ms: u64 },
    /// Scroll to an element
    ScrollTo { selector: String },
    /// Press a keyboard key
    KeyPress { key: String },
    /// Get text content of an element
    GetText { selector: String, variable: String },
    /// Get attribute of an element
    GetAttr { selector: String, attr: String, variable: String },
    /// Set a cookie
    SetCookie { name: String, value: String, domain: Option<String> },
    /// Clear cookies
    ClearCookies,
    /// Assert text matches
    AssertText { selector: String, expected: String },
    /// Assert element exists
    AssertExists { selector: String },
}

/// Result of a browser action
#[derive(Debug, Clone)]
pub enum ActionResult {
    Success,
    Text(String),
    Data(HashMap<String, String>),
    Screenshot(Vec<u8>),
    Error(String),
}

// ============================================================================
// Browser Automation Script
// ============================================================================

/// A browser automation script
pub struct BrowserScript {
    name: String,
    actions: Vec<BrowserAction>,
    variables: HashMap<String, String>,
    config: BrowserConfig,
}

impl BrowserScript {
    /// Create a new script
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            actions: Vec::new(),
            variables: HashMap::new(),
            config: BrowserConfig::default(),
        }
    }

    /// Set configuration
    pub fn with_config(mut self, config: BrowserConfig) -> Self {
        self.config = config;
        self
    }

    /// Set a variable
    pub fn set_var(&mut self, name: &str, value: &str) {
        self.variables.insert(name.to_string(), value.to_string());
    }

    /// Add an action to the script
    pub fn add(&mut self, action: BrowserAction) -> &mut Self {
        self.actions.push(action);
        self
    }

    /// Navigate to a URL
    pub fn navigate(&mut self, url: &str) -> &mut Self {
        self.add(BrowserAction::Navigate { url: self.interpolate(url) })
    }

    /// Wait for a selector
    pub fn wait_for(&mut self, selector: &str) -> &mut Self {
        self.add(BrowserAction::WaitFor {
            selector: selector.to_string(),
            timeout_ms: 10000,
        })
    }

    /// Click an element
    pub fn click(&mut self, selector: &str) -> &mut Self {
        self.add(BrowserAction::Click { selector: selector.to_string() })
    }

    /// Type text into an element
    pub fn type_text(&mut self, selector: &str, text: &str) -> &mut Self {
        self.add(BrowserAction::Type {
            selector: selector.to_string(),
            text: self.interpolate(text),
        })
    }

    /// Take a screenshot
    pub fn screenshot(&mut self, path: &str, full_page: bool) -> &mut Self {
        self.add(BrowserAction::Screenshot {
            path: PathBuf::from(path),
            full_page,
        })
    }

    /// Execute JavaScript
    pub fn evaluate(&mut self, script: &str) -> &mut Self {
        self.add(BrowserAction::Evaluate { script: script.to_string() })
    }

    /// Sleep for a duration
    pub fn sleep(&mut self, ms: u64) -> &mut Self {
        self.add(BrowserAction::Sleep { duration_ms: ms })
    }

    /// Get text and store in variable
    pub fn get_text(&mut self, selector: &str, variable: &str) -> &mut Self {
        self.add(BrowserAction::GetText {
            selector: selector.to_string(),
            variable: variable.to_string(),
        })
    }

    /// Assert text matches
    pub fn assert_text(&mut self, selector: &str, expected: &str) -> &mut Self {
        self.add(BrowserAction::AssertText {
            selector: selector.to_string(),
            expected: self.interpolate(expected),
        })
    }

    /// Interpolate variables in a string
    fn interpolate(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (key, value) in &self.variables {
            result = result.replace(&format!("${{{}}}", key), value);
        }
        result
    }

    /// Get all actions
    pub fn actions(&self) -> &[BrowserAction] {
        &self.actions
    }
}

// ============================================================================
// Example Scripts
// ============================================================================

/// Example: Login to GitHub
pub fn github_login_script() -> BrowserScript {
    let mut script = BrowserScript::new("GitHub Login");
    
    script
        .navigate("https://github.com/login")
        .wait_for("#login_field")
        .type_text("#login_field", "${GITHUB_USERNAME}")
        .type_text("#password", "${GITHUB_PASSWORD}")
        .click("input[type='submit']")
        .wait_for(".dashboard")
        .screenshot("github_dashboard.png", false);

    script
}

/// Example: Scrape Hacker News
pub fn hacker_news_scraper() -> BrowserScript {
    let mut script = BrowserScript::new("Hacker News Scraper");
    
    script
        .navigate("https://news.ycombinator.com")
        .wait_for(".itemlist")
        .evaluate(r#"
            Array.from(document.querySelectorAll('.athing')).slice(0, 10).map(row => {
                const title = row.querySelector('.titleline a');
                const subtext = row.nextElementSibling;
                const score = subtext?.querySelector('.score')?.textContent;
                return {
                    title: title?.textContent,
                    url: title?.href,
                    score: score || '0 points'
                };
            });
        "#)
        .screenshot("hackernews.png", true);

    script
}

/// Example: Form submission
pub fn contact_form_script() -> BrowserScript {
    let mut script = BrowserScript::new("Contact Form");
    
    script
        .navigate("${FORM_URL}")
        .wait_for("form")
        .type_text("input[name='name']", "${USER_NAME}")
        .type_text("input[name='email']", "${USER_EMAIL}")
        .type_text("textarea[name='message']", "${MESSAGE}")
        .click("button[type='submit']")
        .wait_for(".success-message")
        .assert_text(".success-message", "Thank you")
        .screenshot("form_submitted.png", false);

    script
}

/// Example: E-commerce checkout test
pub fn checkout_test_script() -> BrowserScript {
    let mut script = BrowserScript::new("E-Commerce Checkout Test");
    
    script
        // Visit product page
        .navigate("${STORE_URL}/products/${PRODUCT_ID}")
        .wait_for(".product-title")
        .get_text(".product-title", "product_name")
        .get_text(".product-price", "price")
        
        // Add to cart
        .click(".add-to-cart")
        .wait_for(".cart-count")
        .assert_text(".cart-count", "1")
        
        // Go to cart
        .click(".cart-icon")
        .wait_for(".cart-items")
        .screenshot("cart.png", false)
        
        // Proceed to checkout
        .click(".checkout-button")
        .wait_for("#checkout-form")
        
        // Fill shipping info
        .type_text("#email", "${CUSTOMER_EMAIL}")
        .type_text("#name", "${CUSTOMER_NAME}")
        .type_text("#address", "${CUSTOMER_ADDRESS}")
        .type_text("#city", "${CUSTOMER_CITY}")
        .type_text("#zip", "${CUSTOMER_ZIP}")
        
        // Fill payment (test card)
        .type_text("#card-number", "4242424242424242")
        .type_text("#card-expiry", "12/25")
        .type_text("#card-cvc", "123")
        
        .screenshot("checkout_filled.png", false)
        
        // Submit (dry run - don't actually submit in test)
        // .click("#place-order")
        // .wait_for(".order-confirmation")
        ;

    script
}

/// Example: Screenshot comparison
pub fn visual_regression_script() -> BrowserScript {
    let mut script = BrowserScript::new("Visual Regression");
    
    let pages = [
        ("/", "homepage"),
        ("/about", "about"),
        ("/products", "products"),
        ("/contact", "contact"),
    ];

    for (path, name) in pages {
        script
            .navigate(&format!("${{BASE_URL}}{}", path))
            .wait_for("body")
            .sleep(500) // Wait for animations
            .screenshot(&format!("screenshots/{}.png", name), true);
    }

    script
}

// ============================================================================
// Browser.sr Configuration Example
// ============================================================================

// ```sr
// # .dx/browser.sr
// 
// [browser]
// headless = true
// viewport_width = 1920
// viewport_height = 1080
// timeout_ms = 30000
// javascript = true
// 
// [browser.user_agent]
// # Override user agent
// value = "Mozilla/5.0 (DX Browser Automation)"
// 
// [scripts.github_login]
// name = "GitHub Login"
// description = "Log into GitHub account"
// 
// [[scripts.github_login.steps]]
// action = "navigate"
// url = "https://github.com/login"
// 
// [[scripts.github_login.steps]]
// action = "wait_for"
// selector = "#login_field"
// 
// [[scripts.github_login.steps]]
// action = "type"
// selector = "#login_field"
// text = "${GITHUB_USERNAME}"
// 
// [[scripts.github_login.steps]]
// action = "type"
// selector = "#password"
// text = "${GITHUB_PASSWORD}"
// 
// [[scripts.github_login.steps]]
// action = "click"
// selector = "input[type='submit']"
// 
// [[scripts.github_login.steps]]
// action = "wait_for"
// selector = ".dashboard"
// 
// [[scripts.github_login.steps]]
// action = "screenshot"
// path = "github_dashboard.png"
// full_page = false
// 
// [scripts.hackernews]
// name = "Hacker News Scraper"
// description = "Scrape top stories from Hacker News"
// 
// [[scripts.hackernews.steps]]
// action = "navigate"
// url = "https://news.ycombinator.com"
// 
// [[scripts.hackernews.steps]]
// action = "evaluate"
// script = """
// Array.from(document.querySelectorAll('.athing')).slice(0, 10).map(row => ({
//     title: row.querySelector('.titleline a')?.textContent,
//     url: row.querySelector('.titleline a')?.href,
// }));
// """
// 
// [cookies]
// # Pre-load cookies for authenticated sessions
// [[cookies.entries]]
// name = "session"
// value = "${SESSION_TOKEN}"
// domain = ".example.com"
// ```

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_creation() {
        let script = BrowserScript::new("test");
        assert_eq!(script.actions().len(), 0);
    }

    #[test]
    fn test_script_actions() {
        let mut script = BrowserScript::new("test");
        script
            .navigate("https://example.com")
            .wait_for("body")
            .click("button");

        assert_eq!(script.actions().len(), 3);
    }

    #[test]
    fn test_variable_interpolation() {
        let mut script = BrowserScript::new("test");
        script.set_var("URL", "https://example.com");
        script.navigate("${URL}/page");

        if let BrowserAction::Navigate { url } = &script.actions()[0] {
            assert_eq!(url, "https://example.com/page");
        } else {
            panic!("Expected Navigate action");
        }
    }

    #[test]
    fn test_github_login_script() {
        let script = github_login_script();
        assert_eq!(script.actions().len(), 7);
    }

    #[test]
    fn test_browser_config_default() {
        let config = BrowserConfig::default();
        assert!(config.headless);
        assert_eq!(config.viewport_width, 1920);
        assert_eq!(config.viewport_height, 1080);
        assert!(config.javascript);
    }
}
