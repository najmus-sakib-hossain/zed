//! Marker expression evaluator
//!
//! Evaluates PEP 508 marker expressions against an environment.

use super::{MarkerCache, MarkerError, MarkerExpr, MarkerOp, MarkerParser, MarkerValue};
use crate::runtime::PythonRuntime;
use serde::{Deserialize, Serialize};

/// Environment for marker evaluation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkerEnvironment {
    /// os.name (e.g., "posix", "nt")
    pub os_name: String,
    /// sys.platform (e.g., "linux", "win32", "darwin")
    pub sys_platform: String,
    /// platform.machine() (e.g., "x86_64", "arm64")
    pub platform_machine: String,
    /// platform.python_implementation() (e.g., "CPython", "PyPy")
    pub platform_python_implementation: String,
    /// platform.release()
    pub platform_release: String,
    /// platform.system() (e.g., "Linux", "Windows", "Darwin")
    pub platform_system: String,
    /// platform.version()
    pub platform_version: String,
    /// Python version (e.g., "3.12")
    pub python_version: String,
    /// Full Python version (e.g., "3.12.0")
    pub python_full_version: String,
    /// Implementation name (e.g., "cpython")
    pub implementation_name: String,
    /// Implementation version
    pub implementation_version: String,
    /// Extra marker (for optional dependencies)
    pub extra: String,
}

impl Default for MarkerEnvironment {
    fn default() -> Self {
        Self::current()
    }
}

impl MarkerEnvironment {
    /// Create from current system
    pub fn current() -> Self {
        Self {
            os_name: Self::detect_os_name(),
            sys_platform: Self::detect_sys_platform(),
            platform_machine: Self::detect_platform_machine(),
            platform_python_implementation: "CPython".to_string(),
            platform_release: Self::detect_platform_release(),
            platform_system: Self::detect_platform_system(),
            platform_version: String::new(),
            python_version: "3.12".to_string(),
            python_full_version: "3.12.0".to_string(),
            implementation_name: "cpython".to_string(),
            implementation_version: "3.12.0".to_string(),
            extra: String::new(),
        }
    }

    /// Create from a specific Python runtime
    pub fn from_runtime(runtime: &PythonRuntime) -> Self {
        let mut env = Self::current();
        env.python_version = format!("{}.{}", runtime.version.major, runtime.version.minor);
        env.python_full_version = runtime.version.to_string();
        env.implementation_version = runtime.version.to_string();
        env
    }

    /// Set the extra marker
    pub fn with_extra(mut self, extra: impl Into<String>) -> Self {
        self.extra = extra.into();
        self
    }

    fn detect_os_name() -> String {
        #[cfg(target_family = "unix")]
        return "posix".to_string();
        #[cfg(target_family = "windows")]
        return "nt".to_string();
        #[cfg(not(any(target_family = "unix", target_family = "windows")))]
        return std::env::consts::FAMILY.to_string();
    }

    fn detect_sys_platform() -> String {
        #[cfg(target_os = "linux")]
        return "linux".to_string();
        #[cfg(target_os = "windows")]
        return "win32".to_string();
        #[cfg(target_os = "macos")]
        return "darwin".to_string();
        #[cfg(target_os = "freebsd")]
        return "freebsd".to_string();
        #[cfg(not(any(
            target_os = "linux",
            target_os = "windows",
            target_os = "macos",
            target_os = "freebsd"
        )))]
        return std::env::consts::OS.to_string();
    }

    fn detect_platform_machine() -> String {
        #[cfg(target_arch = "x86_64")]
        return "x86_64".to_string();
        #[cfg(target_arch = "x86")]
        return "i686".to_string();
        #[cfg(target_arch = "aarch64")]
        return "aarch64".to_string();
        #[cfg(target_arch = "arm")]
        return "armv7l".to_string();
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "aarch64",
            target_arch = "arm"
        )))]
        return std::env::consts::ARCH.to_string();
    }

    fn detect_platform_release() -> String {
        // This would require platform-specific code to get the actual release
        String::new()
    }

    fn detect_platform_system() -> String {
        #[cfg(target_os = "linux")]
        return "Linux".to_string();
        #[cfg(target_os = "windows")]
        return "Windows".to_string();
        #[cfg(target_os = "macos")]
        return "Darwin".to_string();
        #[cfg(target_os = "freebsd")]
        return "FreeBSD".to_string();
        #[cfg(not(any(
            target_os = "linux",
            target_os = "windows",
            target_os = "macos",
            target_os = "freebsd"
        )))]
        return std::env::consts::OS.to_string();
    }

    /// Get the value of a marker variable
    pub fn get(&self, name: &str) -> Option<&str> {
        match name {
            "os_name" | "os.name" => Some(&self.os_name),
            "sys_platform" | "sys.platform" => Some(&self.sys_platform),
            "platform_machine" | "platform.machine" => Some(&self.platform_machine),
            "platform_python_implementation" | "platform.python_implementation" => {
                Some(&self.platform_python_implementation)
            }
            "platform_release" | "platform.release" => Some(&self.platform_release),
            "platform_system" | "platform.system" => Some(&self.platform_system),
            "platform_version" | "platform.version" => Some(&self.platform_version),
            "python_version" => Some(&self.python_version),
            "python_full_version" => Some(&self.python_full_version),
            "implementation_name" => Some(&self.implementation_name),
            "implementation_version" => Some(&self.implementation_version),
            "extra" => Some(&self.extra),
            _ => None,
        }
    }
}

/// Evaluates PEP 508 marker expressions
pub struct MarkerEvaluator {
    environment: MarkerEnvironment,
    cache: MarkerCache,
}

impl MarkerEvaluator {
    /// Create evaluator for an environment
    pub fn new(environment: MarkerEnvironment) -> Self {
        Self {
            environment,
            cache: MarkerCache::new(100),
        }
    }

    /// Evaluate a marker expression string
    pub fn evaluate(&mut self, marker: &str) -> Result<bool, MarkerError> {
        // Check cache first
        if let Some(result) = self.cache.get(marker) {
            return Ok(result);
        }

        // Parse and evaluate
        let mut parser = MarkerParser::new(marker);
        let expr = parser.parse()?;
        let result = self.evaluate_expr(&expr)?;

        // Cache the result
        self.cache.insert(marker.to_string(), result);

        Ok(result)
    }

    /// Evaluate a parsed marker expression
    pub fn evaluate_expr(&self, expr: &MarkerExpr) -> Result<bool, MarkerError> {
        match expr {
            MarkerExpr::Compare { left, op, right } => {
                let left_val = self.resolve_value(left)?;
                let right_val = self.resolve_value(right)?;
                self.compare(&left_val, op, &right_val)
            }
            MarkerExpr::And(left, right) => {
                Ok(self.evaluate_expr(left)? && self.evaluate_expr(right)?)
            }
            MarkerExpr::Or(left, right) => {
                Ok(self.evaluate_expr(left)? || self.evaluate_expr(right)?)
            }
        }
    }

    /// Resolve a marker value to a string
    fn resolve_value(&self, value: &MarkerValue) -> Result<String, MarkerError> {
        match value {
            MarkerValue::Literal(s) => Ok(s.clone()),
            MarkerValue::Variable(name) => self
                .environment
                .get(name)
                .map(|s| s.to_string())
                .ok_or_else(|| MarkerError::UnknownVariable(name.clone())),
        }
    }

    /// Compare two values with an operator
    fn compare(&self, left: &str, op: &MarkerOp, right: &str) -> Result<bool, MarkerError> {
        match op {
            MarkerOp::Equal => Ok(left == right),
            MarkerOp::NotEqual => Ok(left != right),
            MarkerOp::LessThan => Ok(self.version_compare(left, right) == std::cmp::Ordering::Less),
            MarkerOp::LessEqual => {
                Ok(self.version_compare(left, right) != std::cmp::Ordering::Greater)
            }
            MarkerOp::GreaterThan => {
                Ok(self.version_compare(left, right) == std::cmp::Ordering::Greater)
            }
            MarkerOp::GreaterEqual => {
                Ok(self.version_compare(left, right) != std::cmp::Ordering::Less)
            }
            MarkerOp::Compatible => self.compatible_release(left, right),
            MarkerOp::In => Ok(right.contains(left)),
            MarkerOp::NotIn => Ok(!right.contains(left)),
        }
    }

    /// Compare version strings
    fn version_compare(&self, left: &str, right: &str) -> std::cmp::Ordering {
        let left_parts: Vec<u32> = left.split('.').filter_map(|s| s.parse().ok()).collect();
        let right_parts: Vec<u32> = right.split('.').filter_map(|s| s.parse().ok()).collect();

        for (l, r) in left_parts.iter().zip(right_parts.iter()) {
            match l.cmp(r) {
                std::cmp::Ordering::Equal => continue,
                ord => return ord,
            }
        }

        left_parts.len().cmp(&right_parts.len())
    }

    /// Check compatible release (~=)
    fn compatible_release(&self, left: &str, right: &str) -> Result<bool, MarkerError> {
        let left_parts: Vec<u32> = left.split('.').filter_map(|s| s.parse().ok()).collect();
        let right_parts: Vec<u32> = right.split('.').filter_map(|s| s.parse().ok()).collect();

        if right_parts.is_empty() {
            return Ok(false);
        }

        // For ~=X.Y, left must be >= X.Y and < X.(Y+1)
        // Check prefix match
        for (i, r) in right_parts.iter().enumerate().take(right_parts.len() - 1) {
            if left_parts.get(i) != Some(r) {
                return Ok(false);
            }
        }

        // Check last component is >= right's last
        let last_idx = right_parts.len() - 1;
        if let (Some(&l), Some(&r)) = (left_parts.get(last_idx), right_parts.get(last_idx)) {
            Ok(l >= r)
        } else {
            Ok(false)
        }
    }

    /// Parse a marker expression into AST
    pub fn parse(marker: &str) -> Result<MarkerExpr, MarkerError> {
        let mut parser = MarkerParser::new(marker);
        parser.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_simple() {
        let env = MarkerEnvironment {
            python_version: "3.12".to_string(),
            ..Default::default()
        };
        let mut evaluator = MarkerEvaluator::new(env);

        assert!(evaluator.evaluate("python_version >= '3.8'").unwrap());
        assert!(!evaluator.evaluate("python_version < '3.8'").unwrap());
    }

    #[test]
    fn test_evaluate_platform() {
        let env = MarkerEnvironment::current();
        let mut evaluator = MarkerEvaluator::new(env);

        #[cfg(target_os = "linux")]
        assert!(evaluator.evaluate("sys_platform == 'linux'").unwrap());

        #[cfg(target_os = "windows")]
        assert!(evaluator.evaluate("sys_platform == 'win32'").unwrap());

        #[cfg(target_os = "macos")]
        assert!(evaluator.evaluate("sys_platform == 'darwin'").unwrap());
    }

    #[test]
    fn test_evaluate_and() {
        let env = MarkerEnvironment {
            python_version: "3.12".to_string(),
            sys_platform: "linux".to_string(),
            ..Default::default()
        };
        let mut evaluator = MarkerEvaluator::new(env);

        assert!(evaluator
            .evaluate("python_version >= '3.8' and sys_platform == 'linux'")
            .unwrap());
        assert!(!evaluator
            .evaluate("python_version >= '3.8' and sys_platform == 'win32'")
            .unwrap());
    }

    #[test]
    fn test_evaluate_or() {
        let env = MarkerEnvironment {
            sys_platform: "linux".to_string(),
            ..Default::default()
        };
        let mut evaluator = MarkerEvaluator::new(env);

        assert!(evaluator
            .evaluate("sys_platform == 'linux' or sys_platform == 'darwin'")
            .unwrap());
        assert!(!evaluator
            .evaluate("sys_platform == 'win32' or sys_platform == 'darwin'")
            .unwrap());
    }

    #[test]
    fn test_caching() {
        let env = MarkerEnvironment::current();
        let mut evaluator = MarkerEvaluator::new(env);

        // First evaluation
        let result1 = evaluator.evaluate("python_version >= '3.8'").unwrap();
        // Second evaluation (should use cache)
        let result2 = evaluator.evaluate("python_version >= '3.8'").unwrap();

        assert_eq!(result1, result2);
    }
}
