//! Browser control via Chrome DevTools Protocol (CDP).
//!
//! Pure Rust implementation using chromiumoxide for
//! headless and headed Chrome/Chromium automation.

pub mod controller;
pub mod screenshot;

#[cfg(feature = "webdriver")]
pub mod webdriver;

pub use controller::{BrowserConfig, BrowserController, BrowserCookie, BrowserKind};

#[cfg(feature = "webdriver")]
pub use webdriver::{WebDriverConfig, WebDriverController};
