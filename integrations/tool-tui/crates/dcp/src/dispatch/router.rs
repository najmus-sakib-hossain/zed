//! Binary Trie Router for O(1) tool dispatch.

use std::collections::HashMap;

use crate::dispatch::handler::{SharedArgs, ToolHandler, ToolResult};
use crate::DCPError;

/// Compile-time generated tool router with O(1) dispatch
pub struct BinaryTrieRouter {
    /// Direct dispatch table - tool_id is array index
    handlers: Vec<Option<Box<dyn ToolHandler>>>,
    /// Tool name to ID mapping (for MCP compatibility)
    name_to_id: HashMap<String, u16>,
    /// Maximum registered tool ID
    max_tool_id: u16,
}

impl BinaryTrieRouter {
    /// Maximum number of tools supported
    pub const MAX_TOOLS: usize = 65536;

    /// Create a new empty router
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            name_to_id: HashMap::new(),
            max_tool_id: 0,
        }
    }

    /// Create a router with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            handlers: Vec::with_capacity(capacity.min(Self::MAX_TOOLS)),
            name_to_id: HashMap::with_capacity(capacity),
            max_tool_id: 0,
        }
    }

    /// Register a tool handler
    pub fn register(&mut self, handler: Box<dyn ToolHandler>) -> Result<u16, DCPError> {
        let tool_id = handler.tool_id();
        let tool_name = handler.tool_name().to_string();

        // Ensure handlers vec is large enough
        let id_usize = tool_id as usize;
        if id_usize >= Self::MAX_TOOLS {
            return Err(DCPError::ValidationFailed);
        }

        while self.handlers.len() <= id_usize {
            self.handlers.push(None);
        }

        // Register the handler
        self.handlers[id_usize] = Some(handler);
        self.name_to_id.insert(tool_name, tool_id);

        if tool_id > self.max_tool_id {
            self.max_tool_id = tool_id;
        }

        Ok(tool_id)
    }

    /// O(1) dispatch by tool ID
    #[inline(always)]
    pub fn dispatch(&self, tool_id: u16) -> Option<&dyn ToolHandler> {
        self.handlers.get(tool_id as usize).and_then(|h| h.as_ref()).map(|h| h.as_ref())
    }

    /// Execute a tool by ID with arguments
    pub fn execute(&self, tool_id: u16, args: &SharedArgs) -> Result<ToolResult, DCPError> {
        let handler = self.dispatch(tool_id).ok_or(DCPError::ToolNotFound)?;
        handler.execute(args)
    }

    /// Resolve tool name to ID (for MCP compatibility)
    pub fn resolve_name(&self, name: &str) -> Option<u16> {
        self.name_to_id.get(name).copied()
    }

    /// Get the maximum registered tool ID
    pub fn max_tool_id(&self) -> u16 {
        self.max_tool_id
    }

    /// Get the number of registered tools
    pub fn tool_count(&self) -> usize {
        self.handlers.iter().filter(|h| h.is_some()).count()
    }

    /// Check if a tool ID is registered
    pub fn has_tool(&self, tool_id: u16) -> bool {
        self.dispatch(tool_id).is_some()
    }

    /// Get all registered tool names
    pub fn tool_names(&self) -> impl Iterator<Item = &str> {
        self.name_to_id.keys().map(|s| s.as_str())
    }

    /// Get server capabilities based on registered tools
    pub fn capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            tools: self.tool_count() > 0,
            resources: false, // TODO: implement resource handlers
            prompts: false,   // TODO: implement prompt handlers
            logging: true,
        }
    }
}

/// Server capabilities
#[derive(Debug, Clone, Default)]
pub struct ServerCapabilities {
    pub tools: bool,
    pub resources: bool,
    pub prompts: bool,
    pub logging: bool,
}

impl Default for BinaryTrieRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::schema::{InputSchema, ToolSchema};

    // Test handler implementation
    struct TestHandler {
        schema: ToolSchema,
    }

    impl TestHandler {
        fn new(id: u16, name: &'static str) -> Self {
            Self {
                schema: ToolSchema {
                    name,
                    id,
                    description: "Test tool",
                    input: InputSchema::new(),
                },
            }
        }
    }

    impl ToolHandler for TestHandler {
        fn execute(&self, _args: &SharedArgs) -> Result<ToolResult, DCPError> {
            Ok(ToolResult::success(vec![self.schema.id as u8]))
        }

        fn schema(&self) -> &ToolSchema {
            &self.schema
        }
    }

    #[test]
    fn test_register_and_dispatch() {
        let mut router = BinaryTrieRouter::new();

        let handler = Box::new(TestHandler::new(1, "test_tool"));
        router.register(handler).unwrap();

        assert!(router.has_tool(1));
        assert!(!router.has_tool(0));
        assert!(!router.has_tool(2));

        let dispatched = router.dispatch(1).unwrap();
        assert_eq!(dispatched.tool_id(), 1);
    }

    #[test]
    fn test_resolve_name() {
        let mut router = BinaryTrieRouter::new();

        router.register(Box::new(TestHandler::new(42, "my_tool"))).unwrap();

        assert_eq!(router.resolve_name("my_tool"), Some(42));
        assert_eq!(router.resolve_name("unknown"), None);
    }

    #[test]
    fn test_execute() {
        let mut router = BinaryTrieRouter::new();
        router.register(Box::new(TestHandler::new(5, "exec_test"))).unwrap();

        let args = SharedArgs::new(&[], 0);
        let result = router.execute(5, &args).unwrap();

        assert!(result.is_success());
        assert_eq!(result.payload(), Some(&[5u8][..]));
    }

    #[test]
    fn test_execute_not_found() {
        let router = BinaryTrieRouter::new();
        let args = SharedArgs::new(&[], 0);

        let result = router.execute(999, &args);
        assert!(matches!(result, Err(DCPError::ToolNotFound)));
    }

    #[test]
    fn test_multiple_tools() {
        let mut router = BinaryTrieRouter::new();

        for i in 0..10 {
            let name: &'static str = Box::leak(format!("tool_{}", i).into_boxed_str());
            router.register(Box::new(TestHandler::new(i, name))).unwrap();
        }

        assert_eq!(router.tool_count(), 10);
        assert_eq!(router.max_tool_id(), 9);

        for i in 0..10 {
            assert!(router.has_tool(i));
        }
    }

    #[test]
    fn test_sparse_registration() {
        let mut router = BinaryTrieRouter::new();

        router.register(Box::new(TestHandler::new(0, "first"))).unwrap();
        router.register(Box::new(TestHandler::new(100, "hundredth"))).unwrap();
        router.register(Box::new(TestHandler::new(1000, "thousandth"))).unwrap();

        assert_eq!(router.tool_count(), 3);
        assert!(router.has_tool(0));
        assert!(router.has_tool(100));
        assert!(router.has_tool(1000));
        assert!(!router.has_tool(50));
    }
}
