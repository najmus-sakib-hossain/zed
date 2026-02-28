//! RPC method registry for gateway

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub type RpcHandler = Arc<dyn Fn(Value) -> Result<Value> + Send + Sync>;

#[derive(Clone)]
pub struct RpcMethod {
    pub name: String,
    pub handler: RpcHandler,
}

pub struct RpcRegistry {
    methods: HashMap<String, RpcMethod>,
}

impl RpcRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            methods: HashMap::new(),
        };
        registry.register_default_methods();
        registry
    }

    pub fn register(&mut self, name: String, handler: RpcHandler) {
        self.methods.insert(name.clone(), RpcMethod { name, handler });
    }

    pub fn call(&self, method: &str, params: Value) -> Result<Value> {
        if let Some(rpc_method) = self.methods.get(method) {
            (rpc_method.handler)(params)
        } else {
            anyhow::bail!("Method not found: {}", method)
        }
    }

    pub fn list_methods(&self) -> Vec<String> {
        self.methods.keys().cloned().collect()
    }

    fn register_default_methods(&mut self) {
        // Ping
        self.register("ping".to_string(), Arc::new(|_| Ok(serde_json::json!({"pong": true}))));

        // Echo
        self.register("echo".to_string(), Arc::new(|params| Ok(params)));

        // Version
        self.register(
            "version".to_string(),
            Arc::new(|_| Ok(serde_json::json!({"version": env!("CARGO_PKG_VERSION")}))),
        );
    }
}

impl Default for RpcRegistry {
    fn default() -> Self {
        Self::new()
    }
}
