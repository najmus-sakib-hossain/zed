use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Hello { client: String, version: String },
    PushManifest { commit_id: String },
    ChunkData { hash: String, data_b64: String },
    PullRequest { commit_id: String },
    Ack { id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Ok { message: String },
    NeedChunks { hashes: Vec<String> },
    Manifest { commit_id: String, data_b64: String },
    ChunkData { hash: String, data_b64: String },
    AckCommit { commit_id: String },
}

pub fn serialize_client_message(msg: &ClientMessage) -> Result<Vec<u8>> {
    serde_json::to_vec(msg).context("serialize client message")
}

pub fn deserialize_client_message(bytes: &[u8]) -> Result<ClientMessage> {
    serde_json::from_slice(bytes).context("deserialize client message")
}

pub fn serialize_server_message(msg: &ServerMessage) -> Result<Vec<u8>> {
    serde_json::to_vec(msg).context("serialize server message")
}

pub fn deserialize_server_message(bytes: &[u8]) -> Result<ServerMessage> {
    serde_json::from_slice(bytes).context("deserialize server message")
}
