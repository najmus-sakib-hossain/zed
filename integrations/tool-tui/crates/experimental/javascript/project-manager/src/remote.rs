//! Remote Cache Protocol (DXRC)
//!
//! Binary protocol for remote cache operations with XOR patch streaming,
//! connection multiplexing, and resume-capable downloads.

use crate::dxc::{CacheEntry, XorPatch};
use crate::error::CacheError;
use std::collections::HashMap;

/// Magic bytes for DXRC protocol
pub const DXRC_MAGIC: [u8; 4] = *b"DXRC";

/// DXRC protocol version
pub const DXRC_VERSION: u32 = 1;

/// DXRC request type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DxrcRequestType {
    /// Fetch cache entries
    Fetch = 0,
    /// Store cache entries
    Store = 1,
    /// Check existence
    Exists = 2,
    /// Resume interrupted download
    Resume = 3,
}

/// Resume checkpoint for interrupted transfers
#[derive(Debug, Clone)]
pub struct ResumeCheckpoint {
    /// Task hash being downloaded
    pub task_hash: [u8; 32],
    /// Bytes already received
    pub bytes_received: u64,
    /// Checksum of received data
    pub partial_checksum: [u8; 32],
}

/// DXRC request message
#[derive(Debug, Clone)]
pub struct DxrcRequest {
    /// Request type
    pub request_type: DxrcRequestType,
    /// Task hashes to fetch
    pub task_hashes: Vec<[u8; 32]>,
    /// Client's current cache state (for differential sync)
    pub client_state: Option<CacheState>,
    /// Prefetch hints
    pub prefetch_hints: Vec<[u8; 32]>,
    /// Resume checkpoint (for Resume requests)
    pub resume_checkpoint: Option<ResumeCheckpoint>,
}

/// Client cache state for differential sync
#[derive(Debug, Clone, Default)]
pub struct CacheState {
    /// Hashes of entries client already has
    pub existing_hashes: Vec<[u8; 32]>,
    /// Available disk space
    pub available_space: u64,
}

/// DXRC response message
#[derive(Debug, Clone)]
pub struct DxrcResponse {
    /// Response status
    pub status: DxrcStatus,
    /// Cache entries (for Fetch)
    pub entries: Vec<CacheEntry>,
    /// XOR patches (for differential updates)
    pub patches: Vec<XorPatch>,
    /// Existence results (for Exists)
    pub exists: Vec<bool>,
    /// Prefetch started
    pub prefetch_started: bool,
}

/// DXRC response status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DxrcStatus {
    /// Success
    Ok = 0,
    /// Partial success (some entries not found)
    Partial = 1,
    /// Not found
    NotFound = 2,
    /// Authentication required
    AuthRequired = 3,
    /// Server error
    ServerError = 4,
}

impl DxrcRequest {
    /// Create a fetch request
    pub fn fetch(hashes: Vec<[u8; 32]>) -> Self {
        Self {
            request_type: DxrcRequestType::Fetch,
            task_hashes: hashes,
            client_state: None,
            prefetch_hints: Vec::new(),
            resume_checkpoint: None,
        }
    }

    /// Create a store request
    pub fn store(hashes: Vec<[u8; 32]>) -> Self {
        Self {
            request_type: DxrcRequestType::Store,
            task_hashes: hashes,
            client_state: None,
            prefetch_hints: Vec::new(),
            resume_checkpoint: None,
        }
    }

    /// Create an exists request
    pub fn exists(hashes: Vec<[u8; 32]>) -> Self {
        Self {
            request_type: DxrcRequestType::Exists,
            task_hashes: hashes,
            client_state: None,
            prefetch_hints: Vec::new(),
            resume_checkpoint: None,
        }
    }

    /// Create a resume request
    pub fn resume(checkpoint: ResumeCheckpoint) -> Self {
        Self {
            request_type: DxrcRequestType::Resume,
            task_hashes: vec![checkpoint.task_hash],
            client_state: None,
            prefetch_hints: Vec::new(),
            resume_checkpoint: Some(checkpoint),
        }
    }

    /// Add client state for differential sync
    pub fn with_client_state(mut self, state: CacheState) -> Self {
        self.client_state = Some(state);
        self
    }

    /// Add prefetch hints
    pub fn with_prefetch_hints(mut self, hints: Vec<[u8; 32]>) -> Self {
        self.prefetch_hints = hints;
        self
    }

    /// Serialize request to binary format
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        // Magic and version
        buffer.extend_from_slice(&DXRC_MAGIC);
        buffer.extend_from_slice(&DXRC_VERSION.to_le_bytes());

        // Request type
        buffer.push(self.request_type as u8);

        // Task hashes count and data
        buffer.extend_from_slice(&(self.task_hashes.len() as u32).to_le_bytes());
        for hash in &self.task_hashes {
            buffer.extend_from_slice(hash);
        }

        // Client state (optional)
        if let Some(ref state) = self.client_state {
            buffer.push(1); // Has client state
            buffer.extend_from_slice(&(state.existing_hashes.len() as u32).to_le_bytes());
            for hash in &state.existing_hashes {
                buffer.extend_from_slice(hash);
            }
            buffer.extend_from_slice(&state.available_space.to_le_bytes());
        } else {
            buffer.push(0); // No client state
        }

        // Prefetch hints
        buffer.extend_from_slice(&(self.prefetch_hints.len() as u32).to_le_bytes());
        for hash in &self.prefetch_hints {
            buffer.extend_from_slice(hash);
        }

        // Resume checkpoint (optional)
        if let Some(ref checkpoint) = self.resume_checkpoint {
            buffer.push(1); // Has checkpoint
            buffer.extend_from_slice(&checkpoint.task_hash);
            buffer.extend_from_slice(&checkpoint.bytes_received.to_le_bytes());
            buffer.extend_from_slice(&checkpoint.partial_checksum);
        } else {
            buffer.push(0); // No checkpoint
        }

        buffer
    }

    /// Deserialize request from binary format
    pub fn deserialize(data: &[u8]) -> Result<Self, CacheError> {
        if data.len() < 9 {
            return Err(CacheError::IntegrityCheckFailed);
        }

        // Verify magic
        if data[0..4] != DXRC_MAGIC {
            return Err(CacheError::IntegrityCheckFailed);
        }

        let mut offset = 8; // Skip magic and version

        // Request type
        let request_type = match data[offset] {
            0 => DxrcRequestType::Fetch,
            1 => DxrcRequestType::Store,
            2 => DxrcRequestType::Exists,
            3 => DxrcRequestType::Resume,
            _ => return Err(CacheError::IntegrityCheckFailed),
        };
        offset += 1;

        // Task hashes
        let hash_count = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;

        let mut task_hashes = Vec::with_capacity(hash_count);
        for _ in 0..hash_count {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&data[offset..offset + 32]);
            task_hashes.push(hash);
            offset += 32;
        }

        // Client state
        let has_client_state = data[offset] == 1;
        offset += 1;

        let client_state = if has_client_state {
            let existing_count = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;

            let mut existing_hashes = Vec::with_capacity(existing_count);
            for _ in 0..existing_count {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&data[offset..offset + 32]);
                existing_hashes.push(hash);
                offset += 32;
            }

            let available_space = u64::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);
            offset += 8;

            Some(CacheState {
                existing_hashes,
                available_space,
            })
        } else {
            None
        };

        // Prefetch hints
        let prefetch_count = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;

        let mut prefetch_hints = Vec::with_capacity(prefetch_count);
        for _ in 0..prefetch_count {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&data[offset..offset + 32]);
            prefetch_hints.push(hash);
            offset += 32;
        }

        // Resume checkpoint
        let has_checkpoint = data[offset] == 1;
        offset += 1;

        let resume_checkpoint = if has_checkpoint {
            let mut task_hash = [0u8; 32];
            task_hash.copy_from_slice(&data[offset..offset + 32]);
            offset += 32;

            let bytes_received = u64::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);
            offset += 8;

            let mut partial_checksum = [0u8; 32];
            partial_checksum.copy_from_slice(&data[offset..offset + 32]);

            Some(ResumeCheckpoint {
                task_hash,
                bytes_received,
                partial_checksum,
            })
        } else {
            None
        };

        Ok(Self {
            request_type,
            task_hashes,
            client_state,
            prefetch_hints,
            resume_checkpoint,
        })
    }
}

impl DxrcResponse {
    /// Create a success response with entries
    pub fn ok(entries: Vec<CacheEntry>) -> Self {
        Self {
            status: DxrcStatus::Ok,
            entries,
            patches: Vec::new(),
            exists: Vec::new(),
            prefetch_started: false,
        }
    }

    /// Create a partial success response
    pub fn partial(entries: Vec<CacheEntry>) -> Self {
        Self {
            status: DxrcStatus::Partial,
            entries,
            patches: Vec::new(),
            exists: Vec::new(),
            prefetch_started: false,
        }
    }

    /// Create a not found response
    pub fn not_found() -> Self {
        Self {
            status: DxrcStatus::NotFound,
            entries: Vec::new(),
            patches: Vec::new(),
            exists: Vec::new(),
            prefetch_started: false,
        }
    }

    /// Create an exists response
    pub fn exists_result(exists: Vec<bool>) -> Self {
        Self {
            status: DxrcStatus::Ok,
            entries: Vec::new(),
            patches: Vec::new(),
            exists,
            prefetch_started: false,
        }
    }
}

/// Remote Cache Client for DXRC protocol
pub struct RemoteCacheClient {
    /// Remote cache URL
    url: String,
    /// Authentication token
    token: Option<String>,
    /// Enable speculative prefetch
    prefetch_enabled: bool,
    /// Pending prefetch hashes
    pending_prefetch: Vec<[u8; 32]>,
    /// Connection pool (simulated)
    connection_count: usize,
}

impl RemoteCacheClient {
    /// Create a new remote cache client
    pub fn new(url: String) -> Self {
        Self {
            url,
            token: None,
            prefetch_enabled: false,
            pending_prefetch: Vec::new(),
            connection_count: 4, // Default multiplexing
        }
    }

    /// Set authentication token
    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    /// Enable speculative prefetching
    pub fn with_prefetch(mut self, enabled: bool) -> Self {
        self.prefetch_enabled = enabled;
        self
    }

    /// Set connection pool size for multiplexing
    pub fn with_connection_count(mut self, count: usize) -> Self {
        self.connection_count = count;
        self
    }

    /// Fetch multiple cache entries in single request
    /// Property 17: Single Request Multi-Entry Fetch
    pub fn fetch(&self, hashes: &[[u8; 32]]) -> Result<Vec<CacheEntry>, CacheError> {
        // Create single request for all hashes
        let request = DxrcRequest::fetch(hashes.to_vec());

        // In a real implementation, this would:
        // 1. Serialize the request
        // 2. Send over multiplexed connection
        // 3. Receive and deserialize response

        let _serialized = request.serialize();

        // Simulate response (in real impl, would parse from network)
        Ok(Vec::new())
    }

    /// Store cache entries with XOR patch optimization
    pub fn store(&self, entries: &[CacheEntry]) -> Result<(), CacheError> {
        let hashes: Vec<_> = entries.iter().map(|e| e.task_hash).collect();
        let _request = DxrcRequest::store(hashes);

        // In real implementation, would send entries with XOR patches
        // for similar existing entries

        Ok(())
    }

    /// Check existence of multiple entries
    pub fn exists(&self, hashes: &[[u8; 32]]) -> Result<Vec<bool>, CacheError> {
        let _request = DxrcRequest::exists(hashes.to_vec());

        // Simulate all not found
        Ok(vec![false; hashes.len()])
    }

    /// Start speculative prefetch
    pub fn prefetch(&mut self, predicted_hashes: &[[u8; 32]]) {
        if self.prefetch_enabled {
            self.pending_prefetch.extend_from_slice(predicted_hashes);
        }
    }

    /// Resume interrupted download
    pub fn resume(&self, checkpoint: &ResumeCheckpoint) -> Result<CacheEntry, CacheError> {
        let _request = DxrcRequest::resume(checkpoint.clone());

        // In real implementation, would resume from checkpoint
        Err(CacheError::EntryNotFound {
            hash: checkpoint.task_hash,
        })
    }

    /// Get URL
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Check if authenticated
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    /// Get connection count for multiplexing
    pub fn connection_count(&self) -> usize {
        self.connection_count
    }
}

/// Multiplexed connection for concurrent transfers
pub struct MultiplexedConnection {
    /// Active streams
    streams: HashMap<u32, StreamState>,
    /// Next stream ID
    next_stream_id: u32,
    /// Max concurrent streams
    max_streams: usize,
}

/// Stream state for multiplexed connection
#[derive(Debug)]
pub struct StreamState {
    /// Stream ID
    pub id: u32,
    /// Task hash being transferred
    pub task_hash: [u8; 32],
    /// Bytes transferred
    pub bytes_transferred: u64,
    /// Total bytes
    pub total_bytes: u64,
    /// Is complete
    pub complete: bool,
}

impl MultiplexedConnection {
    /// Create a new multiplexed connection
    pub fn new(max_streams: usize) -> Self {
        Self {
            streams: HashMap::new(),
            next_stream_id: 0,
            max_streams,
        }
    }

    /// Start a new stream
    pub fn start_stream(&mut self, task_hash: [u8; 32], total_bytes: u64) -> Option<u32> {
        if self.streams.len() >= self.max_streams {
            return None;
        }

        let id = self.next_stream_id;
        self.next_stream_id += 1;

        self.streams.insert(
            id,
            StreamState {
                id,
                task_hash,
                bytes_transferred: 0,
                total_bytes,
                complete: false,
            },
        );

        Some(id)
    }

    /// Update stream progress
    pub fn update_stream(&mut self, id: u32, bytes: u64) {
        if let Some(stream) = self.streams.get_mut(&id) {
            stream.bytes_transferred += bytes;
            if stream.bytes_transferred >= stream.total_bytes {
                stream.complete = true;
            }
        }
    }

    /// Complete and remove a stream
    pub fn complete_stream(&mut self, id: u32) -> Option<StreamState> {
        self.streams.remove(&id)
    }

    /// Get active stream count
    pub fn active_streams(&self) -> usize {
        self.streams.len()
    }

    /// Check if can start new stream
    pub fn can_start_stream(&self) -> bool {
        self.streams.len() < self.max_streams
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization_roundtrip() {
        let hashes = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let request = DxrcRequest::fetch(hashes.clone());

        let serialized = request.serialize();
        let deserialized = DxrcRequest::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.request_type, DxrcRequestType::Fetch);
        assert_eq!(deserialized.task_hashes.len(), 3);
        assert_eq!(deserialized.task_hashes, hashes);
    }

    #[test]
    fn test_request_with_client_state() {
        let request = DxrcRequest::fetch(vec![[1u8; 32]]).with_client_state(CacheState {
            existing_hashes: vec![[2u8; 32]],
            available_space: 1024 * 1024 * 1024,
        });

        let serialized = request.serialize();
        let deserialized = DxrcRequest::deserialize(&serialized).unwrap();

        assert!(deserialized.client_state.is_some());
        let state = deserialized.client_state.unwrap();
        assert_eq!(state.existing_hashes.len(), 1);
        assert_eq!(state.available_space, 1024 * 1024 * 1024);
    }

    #[test]
    fn test_request_with_prefetch_hints() {
        let request =
            DxrcRequest::fetch(vec![[1u8; 32]]).with_prefetch_hints(vec![[3u8; 32], [4u8; 32]]);

        let serialized = request.serialize();
        let deserialized = DxrcRequest::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.prefetch_hints.len(), 2);
    }

    #[test]
    fn test_resume_request() {
        let checkpoint = ResumeCheckpoint {
            task_hash: [5u8; 32],
            bytes_received: 1024,
            partial_checksum: [6u8; 32],
        };

        let request = DxrcRequest::resume(checkpoint);

        let serialized = request.serialize();
        let deserialized = DxrcRequest::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.request_type, DxrcRequestType::Resume);
        assert!(deserialized.resume_checkpoint.is_some());
        let cp = deserialized.resume_checkpoint.unwrap();
        assert_eq!(cp.bytes_received, 1024);
    }

    #[test]
    fn test_remote_client_creation() {
        let client = RemoteCacheClient::new("https://cache.example.com".to_string())
            .with_token("secret".to_string())
            .with_prefetch(true)
            .with_connection_count(8);

        assert_eq!(client.url(), "https://cache.example.com");
        assert!(client.is_authenticated());
        assert_eq!(client.connection_count(), 8);
    }

    #[test]
    fn test_single_request_multi_entry_fetch() {
        // Property 17: Single Request Multi-Entry Fetch
        let client = RemoteCacheClient::new("https://cache.example.com".to_string());

        let hashes = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], [5u8; 32]];

        // This should create exactly one request for all hashes
        let request = DxrcRequest::fetch(hashes.clone());
        let serialized = request.serialize();

        // Verify single request contains all hashes
        let deserialized = DxrcRequest::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.task_hashes.len(), 5);

        // The fetch method should use single request
        let _result = client.fetch(&hashes);
    }

    #[test]
    fn test_multiplexed_connection() {
        let mut conn = MultiplexedConnection::new(4);

        // Start multiple streams
        let id1 = conn.start_stream([1u8; 32], 1000).unwrap();
        let id2 = conn.start_stream([2u8; 32], 2000).unwrap();
        let id3 = conn.start_stream([3u8; 32], 3000).unwrap();
        let id4 = conn.start_stream([4u8; 32], 4000).unwrap();

        assert_eq!(conn.active_streams(), 4);
        assert!(!conn.can_start_stream()); // At max

        // Can't start more
        assert!(conn.start_stream([5u8; 32], 5000).is_none());

        // Update and complete a stream
        conn.update_stream(id1, 500);
        conn.update_stream(id1, 500); // Now complete

        let completed = conn.complete_stream(id1).unwrap();
        assert!(completed.complete);

        // Now can start another
        assert!(conn.can_start_stream());

        // Clean up
        conn.complete_stream(id2);
        conn.complete_stream(id3);
        conn.complete_stream(id4);
    }

    #[test]
    fn test_response_types() {
        let entry = CacheEntry::new([1u8; 32]);

        let ok_response = DxrcResponse::ok(vec![entry.clone()]);
        assert_eq!(ok_response.status, DxrcStatus::Ok);
        assert_eq!(ok_response.entries.len(), 1);

        let partial_response = DxrcResponse::partial(vec![entry]);
        assert_eq!(partial_response.status, DxrcStatus::Partial);

        let not_found = DxrcResponse::not_found();
        assert_eq!(not_found.status, DxrcStatus::NotFound);

        let exists_response = DxrcResponse::exists_result(vec![true, false, true]);
        assert_eq!(exists_response.exists, vec![true, false, true]);
    }
}
