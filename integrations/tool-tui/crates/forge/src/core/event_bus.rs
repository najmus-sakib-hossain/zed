//! Event bus for publish/subscribe pattern

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;

/// Forge event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ForgeEvent {
    ToolStarted {
        tool_id: String,
        timestamp: i64,
    },
    ToolCompleted {
        tool_id: String,
        duration_ms: u64,
        timestamp: i64,
    },
    PipelineStarted {
        pipeline_id: String,
        timestamp: i64,
    },
    PipelineCompleted {
        pipeline_id: String,
        duration_ms: u64,
        timestamp: i64,
    },
    PackageInstallationBegin {
        package_id: String,
        timestamp: i64,
    },
    PackageInstallationSuccess {
        package_id: String,
        timestamp: i64,
    },
    SecurityViolationDetected {
        description: String,
        severity: String,
        timestamp: i64,
    },
    MagicalConfigInjection {
        config_section: String,
        timestamp: i64,
    },
    Custom {
        event_type: String,
        data: serde_json::Value,
        timestamp: i64,
    },
}

/// Event bus for publish/subscribe pattern
pub struct EventBus {
    subscribers: HashMap<String, Vec<broadcast::Sender<ForgeEvent>>>,
    global_sender: broadcast::Sender<ForgeEvent>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        let (global_sender, _) = broadcast::channel(10000);
        Self {
            subscribers: HashMap::new(),
            global_sender,
        }
    }

    /// Publish an event to all subscribers
    pub fn publish(&self, event: ForgeEvent) -> Result<()> {
        // Send to global channel
        let _ = self.global_sender.send(event.clone());

        // Send to specific event type subscribers
        let event_type = match &event {
            ForgeEvent::ToolStarted { .. } => "tool_started",
            ForgeEvent::ToolCompleted { .. } => "tool_completed",
            ForgeEvent::PipelineStarted { .. } => "pipeline_started",
            ForgeEvent::PipelineCompleted { .. } => "pipeline_completed",
            ForgeEvent::PackageInstallationBegin { .. } => "package_installation_begin",
            ForgeEvent::PackageInstallationSuccess { .. } => "package_installation_success",
            ForgeEvent::SecurityViolationDetected { .. } => "security_violation_detected",
            ForgeEvent::MagicalConfigInjection { .. } => "magical_config_injection",
            ForgeEvent::Custom { event_type, .. } => event_type,
        };

        if let Some(senders) = self.subscribers.get(event_type) {
            for sender in senders {
                let _ = sender.send(event.clone());
            }
        }

        Ok(())
    }

    /// Subscribe to all events
    pub fn subscribe(&self) -> broadcast::Receiver<ForgeEvent> {
        self.global_sender.subscribe()
    }

    /// Subscribe to events of a specific type
    pub fn subscribe_to_type(&mut self, event_type: &str) -> broadcast::Receiver<ForgeEvent> {
        let (sender, receiver) = broadcast::channel(1000);
        self.subscribers.entry(event_type.to_string()).or_default().push(sender);
        receiver
    }

    /// Emit a tool started event
    pub fn emit_tool_started(&self, tool_id: &str) -> Result<()> {
        self.publish(ForgeEvent::ToolStarted {
            tool_id: tool_id.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Emit a tool completed event
    pub fn emit_tool_completed(&self, tool_id: &str, duration_ms: u64) -> Result<()> {
        self.publish(ForgeEvent::ToolCompleted {
            tool_id: tool_id.to_string(),
            duration_ms,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Emit a pipeline started event
    pub fn emit_pipeline_started(&self, pipeline_id: &str) -> Result<()> {
        self.publish(ForgeEvent::PipelineStarted {
            pipeline_id: pipeline_id.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Emit a pipeline completed event
    pub fn emit_pipeline_completed(&self, pipeline_id: &str, duration_ms: u64) -> Result<()> {
        self.publish(ForgeEvent::PipelineCompleted {
            pipeline_id: pipeline_id.to_string(),
            duration_ms,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Emit a package installation begin event
    pub fn emit_package_installation_begin(&self, package_id: &str) -> Result<()> {
        self.publish(ForgeEvent::PackageInstallationBegin {
            package_id: package_id.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Emit a package installation success event
    pub fn emit_package_installation_success(&self, package_id: &str) -> Result<()> {
        self.publish(ForgeEvent::PackageInstallationSuccess {
            package_id: package_id.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Emit a security violation detected event
    pub fn emit_security_violation_detected(
        &self,
        description: &str,
        severity: &str,
    ) -> Result<()> {
        self.publish(ForgeEvent::SecurityViolationDetected {
            description: description.to_string(),
            severity: severity.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Emit a magical config injection event
    pub fn emit_magical_config_injection(&self, config_section: &str) -> Result<()> {
        self.publish(ForgeEvent::MagicalConfigInjection {
            config_section: config_section.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Emit a custom event
    pub fn emit_custom(&self, event_type: &str, data: serde_json::Value) -> Result<()> {
        self.publish(ForgeEvent::Custom {
            event_type: event_type.to_string(),
            data,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        // Publish an event
        event_bus.emit_tool_started("test-tool").unwrap();

        // Receive the event
        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::ToolStarted { tool_id, .. } => {
                assert_eq!(tool_id, "test-tool");
            }
            _ => panic!("Expected ToolStarted event"),
        }
    }

    #[tokio::test]
    async fn test_event_bus_type_specific_subscription() {
        let mut event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe_to_type("tool_started");

        // Publish different types of events
        event_bus.emit_tool_started("test-tool").unwrap();
        event_bus.emit_pipeline_started("test-pipeline").unwrap();

        // Should only receive the tool_started event
        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::ToolStarted { tool_id, .. } => {
                assert_eq!(tool_id, "test-tool");
            }
            _ => panic!("Expected ToolStarted event"),
        }

        // Should not receive the pipeline event on this subscription
        assert!(receiver.try_recv().is_err());
    }
}
