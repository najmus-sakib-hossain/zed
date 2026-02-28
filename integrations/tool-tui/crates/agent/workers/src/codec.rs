use bytes::Bytes;
use chrono::{DateTime, Utc};
use dx_agent_protocol::framing::{Frame, FrameType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkerMessageKind {
    Ping,
    Pong,
    Event,
    Command,
    Result,
    Error,
    Health,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEnvelope {
    pub id: String,
    pub worker_id: String,
    pub kind: WorkerMessageKind,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

impl WorkerEnvelope {
    pub fn new(
        worker_id: impl Into<String>,
        kind: WorkerMessageKind,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            worker_id: worker_id.into(),
            kind,
            payload,
            timestamp: Utc::now(),
        }
    }
}

pub fn encode_envelope(envelope: &WorkerEnvelope) -> anyhow::Result<Vec<u8>> {
    let payload = serde_json::to_vec(envelope)?;
    let frame = Frame::binary(Bytes::from(payload));
    Ok(frame.encode().to_vec())
}

pub fn decode_envelope(data: &[u8]) -> anyhow::Result<WorkerEnvelope> {
    let (frame, _) = Frame::decode(data)?;
    if frame.frame_type != FrameType::Binary {
        anyhow::bail!("expected binary frame, got {:?}", frame.frame_type);
    }
    let envelope: WorkerEnvelope = serde_json::from_slice(&frame.payload)?;
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_roundtrip() {
        let env = WorkerEnvelope::new(
            "worker-a",
            WorkerMessageKind::Command,
            serde_json::json!({"cmd":"send"}),
        );
        let encoded = encode_envelope(&env).expect("encode");
        let decoded = decode_envelope(&encoded).expect("decode");
        assert_eq!(decoded.worker_id, "worker-a");
        assert_eq!(decoded.kind, WorkerMessageKind::Command);
        assert_eq!(decoded.payload["cmd"], "send");
    }
}
