//! Encoder/Decoder for Exchange Messages
//!
//! Converts between Envelope and .sr wire format

use anyhow::Result;

use super::{DaemonId, Envelope, MessageType};

/// Encode envelope to .sr format
pub fn encode_envelope(envelope: &Envelope) -> Result<Vec<u8>> {
    let mut buffer = Vec::with_capacity(256);

    // Write ID (36 bytes UUID)
    buffer.extend_from_slice(envelope.id.as_bytes());

    // Write message type (1 byte)
    buffer.push(encode_message_type(&envelope.msg_type));

    // Write sender (variable)
    encode_daemon_id(&envelope.sender, &mut buffer);

    // Write recipient (variable, with presence flag)
    if let Some(ref recipient) = envelope.recipient {
        buffer.push(1); // Present
        encode_daemon_id(recipient, &mut buffer);
    } else {
        buffer.push(0); // Absent
    }

    // Write timestamp (8 bytes)
    buffer.extend_from_slice(&envelope.timestamp.to_le_bytes());

    // Write correlation ID (variable, with presence flag)
    if let Some(ref correlation) = envelope.correlation_id {
        buffer.push(1); // Present
        buffer.push(correlation.len() as u8);
        buffer.extend_from_slice(correlation.as_bytes());
    } else {
        buffer.push(0); // Absent
    }

    // Write TTL (4 bytes)
    buffer.extend_from_slice(&envelope.ttl.to_le_bytes());

    // Write payload length and payload
    buffer.extend_from_slice(&(envelope.payload.len() as u32).to_le_bytes());
    buffer.extend_from_slice(&envelope.payload);

    Ok(buffer)
}

/// Decode envelope from .sr format
pub fn decode_envelope(data: &[u8]) -> Result<Envelope> {
    let mut pos = 0;

    // Read ID
    if data.len() < pos + 36 {
        return Err(anyhow::anyhow!("Buffer too short for ID"));
    }
    let id = String::from_utf8(data[pos..pos + 36].to_vec())?;
    pos += 36;

    // Read message type
    if data.len() < pos + 1 {
        return Err(anyhow::anyhow!("Buffer too short for message type"));
    }
    let msg_type = decode_message_type(data[pos])?;
    pos += 1;

    // Read sender
    let (sender, new_pos) = decode_daemon_id(data, pos)?;
    pos = new_pos;

    // Read recipient
    if data.len() < pos + 1 {
        return Err(anyhow::anyhow!("Buffer too short for recipient flag"));
    }
    let recipient = if data[pos] == 1 {
        pos += 1;
        let (r, new_pos) = decode_daemon_id(data, pos)?;
        pos = new_pos;
        Some(r)
    } else {
        pos += 1;
        None
    };

    // Read timestamp
    if data.len() < pos + 8 {
        return Err(anyhow::anyhow!("Buffer too short for timestamp"));
    }
    let timestamp = u64::from_le_bytes(data[pos..pos + 8].try_into()?);
    pos += 8;

    // Read correlation ID
    if data.len() < pos + 1 {
        return Err(anyhow::anyhow!("Buffer too short for correlation flag"));
    }
    let correlation_id = if data[pos] == 1 {
        pos += 1;
        if data.len() < pos + 1 {
            return Err(anyhow::anyhow!("Buffer too short for correlation length"));
        }
        let len = data[pos] as usize;
        pos += 1;
        if data.len() < pos + len {
            return Err(anyhow::anyhow!("Buffer too short for correlation ID"));
        }
        let correlation = String::from_utf8(data[pos..pos + len].to_vec())?;
        pos += len;
        Some(correlation)
    } else {
        pos += 1;
        None
    };

    // Read TTL
    if data.len() < pos + 4 {
        return Err(anyhow::anyhow!("Buffer too short for TTL"));
    }
    let ttl = u32::from_le_bytes(data[pos..pos + 4].try_into()?);
    pos += 4;

    // Read payload
    if data.len() < pos + 4 {
        return Err(anyhow::anyhow!("Buffer too short for payload length"));
    }
    let payload_len = u32::from_le_bytes(data[pos..pos + 4].try_into()?) as usize;
    pos += 4;

    if data.len() < pos + payload_len {
        return Err(anyhow::anyhow!("Buffer too short for payload"));
    }
    let payload = data[pos..pos + payload_len].to_vec();

    Ok(Envelope {
        id,
        msg_type,
        sender,
        recipient,
        timestamp,
        correlation_id,
        ttl,
        payload,
    })
}

fn encode_message_type(msg_type: &MessageType) -> u8 {
    match msg_type {
        MessageType::CheckRequest => 0x01,
        MessageType::CheckResult => 0x02,
        MessageType::ScoreRequest => 0x03,
        MessageType::ScoreResult => 0x04,
        MessageType::SyncRequest => 0x05,
        MessageType::SyncProgress => 0x06,
        MessageType::SyncComplete => 0x07,
        MessageType::BranchConfig => 0x08,
        MessageType::BranchUpdate => 0x09,
        MessageType::AiUpdateRequest => 0x0A,
        MessageType::AiUpdateApproval => 0x0B,
        MessageType::AiUpdateResult => 0x0C,
        MessageType::Heartbeat => 0x0D,
        MessageType::StatusRequest => 0x0E,
        MessageType::StatusResponse => 0x0F,
        MessageType::Error => 0xFE,
        MessageType::Ack => 0xFF,
    }
}

fn decode_message_type(byte: u8) -> Result<MessageType> {
    match byte {
        0x01 => Ok(MessageType::CheckRequest),
        0x02 => Ok(MessageType::CheckResult),
        0x03 => Ok(MessageType::ScoreRequest),
        0x04 => Ok(MessageType::ScoreResult),
        0x05 => Ok(MessageType::SyncRequest),
        0x06 => Ok(MessageType::SyncProgress),
        0x07 => Ok(MessageType::SyncComplete),
        0x08 => Ok(MessageType::BranchConfig),
        0x09 => Ok(MessageType::BranchUpdate),
        0x0A => Ok(MessageType::AiUpdateRequest),
        0x0B => Ok(MessageType::AiUpdateApproval),
        0x0C => Ok(MessageType::AiUpdateResult),
        0x0D => Ok(MessageType::Heartbeat),
        0x0E => Ok(MessageType::StatusRequest),
        0x0F => Ok(MessageType::StatusResponse),
        0xFE => Ok(MessageType::Error),
        0xFF => Ok(MessageType::Ack),
        _ => Err(anyhow::anyhow!("Unknown message type: {}", byte)),
    }
}

fn encode_daemon_id(id: &DaemonId, buffer: &mut Vec<u8>) {
    match id {
        DaemonId::Agent => {
            buffer.push(0x01); // Type: Agent
        }
        DaemonId::Project(path_hash) => {
            buffer.push(0x02); // Type: Project
            buffer.push(path_hash.len() as u8);
            buffer.extend_from_slice(path_hash.as_bytes());
        }
    }
}

fn decode_daemon_id(data: &[u8], pos: usize) -> Result<(DaemonId, usize)> {
    if data.len() < pos + 1 {
        return Err(anyhow::anyhow!("Buffer too short for daemon ID type"));
    }

    match data[pos] {
        0x01 => Ok((DaemonId::Agent, pos + 1)),
        0x02 => {
            if data.len() < pos + 2 {
                return Err(anyhow::anyhow!("Buffer too short for project hash length"));
            }
            let len = data[pos + 1] as usize;
            if data.len() < pos + 2 + len {
                return Err(anyhow::anyhow!("Buffer too short for project hash"));
            }
            let hash = String::from_utf8(data[pos + 2..pos + 2 + len].to_vec())?;
            Ok((DaemonId::Project(hash), pos + 2 + len))
        }
        _ => Err(anyhow::anyhow!("Unknown daemon ID type: {}", data[pos])),
    }
}

/// Encode message payload to .sr format
pub fn encode_payload<T: serde::Serialize>(msg: &T) -> Result<Vec<u8>> {
    // Using bincode for now, could use custom .sr format
    Ok(bincode::serialize(msg)?)
}

/// Decode message payload from .sr format
pub fn decode_payload<T: serde::de::DeserializeOwned>(data: &[u8]) -> Result<T> {
    Ok(bincode::deserialize(data)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_roundtrip() {
        let envelope = Envelope::new(MessageType::Heartbeat, DaemonId::Agent, vec![1, 2, 3, 4]);

        let encoded = encode_envelope(&envelope).unwrap();
        let decoded = decode_envelope(&encoded).unwrap();

        assert_eq!(envelope.id, decoded.id);
        assert_eq!(envelope.payload, decoded.payload);
    }
}
