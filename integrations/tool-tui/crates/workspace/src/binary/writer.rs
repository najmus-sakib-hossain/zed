//! Binary format writer.

use super::header::{BinaryHeader, HEADER_SIZE};
use super::string_table::StringTable;
use crate::{Error, Result, WorkspaceConfig};
use std::fs::File;
use std::io::{BufWriter, Seek, Write};
use std::path::Path;

/// Writer for binary workspace configuration.
pub struct BinaryWriter {
    /// String table for deduplication.
    string_table: StringTable,
}

impl BinaryWriter {
    /// Create a new binary writer.
    pub fn new() -> Self {
        Self {
            string_table: StringTable::new(),
        }
    }

    /// Write workspace configuration to a binary file.
    pub fn write(&self, config: &WorkspaceConfig, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let file = File::create(path).map_err(|e| Error::io(path, e))?;
        let mut writer = BufWriter::new(file);

        self.write_to(&mut writer, config)?;

        writer.flush().map_err(|e| Error::io(path, e))?;
        Ok(())
    }

    /// Write workspace configuration to a writer.
    pub fn write_to<W: Write + Seek>(
        &self,
        writer: &mut W,
        config: &WorkspaceConfig,
    ) -> Result<()> {
        // Serialize config to JSON bytes first (for hashing and size calculation)
        let config_json = serde_json::to_vec(config).map_err(|e| Error::Serialization {
            format: "json".into(),
            details: e.to_string(),
        })?;

        // Calculate content hash
        let content_hash = blake3::hash(&config_json);

        // Build string table from config
        let mut string_table = StringTable::new();
        Self::collect_strings(config, &mut string_table);

        // Serialize string table
        let string_table_bytes = string_table.to_bytes();

        // Calculate offsets
        let string_table_offset = HEADER_SIZE as u64;
        let config_data_offset = string_table_offset + string_table_bytes.len() as u64;
        let total_size = config_data_offset + config_json.len() as u64;

        // Build header
        let mut header = BinaryHeader::new();
        header.content_hash = *content_hash.as_bytes();
        header.string_table_offset = string_table_offset;
        header.config_data_offset = config_data_offset;
        header.total_size = total_size;
        header.flags.set_string_table(!string_table.is_empty());

        // Write header
        header.write_to(writer)?;

        // Write string table
        writer
            .write_all(&string_table_bytes)
            .map_err(|e| Error::io("string_table", e))?;

        // Write config data
        writer.write_all(&config_json).map_err(|e| Error::io("config_data", e))?;

        Ok(())
    }

    /// Write to bytes.
    pub fn write_to_bytes(&self, config: &WorkspaceConfig) -> Result<Vec<u8>> {
        let mut buffer = std::io::Cursor::new(Vec::new());
        self.write_to(&mut buffer, config)?;
        Ok(buffer.into_inner())
    }

    /// Collect strings from config for deduplication.
    fn collect_strings(config: &WorkspaceConfig, table: &mut StringTable) {
        // Add common strings
        table.add(&config.name);

        if !config.description.is_empty() {
            table.add(&config.description);
        }

        // Add editor settings strings
        if let Some(font) = &config.editor.font_family {
            table.add(font);
        }

        if let Some(theme) = &config.editor.theme {
            table.add(theme);
        }

        // Add task labels and commands
        for task in &config.tasks.tasks {
            table.add(&task.label);
            table.add(&task.command);
            for arg in &task.args {
                table.add(arg);
            }
        }

        // Add extension IDs - core extensions
        for ext in &config.extensions.core {
            table.add(&ext.id);
            table.add(&ext.name);
        }

        // Add extension IDs - recommended extensions
        for ext in &config.extensions.recommended {
            table.add(&ext.id);
            table.add(&ext.name);
        }

        // Add debug configuration strings
        for launch in &config.debug.launch_configs {
            table.add(&launch.name);
            if let Some(program) = &launch.program {
                table.add(program);
            }
            if let Some(cwd) = &launch.cwd {
                table.add(cwd);
            }
        }
    }
}

impl Default for BinaryWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_to_bytes() {
        let config = WorkspaceConfig::new("test-project");
        let writer = BinaryWriter::new();

        let bytes = writer.write_to_bytes(&config).unwrap();

        // Should have header + string table + config
        assert!(bytes.len() > HEADER_SIZE);

        // Check magic bytes
        assert_eq!(&bytes[0..4], b"DXWS");
    }

    #[test]
    fn test_string_collection() {
        let mut config = WorkspaceConfig::new("my-project");
        config.description = "A test project".into();
        config.editor.theme = Some("One Dark".into());

        let mut table = StringTable::new();
        BinaryWriter::collect_strings(&config, &mut table);

        // Should contain project name, description, theme
        assert!(table.len() >= 3);
    }
}
