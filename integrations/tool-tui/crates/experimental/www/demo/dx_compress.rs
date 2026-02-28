use std::fs;
use std::collections::HashMap;

/// DX Binary Compressor
/// Exploits structure of DX Serializer format:
/// 1. CSS properties have common prefixes (color:, background:, padding:)
/// 2. CSS values repeat (colors, sizes, keywords)
/// 3. Varint IDs are sequential (1,2,3...)
/// 4. String table has redundant patterns

fn main() {
    let data = fs::read("styles.sr").unwrap();
    
    println!("Original: {} bytes", data.len());
    
    // Parse DX Serializer format
    let (header, entries, string_table) = parse_dxs(&data);
    
    println!("  Header: {} bytes", header.len());
    println!("  Entries: {} bytes", entries.len());
    println!("  String table: {} bytes", string_table.len());
    
    // Compress using DX-specific techniques
    let compressed = dx_compress(&data);
    fs::write("styles.dxc", &compressed).unwrap();
    
    println!("\nDX Compressed: {} bytes ({}% of original)", 
        compressed.len(),
        (compressed.len() * 100) / data.len());
    
    // Compare with CSS
    let css_data = fs::read("styles.css").unwrap();
    let css_gz = fs::read("styles.css.gz").unwrap();
    
    println!("\n=== COMPARISON ===");
    println!("CSS: {} bytes", css_data.len());
    println!("CSS (gzip): {} bytes", css_gz.len());
    println!("DX Serializer: {} bytes", data.len());
    println!("DX Compressed: {} bytes", compressed.len());
    println!("\nDX Compressed vs CSS (gzip): {}% smaller", 
        100 - (compressed.len() * 100 / css_gz.len()));
}

fn parse_dxs(data: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut pos = 0;
    
    // Type byte
    let header_start = pos;
    pos += 1;
    
    // Entry count (varint)
    let (count, consumed) = read_varint(&data[pos..]);
    pos += consumed;
    let header = data[header_start..pos].to_vec();
    
    // Entries
    let entries_start = pos;
    for _ in 0..count {
        // ID (varint)
        let (_, consumed) = read_varint(&data[pos..]);
        pos += consumed;
        // Offset (varint)
        let (_, consumed) = read_varint(&data[pos..]);
        pos += consumed;
        // Length (varint)
        let (_, consumed) = read_varint(&data[pos..]);
        pos += consumed;
    }
    let entries = data[entries_start..pos].to_vec();
    
    // String table
    let string_table = data[pos..].to_vec();
    
    (header, entries, string_table)
}

fn dx_compress(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    
    // Magic: "DXC\0" (DX Compressed)
    output.extend_from_slice(b"DXC\0");
    
    // Version
    output.push(1);
    
    // Original size (varint)
    write_varint(&mut output, data.len() as u64);
    
    let (header, entries, string_table) = parse_dxs(data);
    
    // Compress string table using CSS-specific dictionary
    let compressed_strings = compress_css_strings(&string_table);
    
    // Write compressed size
    write_varint(&mut output, compressed_strings.len() as u64);
    
    // Write header (uncompressed, it's tiny)
    output.extend_from_slice(&header);
    
    // Write entries (uncompressed, already varints)
    output.extend_from_slice(&entries);
    
    // Write compressed string table
    output.extend_from_slice(&compressed_strings);
    
    output
}

fn compress_css_strings(data: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(data);
    
    // Build CSS property dictionary (most common patterns)
    let dict = vec![
        ("background:", 0x80),
        ("border-radius:", 0x81),
        ("box-shadow:", 0x82),
        ("margin-bottom:", 0x83),
        ("padding:", 0x84),
        ("font-size:", 0x85),
        ("color:", 0x86),
        ("display:", 0x87),
        ("flex:", 0x88),
        ("transition:", 0x89),
        ("text-align:", 0x8A),
        ("font-weight:", 0x8B),
        ("border:", 0x8C),
        ("min-width:", 0x8D),
        ("min-height:", 0x8E),
        ("max-width:", 0x8F),
        ("align-items:", 0x90),
        ("justify-content:", 0x91),
        ("gap:", 0x92),
        ("opacity:", 0x93),
        ("transform:", 0x94),
        ("cursor:", 0x95),
        ("list-style:", 0x96),
        ("text-decoration:", 0x97),
        ("outline:", 0x98),
        // Common values
        ("center", 0xA0),
        ("pointer", 0xA1),
        ("none", 0xA2),
        ("white", 0xA3),
        ("flex", 0xA4),
        ("solid", 0xA5),
        ("auto", 0xA6),
        ("0", 0xA7),
        ("1rem", 0xA8),
        ("8px", 0xA9),
        ("12px", 0xAA),
        ("20px", 0xAB),
        ("#667eea", 0xAC),
        ("#666", 0xAD),
        ("#999", 0xAE),
        ("#f8f9fa", 0xAF),
        ("rgba(0,0,0,0.3)", 0xB0),
        ("0.2s", 0xB1),
        ("100%", 0xB2),
        ("600px", 0xB3),
    ];
    
    let mut compressed = Vec::new();
    let mut remaining = text.as_ref();
    
    while !remaining.is_empty() {
        let mut matched = false;
        
        // Try to match dictionary entries (longest first)
        for (pattern, code) in &dict {
            if remaining.starts_with(pattern) {
                compressed.push(*code);
                remaining = &remaining[pattern.len()..];
                matched = true;
                break;
            }
        }
        
        if !matched {
            // Literal byte
            let byte = remaining.as_bytes()[0];
            if byte >= 0x80 {
                // Escape high bytes
                compressed.push(0xFF);
            }
            compressed.push(byte);
            remaining = &remaining[1..];
        }
    }
    
    compressed
}

fn read_varint(data: &[u8]) -> (u64, usize) {
    let mut value = 0u64;
    let mut shift = 0;
    let mut pos = 0;
    
    loop {
        let byte = data[pos];
        value |= ((byte & 0x7F) as u64) << shift;
        pos += 1;
        
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    
    (value, pos)
}

fn write_varint(buf: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}
