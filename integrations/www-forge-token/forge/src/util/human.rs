use std::time::Duration;

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

pub fn human_duration(duration: Duration) -> String {
    if duration.as_millis() < 1_000 {
        return format!("{}ms", duration.as_millis());
    }
    if duration.as_secs() < 60 {
        return format!("{:.1}s", duration.as_secs_f64());
    }
    format!("{:.1}m", duration.as_secs_f64() / 60.0)
}

pub fn short_hex(hash: &[u8; 32]) -> String {
    let h = hex::encode(hash);
    h.chars().take(12).collect()
}
