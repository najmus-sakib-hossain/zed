//! Resource handler system for DCP.
//!
//! Provides resource registration, URI template matching, and subscription management.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Resource content types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ResourceContent {
    /// Text content
    Text {
        uri: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
        text: String,
    },
    /// Binary content (base64 encoded)
    Blob {
        uri: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
        blob: String,
    },
}

impl ResourceContent {
    /// Create text content
    pub fn text(
        uri: impl Into<String>,
        mime_type: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self::Text {
            uri: uri.into(),
            mime_type: mime_type.into(),
            text: text.into(),
        }
    }

    /// Create binary content (will be base64 encoded)
    pub fn blob(uri: impl Into<String>, mime_type: impl Into<String>, data: &[u8]) -> Self {
        use base64::Engine;
        Self::Blob {
            uri: uri.into(),
            mime_type: mime_type.into(),
            blob: base64::engine::general_purpose::STANDARD.encode(data),
        }
    }

    /// Get the URI
    pub fn uri(&self) -> &str {
        match self {
            Self::Text { uri, .. } => uri,
            Self::Blob { uri, .. } => uri,
        }
    }
}

/// Resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    /// Resource URI
    pub uri: String,
    /// Human-readable name
    pub name: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

impl ResourceInfo {
    /// Create new resource info
    pub fn new(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            description: None,
            mime_type: None,
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set MIME type
    pub fn with_mime_type(mut self, mime: impl Into<String>) -> Self {
        self.mime_type = Some(mime.into());
        self
    }
}

/// Paginated resource list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceList {
    /// Resources in this page
    pub resources: Vec<ResourceInfo>,
    /// Cursor for next page (None if last page)
    #[serde(rename = "nextCursor", skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Resource error
#[derive(Debug, Clone, thiserror::Error)]
pub enum ResourceError {
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("invalid URI: {0}")]
    InvalidUri(String),
    #[error("handler error: {0}")]
    HandlerError(String),
    #[error("subscription not supported")]
    SubscriptionNotSupported,
}

/// Subscription ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(pub u64);

/// Resource handler trait
pub trait ResourceHandler: Send + Sync {
    /// Get the URI template for this handler
    fn uri_template(&self) -> &str;

    /// List available resources
    fn list(&self, cursor: Option<&str>) -> Result<ResourceList, ResourceError>;

    /// Read a specific resource
    fn read(&self, uri: &str) -> Result<ResourceContent, ResourceError>;

    /// Check if this handler supports subscriptions
    fn supports_subscribe(&self) -> bool {
        false
    }

    /// Check if a URI matches this handler's template
    fn matches(&self, uri: &str) -> bool {
        uri_matches_template(uri, self.uri_template())
    }
}

/// Check if a URI matches a template pattern
/// Supports simple patterns like "file:///{path}" where {path} is a wildcard
pub fn uri_matches_template(uri: &str, template: &str) -> bool {
    // Simple matching: split by {param} placeholders
    let mut template_parts = Vec::new();
    let mut current = template;

    while let Some(start) = current.find('{') {
        template_parts.push(&current[..start]);
        if let Some(end) = current[start..].find('}') {
            current = &current[start + end + 1..];
        } else {
            break;
        }
    }
    template_parts.push(current);

    // Match URI against template parts
    let mut uri_pos = 0;
    for (i, part) in template_parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if let Some(pos) = uri[uri_pos..].find(part) {
            if i == 0 && pos != 0 {
                return false; // First part must match at start
            }
            uri_pos += pos + part.len();
        } else {
            return false;
        }
    }

    true
}

/// Resource registry for managing handlers and subscriptions
pub struct ResourceRegistry {
    /// Registered handlers
    handlers: Vec<Box<dyn ResourceHandler>>,
    /// Active subscriptions: URI -> list of subscription IDs
    subscriptions: RwLock<HashMap<String, Vec<SubscriptionId>>>,
    /// Subscription ID counter
    subscription_counter: AtomicU64,
    /// Subscription callbacks: ID -> callback
    callbacks: RwLock<HashMap<SubscriptionId, Arc<dyn Fn(&str) + Send + Sync>>>,
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceRegistry {
    /// Create a new resource registry
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            subscriptions: RwLock::new(HashMap::new()),
            subscription_counter: AtomicU64::new(1),
            callbacks: RwLock::new(HashMap::new()),
        }
    }

    /// Register a resource handler
    pub fn register(&mut self, handler: impl ResourceHandler + 'static) {
        self.handlers.push(Box::new(handler));
    }

    /// Get the number of registered handlers
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }

    /// Find a handler that matches the given URI
    pub fn match_uri(&self, uri: &str) -> Option<&dyn ResourceHandler> {
        self.handlers.iter().find(|h| h.matches(uri)).map(|h| h.as_ref())
    }

    /// List all resources from all handlers
    pub fn list_all(&self, cursor: Option<&str>) -> Result<ResourceList, ResourceError> {
        let mut all_resources = Vec::new();

        for handler in &self.handlers {
            match handler.list(cursor) {
                Ok(list) => all_resources.extend(list.resources),
                Err(e) => return Err(e),
            }
        }

        Ok(ResourceList {
            resources: all_resources,
            next_cursor: None, // Simplified: no pagination across handlers
        })
    }

    /// Read a resource by URI
    pub fn read(&self, uri: &str) -> Result<ResourceContent, ResourceError> {
        let handler =
            self.match_uri(uri).ok_or_else(|| ResourceError::NotFound(uri.to_string()))?;
        handler.read(uri)
    }

    /// Subscribe to resource changes
    pub async fn subscribe<F>(
        &self,
        uri: &str,
        callback: F,
    ) -> Result<SubscriptionId, ResourceError>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        // Check if any handler supports this URI and subscriptions
        let handler =
            self.match_uri(uri).ok_or_else(|| ResourceError::NotFound(uri.to_string()))?;

        if !handler.supports_subscribe() {
            return Err(ResourceError::SubscriptionNotSupported);
        }

        let id = SubscriptionId(self.subscription_counter.fetch_add(1, Ordering::SeqCst));

        // Add to subscriptions
        {
            let mut subs = self.subscriptions.write().await;
            subs.entry(uri.to_string()).or_default().push(id);
        }

        // Store callback
        {
            let mut callbacks = self.callbacks.write().await;
            callbacks.insert(id, Arc::new(callback));
        }

        Ok(id)
    }

    /// Unsubscribe from resource changes
    pub async fn unsubscribe(&self, id: SubscriptionId) -> bool {
        // Remove from callbacks
        let removed = {
            let mut callbacks = self.callbacks.write().await;
            callbacks.remove(&id).is_some()
        };

        if removed {
            // Remove from subscriptions
            let mut subs = self.subscriptions.write().await;
            for ids in subs.values_mut() {
                ids.retain(|&sub_id| sub_id != id);
            }
        }

        removed
    }

    /// Notify all subscribers of a resource change
    pub async fn notify_change(&self, uri: &str) {
        let callbacks_to_call: Vec<Arc<dyn Fn(&str) + Send + Sync>> = {
            let subs = self.subscriptions.read().await;
            let callbacks = self.callbacks.read().await;

            subs.get(uri)
                .map(|ids| ids.iter().filter_map(|id| callbacks.get(id).cloned()).collect())
                .unwrap_or_default()
        };

        for callback in callbacks_to_call {
            callback(uri);
        }
    }

    /// Get subscription count for a URI
    pub async fn subscription_count(&self, uri: &str) -> usize {
        self.subscriptions.read().await.get(uri).map(|ids| ids.len()).unwrap_or(0)
    }
}

/// Simple in-memory resource handler for testing
pub struct MemoryResourceHandler {
    template: String,
    resources: HashMap<String, ResourceContent>,
    supports_subscribe: bool,
}

impl MemoryResourceHandler {
    /// Create a new memory resource handler
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            resources: HashMap::new(),
            supports_subscribe: false,
        }
    }

    /// Enable subscription support
    pub fn with_subscriptions(mut self) -> Self {
        self.supports_subscribe = true;
        self
    }

    /// Add a resource
    pub fn add_resource(&mut self, uri: impl Into<String>, content: ResourceContent) {
        self.resources.insert(uri.into(), content);
    }
}

impl ResourceHandler for MemoryResourceHandler {
    fn uri_template(&self) -> &str {
        &self.template
    }

    fn list(&self, _cursor: Option<&str>) -> Result<ResourceList, ResourceError> {
        let resources: Vec<ResourceInfo> = self
            .resources
            .iter()
            .map(|(uri, content)| {
                let (mime_type, name) = match content {
                    ResourceContent::Text { mime_type, .. } => (mime_type.clone(), uri.clone()),
                    ResourceContent::Blob { mime_type, .. } => (mime_type.clone(), uri.clone()),
                };
                ResourceInfo::new(uri, name).with_mime_type(mime_type)
            })
            .collect();

        Ok(ResourceList {
            resources,
            next_cursor: None,
        })
    }

    fn read(&self, uri: &str) -> Result<ResourceContent, ResourceError> {
        self.resources
            .get(uri)
            .cloned()
            .ok_or_else(|| ResourceError::NotFound(uri.to_string()))
    }

    fn supports_subscribe(&self) -> bool {
        self.supports_subscribe
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_matches_template() {
        assert!(uri_matches_template("file:///path/to/file.txt", "file:///{path}"));
        assert!(uri_matches_template("file:///a.txt", "file:///{path}"));
        assert!(uri_matches_template(
            "http://example.com/api/users/123",
            "http://example.com/api/users/{id}"
        ));
        assert!(!uri_matches_template("ftp://example.com", "http://{host}"));
    }

    #[test]
    fn test_resource_content_text() {
        let content = ResourceContent::text("file:///test.txt", "text/plain", "Hello");
        assert_eq!(content.uri(), "file:///test.txt");
    }

    #[test]
    fn test_resource_info() {
        let info = ResourceInfo::new("file:///test.txt", "Test File")
            .with_description("A test file")
            .with_mime_type("text/plain");

        assert_eq!(info.uri, "file:///test.txt");
        assert_eq!(info.name, "Test File");
        assert_eq!(info.description, Some("A test file".to_string()));
        assert_eq!(info.mime_type, Some("text/plain".to_string()));
    }

    #[test]
    fn test_memory_resource_handler() {
        let mut handler = MemoryResourceHandler::new("file:///{path}");
        handler.add_resource(
            "file:///test.txt",
            ResourceContent::text("file:///test.txt", "text/plain", "Hello"),
        );

        assert!(handler.matches("file:///test.txt"));
        assert!(handler.matches("file:///other.txt"));

        let list = handler.list(None).unwrap();
        assert_eq!(list.resources.len(), 1);

        let content = handler.read("file:///test.txt").unwrap();
        assert_eq!(content.uri(), "file:///test.txt");
    }

    #[tokio::test]
    async fn test_resource_registry() {
        let mut registry = ResourceRegistry::new();

        let mut handler = MemoryResourceHandler::new("file:///{path}");
        handler.add_resource(
            "file:///test.txt",
            ResourceContent::text("file:///test.txt", "text/plain", "Hello"),
        );
        registry.register(handler);

        assert_eq!(registry.handler_count(), 1);

        let content = registry.read("file:///test.txt").unwrap();
        assert_eq!(content.uri(), "file:///test.txt");

        let result = registry.read("file:///nonexistent.txt");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resource_subscriptions() {
        let mut registry = ResourceRegistry::new();

        let handler = MemoryResourceHandler::new("file:///{path}").with_subscriptions();
        registry.register(handler);

        // Subscribe
        let notified = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let notified_clone = Arc::clone(&notified);

        let sub_id = registry
            .subscribe("file:///test.txt", move |_uri| {
                notified_clone.store(true, Ordering::SeqCst);
            })
            .await
            .unwrap();

        assert_eq!(registry.subscription_count("file:///test.txt").await, 1);

        // Notify
        registry.notify_change("file:///test.txt").await;
        assert!(notified.load(Ordering::SeqCst));

        // Unsubscribe
        assert!(registry.unsubscribe(sub_id).await);
        assert_eq!(registry.subscription_count("file:///test.txt").await, 0);
    }
}
