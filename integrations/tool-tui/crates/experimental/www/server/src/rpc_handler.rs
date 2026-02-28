// Server-side RPC handler for dx-query

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

/// Binary RPC request
#[derive(Debug, Deserialize)]
pub struct RPCRequest {
    pub query_id: String,
    pub params: Vec<u8>, // Binary parameters
}

/// Binary RPC response
#[derive(Debug, Serialize)]
pub struct RPCResponse {
    pub data: Vec<u8>, // Binary data
    pub cached: bool,
}

/// Handle RPC query request
#[cfg(feature = "query")]
pub async fn handle_rpc(
    State(_state): State<crate::ServerState>,
    Json(req): Json<RPCRequest>,
) -> impl IntoResponse {
    // Execute query (cache disabled for now - needs ecosystem integration)
    let data = execute_query(&req.query_id, &req.params).await;

    (
        StatusCode::OK,
        Json(RPCResponse {
            data,
            cached: false,
        }),
    )
}

/// Execute query (placeholder)
async fn execute_query(_query_id: &str, _params: &[u8]) -> Vec<u8> {
    // This would dispatch to actual query handlers
    vec![0, 1, 2, 3] // Placeholder response
}

/// Hash query for caching
#[allow(dead_code)] // Used in tests and future caching implementation
fn hash_query(query_id: &str, params: &[u8]) -> u32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    query_id.hash(&mut hasher);
    params.hash(&mut hasher);
    hasher.finish() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_query() {
        let hash1 = hash_query("getUser", &[1, 2, 3]);
        let hash2 = hash_query("getUser", &[1, 2, 3]);
        let hash3 = hash_query("getUser", &[4, 5, 6]);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
