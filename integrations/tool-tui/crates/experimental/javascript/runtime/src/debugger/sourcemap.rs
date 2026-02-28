//! Source map generation and parsing
//!
//! This module provides source map support for mapping generated code back to
//! original source locations. It supports:
//! - Standard source map v3 format
//! - VLQ-encoded mappings
//! - Inline source maps (data URLs)
//! - External .map files

use std::path::Path;

/// Source map for generated code
#[derive(Debug, Clone)]
pub struct SourceMap {
    pub version: u32,
    pub sources: Vec<String>,
    pub sources_content: Vec<Option<String>>,
    pub mappings: String,
    pub names: Vec<String>,
    /// Decoded mappings for fast lookup
    decoded: Vec<MappingSegment>,
}

/// A single mapping segment
#[derive(Debug, Clone)]
struct MappingSegment {
    /// Generated line (0-indexed)
    gen_line: u32,
    /// Generated column (0-indexed)
    gen_col: u32,
    /// Source file index
    source_idx: u32,
    /// Original line (0-indexed)
    orig_line: u32,
    /// Original column (0-indexed)
    orig_col: u32,
    /// Name index (optional)
    name_idx: Option<u32>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            version: 3,
            sources: Vec::new(),
            sources_content: Vec::new(),
            mappings: String::new(),
            names: Vec::new(),
            decoded: Vec::new(),
        }
    }
    
    /// Parse a source map from JSON string
    pub fn from_json(json: &str) -> Option<Self> {
        let parsed: serde_json::Value = serde_json::from_str(json).ok()?;
        
        let version = parsed.get("version")?.as_u64()? as u32;
        if version != 3 {
            return None;
        }
        
        let sources: Vec<String> = parsed.get("sources")?
            .as_array()?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
            
        let sources_content: Vec<Option<String>> = parsed.get("sourcesContent")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_else(|| vec![None; sources.len()]);
            
        let mappings = parsed.get("mappings")?.as_str()?.to_string();
        
        let names: Vec<String> = parsed.get("names")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        
        let mut source_map = Self {
            version,
            sources,
            sources_content,
            mappings,
            names,
            decoded: Vec::new(),
        };
        
        source_map.decode_mappings();
        Some(source_map)
    }
    
    /// Load source map from a file path
    pub fn from_file(path: impl AsRef<Path>) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        Self::from_json(&content)
    }
    
    /// Extract inline source map from source code
    /// Looks for //# sourceMappingURL=data:application/json;base64,...
    pub fn from_inline(source: &str) -> Option<Self> {
        let prefix = "//# sourceMappingURL=data:application/json;base64,";
        let pos = source.find(prefix)?;
        let start = pos + prefix.len();
        
        // Find end of base64 data (newline or end of string)
        let end = source[start..].find('\n')
            .map(|i| start + i)
            .unwrap_or(source.len());
        
        let base64_data = source[start..end].trim();
        let decoded = base64_decode(base64_data)?;
        let json = String::from_utf8(decoded).ok()?;
        
        Self::from_json(&json)
    }
    
    /// Decode VLQ mappings into segments
    fn decode_mappings(&mut self) {
        let mut segments = Vec::new();
        let mut gen_line = 0u32;
        let mut source_idx = 0u32;
        let mut orig_line = 0u32;
        let mut orig_col = 0u32;
        let mut name_idx = 0u32;
        
        for line in self.mappings.split(';') {
            let mut gen_col = 0u32;
            
            for segment_str in line.split(',') {
                if segment_str.is_empty() {
                    continue;
                }
                
                let values = decode_vlq(segment_str);
                if values.is_empty() {
                    continue;
                }
                
                gen_col = (gen_col as i32 + values[0]) as u32;
                
                let mut segment = MappingSegment {
                    gen_line,
                    gen_col,
                    source_idx: 0,
                    orig_line: 0,
                    orig_col: 0,
                    name_idx: None,
                };
                
                if values.len() >= 4 {
                    source_idx = (source_idx as i32 + values[1]) as u32;
                    orig_line = (orig_line as i32 + values[2]) as u32;
                    orig_col = (orig_col as i32 + values[3]) as u32;
                    
                    segment.source_idx = source_idx;
                    segment.orig_line = orig_line;
                    segment.orig_col = orig_col;
                    
                    if values.len() >= 5 {
                        name_idx = (name_idx as i32 + values[4]) as u32;
                        segment.name_idx = Some(name_idx);
                    }
                }
                
                segments.push(segment);
            }
            
            gen_line += 1;
        }
        
        self.decoded = segments;
    }

    pub fn add_source(&mut self, source: String) {
        if !self.sources.contains(&source) {
            self.sources.push(source);
            self.sources_content.push(None);
        }
    }

    pub fn add_mapping(
        &mut self,
        generated_line: usize,
        generated_column: usize,
        source_line: usize,
        source_column: usize,
    ) {
        let mapping =
            format!("{},{},{},{}", generated_line, generated_column, source_line, source_column);
        if !self.mappings.is_empty() {
            self.mappings.push(';');
        }
        self.mappings.push_str(&mapping);
    }

    pub fn to_json(&self) -> String {
        let sources_content_json: Vec<String> = self.sources_content
            .iter()
            .map(|opt| opt.as_ref().map(|s| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))).unwrap_or_else(|| "null".to_string()))
            .collect();
            
        format!(
            r#"{{"version":{},"sources":{:?},"sourcesContent":[{}],"mappings":"{}","names":{:?}}}"#,
            self.version, self.sources, sources_content_json.join(","), self.mappings, self.names
        )
    }

    /// Look up original location for a generated position
    pub fn lookup(&self, generated_line: usize, generated_column: usize) -> Option<SourceLocation> {
        let gen_line = generated_line as u32;
        let gen_col = generated_column as u32;
        
        // Find the best matching segment
        let mut best_match: Option<&MappingSegment> = None;
        
        for segment in &self.decoded {
            if segment.gen_line == gen_line {
                if segment.gen_col <= gen_col {
                    match best_match {
                        None => best_match = Some(segment),
                        Some(prev) if segment.gen_col > prev.gen_col => {
                            best_match = Some(segment);
                        }
                        _ => {}
                    }
                }
            } else if segment.gen_line > gen_line {
                break;
            }
        }
        
        best_match.map(|seg| SourceLocation {
            source: self.sources.get(seg.source_idx as usize).cloned().unwrap_or_default(),
            line: (seg.orig_line + 1) as usize, // Convert to 1-indexed
            column: (seg.orig_col + 1) as usize, // Convert to 1-indexed
            name: seg.name_idx.and_then(|idx| self.names.get(idx as usize).cloned()),
        })
    }
    
    /// Get source content for a source file
    pub fn get_source_content(&self, source_idx: usize) -> Option<&str> {
        self.sources_content.get(source_idx)?.as_deref()
    }
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Original source location from source map lookup
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub source: String,
    pub line: usize,
    pub column: usize,
    pub name: Option<String>,
}

/// Decode a VLQ-encoded segment
fn decode_vlq(segment: &str) -> Vec<i32> {
    const VLQ_BASE: i32 = 32;
    const VLQ_CONTINUATION_BIT: i32 = 32;
    
    let mut values = Vec::new();
    let mut shift = 0;
    let mut value = 0;
    
    for ch in segment.chars() {
        let digit = match ch {
            'A'..='Z' => ch as i32 - 'A' as i32,
            'a'..='z' => ch as i32 - 'a' as i32 + 26,
            '0'..='9' => ch as i32 - '0' as i32 + 52,
            '+' => 62,
            '/' => 63,
            _ => continue,
        };
        
        let continuation = digit & VLQ_CONTINUATION_BIT;
        value += (digit & (VLQ_BASE - 1)) << shift;
        
        if continuation == 0 {
            // Decode sign
            let is_negative = (value & 1) == 1;
            value >>= 1;
            if is_negative {
                value = -value;
            }
            values.push(value);
            value = 0;
            shift = 0;
        } else {
            shift += 5;
        }
    }
    
    values
}

/// Simple base64 decoder
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    
    let mut output = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    
    for ch in input.bytes() {
        if ch == b'=' {
            break;
        }
        
        let value = ALPHABET.iter().position(|&c| c == ch)? as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vlq_decode() {
        // Test basic VLQ decoding
        assert_eq!(decode_vlq("A"), vec![0]);
        assert_eq!(decode_vlq("C"), vec![1]);
        assert_eq!(decode_vlq("D"), vec![-1]);
        assert_eq!(decode_vlq("AAAA"), vec![0, 0, 0, 0]);
    }
    
    #[test]
    fn test_source_map_parse() {
        let json = r#"{"version":3,"sources":["test.ts"],"mappings":"AAAA","names":[]}"#;
        let map = SourceMap::from_json(json).unwrap();
        assert_eq!(map.version, 3);
        assert_eq!(map.sources, vec!["test.ts"]);
    }
    
    #[test]
    fn test_source_map_lookup() {
        let json = r#"{"version":3,"sources":["test.ts"],"mappings":"AAAA,CAAC","names":[]}"#;
        let map = SourceMap::from_json(json).unwrap();
        
        let loc = map.lookup(0, 0).unwrap();
        assert_eq!(loc.source, "test.ts");
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 1);
    }
    
    #[test]
    fn test_inline_source_map() {
        let source = r#"console.log("hello");
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbInRlc3QudHMiXSwibWFwcGluZ3MiOiJBQUFBIiwibmFtZXMiOltdfQ=="#;
        
        let map = SourceMap::from_inline(source).unwrap();
        assert_eq!(map.version, 3);
        assert_eq!(map.sources, vec!["test.ts"]);
    }
}
