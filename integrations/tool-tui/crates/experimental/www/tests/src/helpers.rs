//! Test helper functions for integration tests

use anyhow::Result;
use std::path::Path;

/// DXB magic bytes (DX = DX Binary)
pub const DXB_MAGIC: &[u8] = b"DX";

/// HTIP magic bytes - stored as u16 0x4458 in little-endian = [0x58, 0x44] = "XD"
pub const HTIP_MAGIC_LE: &[u8] = &[0x58, 0x44];

/// HTIP magic bytes (DXB1 = DX Binary v1) - used in binary crate
pub const HTIP_MAGIC: &[u8] = b"DXB1";

/// Validates that a byte slice is a valid HTIP stream (core codegen format)
/// This is the raw HTIP stream format used by core/src/codegen.rs
pub fn validate_htip_stream(data: &[u8]) -> Result<HtipStreamValidation> {
    if data.len() < 16 {
        anyhow::bail!("HTIP stream too short: {} bytes", data.len());
    }

    // Check magic bytes (u16 0x4458 in little-endian = [0x58, 0x44])
    if &data[0..2] != HTIP_MAGIC_LE {
        anyhow::bail!(
            "Invalid HTIP stream magic: expected {:?}, got {:?}",
            HTIP_MAGIC_LE,
            &data[0..2]
        );
    }

    // Check version
    let version = data[2];
    if version != 2 {
        anyhow::bail!("Unsupported HTIP stream version: {}", version);
    }

    // Parse header fields
    let flags = data[3];
    let template_count = u16::from_le_bytes([data[4], data[5]]);
    let string_count = u16::from_le_bytes([data[6], data[7]]);
    let opcode_count = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    let payload_size = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);

    Ok(HtipStreamValidation {
        version,
        flags,
        template_count,
        string_count,
        opcode_count,
        payload_size,
        total_size: data.len(),
    })
}

/// Result of HTIP stream validation
#[derive(Debug, Clone)]
pub struct HtipStreamValidation {
    pub version: u8,
    pub flags: u8,
    pub template_count: u16,
    pub string_count: u16,
    pub opcode_count: u32,
    pub payload_size: u32,
    pub total_size: usize,
}

/// Validates that a byte slice is a valid DXB binary (packer format)
pub fn validate_dxb_binary(data: &[u8]) -> Result<DxbValidation> {
    if data.len() < 8 {
        anyhow::bail!("DXB binary too short: {} bytes", data.len());
    }

    // Check magic bytes (2 bytes: "DX")
    if &data[0..2] != DXB_MAGIC {
        anyhow::bail!("Invalid DXB magic: expected {:?}, got {:?}", DXB_MAGIC, &data[0..2]);
    }

    // Check version
    let version = data[2];
    if version != 1 {
        anyhow::bail!("Unsupported DXB version: {}", version);
    }

    // Check mode flag (byte 3)
    let mode = data[3];
    let is_htip_mode = mode == 0x01;

    // Read payload size (4 bytes, little endian)
    let payload_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;

    Ok(DxbValidation {
        version,
        is_htip_mode,
        payload_size,
        total_size: data.len(),
    })
}

/// Result of DXB validation
#[derive(Debug, Clone)]
pub struct DxbValidation {
    pub version: u8,
    pub is_htip_mode: bool,
    pub payload_size: usize,
    pub total_size: usize,
}

/// Validates that a byte slice is a valid HTIP binary (binary crate format)
pub fn validate_htip_binary(data: &[u8]) -> Result<HtipValidation> {
    if data.len() < 8 {
        anyhow::bail!("HTIP binary too short: {} bytes", data.len());
    }

    // Check magic bytes
    if &data[0..4] != HTIP_MAGIC {
        anyhow::bail!("Invalid HTIP magic: expected {:?}, got {:?}", HTIP_MAGIC, &data[0..4]);
    }

    // Check version
    let version = data[4];
    if version != 1 {
        anyhow::bail!("Unsupported HTIP version: {}", version);
    }

    // Parse flags
    let flags = data[5];
    let has_templates = (flags & 0x01) != 0;
    let has_instructions = (flags & 0x02) != 0;

    // Count templates and instructions
    let mut pos = 8;
    let mut template_count = 0;
    let mut instruction_count = 0;

    while pos < data.len() {
        let opcode = data[pos];
        pos += 1;

        match opcode {
            0x08 => {
                // OP_TEMPLATE_DEF
                if pos + 3 > data.len() {
                    anyhow::bail!("Truncated template definition at position {}", pos);
                }
                let _template_id = data[pos];
                let html_len = u16::from_le_bytes([data[pos + 1], data[pos + 2]]) as usize;
                pos += 3 + html_len;
                template_count += 1;
            }
            0x01 => {
                // OP_CLONE
                pos += 1;
                instruction_count += 1;
            }
            0x02 => {
                // OP_PATCH_TEXT
                if pos + 3 > data.len() {
                    anyhow::bail!("Truncated patch text at position {}", pos);
                }
                let _node = data[pos];
                let text_len = u16::from_le_bytes([data[pos + 1], data[pos + 2]]) as usize;
                pos += 3 + text_len;
                instruction_count += 1;
            }
            0x03 => {
                // OP_PATCH_ATTR
                if pos + 5 > data.len() {
                    anyhow::bail!("Truncated patch attr at position {}", pos);
                }
                let _node = data[pos];
                let key_len = u16::from_le_bytes([data[pos + 1], data[pos + 2]]) as usize;
                let val_len = u16::from_le_bytes([data[pos + 3], data[pos + 4]]) as usize;
                pos += 5 + key_len + val_len;
                instruction_count += 1;
            }
            0x04 => {
                // OP_CLASS_TOGGLE
                if pos + 4 > data.len() {
                    anyhow::bail!("Truncated class toggle at position {}", pos);
                }
                let _node = data[pos];
                let class_len = u16::from_le_bytes([data[pos + 1], data[pos + 2]]) as usize;
                let _enable = data[pos + 3];
                pos += 4 + class_len;
                instruction_count += 1;
            }
            0x05 => {
                // OP_REMOVE
                pos += 1;
                instruction_count += 1;
            }
            0x06 => {
                // OP_EVENT
                if pos + 4 > data.len() {
                    anyhow::bail!("Truncated event at position {}", pos);
                }
                let _node = data[pos];
                let _event_type = data[pos + 1];
                let _handler_id = u16::from_le_bytes([data[pos + 2], data[pos + 3]]);
                pos += 4;
                instruction_count += 1;
            }
            0xFF => {
                // OP_EOF
                break;
            }
            _ => {
                anyhow::bail!("Unknown opcode: 0x{:02X} at position {}", opcode, pos - 1);
            }
        }
    }

    Ok(HtipValidation {
        version,
        has_templates,
        has_instructions,
        template_count,
        instruction_count,
        total_size: data.len(),
    })
}

/// Result of HTIP validation
#[derive(Debug, Clone)]
pub struct HtipValidation {
    pub version: u8,
    pub has_templates: bool,
    pub has_instructions: bool,
    pub template_count: usize,
    pub instruction_count: usize,
    pub total_size: usize,
}

/// Normalizes HTML by removing whitespace between tags
pub fn normalize_html(html: &str) -> String {
    html.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Creates a temporary directory for test files
pub fn create_test_dir(name: &str) -> Result<tempfile::TempDir> {
    let dir = tempfile::Builder::new().prefix(name).tempdir()?;
    Ok(dir)
}

/// Writes a TSX file to the given directory
pub fn write_tsx_file(dir: &Path, name: &str, content: &str) -> Result<std::path::PathBuf> {
    let path = dir.join(name);
    std::fs::write(&path, content)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_htip_too_short() {
        let result = validate_htip_binary(&[0x44, 0x58, 0x42, 0x31]);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_htip_invalid_magic() {
        let result = validate_htip_binary(&[0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00]);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_htip_valid_minimal() {
        // Valid minimal HTIP: magic + version + flags + reserved + EOF
        let data = [
            0x44, 0x58, 0x42, 0x31, // DXB1 magic
            0x01, // version 1
            0x00, // flags
            0x00, 0x00, // reserved
            0xFF, // OP_EOF
        ];
        let result = validate_htip_binary(&data);
        assert!(result.is_ok());
        let validation = result.unwrap();
        assert_eq!(validation.version, 1);
        assert_eq!(validation.template_count, 0);
        assert_eq!(validation.instruction_count, 0);
    }

    #[test]
    fn test_validate_dxb_valid() {
        // Valid DXB: magic + version + mode + size + payload
        let data = [
            0x44, 0x58, // DX magic
            0x01, // version 1
            0x01, // mode (HTIP)
            0x04, 0x00, 0x00, 0x00, // size = 4
            0x00, 0x00, 0x00, 0x00, // payload
        ];
        let result = validate_dxb_binary(&data);
        assert!(result.is_ok());
        let validation = result.unwrap();
        assert_eq!(validation.version, 1);
        assert!(validation.is_htip_mode);
        assert_eq!(validation.payload_size, 4);
    }

    #[test]
    fn test_normalize_html() {
        let html = "<div>  Hello   World  </div>";
        assert_eq!(normalize_html(html), "<div> Hello World </div>");
    }
}
