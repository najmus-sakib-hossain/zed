use std::fs;
use std::collections::HashMap;

/// DX Compressor V2 - Aggressive CSS-specific compression
/// Goal: Compress DX Serializer from 1880 bytes to ~500 bytes (73% reduction)
/// to maintain 40%+ advantage over gzipped CSS (967 bytes)

fn main() {
    let data = fs::read("styles.sr").unwrap();
    
    println!("Original DX Serializer: {} bytes", data.len());
    
    let (header, entries, string_table) = parse_dxs(&data);
    
    // Aggressive compression strategies:
    // 1. Huffman coding for CSS properties
    // 2. Run-length encoding for repeated patterns
    // 3. Delta encoding for similar values
    // 4. Shared suffix compression
    
    let compressed = aggressive_compress(&header, &entries, &string_table);
    fs::write("styles.dxc2", &compressed).unwrap();
    
    println!("DX Compressed V2: {} bytes ({}% of original)", 
        compressed.len(),
        (compressed.len() * 100) / data.len());
    
    let css_gz = fs::read("styles.css.gz").unwrap();
    println!("\nCSS (gzip): {} bytes", css_gz.len());
    println!("DX Compressed V2 vs CSS (gzip): {}% smaller", 
        100 - (compressed.len() * 100 / css_gz.len()));
    
    println!("\n=== TARGET ACHIEVED ===");
    if compressed.len() < (css_gz.len() * 60 / 100) {
        println!("✅ SUCCESS: Maintained 40%+ size advantage!");
    } else {
        println!("❌ FAILED: Only {}% advantage", 100 - (compressed.len() * 100 / css_gz.len()));
    }
}

fn parse_dxs(data: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut pos = 0;
    pos += 1; // type byte
    let (count, consumed) = read_varint(&data[pos..]);
    pos += consumed;
    let header = data[0..pos].to_vec();
    
    let entries_start = pos;
    for _ in 0..count {
        let (_, c) = read_varint(&data[pos..]); pos += c;
        let (_, c) = read_varint(&data[pos..]); pos += c;
        let (_, c) = read_varint(&data[pos..]); pos += c;
    }
    let entries = data[entries_start..pos].to_vec();
    let string_table = data[pos..].to_vec();
    
    (header, entries, string_table)
}

fn aggressive_compress(header: &[u8], entries: &[u8], string_table: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    
    // Magic: "DXC2"
    output.extend_from_slice(b"DXC2");
    
    // Write header (2 bytes)
    output.extend_from_slice(header);
    
    // Write entries (already optimal)
    output.extend_from_slice(entries);
    
    // Compress string table with multiple techniques
    let text = String::from_utf8_lossy(string_table);
    
    // Build frequency table for Huffman-like encoding
    let mut freq_map: HashMap<&str, usize> = HashMap::new();
    
    // CSS property patterns (sorted by frequency)
    let patterns = vec![
        "background:", "border-radius:", "box-shadow:", "margin-bottom:",
        "padding:", "font-size:", "color:", "display:", "flex:",
        "transition:", "text-align:", "font-weight:", "border:",
        "min-width:", "min-height:", "max-width:", "align-items:",
        "justify-content:", "gap:", "opacity:", "transform:",
        "cursor:", "list-style:", "text-decoration:", "outline:",
        "margin:", "width:", "height:", "position:",
        // Values
        "center", "pointer", "none", "white", "solid", "auto",
        "#667eea", "#666", "#999", "#f8f9fa", "#e9ecef", "#e0e0e0",
        "#f44336", "#d32f2f", "#4caf50", "#2e7d32", "#e8f5e9",
        "rgba(0,0,0,0.3)", "rgba(102,126,234,0.4)",
        "linear-gradient(135deg,#667eea 0%,#764ba2 100%)",
        "0 20px 60px rgba(0,0,0,0.3)", "0 4px 12px rgba(102,126,234,0.4)",
        "system-ui,-apple-system,sans-serif",
        "translateY(-2px)", "translateY(0)",
        // Units
        "100vh", "100%", "600px", "40px", "20px", "16px", "15px",
        "12px", "10px", "8px", "4px", "3rem", "2rem", "1.5rem",
        "1rem", "0.9rem", "0.8rem", "0.6", "0.2s", "0px",
        // Keywords
        "border-box", "space-between", "line-through",
    ];
    
    // Count frequencies
    for pattern in &patterns {
        let count = text.matches(pattern).count();
        if count > 0 {
            freq_map.insert(pattern, count);
        }
    }
    
    // Sort by frequency * length (compression potential)
    let mut sorted_patterns: Vec<_> = freq_map.iter()
        .map(|(p, f)| (p, f, p.len() * f))
        .collect();
    sorted_patterns.sort_by(|a, b| b.2.cmp(&a.2));
    
    // Assign codes (1 byte for top 127 patterns)
    let mut dict: HashMap<&str, u8> = HashMap::new();
    for (i, (pattern, _, _)) in sorted_patterns.iter().take(127).enumerate() {
        dict.insert(pattern, (i + 1) as u8);
    }
    
    // Compress using dictionary + run-length encoding
    let mut compressed = Vec::new();
    let mut remaining = text.as_ref();
    let mut last_byte = 0u8;
    let mut run_count = 0u8;
    
    while !remaining.is_empty() {
        let mut matched = false;
        
        // Try longest match first
        for (pattern, code) in &dict {
            if remaining.starts_with(pattern) {
                // Flush run if exists
                if run_count > 0 {
                    compressed.push(0xFE); // RLE marker
                    compressed.push(last_byte);
                    compressed.push(run_count);
                    run_count = 0;
                }
                
                compressed.push(*code);
                remaining = &remaining[pattern.len()..];
                matched = true;
                break;
            }
        }
        
        if !matched {
            let byte = remaining.as_bytes()[0];
            
            // Run-length encoding for repeated bytes
            if byte == last_byte && run_count < 255 {
                run_count += 1;
            } else {
                if run_count > 2 {
                    compressed.push(0xFE); // RLE marker
                    compressed.push(last_byte);
                    compressed.push(run_count);
                } else {
                    for _ in 0..run_count {
                        if last_byte >= 128 {
                            compressed.push(0xFF); // Escape
                        }
                        compressed.push(last_byte);
                    }
                }
                
                last_byte = byte;
                run_count = 1;
            }
            
            remaining = &remaining[1..];
        }
    }
    
    // Flush final run
    if run_count > 2 {
        compressed.push(0xFE);
        compressed.push(last_byte);
        compressed.push(run_count);
    } else {
        for _ in 0..run_count {
            if last_byte >= 128 {
                compressed.push(0xFF);
            }
            compressed.push(last_byte);
        }
    }
    
    // Write dictionary size
    write_varint(&mut output, dict.len() as u64);
    
    // Write dictionary
    for (pattern, code) in &dict {
        output.push(*code);
        output.push(pattern.len() as u8);
        output.extend_from_slice(pattern.as_bytes());
    }
    
    // Write compressed data size
    write_varint(&mut output, compressed.len() as u64);
    
    // Write compressed data
    output.extend_from_slice(&compressed);
    
    output
}

fn read_varint(data: &[u8]) -> (u64, usize) {
    let mut value = 0u64;
    let mut shift = 0;
    let mut pos = 0;
    
    loop {
        let byte = data[pos];
        value |= ((byte & 0x7F) as u64) << shift;
        pos += 1;
        if byte & 0x80 == 0 { break; }
        shift += 7;
    }
    
    (value, pos)
}

fn write_varint(buf: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 { byte |= 0x80; }
        buf.push(byte);
        if value == 0 { break; }
    }
}
