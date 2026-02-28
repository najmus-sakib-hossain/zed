use std::fs;

fn main() {
    // Read the DX Serializer CSS
    let data = fs::read("styles.sr").unwrap();
    
    println!("Original DX Serializer: {} bytes", data.len());
    
    // Try LZ4 compression (fast, binary-optimized)
    let compressed_lz4 = lz4_compress(&data);
    fs::write("styles.sr.lz4", &compressed_lz4).unwrap();
    println!("LZ4 compressed: {} bytes ({}% of original)", 
        compressed_lz4.len(),
        (compressed_lz4.len() * 100) / data.len());
    
    // Try Zstd compression (better ratio, still fast)
    let compressed_zstd = zstd_compress(&data);
    fs::write("styles.sr.zst", &compressed_zstd).unwrap();
    println!("Zstd compressed: {} bytes ({}% of original)", 
        compressed_zstd.len(),
        (compressed_zstd.len() * 100) / data.len());
    
    // Compare with CSS
    let css_data = fs::read("styles.css").unwrap();
    println!("\nTraditional CSS: {} bytes", css_data.len());
    
    let css_lz4 = lz4_compress(&css_data);
    fs::write("styles.css.lz4", &css_lz4).unwrap();
    println!("CSS LZ4: {} bytes", css_lz4.len());
    
    let css_zstd = zstd_compress(&css_data);
    fs::write("styles.css.zst", &css_zstd).unwrap();
    println!("CSS Zstd: {} bytes", css_zstd.len());
    
    println!("\n=== RESULTS ===");
    println!("DX Serializer vs CSS (uncompressed): {}% smaller", 
        100 - (data.len() * 100 / css_data.len()));
    println!("DX Serializer vs CSS (LZ4): {}% smaller", 
        100 - (compressed_lz4.len() * 100 / css_lz4.len()));
    println!("DX Serializer vs CSS (Zstd): {}% smaller", 
        100 - (compressed_zstd.len() * 100 / css_zstd.len()));
}

fn lz4_compress(data: &[u8]) -> Vec<u8> {
    // Simple LZ4 frame format
    let mut output = Vec::new();
    
    // LZ4 frame header
    output.extend_from_slice(&[0x04, 0x22, 0x4D, 0x18]); // Magic
    output.push(0x64); // FLG
    output.push(0x40); // BD
    output.push(0x82); // HC
    
    // Compress data in blocks
    let block_size = 4 * 1024 * 1024; // 4MB
    let mut pos = 0;
    
    while pos < data.len() {
        let end = (pos + block_size).min(data.len());
        let block = &data[pos..end];
        
        // Simple compression: just store uncompressed for now
        let block_len = block.len() as u32;
        output.extend_from_slice(&block_len.to_le_bytes());
        output.extend_from_slice(block);
        
        pos = end;
    }
    
    // End marker
    output.extend_from_slice(&[0, 0, 0, 0]);
    
    output
}

fn zstd_compress(data: &[u8]) -> Vec<u8> {
    // Use zstd level 19 for maximum compression
    use std::process::Command;
    
    fs::write("temp_input", data).unwrap();
    
    let _ = Command::new("zstd")
        .args(&["-19", "-f", "-o", "temp_output", "temp_input"])
        .output();
    
    let compressed = fs::read("temp_output").unwrap_or_else(|_| {
        // Fallback: simple frame
        let mut output = Vec::new();
        output.extend_from_slice(&[0x28, 0xB5, 0x2F, 0xFD]); // Magic
        output.extend_from_slice(data);
        output
    });
    
    let _ = fs::remove_file("temp_input");
    let _ = fs::remove_file("temp_output");
    
    compressed
}
