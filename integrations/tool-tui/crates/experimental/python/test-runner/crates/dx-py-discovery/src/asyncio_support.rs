//! pytest-asyncio support for async test functions
//!
//! This module provides:
//! - Detection of async test functions
//! - pytest-asyncio mode configuration
//! - Event loop fixture handling

use dx_py_core::{Marker, TestCase};
use serde::{Deserialize, Serialize};

/// pytest-asyncio mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AsyncioMode {
    /// Strict mode - only tests marked with @pytest.mark.asyncio are async
    #[default]
    Strict,
    /// Auto mode - all async def tests are automatically async
    Auto,
}

/// Configuration for pytest-asyncio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncioConfig {
    /// The asyncio mode
    pub mode: AsyncioMode,
    /// Default event loop scope
    pub default_loop_scope: EventLoopScope,
}

impl Default for AsyncioConfig {
    fn default() -> Self {
        Self {
            mode: AsyncioMode::Strict,
            default_loop_scope: EventLoopScope::Function,
        }
    }
}

/// Event loop scope for async fixtures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EventLoopScope {
    /// New event loop per test function
    #[default]
    Function,
    /// Shared event loop per test class
    Class,
    /// Shared event loop per module
    Module,
    /// Shared event loop per session
    Session,
}

/// Async test detector
pub struct AsyncTestDetector {
    config: AsyncioConfig,
}

impl AsyncTestDetector {
    /// Create a new async test detector
    pub fn new(config: AsyncioConfig) -> Self {
        Self { config }
    }

    /// Check if a test is an async test
    pub fn is_async_test(&self, test: &TestCase) -> bool {
        // Check for explicit @pytest.mark.asyncio marker
        let has_asyncio_marker = test
            .markers
            .iter()
            .any(|m| m.name == "pytest.mark.asyncio" || m.name == "asyncio");

        match self.config.mode {
            AsyncioMode::Strict => has_asyncio_marker,
            AsyncioMode::Auto => {
                // In auto mode, we'd need to check if the function is async def
                // For now, we rely on the marker or function name pattern
                has_asyncio_marker || test.name.starts_with("test_async_")
            }
        }
    }

    /// Get the event loop scope for a test
    pub fn get_loop_scope(&self, test: &TestCase) -> EventLoopScope {
        // Check for scope in marker
        for marker in &test.markers {
            if marker.name == "pytest.mark.asyncio" || marker.name == "asyncio" {
                for arg in &marker.args {
                    if arg.contains("scope=") {
                        if arg.contains("class") {
                            return EventLoopScope::Class;
                        } else if arg.contains("module") {
                            return EventLoopScope::Module;
                        } else if arg.contains("session") {
                            return EventLoopScope::Session;
                        }
                    }
                }
            }
        }

        self.config.default_loop_scope
    }

    /// Check if a fixture needs an event loop
    pub fn fixture_needs_event_loop(&self, fixture_name: &str, markers: &[Marker]) -> bool {
        // Check for @pytest.fixture with async
        markers.iter().any(|m| {
            (m.name == "fixture" || m.name == "pytest.fixture")
                && m.args.iter().any(|a| a.contains("async"))
        }) || fixture_name == "event_loop"
    }
}

impl Default for AsyncTestDetector {
    fn default() -> Self {
        Self::new(AsyncioConfig::default())
    }
}

/// Parse asyncio mode from pyproject.toml or pytest.ini
#[allow(dead_code)]
pub fn parse_asyncio_mode(config_value: &str) -> AsyncioMode {
    match config_value.to_lowercase().as_str() {
        "auto" => AsyncioMode::Auto,
        "strict" => AsyncioMode::Strict,
        _ => AsyncioMode::Strict,
    }
}

/// Parse event loop scope from string
#[allow(dead_code)]
pub fn parse_loop_scope(scope_value: &str) -> EventLoopScope {
    match scope_value.to_lowercase().as_str() {
        "function" => EventLoopScope::Function,
        "class" => EventLoopScope::Class,
        "module" => EventLoopScope::Module,
        "session" => EventLoopScope::Session,
        _ => EventLoopScope::Function,
    }
}
