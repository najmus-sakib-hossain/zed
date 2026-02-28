use std::sync::Arc;

use anyhow::{Result, anyhow};
use futures::{SinkExt, StreamExt};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

use super::protocol::SyncManager;
use crate::crdt::Operation;
use crate::storage::OperationLog;
use crate::sync::{GLOBAL_CLOCK, SyncMessage};
use colored::*;
use dashmap::DashSet;
use reqwest::Client;
use uuid::Uuid;

/// Connect to a remote WebSocket peer and bridge operations between the
/// in-process SyncManager and the remote. Returns a JoinHandle for the
/// background task managing the connection.
pub async fn connect_peer(
    url: &str,
    actor_id: String,
    repo_id: String,
    sync: SyncManager,
    oplog: Arc<OperationLog>,
) -> Result<JoinHandle<()>> {
    let seen = Arc::new(DashSet::new());
    let url = Url::parse(url).map_err(|e| anyhow!("invalid ws url: {e}"))?;
    let (ws_stream, _) = tokio_tungstenite::connect_async(url.as_str()).await?;

    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    // Send handshake so the peer can deduplicate correctly
    let handshake = SyncMessage::handshake(actor_id.clone(), repo_id.clone());
    let handshake_json = serde_json::to_string(&handshake)?;
    ws_tx.send(Message::Text(handshake_json.into())).await?;

    // Initial cold start sync via HTTP
    if let Some(ops_url) = derive_ops_url(&url) {
        if let Ok(ops) = fetch_initial_ops(ops_url).await {
            for op in ops.into_iter().rev() {
                if insert_seen(&seen, op.id) {
                    if let Some(lamport) = op.lamport() {
                        GLOBAL_CLOCK.observe(lamport);
                    }
                    if let Ok(true) = oplog.append(op.clone()) {
                        let _ = sync.publish(Arc::new(op));
                    }
                }
            }
        }
    }

    // Subscribe to local ops to forward to remote
    let mut rx = sync.subscribe();

    // Spawn forwarder for local -> remote
    let actor_id_clone = actor_id.clone();
    let seen_forward = seen.clone();
    let forward = tokio::spawn(async move {
        while let Ok(op_arc) = rx.recv().await {
            // Only forward our own actor's ops to reduce echo, server will broadcast
            if op_arc.actor_id == actor_id_clone && insert_seen(&seen_forward, op_arc.id) {
                if let Ok(json) = serde_json::to_string(&SyncMessage::operation((*op_arc).clone()))
                {
                    if ws_tx.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Spawn receiver for remote -> local
    let sync_clone = sync.clone();
    let actor_id_clone2 = actor_id.clone();
    let oplog_clone = oplog.clone();
    let seen_recv = seen.clone();
    let recv = tokio::spawn(async move {
        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let text: String = text.to_string();
                    if let Ok(msg) = serde_json::from_str::<SyncMessage>(&text) {
                        match msg {
                            SyncMessage::Handshake { actor_id, repo_id } => {
                                println!(
                                    "{} Connected peer handshake (actor={} repo={})",
                                    "↔".bright_blue(),
                                    actor_id.bright_yellow(),
                                    repo_id.bright_white()
                                );
                            }
                            SyncMessage::Operation { operation: op } => {
                                if op.actor_id != actor_id_clone2 && insert_seen(&seen_recv, op.id)
                                {
                                    if let Some(lamport) = op.lamport() {
                                        GLOBAL_CLOCK.observe(lamport);
                                    }
                                    let _ = oplog_clone.append(op.clone());
                                    let _ = sync_clone.publish(Arc::new(op));
                                }
                            }
                        }
                    } else if let Ok(op) = serde_json::from_str::<Operation>(&text) {
                        if op.actor_id != actor_id_clone2 && insert_seen(&seen_recv, op.id) {
                            if let Some(lamport) = op.lamport() {
                                GLOBAL_CLOCK.observe(lamport);
                            }
                            let _ = oplog_clone.append(op.clone());
                            let _ = sync_clone.publish(Arc::new(op));
                        }
                    }
                }
                Ok(Message::Binary(bin)) => {
                    if let Ok(op) = serde_cbor::from_slice::<Operation>(&bin) {
                        if op.actor_id != actor_id_clone2 && insert_seen(&seen_recv, op.id) {
                            if let Some(lamport) = op.lamport() {
                                GLOBAL_CLOCK.observe(lamport);
                            }
                            let _ = oplog_clone.append(op.clone());
                            let _ = sync_clone.publish(Arc::new(op));
                        }
                    }
                }
                Ok(Message::Frame(_)) => { /* ignore */ }
                Ok(Message::Close(_)) | Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {
                    // no-op
                }
                Err(_) => break,
            }
        }
    });

    // Join both tasks under a single handle
    let handle = tokio::spawn(async move {
        let _ = tokio::join!(forward, recv);
    });

    Ok(handle)
}

const SEEN_LIMIT: usize = 10_000;

fn insert_seen(cache: &DashSet<Uuid>, id: Uuid) -> bool {
    let inserted = cache.insert(id);
    if inserted {
        enforce_seen_limit(cache);
    }
    inserted
}

fn enforce_seen_limit(cache: &DashSet<Uuid>) {
    while cache.len() > SEEN_LIMIT {
        if let Some(entry) = cache.iter().next() {
            let key = *entry.key();
            drop(entry);
            cache.remove(&key);
        } else {
            break;
        }
    }
}

fn derive_ops_url(ws_url: &Url) -> Option<Url> {
    let mut http = ws_url.clone();
    let scheme = match ws_url.scheme() {
        "ws" => "http",
        "wss" => "https",
        _ => return None,
    };

    if http.set_scheme(scheme).is_err() {
        return None;
    }

    http.set_path("/ops");
    http.set_query(Some("limit=200"));
    Some(http)
}

async fn fetch_initial_ops(url: Url) -> Result<Vec<Operation>> {
    let client = Client::new();
    let resp = client.get(url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(anyhow!("failed to fetch ops: {status}"));
    }
    let ops = resp.json::<Vec<Operation>>().await?;
    Ok(ops)
}
