//! # dx-offline — CRDT Offline Sync Engine
//!
//! Replace Firebase Offline + Y.js boilerplate with automatic conflict resolution.
//!
//! ## Features
//! - Yjs CRDT documents
//! - IndexedDB persistence
//! - Automatic conflict resolution
//! - Binary sync protocol

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Doc, GetString, ReadTxn, StateVector, Text, TextRef, Transact, Update};

#[cfg(target_arch = "wasm32")]
use indexed_db_futures::{IdbDatabase, IdbQuerySource};

/// CRDT document wrapper
pub struct CRDTDocument {
    /// Yjs document
    doc: Doc,
    /// Document ID
    id: String,
    /// Text field (most common use case)
    text: TextRef,
}

impl CRDTDocument {
    /// Create new CRDT document
    pub fn new(id: impl Into<String>) -> Self {
        let doc = Doc::new();
        let text = doc.get_or_insert_text("content");

        Self {
            doc,
            id: id.into(),
            text,
        }
    }

    /// Get document ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Insert text at position
    pub fn insert(&self, index: u32, text: &str) {
        let mut txn = self.doc.transact_mut();
        self.text.insert(&mut txn, index, text);
    }

    /// Delete text range
    pub fn delete(&self, index: u32, len: u32) {
        let mut txn = self.doc.transact_mut();
        self.text.remove_range(&mut txn, index, len);
    }

    /// Get full text content
    pub fn get_text(&self) -> String {
        let txn = self.doc.transact();
        self.text.get_string(&txn)
    }

    /// Get state vector (for sync)
    pub fn get_state_vector(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.state_vector().encode_v1()
    }

    /// Apply update from remote
    pub fn apply_update(&self, update: &[u8]) -> Result<(), String> {
        let mut txn = self.doc.transact_mut();
        let update = Update::decode_v1(update).map_err(|e| format!("Decode error: {}", e))?;
        txn.apply_update(update);
        Ok(())
    }

    /// Encode update since state vector
    pub fn encode_state_as_update(&self, state_vector: &[u8]) -> Vec<u8> {
        let txn = self.doc.transact();
        let sv = StateVector::decode_v1(state_vector).unwrap_or_default();
        txn.encode_diff_v1(&sv)
    }

    /// Get full document state
    pub fn get_full_state(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.encode_diff_v1(&StateVector::default())
    }
}

/// CRDT store (manages multiple documents)
pub struct CRDTStore {
    documents: std::collections::HashMap<String, CRDTDocument>,
}

impl CRDTStore {
    /// Create new store
    pub fn new() -> Self {
        Self {
            documents: std::collections::HashMap::new(),
        }
    }

    /// Get or create document
    pub fn get_or_create(&mut self, id: impl Into<String>) -> &mut CRDTDocument {
        let id = id.into();
        self.documents.entry(id.clone()).or_insert_with(|| CRDTDocument::new(id))
    }

    /// Get document
    pub fn get(&self, id: &str) -> Option<&CRDTDocument> {
        self.documents.get(id)
    }

    /// Get document mutably
    pub fn get_mut(&mut self, id: &str) -> Option<&mut CRDTDocument> {
        self.documents.get_mut(id)
    }

    /// Remove document
    pub fn remove(&mut self, id: &str) -> Option<CRDTDocument> {
        self.documents.remove(id)
    }

    /// List all document IDs
    pub fn list_ids(&self) -> Vec<&str> {
        self.documents.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for CRDTStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Offline state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineState {
    pub is_online: bool,
    pub pending_updates: usize,
    pub last_sync: Option<i64>,
}

impl Default for OfflineState {
    fn default() -> Self {
        Self {
            is_online: true,
            pending_updates: 0,
            last_sync: None,
        }
    }
}

/// IndexedDB persistence (WASM only)
#[cfg(target_arch = "wasm32")]
pub mod persistence {
    use super::*;
    use indexed_db_futures::{
        IdbDatabase, IdbObjectStore, IdbQuerySource, IdbTransactionMode, IdbVersionChangeEvent,
    };
    use wasm_bindgen::JsValue;

    const DB_NAME: &str = "dx-offline";
    const STORE_NAME: &str = "documents";

    /// Initialize IndexedDB
    pub async fn init_db() -> Result<IdbDatabase, JsValue> {
        let mut db_req = IdbDatabase::open_u32(DB_NAME, 1)?;

        db_req.set_on_upgrade_needed(Some(|evt: &IdbVersionChangeEvent| {
            // Create object store if it doesn't exist
            if !evt.db().object_store_names().any(|n| n == STORE_NAME) {
                evt.db().create_object_store(STORE_NAME).ok();
            }
            Ok(())
        }));

        db_req.await
    }

    /// Save document to IndexedDB
    pub async fn save_document(
        db: &IdbDatabase,
        doc_id: &str,
        state: &[u8],
    ) -> Result<(), JsValue> {
        let txn = db.transaction_on_one_with_mode(STORE_NAME, IdbTransactionMode::Readwrite)?;
        let store = txn.object_store(STORE_NAME)?;

        let js_state = js_sys::Uint8Array::from(state);
        store.put_key_val(&JsValue::from_str(doc_id), &js_state.into())?;

        txn.await.into_result()?;
        Ok(())
    }

    /// Load document from IndexedDB
    pub async fn load_document(db: &IdbDatabase, doc_id: &str) -> Result<Option<Vec<u8>>, JsValue> {
        let txn = db.transaction_on_one(STORE_NAME)?;
        let store = txn.object_store(STORE_NAME)?;

        let value = store.get(&JsValue::from_str(doc_id))?.await?;

        if value.is_undefined() || value.is_null() {
            return Ok(None);
        }

        let array = js_sys::Uint8Array::from(value);
        Ok(Some(array.to_vec()))
    }

    /// Delete document from IndexedDB
    pub async fn delete_document(db: &IdbDatabase, doc_id: &str) -> Result<(), JsValue> {
        let txn = db.transaction_on_one_with_mode(STORE_NAME, IdbTransactionMode::Readwrite)?;
        let store = txn.object_store(STORE_NAME)?;

        store.delete(&JsValue::from_str(doc_id))?;

        txn.await.into_result()?;
        Ok(())
    }
}

/// Sync manager (handles online/offline sync)
pub struct SyncManager {
    store: CRDTStore,
    state: OfflineState,
}

impl SyncManager {
    /// Create new sync manager
    pub fn new() -> Self {
        Self {
            store: CRDTStore::new(),
            state: OfflineState::default(),
        }
    }

    /// Set online/offline status
    pub fn set_online(&mut self, online: bool) {
        self.state.is_online = online;
    }

    /// Check if online
    pub fn is_online(&self) -> bool {
        self.state.is_online
    }

    /// Get pending update count
    pub fn pending_count(&self) -> usize {
        self.state.pending_updates
    }

    /// Mark sync complete
    pub fn mark_synced(&mut self, timestamp: i64) {
        self.state.last_sync = Some(timestamp);
        self.state.pending_updates = 0;
    }

    /// Get store reference
    pub fn store(&self) -> &CRDTStore {
        &self.store
    }

    /// Get store mutably
    pub fn store_mut(&mut self) -> &mut CRDTStore {
        &mut self.store
    }
}

impl Default for SyncManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crdt_document() {
        let doc = CRDTDocument::new("test-doc");

        doc.insert(0, "Hello");
        assert_eq!(doc.get_text(), "Hello");

        doc.insert(5, " World");
        assert_eq!(doc.get_text(), "Hello World");

        doc.delete(5, 6);
        assert_eq!(doc.get_text(), "Hello");
    }

    #[test]
    fn test_crdt_sync() {
        let doc1 = CRDTDocument::new("doc1");
        let doc2 = CRDTDocument::new("doc2");

        // Doc1 inserts text
        doc1.insert(0, "Alice");

        // Get state from doc1
        let state = doc1.get_full_state();

        // Apply to doc2
        doc2.apply_update(&state).unwrap();

        // Doc2 should have same text
        assert_eq!(doc2.get_text(), "Alice");

        // Doc2 appends text
        doc2.insert(5, " and Bob");

        // Get update from doc2
        let sv = doc1.get_state_vector();
        let update = doc2.encode_state_as_update(&sv);

        // Apply to doc1
        doc1.apply_update(&update).unwrap();

        // Both should match
        assert_eq!(doc1.get_text(), "Alice and Bob");
        assert_eq!(doc2.get_text(), "Alice and Bob");
    }

    #[test]
    fn test_crdt_store() {
        let mut store = CRDTStore::new();

        let doc = store.get_or_create("doc1");
        doc.insert(0, "Test");

        assert_eq!(store.get("doc1").unwrap().get_text(), "Test");
        assert_eq!(store.list_ids(), vec!["doc1"]);

        store.remove("doc1");
        assert!(store.get("doc1").is_none());
    }

    #[test]
    fn test_sync_manager() {
        let mut manager = SyncManager::new();

        assert!(manager.is_online());

        manager.set_online(false);
        assert!(!manager.is_online());

        manager.mark_synced(12345);
        assert_eq!(manager.state.last_sync, Some(12345));
    }
}
