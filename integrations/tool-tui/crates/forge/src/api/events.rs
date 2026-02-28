//! Global Event Bus & Observability APIs

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};
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

static EVENT_BUS: OnceLock<Arc<RwLock<EventBus>>> = OnceLock::new();

struct EventBus {
    sender: broadcast::Sender<ForgeEvent>,
}

impl EventBus {
    fn new() -> Self {
        let (sender, _) = broadcast::channel(10000);
        Self { sender }
    }
}

fn get_event_bus() -> Arc<RwLock<EventBus>> {
    EVENT_BUS.get_or_init(|| Arc::new(RwLock::new(EventBus::new()))).clone()
}

pub fn publish_event(event: ForgeEvent) -> Result<()> {
    let bus = get_event_bus();
    let bus = bus.read();
    let _ = bus.sender.send(event);
    Ok(())
}

pub fn subscribe_to_event_stream() -> broadcast::Receiver<ForgeEvent> {
    let bus = get_event_bus();
    let bus = bus.read();
    bus.sender.subscribe()
}

pub fn emit_tool_started_event(tool_id: &str) -> Result<()> {
    publish_event(ForgeEvent::ToolStarted {
        tool_id: tool_id.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

pub fn emit_tool_completed_event(tool_id: &str, duration_ms: u64) -> Result<()> {
    publish_event(ForgeEvent::ToolCompleted {
        tool_id: tool_id.to_string(),
        duration_ms,
        timestamp: chrono::Utc::now().timestamp(),
    })
}

pub fn emit_pipeline_started_event(pipeline_id: &str) -> Result<()> {
    publish_event(ForgeEvent::PipelineStarted {
        pipeline_id: pipeline_id.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

pub fn emit_pipeline_completed_event(pipeline_id: &str, duration_ms: u64) -> Result<()> {
    publish_event(ForgeEvent::PipelineCompleted {
        pipeline_id: pipeline_id.to_string(),
        duration_ms,
        timestamp: chrono::Utc::now().timestamp(),
    })
}

pub fn emit_package_installation_begin(package_id: &str) -> Result<()> {
    publish_event(ForgeEvent::PackageInstallationBegin {
        package_id: package_id.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

pub fn emit_package_installation_success(package_id: &str) -> Result<()> {
    publish_event(ForgeEvent::PackageInstallationSuccess {
        package_id: package_id.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

pub fn emit_security_violation_detected(description: &str, severity: &str) -> Result<()> {
    publish_event(ForgeEvent::SecurityViolationDetected {
        description: description.to_string(),
        severity: severity.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

pub fn emit_magical_config_injection(config_section: &str) -> Result<()> {
    publish_event(ForgeEvent::MagicalConfigInjection {
        config_section: config_section.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    })
}
