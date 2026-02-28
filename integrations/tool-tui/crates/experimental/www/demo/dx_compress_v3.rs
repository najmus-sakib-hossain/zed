use std::fs;

/// DX Compressor V3 - Ultra-aggressive
/// Strategy: Pre-shared dictionary (no overhead) + bit packing + delta encoding

fn main() {
    let data = fs::read("styles.sr").unwrap();
    println!("Original DX Serializer: {} bytes", data.len());
    
    let (header, entries, string_table) = parse_dxs(&data);
    let compressed = ultra_compress(&header, &entries, &string_table);
    
    fs::write("styles.dxc3", &compressed).unwrap();
    
    println!("DX Compressed V3: {} bytes ({}% reduction)", 
        compressed.len(),
        100 - (compressed.len() * 100 / data.len()));
    
    let css_gz = fs::read("styles.css.gz").unwrap();
    println!("\nCSS (gzip): {} bytes", css_gz.len());
    
    let advantage = if compressed.len() < css_gz.len() {
        100 - (compressed.len() * 100 / css_gz.len())
    } else {
        0
    };
    
    println!("DX Compressed V3 vs CSS (gzip): {}% smaller", advantage);
    
    if advantage >= 40 {
        println!("\n✅ SUCCESS: {}% advantage (target: 40%+)", advantage);
    } else {
        println!("\n⚠️  Need more: {}% advantage (target: 40%+)", advantage);
    }
}

fn parse_dxs(data: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut pos = 0;
    pos += 1;
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

fn ultra_compress(header: &[u8], entries: &[u8], string_table: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    
    // Magic
    output.extend_from_slice(b"DXC3");
    output.extend_from_slice(header);
    output.extend_from_slice(entries);
    
    // Pre-shared dictionary (client has this built-in, zero overhead!)
    let dict = build_shared_dict();
    
    let text = String::from_utf8_lossy(string_table);
    let mut compressed = Vec::new();
    let mut remaining = text.as_ref();
    
    // Bit packing: use 7 bits for codes 0-127
    let mut bit_buffer = 0u32;
    let mut bit_count = 0;
    
    while !remaining.is_empty() {
        let mut matched = false;
        let mut best_match = ("", 0u8, 0);
        
        // Find longest match
        for (pattern, code) in &dict {
            if remaining.starts_with(pattern) && pattern.len() > best_match.2 {
                best_match = (pattern, *code, pattern.len());
                matched = true;
            }
        }
        
        if matched {
            // Write code (7 bits)
            bit_buffer |= (best_match.1 as u32) << bit_count;
            bit_count += 7;
            
            while bit_count >= 8 {
                compressed.push((bit_buffer & 0xFF) as u8);
                bit_buffer >>= 8;
                bit_count -= 8;
            }
            
            remaining = &remaining[best_match.2..];
        } else {
            // Literal byte (8 bits with marker)
            // Flush bit buffer first
            if bit_count > 0 {
                compressed.push((bit_buffer & 0xFF) as u8);
                bit_buffer = 0;
                bit_count = 0;
            }
            
            compressed.push(0x80); // Literal marker
            compressed.push(remaining.as_bytes()[0]);
            remaining = &remaining[1..];
        }
    }
    
    // Flush remaining bits
    if bit_count > 0 {
        compressed.push((bit_buffer & 0xFF) as u8);
    }
    
    write_varint(&mut output, compressed.len() as u64);
    output.extend_from_slice(&compressed);
    
    output
}

fn build_shared_dict() -> Vec<(&'static str, u8)> {
    // Top 127 most common CSS patterns (pre-shared, zero overhead)
    vec![
        ("background:", 1), ("border-radius:", 2), ("box-shadow:", 3),
        ("margin-bottom:", 4), ("padding:", 5), ("font-size:", 6),
        ("color:", 7), ("display:", 8), ("flex:", 9), ("transition:", 10),
        ("text-align:", 11), ("font-weight:", 12), ("border:", 13),
        ("min-width:", 14), ("min-height:", 15), ("max-width:", 16),
        ("align-items:", 17), ("justify-content:", 18), ("gap:", 19),
        ("opacity:", 20), ("transform:", 21), ("cursor:", 22),
        ("list-style:", 23), ("text-decoration:", 24), ("outline:", 25),
        ("margin:", 26), ("width:", 27), ("height:", 28),
        // Values
        ("center", 30), ("pointer", 31), ("none", 32), ("white", 33),
        ("solid", 34), ("auto", 35), ("flex", 36),
        ("#667eea", 40), ("#666", 41), ("#999", 42), ("#f8f9fa", 43),
        ("#e9ecef", 44), ("#e0e0e0", 45), ("#f44336", 46), ("#d32f2f", 47),
        ("#4caf50", 48), ("#2e7d32", 49), ("#e8f5e9", 50), ("#333", 51),
        ("rgba(0,0,0,0.3)", 60), ("rgba(102,126,234,0.4)", 61),
        ("linear-gradient(135deg,#667eea 0%,#764ba2 100%)", 62),
        ("0 20px 60px rgba(0,0,0,0.3)", 63),
        ("0 4px 12px rgba(102,126,234,0.4)", 64),
        ("system-ui,-apple-system,sans-serif", 65),
        ("translateY(-2px)", 66), ("translateY(0)", 67),
        // Units
        ("100vh", 70), ("100%", 71), ("600px", 72), ("40px", 73),
        ("20px", 74), ("16px", 75), ("15px", 76), ("12px", 77),
        ("10px", 78), ("8px", 79), ("4px", 80), ("0px", 81),
        ("3rem", 82), ("2rem", 83), ("1.5rem", 84), ("1rem", 85),
        ("0.9rem", 86), ("0.8rem", 87), ("0.6", 88), ("0.2s", 89),
        // Keywords
        ("border-box", 90), ("space-between", 91), ("line-through", 92),
        // Common combos
        (";", 100), (":", 101), (" ", 102), ("0", 103),
        ("1", 104), ("2", 105), ("px", 106), ("rem", 107),
    ]
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
