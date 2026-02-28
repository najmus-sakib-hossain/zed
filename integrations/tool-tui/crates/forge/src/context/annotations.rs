use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

use crate::storage::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: Uuid,
    pub file_path: String,
    pub anchor_id: Option<Uuid>,
    pub line: usize,
    pub content: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub is_ai: bool,
}

impl Annotation {
    pub fn new(file_path: String, line: usize, content: String, is_ai: bool) -> Self {
        let author = if is_ai {
            "AI Agent".to_string()
        } else {
            whoami::username()
        };

        Self {
            id: Uuid::new_v4(),
            file_path,
            anchor_id: None,
            line,
            content,
            author,
            created_at: Utc::now(),
            is_ai,
        }
    }
}

pub fn store_annotation(db: &Database, annotation: &Annotation) -> Result<()> {
    let conn = db.conn.lock();

    conn.execute(
        "INSERT INTO annotations (id, file_path, anchor_id, line, content, author, created_at, is_ai)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            annotation.id.to_string(),
            annotation.file_path,
            annotation.anchor_id.map(|id| id.to_string()),
            annotation.line as i64,
            annotation.content,
            annotation.author,
            annotation.created_at.to_rfc3339(),
            annotation.is_ai,
        ],
    )?;

    Ok(())
}

pub fn get_annotations(db: &Database, file: &Path, line: Option<usize>) -> Result<Vec<Annotation>> {
    let conn = db.conn.lock();

    let query = if let Some(l) = line {
        format!(
            "SELECT id, file_path, anchor_id, line, content, author, created_at, is_ai
             FROM annotations
             WHERE file_path = '{}' AND line = {}
             ORDER BY created_at DESC",
            file.display(),
            l
        )
    } else {
        format!(
            "SELECT id, file_path, anchor_id, line, content, author, created_at, is_ai
             FROM annotations
             WHERE file_path = '{}'
             ORDER BY created_at DESC",
            file.display()
        )
    };

    let mut stmt = conn.prepare(&query)?;
    let annotations = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let file_path: String = row.get(1)?;
        let anchor_id: Option<String> = row.get(2)?;
        let line: i64 = row.get(3)?;
        let content: String = row.get(4)?;
        let author: String = row.get(5)?;
        let created_at: String = row.get(6)?;
        let is_ai: bool = row.get(7)?;

        // Parse UUID - if invalid, return a database error
        let parsed_id = Uuid::parse_str(&id).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;

        // Parse datetime - if invalid, return a database error
        let parsed_created_at = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?
            .into();

        Ok(Annotation {
            id: parsed_id,
            file_path,
            anchor_id: anchor_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
            line: line as usize,
            content,
            author,
            created_at: parsed_created_at,
            is_ai,
        })
    })?;

    Ok(annotations.collect::<Result<Vec<_>, _>>()?)
}
