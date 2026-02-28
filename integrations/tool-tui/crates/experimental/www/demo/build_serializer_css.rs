use std::fs;
use std::io::Write;

/// DX Serializer Machine Format for CSS
/// Uses varint encoding and compact binary representation
fn main() {
    let styles = vec![
        (1, "margin:0;padding:0;box-sizing:border-box"),
        (2, "font-family:system-ui,-apple-system,sans-serif;background:linear-gradient(135deg,#667eea 0%,#764ba2 100%);min-height:100vh;display:flex;align-items:center;justify-content:center;padding:20px"),
        (3, "background:white;border-radius:16px;box-shadow:0 20px 60px rgba(0,0,0,0.3);padding:40px;max-width:600px;width:100%"),
        (4, "color:#667eea;font-size:2rem;margin-bottom:10px"),
        (5, "color:#666;margin-bottom:30px;font-size:0.9rem"),
        (6, "display:flex;gap:10px;margin-bottom:20px"),
        (7, "flex:1;padding:12px;border:2px solid #e0e0e0;border-radius:8px;font-size:1rem;transition:border-color 0.2s"),
        (8, "outline:none;border-color:#667eea"),
        (9, "background:#667eea;color:white;border:none;padding:12px 24px;font-size:1rem;border-radius:8px;cursor:pointer;transition:all 0.2s;min-width:44px;min-height:44px"),
        (10, "background:#5568d3;transform:translateY(-2px);box-shadow:0 4px 12px rgba(102,126,234,0.4)"),
        (11, "transform:translateY(0)"),
        (12, "margin-bottom:20px"),
        (13, "display:flex;align-items:center;gap:12px;padding:12px;background:#f8f9fa;border-radius:8px;margin-bottom:8px;transition:all 0.2s"),
        (14, "background:#e9ecef"),
        (15, "opacity:0.6"),
        (16, "text-decoration:line-through;color:#999"),
        (17, "width:20px;height:20px;cursor:pointer"),
        (18, "flex:1;font-size:1rem;color:#333"),
        (19, "background:#f44336;padding:8px 16px;font-size:0.9rem;min-width:auto;min-height:auto"),
        (20, "background:#d32f2f"),
        (21, "display:flex;justify-content:space-between;padding:15px;background:#f8f9fa;border-radius:8px;margin-bottom:20px"),
        (22, "text-align:center"),
        (23, "font-size:1.5rem;font-weight:700;color:#667eea"),
        (24, "font-size:0.8rem;color:#666;margin-top:4px"),
        (25, "background:#e8f5e9;padding:15px;border-radius:8px;border-left:4px solid #4caf50"),
        (26, "font-weight:600;color:#2e7d32;margin-bottom:10px"),
        (27, "list-style:none;padding:0"),
        (28, "padding:4px 0;color:#2e7d32;font-size:0.9rem"),
        (29, "text-align:center;padding:40px;color:#999"),
        (30, "font-size:3rem;margin-bottom:10px"),
    ];

    // DX Serializer Machine Format
    // Format: [type_byte][varint_count][entries...]
    // Entry: [varint_id][varint_len][data]
    
    let mut output = Vec::new();
    
    // Type byte: 0x01 = map/dictionary
    output.push(0x01);
    
    // Entry count (varint)
    write_varint(&mut output, styles.len() as u64);
    
    // Write entries inline (id, length, data)
    for (id, css) in &styles {
        write_varint(&mut output, *id as u64);
        write_varint(&mut output, css.len() as u64);
        output.extend_from_slice(css.as_bytes());
    }
    
    // Write to file
    fs::write("crates/www/demo/styles.sr", &output).unwrap();
    
    println!("Generated DX Serializer CSS: {} bytes", output.len());
    println!("  Format: inline (id, length, data)");
    
    // Compress
    use std::process::Command;
    let _ = Command::new("gzip")
        .args(&["-9", "-k", "-f", "crates/www/demo/styles.sr"])
        .output();
    
    if let Ok(metadata) = fs::metadata("crates/www/demo/styles.sr.gz") {
        let compressed_size = metadata.len();
        println!("\nCompressed: {} bytes ({}% of original)", 
            compressed_size,
            (compressed_size * 100) / output.len() as u64);
    }
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
