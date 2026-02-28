use std::fs;
use std::io::Write;

/// Binary Dawn CSS Header
const MAGIC: [u8; 4] = [0x44, 0x58, 0x42, 0x44]; // "DXBD"
const VERSION: u8 = 1;

/// Generate Binary Dawn CSS file
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

    // Calculate sizes
    let mut string_table = Vec::new();
    let mut entries = Vec::new();
    
    for (id, css) in &styles {
        let offset = string_table.len() as u32;
        let len = css.len() as u16;
        
        entries.push((*id, offset, len));
        string_table.extend_from_slice(css.as_bytes());
    }

    // Build binary file
    let mut output = Vec::new();
    
    // Header (12 bytes)
    output.extend_from_slice(&MAGIC);
    output.push(VERSION);
    output.push(0); // flags
    output.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    
    // Checksum (simple sum for demo)
    let checksum: u32 = string_table.iter().map(|&b| b as u32).sum();
    output.extend_from_slice(&checksum.to_le_bytes());
    
    // Entries (id: varint, offset: u32, len: u16)
    let entry_count = entries.len();
    for (id, offset, len) in &entries {
        // Varint encoding for id < 128
        if *id < 128 {
            output.push(*id as u8);
        } else {
            output.push((*id & 0x7F) as u8 | 0x80);
            output.push((*id >> 7) as u8);
        }
        output.extend_from_slice(&offset.to_le_bytes());
        output.extend_from_slice(&len.to_le_bytes());
    }
    
    // String table
    output.extend_from_slice(&string_table);
    
    // Write to file
    let mut file = fs::File::create("styles.binary").unwrap();
    file.write_all(&output).unwrap();
    
    println!("Generated Binary Dawn CSS: {} bytes", output.len());
    println!("  Header: 12 bytes");
    println!("  Entries: {} bytes", entry_count * 7);
    println!("  String table: {} bytes", string_table.len());
    println!("  vs Traditional CSS: ~{} bytes ({}% reduction)", 
        string_table.len() * 3, 
        100 - (output.len() * 100 / (string_table.len() * 3)));
    
    // Compress with gzip
    use std::process::Command;
    let _ = Command::new("gzip")
        .args(&["-9", "-k", "-f", "styles.binary"])
        .output();
    
    if let Ok(metadata) = fs::metadata("styles.binary.gz") {
        let compressed_size = metadata.len();
        println!("\nCompressed: {} bytes ({}% of original)", 
            compressed_size,
            (compressed_size * 100) / output.len() as u64);
    }
}
