/// DX Serializer: Reverse Formatter (Human → Machine)
///
/// Compresses human-readable DX format back to ultra-compact machine format.
/// Enables bidirectional editing in editors.
///
/// # Architecture: Zero-Cache Design
/// - Uses HashMap lookups (O(1)) - no additional cache needed
/// - Mappings loaded once via OnceLock singleton
/// - Every lookup is instant with automatic fallback for custom keys
///
/// # The Smart Logic
/// ```text
/// For every key encountered:
///   IF key exists in mappings.dx:
///       abbreviate it (popular)
///   ELSE:
///       keep it as-is (custom)
/// ```
use crate::mappings::Mappings;
use std::io::Write;

/// Compress human-readable DX to machine format
pub fn format_machine(human_dx: &str) -> Result<Vec<u8>, String> {
    let mappings = Mappings::get();
    let mut output = Vec::new();

    let lines: Vec<&str> = human_dx.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();
        i += 1;

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Handle table headers (skip for now, will be reconstructed)
        if line.starts_with("# ") && line.contains("TABLE") {
            // Skip table header and separator
            while i < lines.len()
                && (lines[i].trim().starts_with('#') || lines[i].trim().starts_with('-'))
            {
                i += 1;
            }
            continue;
        }

        // Parse property line: key : value
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();

            // Handle prefix inheritance (^)
            if let Some(rest) = key.strip_prefix('^') {
                output.extend_from_slice(b"^");
                let rest = rest.trim();
                let compressed = compress_full_key(rest, mappings);
                output.extend_from_slice(compressed.as_bytes());
            } else {
                let compressed = compress_full_key(key, mappings);
                output.extend_from_slice(compressed.as_bytes());
            }

            output.extend_from_slice(b":");
            output.extend_from_slice(value.as_bytes());

            // Look ahead for inline properties (check next line for ^)
            if i < lines.len() && lines[i].trim().starts_with('^') {
                // Continue on same line with ^
                continue;
            }

            output.extend_from_slice(b"\n");
        }
        // Handle array/stream line: key > item | item | item
        else if line.contains('>') {
            // SAFETY: We just checked that line contains '>', so split_once will succeed
            let Some((key, values)) = line.split_once('>') else {
                continue; // Skip malformed lines
            };
            let key = key.trim();
            let values = values.trim();

            let compressed = compress_full_key(key, mappings);
            output.extend_from_slice(compressed.as_bytes());
            output.extend_from_slice(b">");

            // Compress array values (remove spaces around |)
            let compressed_values =
                values.split('|').map(|s| s.trim()).collect::<Vec<_>>().join("|");
            output.extend_from_slice(compressed_values.as_bytes());
            output.extend_from_slice(b"\n");
        }
        // Handle table schema line: Key  OtherKey  ThirdKey
        else if line.contains(char::is_whitespace) && !line.contains(':') {
            // This is a table header or data row
            let parts: Vec<&str> = line.split_whitespace().collect();

            // Check if it's a schema (all caps) or data
            if parts
                .iter()
                .all(|p| p.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
            {
                // Table schema - compress column names
                let compressed_cols: Vec<String> =
                    parts.iter().map(|col| compress_table_column(col, mappings)).collect();

                // Need to find the table key from context (previous line usually)
                // For now, output compressed columns
                output.extend_from_slice(compressed_cols.join(" ").as_bytes());
                output.extend_from_slice(b"\n");
            } else {
                // Table data row - keep as-is
                output.extend_from_slice(line.as_bytes());
                output.extend_from_slice(b"\n");
            }
        }
    }

    Ok(output)
}

/// Compress full key with dots (e.g., "context.name" → "c.n")
///
/// # The Smart Logic
/// - Splits by '.' only (underscore is NOT a separator to avoid ambiguity)
/// - Compresses each part using HashMap lookup
/// - IF popular → abbreviate, ELSE → keep as-is
///
/// # Examples
/// - "context.name" → "c.n" (both popular)
/// - "myModule.name" → "myModule.n" (mixed)
/// - "myModule.myField" → "myModule.myField" (both custom)
/// - "ui_a" → "ui_a" (underscore keys preserved as-is)
#[inline]
fn compress_full_key(full_key: &str, mappings: &Mappings) -> String {
    if full_key.contains('.') {
        // Handle nested keys: each part compressed independently
        full_key
            .split('.')
            .map(|part| mappings.compress_key(part))
            .collect::<Vec<_>>()
            .join(".")
    } else {
        // Single key (including underscore keys): direct HashMap lookup (O(1))
        // Note: We don't split by underscore because it's ambiguous and would
        // cause round-trip failures (e.g., "ui_a" would become "u_a")
        mappings.compress_key(full_key)
    }
}

/// Compress table column name
fn compress_table_column(col: &str, mappings: &Mappings) -> String {
    mappings.compress_key(col)
}

/// Compress with writer output
pub fn compress_to_writer<W: Write>(human_dx: &str, writer: &mut W) -> Result<(), String> {
    let compressed = format_machine(human_dx)?;
    writer.write_all(&compressed).map_err(|e| format!("Failed to write: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_compression() {
        let human = "context.name        : dx\n^version            : 0.0.1";
        let machine = format_machine(human).unwrap();
        let result = String::from_utf8(machine).unwrap();

        // Should compress to: c.n:dx^v:0.0.1
        assert!(result.contains("c.n:dx"));
        assert!(result.contains("v:0.0.1"));
    }

    #[test]
    fn test_array_compression() {
        // Use a key that exists in default mappings
        let human = "name           > frontend/www | frontend/mobile";
        let machine = format_machine(human).unwrap();
        let result = String::from_utf8(machine).unwrap();

        // The compressor outputs the compressed key followed by array values
        // "name" compresses to "n" in default mappings
        assert!(result.contains("n>"));
        assert!(result.contains("frontend/www"));
        assert!(result.contains("frontend/mobile"));
    }

    #[test]
    fn test_roundtrip() {
        let _original = "c.n:dx^v:0.0.1\nws>frontend|backend";

        // This would need format_human to test fully
        // For now just test compression doesn't crash
        let human = "context.name: dx\nworkspace > frontend | backend";
        let compressed = format_machine(human).unwrap();

        assert!(!compressed.is_empty());
    }
}
