//! JSON-RPC encoding for size comparison with DCP.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 request for size comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    pub id: Value,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request
    pub fn new(method: &str, params: Option<Value>, id: u64) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Value::Number(id.into()),
        }
    }
}

/// Encode a tool invocation as JSON-RPC
pub fn encode_json_rpc(method: &str, params: &Value, id: u64) -> String {
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: method.to_string(),
        params: Some(params.clone()),
        id: Value::Number(id.into()),
    };
    serde_json::to_string(&request).unwrap_or_default()
}

/// Measure the size of a JSON-RPC encoded message
pub fn measure_json_rpc_size(method: &str, params: &Value, id: u64) -> usize {
    encode_json_rpc(method, params, id).len()
}

/// DCP binary message header size
pub const DCP_HEADER_SIZE: usize = 8;

/// DCP tool invocation header size
pub const DCP_INVOCATION_SIZE: usize = 20;

/// Measure the size of a DCP encoded message
pub fn measure_dcp_size(args_size: usize) -> usize {
    // DCP message = envelope (8) + invocation header (20) + args
    DCP_HEADER_SIZE + DCP_INVOCATION_SIZE + args_size
}

/// Size comparison result
#[derive(Debug, Clone)]
pub struct SizeComparison {
    /// JSON-RPC message size in bytes
    pub json_rpc_size: usize,
    /// DCP message size in bytes
    pub dcp_size: usize,
    /// Size ratio (JSON-RPC / DCP)
    pub ratio: f64,
}

impl SizeComparison {
    /// Create a new size comparison
    pub fn new(json_rpc_size: usize, dcp_size: usize) -> Self {
        let ratio = if dcp_size > 0 {
            json_rpc_size as f64 / dcp_size as f64
        } else {
            0.0
        };
        Self {
            json_rpc_size,
            dcp_size,
            ratio,
        }
    }

    /// Check if DCP is at least N times smaller
    pub fn dcp_is_smaller_by(&self, factor: f64) -> bool {
        self.ratio >= factor
    }
}

/// Compare sizes for a tool invocation
pub fn compare_sizes(method: &str, params: &Value, args_binary_size: usize) -> SizeComparison {
    let json_size = measure_json_rpc_size(method, params, 1);
    let dcp_size = measure_dcp_size(args_binary_size);
    SizeComparison::new(json_size, dcp_size)
}

/// Estimate binary size for JSON value
pub fn estimate_binary_size(value: &Value) -> usize {
    match value {
        Value::Null => 0,
        Value::Bool(_) => 1,
        Value::Number(n) => {
            if n.is_i64() || n.is_f64() {
                8
            } else {
                4
            }
        }
        Value::String(s) => 4 + s.len(), // length prefix + data
        Value::Array(arr) => {
            4 + arr.iter().map(estimate_binary_size).sum::<usize>() // length + elements
        }
        Value::Object(obj) => {
            4 + obj.iter().map(|(k, v)| 1 + k.len() + estimate_binary_size(v)).sum::<usize>()
        }
    }
}

/// Compare sizes using automatic binary size estimation
pub fn compare_sizes_auto(method: &str, params: &Value) -> SizeComparison {
    let binary_size = estimate_binary_size(params);
    compare_sizes(method, params, binary_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_rpc_encoding() {
        let params = json!({"name": "test", "value": 42});
        let encoded = encode_json_rpc("tools/call", &params, 1);

        assert!(encoded.contains("\"jsonrpc\":\"2.0\""));
        assert!(encoded.contains("\"method\":\"tools/call\""));
        assert!(encoded.contains("\"params\""));
        assert!(encoded.contains("\"id\":1"));
    }

    #[test]
    fn test_size_comparison() {
        let params = json!({"name": "test"});
        let comparison = compare_sizes("test", &params, 8);

        assert!(comparison.json_rpc_size > 0);
        assert!(comparison.dcp_size > 0);
        assert!(comparison.ratio > 0.0);
    }

    #[test]
    fn test_dcp_smaller_for_simple_call() {
        // Simple tool call with minimal params
        let params = json!({"path": "/tmp/test.txt"});
        let comparison = compare_sizes_auto("read_file", &params);

        // DCP should be significantly smaller
        println!(
            "JSON-RPC: {} bytes, DCP: {} bytes, ratio: {:.2}x",
            comparison.json_rpc_size, comparison.dcp_size, comparison.ratio
        );

        // For typical tool calls, DCP should be at least 1.4x smaller
        assert!(comparison.ratio >= 1.4);
    }

    #[test]
    fn test_estimate_binary_size() {
        assert_eq!(estimate_binary_size(&json!(null)), 0);
        assert_eq!(estimate_binary_size(&json!(true)), 1);
        assert_eq!(estimate_binary_size(&json!(42)), 8);
        assert_eq!(estimate_binary_size(&json!(3.14)), 8);
        assert_eq!(estimate_binary_size(&json!("hello")), 4 + 5);
    }

    #[test]
    fn test_size_comparison_ratio() {
        let comparison = SizeComparison::new(100, 20);
        assert_eq!(comparison.ratio, 5.0);
        assert!(comparison.dcp_is_smaller_by(5.0));
        assert!(!comparison.dcp_is_smaller_by(6.0));
    }
}
