/// Level 4: Varint Encoding
///
/// Most apps use < 256 unique utilities. Varint encoding saves 50% payload size.
///
/// Encoding:
/// - ID 0-127:      1 byte  (0x00 - 0x7F)
/// - ID 128-16383:  2 bytes (0x80 0x00 - 0xFF 0x7F)
use std::io::{self, Read, Write};

/// Encode a single u16 as varint
///
/// # Returns
/// * `Vec<u8>` containing 1-2 bytes
pub fn encode_varint(value: u16) -> Vec<u8> {
    if value < 128 {
        // Single byte: 0xxxxxxx
        vec![value as u8]
    } else {
        // Two bytes: 1xxxxxxx xxxxxxxx
        let high = ((value >> 7) as u8) | 0x80;
        let low = (value & 0x7F) as u8;
        vec![high, low]
    }
}

/// Decode a varint from a byte slice
///
/// # Returns
/// * (value, bytes_consumed)
pub fn decode_varint(bytes: &[u8]) -> Result<(u16, usize), &'static str> {
    if bytes.is_empty() {
        return Err("Empty buffer");
    }

    let first = bytes[0];

    if (first & 0x80) == 0 {
        // Single byte: 0xxxxxxx
        Ok((first as u16, 1))
    } else {
        // Two bytes: 1xxxxxxx xxxxxxxx
        if bytes.len() < 2 {
            return Err("Incomplete varint");
        }

        let high = (first & 0x7F) as u16;
        let low = bytes[1] as u16;
        let value = (high << 7) | low;

        Ok((value, 2))
    }
}

/// Encode a list of style IDs as varint stream
pub fn encode_id_list(ids: &[u16]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(ids.len() * 2); // Worst case: 2 bytes per ID

    for &id in ids {
        let encoded = encode_varint(id);
        buffer.extend_from_slice(&encoded);
    }

    buffer
}

/// Decode a varint stream back to ID list
pub fn decode_id_list(bytes: &[u8]) -> Result<Vec<u16>, &'static str> {
    let mut ids = Vec::new();
    let mut pos = 0;

    while pos < bytes.len() {
        let (value, consumed) = decode_varint(&bytes[pos..])?;
        ids.push(value);
        pos += consumed;
    }

    Ok(ids)
}

/// Write varint stream to a Writer
pub fn write_varint_stream<W: Write>(writer: &mut W, ids: &[u16]) -> io::Result<usize> {
    let encoded = encode_id_list(ids);
    writer.write_all(&encoded)?;
    Ok(encoded.len())
}

/// Read varint stream from a Reader
pub fn read_varint_stream<R: Read>(reader: &mut R, _max_ids: usize) -> io::Result<Vec<u16>> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    decode_id_list(&buffer).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Compression statistics
pub struct CompressionStats {
    pub original_size: usize,
    pub compressed_size: usize,
}

impl CompressionStats {
    pub fn ratio(&self) -> f64 {
        if self.original_size == 0 {
            0.0
        } else {
            (self.compressed_size as f64 / self.original_size as f64) * 100.0
        }
    }

    pub fn savings(&self) -> f64 {
        100.0 - self.ratio()
    }
}

/// Calculate compression stats for an ID list
pub fn calculate_compression(ids: &[u16]) -> CompressionStats {
    let original_size = ids.len() * std::mem::size_of::<u16>();
    let compressed = encode_id_list(ids);
    let compressed_size = compressed.len();

    CompressionStats {
        original_size,
        compressed_size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_single_byte() {
        assert_eq!(encode_varint(0), vec![0x00]);
        assert_eq!(encode_varint(1), vec![0x01]);
        assert_eq!(encode_varint(42), vec![0x2A]);
        assert_eq!(encode_varint(127), vec![0x7F]);
    }

    #[test]
    fn test_encode_two_bytes() {
        assert_eq!(encode_varint(128), vec![0x81, 0x00]);
        assert_eq!(encode_varint(255), vec![0x81, 0x7F]);
        assert_eq!(encode_varint(256), vec![0x82, 0x00]);
        assert_eq!(encode_varint(1000), vec![0x87, 0x68]);
    }

    #[test]
    fn test_decode_single_byte() {
        assert_eq!(decode_varint(&[0x00]), Ok((0, 1)));
        assert_eq!(decode_varint(&[0x2A]), Ok((42, 1)));
        assert_eq!(decode_varint(&[0x7F]), Ok((127, 1)));
    }

    #[test]
    fn test_decode_two_bytes() {
        assert_eq!(decode_varint(&[0x81, 0x00]), Ok((128, 2)));
        assert_eq!(decode_varint(&[0x81, 0x7F]), Ok((255, 2)));
        assert_eq!(decode_varint(&[0x87, 0x68]), Ok((1000, 2)));
    }

    #[test]
    fn test_roundtrip() {
        for value in 0..500 {
            let encoded = encode_varint(value);
            let (decoded, consumed) = decode_varint(&encoded).unwrap();
            assert_eq!(decoded, value);
            assert_eq!(consumed, encoded.len());
        }
    }

    #[test]
    fn test_encode_id_list() {
        let ids = vec![42, 87, 12]; // All < 128
        let encoded = encode_id_list(&ids);

        // Should be 3 bytes (1 byte each)
        assert_eq!(encoded, vec![0x2A, 0x57, 0x0C]);
    }

    #[test]
    fn test_decode_id_list() {
        let bytes = vec![0x2A, 0x57, 0x0C];
        let ids = decode_id_list(&bytes).unwrap();
        assert_eq!(ids, vec![42, 87, 12]);
    }

    #[test]
    fn test_mixed_sizes() {
        let ids = vec![42, 255, 12]; // Middle one needs 2 bytes
        let encoded = encode_id_list(&ids);
        let decoded = decode_id_list(&encoded).unwrap();
        assert_eq!(ids, decoded);
    }

    #[test]
    fn test_compression_stats() {
        // All IDs < 128 (typical case)
        let ids = vec![4, 26, 35, 42, 87, 12];
        let stats = calculate_compression(&ids);

        // Original: 6 IDs × 2 bytes = 12 bytes
        // Compressed: 6 IDs × 1 byte = 6 bytes
        assert_eq!(stats.original_size, 12);
        assert_eq!(stats.compressed_size, 6);
        assert_eq!(stats.savings(), 50.0);
    }

    #[test]
    fn test_worst_case_compression() {
        // All IDs > 127
        let ids = vec![128, 255, 500];
        let stats = calculate_compression(&ids);

        // Original: 3 IDs × 2 bytes = 6 bytes
        // Compressed: 3 IDs × 2 bytes = 6 bytes
        assert_eq!(stats.original_size, 6);
        assert_eq!(stats.compressed_size, 6);
        assert_eq!(stats.savings(), 0.0);
    }

    #[test]
    fn test_typical_app_compression() {
        // Realistic app: mostly common utilities (ID < 128)
        // 80% single-byte, 20% two-byte
        let ids: Vec<u16> = (0..100).map(|i| if i < 80 { i } else { 128 + i }).collect();

        let stats = calculate_compression(&ids);

        // Original: 100 IDs × 2 bytes = 200 bytes
        // Compressed: 80 × 1 byte + 20 × 2 bytes = 120 bytes
        assert_eq!(stats.original_size, 200);
        assert_eq!(stats.compressed_size, 120);
        assert_eq!(stats.savings(), 40.0); // 40% reduction
    }

    #[test]
    fn test_empty_list() {
        let ids: Vec<u16> = vec![];
        let encoded = encode_id_list(&ids);
        assert_eq!(encoded.len(), 0);

        let decoded = decode_id_list(&encoded).unwrap();
        assert_eq!(decoded.len(), 0);
    }

    #[test]
    fn test_decode_error_incomplete() {
        // Two-byte varint with only one byte present
        let bytes = vec![0x81]; // Missing second byte
        let result = decode_varint(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_error_empty() {
        let bytes: Vec<u8> = vec![];
        let result = decode_varint(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_performance() {
        use std::time::Instant;

        let ids: Vec<u16> = (0..1000).collect();

        // Encoding performance
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = encode_id_list(&ids);
        }
        let encode_time = start.elapsed();

        // Decoding performance
        let encoded = encode_id_list(&ids);
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = decode_id_list(&encoded).unwrap();
        }
        let decode_time = start.elapsed();

        println!("Encode: {:?}, Decode: {:?}", encode_time, decode_time);

        // In debug builds, performance can be slower due to lack of optimizations.
        // Use a more generous threshold that works in both debug and release.
        assert!(encode_time.as_millis() < 500);
        assert!(decode_time.as_millis() < 500);
    }
}
