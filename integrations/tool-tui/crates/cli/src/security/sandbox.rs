//! WASM Sandbox
//!
//! Secure sandboxed execution environment for WebAssembly modules.
//! Implements capability-based security with resource limits.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::{Capability, SecurityError, TrustLevel};

/// WASM sandbox configuration
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Maximum memory (bytes)
    pub max_memory: usize,
    /// Maximum table elements
    pub max_table_elements: u32,
    /// Maximum execution time
    pub max_execution_time: Duration,
    /// Maximum fuel (instruction count)
    pub max_fuel: u64,
    /// Allow threading
    pub allow_threads: bool,
    /// Allow SIMD
    pub allow_simd: bool,
    /// Trust level
    pub trust_level: TrustLevel,
    /// Allowed imports
    pub allowed_imports: Vec<String>,
    /// Allowed exports
    pub allowed_exports: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_memory: 64 * 1024 * 1024, // 64 MB
            max_table_elements: 10_000,
            max_execution_time: Duration::from_secs(30),
            max_fuel: 1_000_000_000, // 1B instructions
            allow_threads: false,
            allow_simd: true,
            trust_level: TrustLevel::Basic,
            allowed_imports: vec![
                "console.log".into(),
                "console.error".into(),
                "console.warn".into(),
            ],
            allowed_exports: vec!["main".into(), "init".into(), "_start".into()],
        }
    }
}

/// WASM sandbox
pub struct WasmSandbox {
    /// Configuration
    config: SandboxConfig,
    /// Memory usage tracking
    memory_used: usize,
    /// Fuel consumed
    fuel_consumed: u64,
    /// Start time
    start_time: Option<Instant>,
    /// Imported functions
    imports: HashMap<String, ImportFunction>,
    /// Security violations
    violations: Vec<SecurityViolation>,
    /// Is running
    running: bool,
}

/// Import function wrapper
pub struct ImportFunction {
    /// Function name
    pub name: String,
    /// Module name
    pub module: String,
    /// Capabilities required
    pub capabilities: Vec<Capability>,
    /// Call count
    pub call_count: u64,
}

/// Security violation
#[derive(Debug, Clone)]
pub struct SecurityViolation {
    /// Violation type
    pub violation_type: ViolationType,
    /// Description
    pub description: String,
    /// Timestamp
    pub timestamp: Instant,
    /// Was blocked
    pub blocked: bool,
}

/// Violation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationType {
    MemoryLimit,
    TimeLimit,
    FuelLimit,
    IllegalImport,
    IllegalExport,
    IllegalInstruction,
    CapabilityViolation,
}

impl WasmSandbox {
    /// Create new sandbox
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            memory_used: 0,
            fuel_consumed: 0,
            start_time: None,
            imports: HashMap::new(),
            violations: Vec::new(),
            running: false,
        }
    }

    /// Create sandbox with default config
    pub fn default_sandbox() -> Self {
        Self::new(SandboxConfig::default())
    }

    /// Create high-security sandbox
    pub fn high_security() -> Self {
        Self::new(SandboxConfig {
            max_memory: 16 * 1024 * 1024, // 16 MB
            max_table_elements: 1_000,
            max_execution_time: Duration::from_secs(5),
            max_fuel: 100_000_000, // 100M instructions
            allow_threads: false,
            allow_simd: false,
            trust_level: TrustLevel::Untrusted,
            allowed_imports: vec![],
            allowed_exports: vec!["main".into()],
        })
    }

    /// Validate WASM module before execution
    pub fn validate_module(
        &mut self,
        wasm_bytes: &[u8],
    ) -> Result<ValidationResult, SecurityError> {
        let mut result = ValidationResult {
            valid: true,
            imports: Vec::new(),
            exports: Vec::new(),
            memory_pages: 0,
            table_elements: 0,
            has_start: false,
            warnings: Vec::new(),
        };

        // Simple validation - in real impl would parse WASM
        if wasm_bytes.len() < 8 {
            return Err(SecurityError::SandboxViolation("Invalid WASM module".into()));
        }

        // Check magic number
        if &wasm_bytes[0..4] != b"\0asm" {
            return Err(SecurityError::SandboxViolation("Invalid WASM magic number".into()));
        }

        // Check version
        let version =
            u32::from_le_bytes([wasm_bytes[4], wasm_bytes[5], wasm_bytes[6], wasm_bytes[7]]);
        if version != 1 {
            result.warnings.push(format!("Unusual WASM version: {}", version));
        }

        // Estimate memory from module size (simplified)
        result.memory_pages = (wasm_bytes.len() / 65536) as u32 + 1;

        if result.memory_pages as usize * 65536 > self.config.max_memory {
            self.record_violation(ViolationType::MemoryLimit, "Module exceeds memory limit");
            result.valid = false;
        }

        Ok(result)
    }

    /// Register import function
    pub fn register_import(
        &mut self,
        module: &str,
        name: &str,
        capabilities: Vec<Capability>,
    ) -> Result<(), SecurityError> {
        let full_name = format!("{}.{}", module, name);

        // Check if import is allowed
        if !self.config.allowed_imports.contains(&full_name)
            && !self.config.allowed_imports.contains(&format!("{}.*", module))
        {
            self.record_violation(
                ViolationType::IllegalImport,
                &format!("Import '{}' not allowed", full_name),
            );
            return Err(SecurityError::SandboxViolation(format!(
                "Import '{}' not allowed",
                full_name
            )));
        }

        // Check capabilities
        for cap in &capabilities {
            if !self.config.trust_level.capabilities().contains(cap) {
                return Err(SecurityError::PermissionDenied(format!(
                    "Import '{}' requires capability {:?}",
                    full_name, cap
                )));
            }
        }

        self.imports.insert(
            full_name.clone(),
            ImportFunction {
                name: name.to_string(),
                module: module.to_string(),
                capabilities,
                call_count: 0,
            },
        );

        Ok(())
    }

    /// Start execution
    pub fn start(&mut self) -> Result<(), SecurityError> {
        if self.running {
            return Err(SecurityError::SandboxViolation("Already running".into()));
        }
        self.running = true;
        self.start_time = Some(Instant::now());
        Ok(())
    }

    /// Stop execution
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Check resource limits
    pub fn check_limits(&mut self) -> Result<(), SecurityError> {
        // Check time limit
        if let Some(start) = self.start_time {
            if start.elapsed() > self.config.max_execution_time {
                self.record_violation(ViolationType::TimeLimit, "Execution time exceeded");
                return Err(SecurityError::SandboxViolation("Time limit exceeded".into()));
            }
        }

        // Check fuel limit
        if self.fuel_consumed > self.config.max_fuel {
            self.record_violation(ViolationType::FuelLimit, "Fuel limit exceeded");
            return Err(SecurityError::SandboxViolation("Fuel limit exceeded".into()));
        }

        // Check memory limit
        if self.memory_used > self.config.max_memory {
            self.record_violation(ViolationType::MemoryLimit, "Memory limit exceeded");
            return Err(SecurityError::SandboxViolation("Memory limit exceeded".into()));
        }

        Ok(())
    }

    /// Allocate memory
    pub fn allocate(&mut self, bytes: usize) -> Result<(), SecurityError> {
        if self.memory_used + bytes > self.config.max_memory {
            self.record_violation(
                ViolationType::MemoryLimit,
                &format!("Allocation of {} bytes would exceed limit", bytes),
            );
            return Err(SecurityError::SandboxViolation("Memory limit exceeded".into()));
        }

        self.memory_used += bytes;
        Ok(())
    }

    /// Free memory
    pub fn free(&mut self, bytes: usize) {
        self.memory_used = self.memory_used.saturating_sub(bytes);
    }

    /// Consume fuel
    pub fn consume_fuel(&mut self, amount: u64) -> Result<(), SecurityError> {
        self.fuel_consumed += amount;
        self.check_limits()
    }

    /// Record security violation
    fn record_violation(&mut self, violation_type: ViolationType, description: &str) {
        self.violations.push(SecurityViolation {
            violation_type,
            description: description.to_string(),
            timestamp: Instant::now(),
            blocked: true,
        });
    }

    /// Get violations
    pub fn violations(&self) -> &[SecurityViolation] {
        &self.violations
    }

    /// Get memory usage
    pub fn memory_used(&self) -> usize {
        self.memory_used
    }

    /// Get fuel consumed
    pub fn fuel_consumed(&self) -> u64 {
        self.fuel_consumed
    }

    /// Get execution time
    pub fn execution_time(&self) -> Option<Duration> {
        self.start_time.map(|s| s.elapsed())
    }

    /// Get statistics
    pub fn stats(&self) -> SandboxStats {
        SandboxStats {
            memory_used: self.memory_used,
            memory_limit: self.config.max_memory,
            fuel_consumed: self.fuel_consumed,
            fuel_limit: self.config.max_fuel,
            execution_time: self.execution_time(),
            time_limit: self.config.max_execution_time,
            violations: self.violations.len(),
            imports_registered: self.imports.len(),
        }
    }

    /// Reset sandbox state
    pub fn reset(&mut self) {
        self.memory_used = 0;
        self.fuel_consumed = 0;
        self.start_time = None;
        self.running = false;
        self.violations.clear();
        for import in self.imports.values_mut() {
            import.call_count = 0;
        }
    }
}

/// Validation result
#[derive(Debug)]
pub struct ValidationResult {
    pub valid: bool,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub memory_pages: u32,
    pub table_elements: u32,
    pub has_start: bool,
    pub warnings: Vec<String>,
}

/// Sandbox statistics
#[derive(Debug)]
pub struct SandboxStats {
    pub memory_used: usize,
    pub memory_limit: usize,
    pub fuel_consumed: u64,
    pub fuel_limit: u64,
    pub execution_time: Option<Duration>,
    pub time_limit: Duration,
    pub violations: usize,
    pub imports_registered: usize,
}

impl SandboxStats {
    /// Memory usage percentage
    pub fn memory_percent(&self) -> f64 {
        (self.memory_used as f64 / self.memory_limit as f64) * 100.0
    }

    /// Fuel usage percentage
    pub fn fuel_percent(&self) -> f64 {
        (self.fuel_consumed as f64 / self.fuel_limit as f64) * 100.0
    }

    /// Time usage percentage
    pub fn time_percent(&self) -> f64 {
        self.execution_time
            .map(|t| (t.as_secs_f64() / self.time_limit.as_secs_f64()) * 100.0)
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_creation() {
        let sandbox = WasmSandbox::default_sandbox();
        assert_eq!(sandbox.memory_used(), 0);
        assert_eq!(sandbox.fuel_consumed(), 0);
    }

    #[test]
    fn test_memory_allocation() {
        let mut sandbox = WasmSandbox::new(SandboxConfig {
            max_memory: 1024,
            ..Default::default()
        });

        assert!(sandbox.allocate(512).is_ok());
        assert_eq!(sandbox.memory_used(), 512);

        assert!(sandbox.allocate(600).is_err()); // Would exceed limit
        assert_eq!(sandbox.violations().len(), 1);
    }

    #[test]
    fn test_fuel_consumption() {
        let mut sandbox = WasmSandbox::new(SandboxConfig {
            max_fuel: 1000,
            ..Default::default()
        });

        assert!(sandbox.consume_fuel(500).is_ok());
        assert!(sandbox.consume_fuel(600).is_err()); // Exceeds limit
    }

    #[test]
    fn test_import_validation() {
        let mut sandbox = WasmSandbox::new(SandboxConfig {
            allowed_imports: vec!["console.log".into()],
            trust_level: TrustLevel::Basic,
            ..Default::default()
        });

        assert!(sandbox.register_import("console", "log", vec![]).is_ok());
        assert!(sandbox.register_import("fs", "read", vec![]).is_err());
    }

    #[test]
    fn test_wasm_validation() {
        let mut sandbox = WasmSandbox::default_sandbox();

        // Invalid magic number
        let invalid = vec![0, 0, 0, 0, 1, 0, 0, 0];
        assert!(sandbox.validate_module(&invalid).is_err());

        // Valid magic number
        let valid = b"\0asm\x01\x00\x00\x00";
        assert!(sandbox.validate_module(valid).is_ok());
    }
}
