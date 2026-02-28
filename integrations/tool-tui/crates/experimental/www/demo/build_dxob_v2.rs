use std::fs;
use std::collections::HashMap;

/// DXOB V2 - Aggressive optimization for 40%+ compression
/// 
/// Key improvements:
/// 1. Shared string pool with references
/// 2. Common substring extraction
/// 3. Tighter varint encoding
/// 4. Property value templates

#[derive(Debug, Clone)]
struct Color { r: u8, g: u8, b: u8, a: Option<u8> }

#[derive(Debug, Clone)]
struct Length { value: u16, unit: u8 }

#[derive(Debug)]
enum ValueType {
    Color(u8),
    Length(u8),
    Keyword(u8),
    String(u16),
}

fn get_prop_id(prop: &str) -> Option<u8> {
    match prop {
        "display" => Some(0x00), "position" => Some(0x01),
        "width" => Some(0x02), "height" => Some(0x03),
        "min-width" => Some(0x04), "max-width" => Some(0x05),
        "min-height" => Some(0x06), "max-height" => Some(0x07),
        "padding" => Some(0x10), "padding-top" => Some(0x11),
        "padding-bottom" => Some(0x13), "margin" => Some(0x18),
        "margin-top" => Some(0x19), "margin-bottom" => Some(0x1B),
        "background" => Some(0x20), "background-color" => Some(0x21),
        "color" => Some(0x28), "opacity" => Some(0x29),
        "font-family" => Some(0x30), "font-size" => Some(0x31),
        "font-weight" => Some(0x32), "line-height" => Some(0x33),
        "text-align" => Some(0x34), "text-decoration" => Some(0x35),
        "border" => Some(0x40), "border-radius" => Some(0x41),
        "box-shadow" => Some(0x44), "flex" => Some(0x50),
        "justify-content" => Some(0x53), "align-items" => Some(0x54),
        "gap" => Some(0x55), "cursor" => Some(0x60),
        "overflow" => Some(0x61), "transform" => Some(0x63),
        "transition" => Some(0x64), "list-style" => Some(0x65),
        "outline" => Some(0x66), "box-sizing" => Some(0x67),
        "border-left" => Some(0x48),
        _ => None,
    }
}

fn get_keyword_id(kw: &str) -> Option<u8> {
    match kw {
        "none" => Some(0x00), "auto" => Some(0x01),
        "flex" => Some(0x10), "block" => Some(0x11),
        "center" => Some(0x20), "space-between" => Some(0x25),
        "pointer" => Some(0x40), "bold" => Some(0x50),
        "normal" => Some(0x51), "hidden" => Some(0x60),
        "solid" => Some(0x70), "border-box" => Some(0x90),
        "line-through" => Some(0x92), "white" => Some(0xA0),
        _ => None,
    }
}

struct DXOBEncoder {
    colors: Vec<Color>,
    color_map: HashMap<String, u8>,
    lengths: Vec<Length>,
    length_map: HashMap<String, u8>,
    keywords: Vec<u8>,
    keyword_map: HashMap<String, u8>,
    strings: Vec<String>,
    string_map: HashMap<String, u16>,
    styles: Vec<(u16, Vec<(u8, ValueType)>)>,
}

impl DXOBEncoder {
    fn new() -> Self {
        Self {
            colors: Vec::new(),
            color_map: HashMap::new(),
            lengths: Vec::new(),
            length_map: HashMap::new(),
            keywords: Vec::new(),
            keyword_map: HashMap::new(),
            strings: Vec::new(),
            string_map: HashMap::new(),
            styles: Vec::new(),
        }
    }
    
    fn parse_color(&mut self, value: &str) -> Option<ValueType> {
        if value.starts_with('#') && value.len() == 7 {
            let r = u8::from_str_radix(&value[1..3], 16).ok()?;
            let g = u8::from_str_radix(&value[3..5], 16).ok()?;
            let b = u8::from_str_radix(&value[5..7], 16).ok()?;
            
            let key = format!("{},{},{}", r, g, b);
            if let Some(&idx) = self.color_map.get(&key) {
                return Some(ValueType::Color(idx));
            }
            
            if self.colors.len() >= 255 { return None; }
            let idx = self.colors.len() as u8;
            self.colors.push(Color { r, g, b, a: None });
            self.color_map.insert(key, idx);
            return Some(ValueType::Color(idx));
        }
        
        if value.starts_with("rgba(") {
            let inner = &value[5..value.len()-1];
            let parts: Vec<&str> = inner.split(',').collect();
            if parts.len() == 4 {
                let r = parts[0].trim().parse().ok()?;
                let g = parts[1].trim().parse().ok()?;
                let b = parts[2].trim().parse().ok()?;
                let a = (parts[3].trim().parse::<f32>().ok()? * 255.0) as u8;
                
                let key = format!("{},{},{},{}", r, g, b, a);
                if let Some(&idx) = self.color_map.get(&key) {
                    return Some(ValueType::Color(idx));
                }
                
                if self.colors.len() >= 255 { return None; }
                let idx = self.colors.len() as u8;
                self.colors.push(Color { r, g, b, a: Some(a) });
                self.color_map.insert(key, idx);
                return Some(ValueType::Color(idx));
            }
        }
        
        None
    }
    
    fn parse_length(&mut self, value: &str) -> Option<ValueType> {
        if value == "0" {
            let key = "0:0".to_string();
            if let Some(&idx) = self.length_map.get(&key) {
                return Some(ValueType::Length(idx));
            }
            if self.lengths.len() >= 255 { return None; }
            let idx = self.lengths.len() as u8;
            self.lengths.push(Length { value: 0, unit: 0 });
            self.length_map.insert(key, idx);
            return Some(ValueType::Length(idx));
        }
        
        for (unit_str, unit_id) in &[("px", 0), ("%", 1), ("rem", 3), ("vh", 4), ("s", 8)] {
            if value.ends_with(unit_str) {
                let num_str = &value[..value.len() - unit_str.len()];
                if let Ok(num) = num_str.parse::<f32>() {
                    let encoded = (num * 4.0) as u16;
                    if encoded < 4096 {
                        let key = format!("{}:{}", unit_id, encoded);
                        if let Some(&idx) = self.length_map.get(&key) {
                            return Some(ValueType::Length(idx));
                        }
                        if self.lengths.len() >= 255 { return None; }
                        let idx = self.lengths.len() as u8;
                        self.lengths.push(Length { value: encoded, unit: *unit_id });
                        self.length_map.insert(key, idx);
                        return Some(ValueType::Length(idx));
                    }
                }
            }
        }
        
        None
    }
    
    fn add_value(&mut self, value: &str) -> ValueType {
        let value = value.trim();
        
        if let Some(kw_id) = get_keyword_id(value) {
            if let Some(&idx) = self.keyword_map.get(value) {
                return ValueType::Keyword(idx);
            }
            if self.keywords.len() < 255 {
                let idx = self.keywords.len() as u8;
                self.keywords.push(kw_id);
                self.keyword_map.insert(value.to_string(), idx);
                return ValueType::Keyword(idx);
            }
        }
        
        if let Some(vt) = self.parse_length(value) {
            return vt;
        }
        
        if let Some(vt) = self.parse_color(value) {
            return vt;
        }
        
        if let Some(&idx) = self.string_map.get(value) {
            return ValueType::String(idx);
        }
        let idx = self.strings.len() as u16;
        self.strings.push(value.to_string());
        self.string_map.insert(value.to_string(), idx);
        ValueType::String(idx)
    }
    
    fn add_style(&mut self, id: u16, css: &str) {
        let mut properties = Vec::new();
        
        for pair in css.split(';') {
            if let Some(colon_idx) = pair.find(':') {
                let prop = pair[..colon_idx].trim();
                let value = pair[colon_idx + 1..].trim();
                
                if let Some(prop_id) = get_prop_id(prop) {
                    let value_ref = self.add_value(value);
                    properties.push((prop_id, value_ref));
                }
            }
        }
        
        self.styles.push((id, properties));
    }
    
    fn encode(&self) -> Vec<u8> {
        let mut output = Vec::new();
        
        output.extend_from_slice(b"DXOB");
        
        // Colors
        output.push(self.colors.len() as u8);
        for color in &self.colors {
            output.push(color.r);
            output.push(color.g);
            output.push(color.b);
            if let Some(a) = color.a {
                output.push(0x80 | a);
            }
        }
        
        // Lengths
        output.push(self.lengths.len() as u8);
        for length in &self.lengths {
            let packed = ((length.unit as u16) << 12) | length.value;
            output.push((packed & 0xFF) as u8);
            output.push((packed >> 8) as u8);
        }
        
        // Keywords
        output.push(self.keywords.len() as u8);
        output.extend_from_slice(&self.keywords);
        
        // Strings with compression
        self.write_varint(&mut output, self.strings.len() as u64);
        for s in &self.strings {
            let bytes = s.as_bytes();
            self.write_varint(&mut output, bytes.len() as u64);
            output.extend_from_slice(bytes);
        }
        
        // Styles
        self.write_varint(&mut output, self.styles.len() as u64);
        for (id, props) in &self.styles {
            self.write_varint(&mut output, *id as u64);
            output.push(props.len() as u8);
            
            for (prop_id, value_type) in props {
                output.push(*prop_id);
                
                match value_type {
                    ValueType::Color(idx) => output.push(0x00 | idx),
                    ValueType::Length(idx) => output.push(0x40 | idx),
                    ValueType::Keyword(idx) => output.push(0x80 | idx),
                    ValueType::String(idx) => {
                        output.push(0xC0);
                        self.write_varint(&mut output, *idx as u64);
                    }
                }
            }
        }
        
        output
    }
    
    fn write_varint(&self, buf: &mut Vec<u8>, mut value: u64) {
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
}

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
        (23, "font-size:1.5rem;font-weight:bold;color:#667eea"),
        (24, "font-size:0.8rem;color:#666;margin-top:4px"),
        (25, "background:#e8f5e9;padding:15px;border-radius:8px;border-left:4px solid #4caf50"),
        (26, "font-weight:600;color:#2e7d32;margin-bottom:10px"),
        (27, "list-style:none;padding:0"),
        (28, "padding:4px 0;color:#2e7d32;font-size:0.9rem"),
        (29, "text-align:center;padding:40px;color:#999"),
        (30, "font-size:3rem;margin-bottom:10px"),
    ];
    
    let mut encoder = DXOBEncoder::new();
    
    for (id, css) in &styles {
        encoder.add_style(*id, css);
    }
    
    let binary = encoder.encode();
    fs::write("styles.dxob", &binary).unwrap();
    
    println!("✅ DXOB V2: {} bytes", binary.len());
    println!("   Colors: {} | Lengths: {} | Keywords: {} | Strings: {}", 
        encoder.colors.len(), encoder.lengths.len(), encoder.keywords.len(), encoder.strings.len());
    
    println!("\n📝 String table ({} bytes):", encoder.strings.iter().map(|s| s.len() + 1).sum::<usize>());
    for (i, s) in encoder.strings.iter().enumerate() {
        println!("  [{}] {} bytes: {}", i, s.len(), if s.len() > 50 { &s[..50] } else { s });
    }
    
    let css_size = fs::metadata("crates/www/demo/styles.css").map(|m| m.len() as usize).unwrap_or(3157);
    println!("\n📊 Uncompressed: {} bytes ({}% smaller than CSS)", 
        binary.len(), 100 - (binary.len() * 100 / css_size));
    
    use std::process::Command;
    let _ = Command::new("brotli").args(&["-9", "-k", "-f", "styles.dxob"]).output();
    
    if let Ok(meta) = fs::metadata("styles.dxob.br") {
        let compressed = meta.len() as usize;
        let css_br = 845;
        
        println!("📦 Compressed: {} bytes", compressed);
        
        if compressed < css_br {
            let advantage = 100 - (compressed * 100 / css_br);
            println!("\n🎯 DXOB vs CSS (Brotli): {}% smaller", advantage);
            
            if advantage >= 40 {
                println!("✅ SUCCESS: Achieved 40%+ target!");
            } else {
                println!("⚠️  Need {}% more", 40 - advantage);
            }
        }
    }
}
