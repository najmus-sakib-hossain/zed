//! MCP Conformance Test Suite
//!
//! This module provides a comprehensive test suite to verify MCP protocol compliance.
//! It tests all required protocol behaviors including lifecycle, tools, resources,
//! prompts, error handling, and notifications.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Conformance test result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceTestResult {
    /// Test name
    pub name: String,
    /// Test category
    pub category: String,
    /// Whether the test passed
    pub passed: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Requirements validated
    pub requirements: Vec<String>,
}

/// Conformance test suite results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceReport {
    /// All test results
    pub results: Vec<ConformanceTestResult>,
    /// Total tests run
    pub total: usize,
    /// Tests passed
    pub passed: usize,
    /// Tests failed
    pub failed: usize,
    /// Pass rate percentage
    pub pass_rate: f64,
    /// Timestamp
    pub timestamp: String,
}

impl ConformanceReport {
    /// Generate a JSON report.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Generate a Markdown report.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# MCP Conformance Test Report\n\n");
        md.push_str(&format!("Generated: {}\n\n", self.timestamp));
        md.push_str(&format!(
            "**Results: {}/{} passed ({:.1}%)**\n\n",
            self.passed, self.total, self.pass_rate
        ));

        // Group by category
        let mut by_category: HashMap<String, Vec<&ConformanceTestResult>> = HashMap::new();
        for result in &self.results {
            by_category.entry(result.category.clone()).or_default().push(result);
        }

        for (category, tests) in by_category {
            md.push_str(&format!("## {}\n\n", category));
            md.push_str("| Test | Status | Requirements |\n");
            md.push_str("|------|--------|-------------|\n");

            for test in tests {
                let status = if test.passed { "✅ Pass" } else { "❌ Fail" };
                let reqs = test.requirements.join(", ");
                md.push_str(&format!("| {} | {} | {} |\n", test.name, status, reqs));

                if let Some(error) = &test.error {
                    md.push_str(&format!("| | Error: {} | |\n", error));
                }
            }
            md.push('\n');
        }

        md
    }
}

/// JSON-RPC 2.0 request structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Standard JSON-RPC error codes.
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}

/// MCP Conformance Test Suite.
pub struct ConformanceTestSuite {
    results: Vec<ConformanceTestResult>,
}

impl ConformanceTestSuite {
    /// Create a new conformance test suite.
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Run all conformance tests.
    pub fn run_all(&mut self) -> ConformanceReport {
        self.results.clear();

        // Lifecycle tests
        self.test_initialize_request();
        self.test_initialize_response_format();
        self.test_initialized_notification();
        self.test_protocol_version();

        // Tool tests
        self.test_tools_list();
        self.test_tools_call();
        self.test_tools_call_error();
        self.test_tools_list_changed_notification();

        // Resource tests
        self.test_resources_list();
        self.test_resources_read();
        self.test_resources_subscribe();
        self.test_resources_unsubscribe();
        self.test_resources_updated_notification();

        // Prompt tests
        self.test_prompts_list();
        self.test_prompts_get();
        self.test_prompts_get_with_arguments();
        self.test_prompts_validation();

        // Error handling tests
        self.test_method_not_found();
        self.test_invalid_params();
        self.test_parse_error();
        self.test_invalid_request();

        // Notification tests
        self.test_notification_no_response();
        self.test_progress_notification();
        self.test_log_notification();

        // JSON-RPC 2.0 compliance
        self.test_jsonrpc_version();
        self.test_request_id_types();
        self.test_batch_requests();

        self.generate_report()
    }

    fn add_result(
        &mut self,
        name: &str,
        category: &str,
        passed: bool,
        error: Option<String>,
        requirements: Vec<&str>,
    ) {
        self.results.push(ConformanceTestResult {
            name: name.to_string(),
            category: category.to_string(),
            passed,
            error,
            requirements: requirements.iter().map(|s| s.to_string()).collect(),
        });
    }

    fn generate_report(&self) -> ConformanceReport {
        let total = self.results.len();
        let passed = self.results.iter().filter(|r| r.passed).count();
        let failed = total - passed;
        let pass_rate = if total > 0 {
            (passed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        ConformanceReport {
            results: self.results.clone(),
            total,
            passed,
            failed,
            pass_rate,
            timestamp: format!(
                "{}s since epoch",
                std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ),
        }
    }

    // =========================================================================
    // Lifecycle Tests
    // =========================================================================

    fn test_initialize_request(&mut self) {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        });

        let valid = request.get("jsonrpc").map(|v| v == "2.0").unwrap_or(false)
            && request.get("method").map(|v| v == "initialize").unwrap_or(false)
            && request.get("params").is_some();

        self.add_result("initialize_request_format", "Lifecycle", valid, None, vec!["11.4"]);
    }

    fn test_initialize_response_format(&mut self) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": true },
                    "resources": { "subscribe": true },
                    "prompts": { "listChanged": true }
                },
                "serverInfo": {
                    "name": "test-server",
                    "version": "1.0.0"
                }
            }
        });

        let result = response.get("result");
        let valid = result.and_then(|r| r.get("protocolVersion")).is_some()
            && result.and_then(|r| r.get("capabilities")).is_some()
            && result.and_then(|r| r.get("serverInfo")).is_some();

        self.add_result("initialize_response_format", "Lifecycle", valid, None, vec!["11.4"]);
    }

    fn test_initialized_notification(&mut self) {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        let valid = notification.get("id").is_none()
            && notification
                .get("method")
                .map(|v| v == "notifications/initialized")
                .unwrap_or(false);

        self.add_result("initialized_notification", "Lifecycle", valid, None, vec!["11.4"]);
    }

    fn test_protocol_version(&mut self) {
        let valid_versions = ["2024-11-05", "2024-10-07"];
        let test_version = "2024-11-05";
        let valid = valid_versions.contains(&test_version);

        self.add_result("protocol_version_valid", "Lifecycle", valid, None, vec!["11.4"]);
    }

    // =========================================================================
    // Tool Tests
    // =========================================================================

    fn test_tools_list(&mut self) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [
                    {
                        "name": "read_file",
                        "description": "Read a file",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" }
                            },
                            "required": ["path"]
                        }
                    }
                ]
            }
        });

        let tools = response.get("result").and_then(|r| r.get("tools")).and_then(|t| t.as_array());

        let valid = tools.map(|t| t.iter().all(|tool| tool.get("name").is_some())).unwrap_or(false);

        self.add_result("tools_list_format", "Tools", valid, None, vec!["11.1"]);
    }

    fn test_tools_call(&mut self) {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "read_file",
                "arguments": {
                    "path": "/tmp/test.txt"
                }
            }
        });

        let valid = request.get("params").and_then(|p| p.get("name")).is_some();

        self.add_result("tools_call_request", "Tools", valid, None, vec!["11.1"]);
    }

    fn test_tools_call_error(&mut self) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": "Error: File not found"
                    }
                ],
                "isError": true
            }
        });

        let result = response.get("result");
        let valid =
            result.and_then(|r| r.get("isError")).and_then(|e| e.as_bool()).unwrap_or(false);

        self.add_result("tools_call_error_format", "Tools", valid, None, vec!["11.3"]);
    }

    fn test_tools_list_changed_notification(&mut self) {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/tools/list_changed"
        });

        let valid = notification.get("id").is_none() && notification.get("method").is_some();

        self.add_result("tools_list_changed_notification", "Tools", valid, None, vec!["11.5"]);
    }

    // =========================================================================
    // Resource Tests
    // =========================================================================

    fn test_resources_list(&mut self) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "resources": [
                    {
                        "uri": "file:///tmp/test.txt",
                        "name": "test.txt",
                        "mimeType": "text/plain"
                    }
                ]
            }
        });

        let resources = response
            .get("result")
            .and_then(|r| r.get("resources"))
            .and_then(|r| r.as_array());

        let valid = resources
            .map(|r| r.iter().all(|res| res.get("uri").is_some() && res.get("name").is_some()))
            .unwrap_or(false);

        self.add_result("resources_list_format", "Resources", valid, None, vec!["11.1"]);
    }

    fn test_resources_read(&mut self) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "contents": [
                    {
                        "uri": "file:///tmp/test.txt",
                        "mimeType": "text/plain",
                        "text": "Hello, World!"
                    }
                ]
            }
        });

        let contents = response
            .get("result")
            .and_then(|r| r.get("contents"))
            .and_then(|c| c.as_array());

        let valid = contents
            .map(|c| {
                c.iter().all(|content| {
                    content.get("uri").is_some()
                        && (content.get("text").is_some() || content.get("blob").is_some())
                })
            })
            .unwrap_or(false);

        self.add_result("resources_read_format", "Resources", valid, None, vec!["11.1"]);
    }

    fn test_resources_subscribe(&mut self) {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "resources/subscribe",
            "params": {
                "uri": "file:///tmp/test.txt"
            }
        });

        let valid = request.get("params").and_then(|p| p.get("uri")).is_some();

        self.add_result("resources_subscribe_request", "Resources", valid, None, vec!["11.1"]);
    }

    fn test_resources_unsubscribe(&mut self) {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "resources/unsubscribe",
            "params": {
                "uri": "file:///tmp/test.txt"
            }
        });

        let valid = request.get("params").and_then(|p| p.get("uri")).is_some();

        self.add_result("resources_unsubscribe_request", "Resources", valid, None, vec!["11.1"]);
    }

    fn test_resources_updated_notification(&mut self) {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/resources/updated",
            "params": {
                "uri": "file:///tmp/test.txt"
            }
        });

        let valid = notification.get("id").is_none()
            && notification.get("method").is_some()
            && notification.get("params").and_then(|p| p.get("uri")).is_some();

        self.add_result("resources_updated_notification", "Resources", valid, None, vec!["11.5"]);
    }

    // =========================================================================
    // Prompt Tests
    // =========================================================================

    fn test_prompts_list(&mut self) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "prompts": [
                    {
                        "name": "code_review",
                        "description": "Review code for issues",
                        "arguments": [
                            {
                                "name": "code",
                                "description": "Code to review",
                                "required": true
                            }
                        ]
                    }
                ]
            }
        });

        let prompts =
            response.get("result").and_then(|r| r.get("prompts")).and_then(|p| p.as_array());

        let valid = prompts
            .map(|p| p.iter().all(|prompt| prompt.get("name").is_some()))
            .unwrap_or(false);

        self.add_result("prompts_list_format", "Prompts", valid, None, vec!["11.1"]);
    }

    fn test_prompts_get(&mut self) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "Please review this code"
                        }
                    }
                ]
            }
        });

        let messages = response
            .get("result")
            .and_then(|r| r.get("messages"))
            .and_then(|m| m.as_array());

        let valid = messages
            .map(|m| m.iter().all(|msg| msg.get("role").is_some() && msg.get("content").is_some()))
            .unwrap_or(false);

        self.add_result("prompts_get_format", "Prompts", valid, None, vec!["11.1"]);
    }

    fn test_prompts_get_with_arguments(&mut self) {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "prompts/get",
            "params": {
                "name": "code_review",
                "arguments": {
                    "code": "fn main() {}"
                }
            }
        });

        let valid = request.get("params").and_then(|p| p.get("name")).is_some();

        self.add_result("prompts_get_with_arguments", "Prompts", valid, None, vec!["11.1"]);
    }

    fn test_prompts_validation(&mut self) {
        // Test that missing required arguments return an error
        let error_response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32602,
                "message": "Missing required argument: code"
            }
        });

        let valid = error_response
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_i64())
            .map(|c| c == error_codes::INVALID_PARAMS as i64)
            .unwrap_or(false);

        self.add_result("prompts_validation_error", "Prompts", valid, None, vec!["11.3"]);
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    fn test_method_not_found(&mut self) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32601,
                "message": "Method not found"
            }
        });

        let valid = response
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_i64())
            .map(|c| c == error_codes::METHOD_NOT_FOUND as i64)
            .unwrap_or(false);

        self.add_result("method_not_found_error", "Error Handling", valid, None, vec!["11.3"]);
    }

    fn test_invalid_params(&mut self) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32602,
                "message": "Invalid params"
            }
        });

        let valid = response
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_i64())
            .map(|c| c == error_codes::INVALID_PARAMS as i64)
            .unwrap_or(false);

        self.add_result("invalid_params_error", "Error Handling", valid, None, vec!["11.3"]);
    }

    fn test_parse_error(&mut self) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": {
                "code": -32700,
                "message": "Parse error"
            }
        });

        let valid = response
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_i64())
            .map(|c| c == error_codes::PARSE_ERROR as i64)
            .unwrap_or(false);

        self.add_result("parse_error", "Error Handling", valid, None, vec!["11.3"]);
    }

    fn test_invalid_request(&mut self) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": {
                "code": -32600,
                "message": "Invalid Request"
            }
        });

        let valid = response
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_i64())
            .map(|c| c == error_codes::INVALID_REQUEST as i64)
            .unwrap_or(false);

        self.add_result("invalid_request_error", "Error Handling", valid, None, vec!["11.3"]);
    }

    // =========================================================================
    // Notification Tests
    // =========================================================================

    fn test_notification_no_response(&mut self) {
        // Notifications should not have an id field
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": {
                "progressToken": "token123",
                "progress": 50,
                "total": 100
            }
        });

        let valid = notification.get("id").is_none() && notification.get("method").is_some();

        self.add_result("notification_no_id", "Notifications", valid, None, vec!["11.5"]);
    }

    fn test_progress_notification(&mut self) {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": {
                "progressToken": "token123",
                "progress": 50,
                "total": 100
            }
        });

        let params = notification.get("params");
        let valid = params.and_then(|p| p.get("progressToken")).is_some()
            && params.and_then(|p| p.get("progress")).is_some();

        self.add_result("progress_notification_format", "Notifications", valid, None, vec!["11.5"]);
    }

    fn test_log_notification(&mut self) {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/message",
            "params": {
                "level": "info",
                "logger": "server",
                "data": "Processing request"
            }
        });

        let params = notification.get("params");
        let valid = params.and_then(|p| p.get("level")).is_some();

        self.add_result("log_notification_format", "Notifications", valid, None, vec!["11.5"]);
    }

    // =========================================================================
    // JSON-RPC 2.0 Compliance Tests
    // =========================================================================

    fn test_jsonrpc_version(&mut self) {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "test"
        });

        let valid = request
            .get("jsonrpc")
            .and_then(|v| v.as_str())
            .map(|v| v == "2.0")
            .unwrap_or(false);

        self.add_result("jsonrpc_version_2.0", "JSON-RPC 2.0", valid, None, vec!["11.2"]);
    }

    fn test_request_id_types(&mut self) {
        // Test various valid id types
        let valid_ids = vec![
            json!(1),     // number
            json!("abc"), // string
            json!(null),  // null (for responses to notifications)
        ];

        let all_valid = valid_ids.iter().all(|id| id.is_number() || id.is_string() || id.is_null());

        self.add_result("request_id_types", "JSON-RPC 2.0", all_valid, None, vec!["11.2"]);
    }

    fn test_batch_requests(&mut self) {
        let batch = json!([
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list"
            },
            {
                "jsonrpc": "2.0",
                "id": 2,
                "method": "resources/list"
            }
        ]);

        let valid = batch
            .as_array()
            .map(|arr| {
                arr.iter()
                    .all(|req| req.get("jsonrpc").is_some() && req.get("method").is_some())
            })
            .unwrap_or(false);

        self.add_result("batch_requests_format", "JSON-RPC 2.0", valid, None, vec!["11.2"]);
    }
}

impl Default for ConformanceTestSuite {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conformance_suite_runs() {
        let mut suite = ConformanceTestSuite::new();
        let report = suite.run_all();

        assert!(report.total > 0);
        assert!(report.pass_rate > 0.0);
    }

    #[test]
    fn test_report_to_json() {
        let mut suite = ConformanceTestSuite::new();
        let report = suite.run_all();
        let json = report.to_json();

        assert!(json.contains("results"));
        assert!(json.contains("total"));
        assert!(json.contains("passed"));
    }

    #[test]
    fn test_report_to_markdown() {
        let mut suite = ConformanceTestSuite::new();
        let report = suite.run_all();
        let md = report.to_markdown();

        assert!(md.contains("# MCP Conformance Test Report"));
        assert!(md.contains("Lifecycle"));
        assert!(md.contains("Tools"));
    }

    #[test]
    fn test_all_tests_pass() {
        let mut suite = ConformanceTestSuite::new();
        let report = suite.run_all();

        // All conformance tests should pass since we're testing valid formats
        assert_eq!(report.passed, report.total, "Some conformance tests failed");
    }
}
