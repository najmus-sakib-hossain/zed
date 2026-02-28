//! Format conversion functions
//!
//! Provides conversion between DX Serializer (LLM), Human, and Machine formats.
//! All conversions go through the common DxDocument representation.

use crate::llm::human_formatter::{HumanFormatConfig, HumanFormatter};
use crate::llm::human_parser::{HumanParseError, HumanParser};
use crate::llm::parser::{LlmParser, ParseError};
use crate::llm::serializer::LlmSerializer;
use crate::llm::types::DxDocument;
use thiserror::Error;

/// Conversion errors
#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("DX Serializer parse error: {0}")]
    LlmParse(#[from] ParseError),

    #[error("Human parse error: {0}")]
    HumanParse(#[from] HumanParseError),

    #[error("Machine format error: {msg}")]
    MachineFormat { msg: String },
}

/// Convert DX Serializer format string to Human format string
#[must_use = "conversion result should be used"]
pub fn llm_to_human(llm_input: &str) -> Result<String, ConvertError> {
    let doc = LlmParser::parse(llm_input)?;
    let formatter = HumanFormatter::new();
    Ok(formatter.format(&doc))
}

/// Convert DX Serializer format string to Human format string with custom config
pub fn llm_to_human_with_config(
    llm_input: &str,
    config: HumanFormatConfig,
) -> Result<String, ConvertError> {
    let doc = LlmParser::parse(llm_input)?;
    let formatter = HumanFormatter::with_config(config);
    Ok(formatter.format(&doc))
}

/// Convert Human format string to DX Serializer format string
#[must_use = "conversion result should be used"]
pub fn human_to_llm(human_input: &str) -> Result<String, ConvertError> {
    let trimmed = human_input.trim();

    // Check if input is already DX Serializer format
    if is_dsr_format(trimmed) {
        return Ok(human_input.to_string());
    }

    // Parse as Human format and convert to DX Serializer
    let parser = HumanParser::new();
    let doc = parser.parse(human_input)?;
    let serializer = LlmSerializer::new();
    Ok(serializer.serialize(&doc))
}

/// Check if input is in DX Serializer format
#[must_use]
pub fn is_dsr_format(input: &str) -> bool {
    let trimmed = input.trim();

    // DX Serializer format indicators:
    // - name[key=value,...] (objects) - NOT [name] which is TOML section
    // - name:count(schema)[data] (tables)
    // - name:count=items (arrays)
    // - key=value (simple pairs, NO spaces around =)

    // Human format indicators (should return false):
    // - [section] (TOML section headers)
    // - key = value (spaces around =)
    // - key[count]: followed by - items (list format)

    let mut has_dsr_indicators = false;
    let mut has_human_indicators = false;

    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // TOML section headers start with [ - this is HUMAN format
        if line.starts_with('[') {
            has_human_indicators = true;
            continue;
        }

        // List items starting with - are HUMAN format
        if line.starts_with('-') {
            has_human_indicators = true;
            continue;
        }

        // Check for spaces around = (HUMAN format: "key = value")
        if line.contains(" = ") {
            has_human_indicators = true;
            continue;
        }

        // Check for table syntax: name:count(schema)[
        if line.contains(':') && line.contains('(') && line.contains('[') {
            has_dsr_indicators = true;
            continue;
        }

        // Check for array syntax: name:count=items (DSR format)
        if line.contains(':') && line.contains('=') {
            let colon_pos = line.find(':');
            let eq_pos = line.find('=');
            if let (Some(cp), Some(ep)) = (colon_pos, eq_pos) {
                if cp < ep {
                    has_dsr_indicators = true;
                    continue;
                }
            }
        }

        // Check for compact key=value (NO spaces around =) - DSR format
        if line.contains('=') && !line.contains(" = ") {
            if let Some(eq_pos) = line.find('=') {
                let before = &line[..eq_pos];
                let after = &line[eq_pos + 1..];
                // DSR has no trailing space before = and no leading space after =
                if !before.ends_with(' ') && !after.starts_with(' ') {
                    has_dsr_indicators = true;
                    continue;
                }
            }
        }
    }

    // If we found human format indicators, it's NOT DSR format
    if has_human_indicators {
        return false;
    }

    // Only return true if we found DSR indicators
    has_dsr_indicators
}

/// Check if input is in LLM format (alias for is_dsr_format)
#[must_use]
pub fn is_llm_format(input: &str) -> bool {
    is_dsr_format(input)
}

/// Convert DX Serializer format string to DxDocument
#[must_use = "parsing result should be used"]
pub fn llm_to_document(llm_input: &str) -> Result<DxDocument, ConvertError> {
    Ok(LlmParser::parse(llm_input)?)
}

/// Convert Human format string to DxDocument
#[must_use = "parsing result should be used"]
pub fn human_to_document(human_input: &str) -> Result<DxDocument, ConvertError> {
    let parser = HumanParser::new();
    Ok(parser.parse(human_input)?)
}

/// Convert DxDocument to DX Serializer format string
#[must_use]
pub fn document_to_llm(doc: &DxDocument) -> String {
    let serializer = LlmSerializer::new();
    serializer.serialize(doc)
}

/// Convert DxDocument to Human format string
#[must_use]
pub fn document_to_human(doc: &DxDocument) -> String {
    let formatter = HumanFormatter::new();
    formatter.format(doc)
}

/// Convert DxDocument to Human format string with custom config
pub fn document_to_human_with_config(doc: &DxDocument, config: HumanFormatConfig) -> String {
    let formatter = HumanFormatter::with_config(config);
    formatter.format(doc)
}

/// Compression algorithm for machine format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionAlgorithm {
    /// LZ4 compression (fastest, default)
    #[default]
    Lz4,
    /// Zstd compression (better compression ratio)
    Zstd,
    /// No compression
    None,
}

/// Machine format representation (binary)
///
/// Includes automatic decompression caching for optimal performance.
#[derive(Debug, Clone)]
pub struct MachineFormat {
    pub data: Vec<u8>,
    /// Cached decompressed data (lazy) - first access decompresses, subsequent accesses use cache
    #[cfg(feature = "compression")]
    cached: std::cell::RefCell<Option<Vec<u8>>>,
}

impl MachineFormat {
    /// Create a new MachineFormat from raw data
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            #[cfg(feature = "compression")]
            cached: std::cell::RefCell::new(None),
        }
    }

    /// Get the raw data
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

/// Convert DX Serializer format to Machine format (RKYV + compression)
pub fn llm_to_machine(llm_input: &str) -> Result<MachineFormat, ConvertError> {
    let doc = LlmParser::parse(llm_input)?;
    Ok(document_to_machine(&doc))
}

/// Convert DX Serializer format to Machine format with specific compression
pub fn llm_to_machine_with_compression(
    llm_input: &str,
    compression: CompressionAlgorithm,
) -> Result<MachineFormat, ConvertError> {
    let doc = LlmParser::parse(llm_input)?;
    Ok(document_to_machine_with_compression(&doc, compression))
}

/// Convert Human format to Machine format (RKYV + compression)
pub fn human_to_machine(human_input: &str) -> Result<MachineFormat, ConvertError> {
    let parser = HumanParser::new();
    let doc = parser.parse(human_input)?;
    Ok(document_to_machine(&doc))
}

/// Convert Human format to Machine format without compression (raw RKYV)
pub fn human_to_machine_uncompressed(human_input: &str) -> Result<MachineFormat, ConvertError> {
    let parser = HumanParser::new();
    let doc = parser.parse(human_input)?;
    Ok(document_to_machine_with_compression(&doc, CompressionAlgorithm::None))
}

/// Convert Human format to Machine format with specific compression
pub fn human_to_machine_with_compression(
    human_input: &str,
    compression: CompressionAlgorithm,
) -> Result<MachineFormat, ConvertError> {
    let parser = HumanParser::new();
    let doc = parser.parse(human_input)?;
    Ok(document_to_machine_with_compression(&doc, compression))
}

/// Convert DxDocument to Machine format (RKYV + LZ4 by default)
pub fn document_to_machine(doc: &DxDocument) -> MachineFormat {
    document_to_machine_with_compression(doc, CompressionAlgorithm::default())
}

/// Convert DxDocument to Machine format with specific compression
pub fn document_to_machine_with_compression(
    doc: &DxDocument,
    compression: CompressionAlgorithm,
) -> MachineFormat {
    use crate::machine::machine_types::MachineDocument;
    use crate::machine::serialize;

    let machine_doc = MachineDocument::from(doc);
    let rkyv_data = serialize(&machine_doc)
        .map_err(|e| panic!("RKYV serialization failed: {}", e))
        .unwrap()
        .into_vec();

    match compression {
        CompressionAlgorithm::None => MachineFormat::new(rkyv_data),

        #[cfg(feature = "compression-lz4")]
        CompressionAlgorithm::Lz4 => {
            use crate::machine::compress::compress_lz4;
            match compress_lz4(&rkyv_data) {
                Ok(compressed) => {
                    let savings_ratio = 1.0 - (compressed.len() as f64 / rkyv_data.len() as f64);
                    if savings_ratio > 0.10 {
                        MachineFormat::new(compressed)
                    } else {
                        MachineFormat::new(rkyv_data)
                    }
                }
                Err(_) => MachineFormat::new(rkyv_data),
            }
        }

        #[cfg(feature = "compression-zstd")]
        CompressionAlgorithm::Zstd => {
            use crate::machine::compress::{CompressionLevel, compress_zstd_level};
            match compress_zstd_level(&rkyv_data, CompressionLevel::Fast) {
                Ok(compressed) => {
                    let savings_ratio = 1.0 - (compressed.len() as f64 / rkyv_data.len() as f64);
                    if savings_ratio > 0.10 {
                        MachineFormat::new(compressed)
                    } else {
                        MachineFormat::new(rkyv_data)
                    }
                }
                Err(_) => MachineFormat::new(rkyv_data),
            }
        }

        #[cfg(not(feature = "compression-lz4"))]
        CompressionAlgorithm::Lz4 => MachineFormat::new(rkyv_data),

        #[cfg(not(feature = "compression-zstd"))]
        CompressionAlgorithm::Zstd => MachineFormat::new(rkyv_data),
    }
}

/// Convert Machine format to DxDocument (auto-detects compression)
pub fn machine_to_document(machine: &MachineFormat) -> Result<DxDocument, ConvertError> {
    use crate::machine::machine_types::MachineDocument;

    #[cfg(feature = "compression")]
    let doc_data = {
        // Check cache first
        if let Some(cached) = machine.cached.borrow().as_ref() {
            cached.clone()
        } else {
            // Try decompression (auto-detect format)
            let decompressed = decompress_auto(&machine.data)?;

            // Cache for next time
            *machine.cached.borrow_mut() = Some(decompressed.clone());
            decompressed
        }
    };

    #[cfg(not(feature = "compression"))]
    let doc_data = machine.data.clone();

    // Deserialize using rkyv::from_bytes
    let machine_doc: MachineDocument =
        rkyv::from_bytes(&doc_data).map_err(|e: rkyv::rancor::Error| {
            ConvertError::MachineFormat {
                msg: format!("RKYV deserialize failed: {}", e),
            }
        })?;

    Ok(DxDocument::from(&machine_doc))
}

/// Auto-detect and decompress data (tries LZ4, then Zstd, then raw)
#[cfg(feature = "compression")]
fn decompress_auto(data: &[u8]) -> Result<Vec<u8>, ConvertError> {
    // Try LZ4 first (most common, fastest)
    #[cfg(feature = "compression-lz4")]
    {
        use crate::machine::compress::decompress_lz4;
        if let Ok(decompressed) = decompress_lz4(data) {
            return Ok(decompressed);
        }
    }

    // Try Zstd
    #[cfg(feature = "compression-zstd")]
    {
        use crate::machine::compress::decompress_zstd;
        if let Ok(decompressed) = decompress_zstd(data) {
            return Ok(decompressed);
        }
    }

    // Not compressed, return as-is
    Ok(data.to_vec())
}

/// Convert Machine format to DX Serializer format string
pub fn machine_to_llm(machine: &MachineFormat) -> Result<String, ConvertError> {
    let doc = machine_to_document(machine)?;
    Ok(document_to_llm(&doc))
}

/// Convert Machine format to Human format string
pub fn machine_to_human(machine: &MachineFormat) -> Result<String, ConvertError> {
    let doc = machine_to_document(machine)?;
    Ok(document_to_human(&doc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::DxLlmValue;

    #[test]
    fn test_llm_to_human() {
        let llm = "name=Test\ncount=42";
        let human = llm_to_human(llm).unwrap();
        assert!(human.contains("name") || human.contains("Test"));
    }

    #[test]
    fn test_human_to_llm() {
        let human = r#"
[config]
    name = "Test"
    count = 42
"#;
        let llm = human_to_llm(human).unwrap();
        // DX Serializer format uses : or :: for key-value pairs
        assert!(llm.contains(":") || llm.contains("Test"));
    }

    #[test]
    fn test_machine_format_round_trip() {
        let mut doc = DxDocument::new();
        doc.context.insert("name".to_string(), DxLlmValue::Str("Test".to_string()));
        doc.context.insert("count".to_string(), DxLlmValue::Num(42.0));
        doc.context.insert("active".to_string(), DxLlmValue::Bool(true));

        let machine = document_to_machine(&doc);
        let round_trip_doc = machine_to_document(&machine).unwrap();

        assert_eq!(doc.context.len(), round_trip_doc.context.len());
        assert_eq!(round_trip_doc.context.get("name").unwrap().as_str(), Some("Test"));
        assert_eq!(round_trip_doc.context.get("count").unwrap().as_num(), Some(42.0));
    }

    #[test]
    fn test_is_dsr_format() {
        // DX Serializer format
        assert!(is_dsr_format("name=Test"));
        assert!(is_dsr_format("config[host=localhost,port=8080]"));
        assert!(is_dsr_format("friends:3=ana,luis,sam"));
        assert!(is_dsr_format("table:2(id,name)[1,John\n2,Jane]"));

        // Not DX Serializer format (Human/TOML-like)
        assert!(!is_dsr_format("[config]\nname = Test"));
    }
}
