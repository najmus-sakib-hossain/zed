use anyhow::{Context, Result};
use automerge::{AutoCommit, ROOT, ReadDoc, transaction::Transactable};
use parking_lot::RwLock;
use ropey::Rope;
use std::path::PathBuf;
use std::sync::Arc;

use super::operations::{Operation, OperationType, Position};

#[allow(dead_code)]
pub struct CrdtDocument {
    pub path: PathBuf,
    /// Automerge document for CRDT operations
    pub doc: Arc<RwLock<AutoCommit>>,
    /// Rope for efficient text editing
    pub rope: Arc<RwLock<Rope>>,
    /// Lamport timestamp for ordering
    pub lamport: Arc<parking_lot::Mutex<u64>>,
}

#[allow(dead_code)]
impl CrdtDocument {
    pub fn new(path: PathBuf, initial_content: &str) -> Result<Self> {
        let mut doc = AutoCommit::new();
        doc.put(ROOT, "content", initial_content)
            .context("Failed to initialize CRDT document with initial content")?;

        Ok(Self {
            path,
            doc: Arc::new(RwLock::new(doc)),
            rope: Arc::new(RwLock::new(Rope::from_str(initial_content))),
            lamport: Arc::new(parking_lot::Mutex::new(0)),
        })
    }

    pub fn apply_operation(&self, op: &Operation) -> Result<()> {
        let mut lamport = self.lamport.lock();
        *lamport += 1;

        match &op.op_type {
            OperationType::Insert {
                position, content, ..
            } => {
                let mut rope = self.rope.write();
                let char_idx = self.line_col_to_char(&rope, position.line, position.column);
                rope.insert(char_idx, content);

                // Update CRDT
                let mut doc = self.doc.write();
                if let Some((value, _)) = doc.get(ROOT, "content")? {
                    let current: String = value.to_string();
                    let mut chars: Vec<char> = current.chars().collect();
                    chars.splice(char_idx..char_idx, content.chars());
                    doc.put(ROOT, "content", chars.iter().collect::<String>())?;
                }
            }

            OperationType::Delete { position, length } => {
                let mut rope = self.rope.write();
                let char_idx = self.line_col_to_char(&rope, position.line, position.column);
                rope.remove(char_idx..char_idx + length);

                // Update CRDT
                let mut doc = self.doc.write();
                if let Some((value, _)) = doc.get(ROOT, "content")? {
                    let current: String = value.to_string();
                    let mut chars: Vec<char> = current.chars().collect();
                    chars.drain(char_idx..char_idx + length);
                    doc.put(ROOT, "content", chars.iter().collect::<String>())?;
                }
            }

            OperationType::Replace {
                position,
                old_content,
                new_content,
            } => {
                let mut rope = self.rope.write();
                let char_idx = self.line_col_to_char(&rope, position.line, position.column);
                rope.remove(char_idx..char_idx + old_content.len());
                rope.insert(char_idx, new_content);

                // Update CRDT
                let mut doc = self.doc.write();
                if let Some((value, _)) = doc.get(ROOT, "content")? {
                    let current: String = value.to_string();
                    let mut chars: Vec<char> = current.chars().collect();
                    chars.splice(char_idx..char_idx + old_content.len(), new_content.chars());
                    doc.put(ROOT, "content", chars.iter().collect::<String>())?;
                }
            }

            _ => {}
        }

        Ok(())
    }

    pub fn get_content(&self) -> String {
        self.rope.read().to_string()
    }

    pub fn create_position(&self, line: usize, column: usize, actor_id: String) -> Position {
        let rope = self.rope.read();
        let offset = self.line_col_to_char(&rope, line, column);
        let lamport = *self.lamport.lock();

        Position::new(line, column, offset, actor_id, lamport)
    }

    fn line_col_to_char(&self, rope: &Rope, line: usize, col: usize) -> usize {
        rope.line_to_char(line.saturating_sub(1)) + col.saturating_sub(1)
    }

    /// Get the current position from a stable anchor
    pub fn resolve_anchor(&self, _stable_id: &str) -> Option<(usize, usize)> {
        // This would use the CRDT's operation history to resolve
        // the current position of an anchor
        // For now, simplified implementation
        None
    }
}
