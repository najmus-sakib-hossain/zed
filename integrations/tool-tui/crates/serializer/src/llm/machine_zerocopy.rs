//! Zero-copy machine format implementation
//!
//! Optimized binary format with minimal deserialization overhead

use crate::llm::types::{DxDocument, DxLlmValue, DxSection};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ZeroCopyError {
    #[error("Invalid magic number")]
    InvalidMagic,
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u8),
    #[error("Invalid data: {0}")]
    InvalidData(String),
    #[error("Out of bounds access")]
    OutOfBounds,
}

const MAGIC: &[u8; 4] = b"DXZC";
const VERSION: u8 = 1;

/// Zero-copy machine format
pub struct ZeroCopyMachine {
    data: Vec<u8>,
}

impl ZeroCopyMachine {
    pub fn from_document(doc: &DxDocument) -> Self {
        let mut data = Vec::new();

        // Header
        data.extend_from_slice(MAGIC);
        data.push(VERSION);

        // String table
        let mut string_table: HashMap<&str, u32> = HashMap::new();
        let mut string_data = Vec::new();

        // Collect all strings
        for key in doc.context.keys() {
            if !string_table.contains_key(key.as_str()) {
                let id = string_table.len() as u32;
                string_table.insert(key, id);
                string_data.push(key.as_bytes());
            }
        }
        for value in doc.context.values() {
            collect_strings_from_value(value, &mut string_table, &mut string_data);
        }
        for section in doc.sections.values() {
            for col in &section.schema {
                if !string_table.contains_key(col.as_str()) {
                    let id = string_table.len() as u32;
                    string_table.insert(col, id);
                    string_data.push(col.as_bytes());
                }
            }
            for row in &section.rows {
                for value in row {
                    collect_strings_from_value(value, &mut string_table, &mut string_data);
                }
            }
        }

        // Write string table
        data.extend_from_slice(&(string_table.len() as u32).to_le_bytes());
        for s in &string_data {
            data.extend_from_slice(&(s.len() as u32).to_le_bytes());
            data.extend_from_slice(s);
        }

        // Write context
        data.extend_from_slice(&(doc.context.len() as u32).to_le_bytes());
        for (key, value) in &doc.context {
            let key_id = string_table[key.as_str()];
            data.extend_from_slice(&key_id.to_le_bytes());
            write_value(&mut data, value, &string_table);
        }

        // Write sections
        data.extend_from_slice(&(doc.sections.len() as u32).to_le_bytes());
        for (id, section) in &doc.sections {
            data.push(*id as u8);
            data.extend_from_slice(&(section.schema.len() as u32).to_le_bytes());
            for col in &section.schema {
                let col_id = string_table[col.as_str()];
                data.extend_from_slice(&col_id.to_le_bytes());
            }
            data.extend_from_slice(&(section.rows.len() as u32).to_le_bytes());
            for row in &section.rows {
                for value in row {
                    write_value(&mut data, value, &string_table);
                }
            }
        }

        Self { data }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn to_document(&self) -> Result<DxDocument, ZeroCopyError> {
        let mut pos = 0;
        let data = &self.data;

        // Check magic
        if data.len() < 5 || &data[0..4] != MAGIC {
            return Err(ZeroCopyError::InvalidMagic);
        }
        pos += 4;

        let version = data[pos];
        if version != VERSION {
            return Err(ZeroCopyError::UnsupportedVersion(version));
        }
        pos += 1;

        // Read string table
        let string_count = read_u32(data, &mut pos)?;
        let mut strings = Vec::with_capacity(string_count as usize);
        for _ in 0..string_count {
            let len = read_u32(data, &mut pos)? as usize;
            if pos + len > data.len() {
                return Err(ZeroCopyError::OutOfBounds);
            }
            let s = std::str::from_utf8(&data[pos..pos + len])
                .map_err(|_| ZeroCopyError::InvalidData("Invalid UTF-8".to_string()))?
                .to_string();
            strings.push(s);
            pos += len;
        }

        let mut doc = DxDocument::new();

        // Read context
        let context_count = read_u32(data, &mut pos)?;
        for _ in 0..context_count {
            let key_id = read_u32(data, &mut pos)? as usize;
            if key_id >= strings.len() {
                return Err(ZeroCopyError::OutOfBounds);
            }
            let key = strings[key_id].clone();
            let value = read_value(data, &mut pos, &strings)?;
            doc.context.insert(key, value);
        }

        // Read sections
        let sections_count = read_u32(data, &mut pos)?;
        for _ in 0..sections_count {
            if pos >= data.len() {
                return Err(ZeroCopyError::OutOfBounds);
            }
            let id = data[pos] as char;
            pos += 1;

            let schema_count = read_u32(data, &mut pos)? as usize;
            let mut schema = Vec::with_capacity(schema_count);
            for _ in 0..schema_count {
                let col_id = read_u32(data, &mut pos)? as usize;
                if col_id >= strings.len() {
                    return Err(ZeroCopyError::OutOfBounds);
                }
                schema.push(strings[col_id].clone());
            }

            let mut section = DxSection::new(schema.clone());
            let rows_count = read_u32(data, &mut pos)?;
            for _ in 0..rows_count {
                let mut row = Vec::with_capacity(schema.len());
                for _ in 0..schema.len() {
                    row.push(read_value(data, &mut pos, &strings)?);
                }
                section.rows.push(row);
            }

            doc.sections.insert(id, section);
        }

        Ok(doc)
    }

    pub fn access(&self) -> Result<ZeroCopyDocument<'_>, ZeroCopyError> {
        ZeroCopyDocument::new(&self.data)
    }
}

pub struct ZeroCopyDocument<'a> {
    _data: &'a [u8],
}

impl<'a> ZeroCopyDocument<'a> {
    fn new(data: &'a [u8]) -> Result<Self, ZeroCopyError> {
        if data.len() < 5 || &data[0..4] != MAGIC {
            return Err(ZeroCopyError::InvalidMagic);
        }
        if data[4] != VERSION {
            return Err(ZeroCopyError::UnsupportedVersion(data[4]));
        }
        Ok(Self { _data: data })
    }

    pub fn to_document(&self) -> Result<DxDocument, ZeroCopyError> {
        ZeroCopyMachine {
            data: self._data.to_vec(),
        }
        .to_document()
    }
}

fn collect_strings_from_value<'a>(
    value: &'a DxLlmValue,
    table: &mut HashMap<&'a str, u32>,
    data: &mut Vec<&'a [u8]>,
) {
    match value {
        DxLlmValue::Str(s) => {
            if !table.contains_key(s.as_str()) {
                let id = table.len() as u32;
                table.insert(s, id);
                data.push(s.as_bytes());
            }
        }
        DxLlmValue::Ref(s) => {
            if !table.contains_key(s.as_str()) {
                let id = table.len() as u32;
                table.insert(s, id);
                data.push(s.as_bytes());
            }
        }
        DxLlmValue::Arr(items) => {
            for item in items {
                collect_strings_from_value(item, table, data);
            }
        }
        DxLlmValue::Obj(fields) => {
            for (k, v) in fields {
                if !table.contains_key(k.as_str()) {
                    let id = table.len() as u32;
                    table.insert(k, id);
                    data.push(k.as_bytes());
                }
                collect_strings_from_value(v, table, data);
            }
        }
        _ => {}
    }
}

fn write_value(data: &mut Vec<u8>, value: &DxLlmValue, string_table: &HashMap<&str, u32>) {
    match value {
        DxLlmValue::Str(s) => {
            data.push(0);
            let id = string_table[s.as_str()];
            data.extend_from_slice(&id.to_le_bytes());
        }
        DxLlmValue::Num(n) => {
            data.push(1);
            data.extend_from_slice(&n.to_le_bytes());
        }
        DxLlmValue::Bool(b) => {
            data.push(2);
            data.push(if *b { 1 } else { 0 });
        }
        DxLlmValue::Null => {
            data.push(3);
        }
        _ => {
            data.push(3); // Treat complex types as null for now
        }
    }
}

fn read_u32(data: &[u8], pos: &mut usize) -> Result<u32, ZeroCopyError> {
    if *pos + 4 > data.len() {
        return Err(ZeroCopyError::OutOfBounds);
    }
    let bytes = [data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]];
    *pos += 4;
    Ok(u32::from_le_bytes(bytes))
}

fn read_value(
    data: &[u8],
    pos: &mut usize,
    strings: &[String],
) -> Result<DxLlmValue, ZeroCopyError> {
    if *pos >= data.len() {
        return Err(ZeroCopyError::OutOfBounds);
    }

    let type_tag = data[*pos];
    *pos += 1;

    match type_tag {
        0 => {
            let id = read_u32(data, pos)? as usize;
            if id >= strings.len() {
                return Err(ZeroCopyError::OutOfBounds);
            }
            Ok(DxLlmValue::Str(strings[id].clone()))
        }
        1 => {
            if *pos + 8 > data.len() {
                return Err(ZeroCopyError::OutOfBounds);
            }
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&data[*pos..*pos + 8]);
            *pos += 8;
            Ok(DxLlmValue::Num(f64::from_le_bytes(bytes)))
        }
        2 => {
            if *pos >= data.len() {
                return Err(ZeroCopyError::OutOfBounds);
            }
            let b = data[*pos] != 0;
            *pos += 1;
            Ok(DxLlmValue::Bool(b))
        }
        3 => Ok(DxLlmValue::Null),
        _ => Err(ZeroCopyError::InvalidData(format!("Unknown type tag: {}", type_tag))),
    }
}
