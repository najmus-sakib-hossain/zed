//! Base62 encoding/decoding for DX ∞
//!
//! Encodes integers using 0-9a-zA-Z (62 characters) for maximum density.
//! Examples: 320→5A, 540→8k, 10000→2Bi

use crate::error::{DxError, Result};

// Standard Base62: 0-9 (0-9), A-Z (10-35), a-z (36-61)
const BASE62_CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Encode an integer to Base62
#[must_use]
pub fn encode_base62(mut n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }

    let mut result = Vec::new();
    while n > 0 {
        result.push(BASE62_CHARS[(n % 62) as usize] as char);
        n /= 62;
    }
    result.reverse();
    result.into_iter().collect()
}

/// Decode a Base62 string to integer
#[must_use = "decoding result should be used"]
pub fn decode_base62(s: &str) -> Result<u64> {
    let mut result = 0u64;

    for (position, c) in s.chars().enumerate() {
        let digit = match c {
            '0'..='9' => (c as u8 - b'0') as u64,
            'A'..='Z' => (c as u8 - b'A' + 10) as u64,
            'a'..='z' => (c as u8 - b'a' + 36) as u64,
            _ => {
                return Err(DxError::Base62Error {
                    char: c,
                    position,
                    message: "expected 0-9, A-Z, or a-z".to_string(),
                });
            }
        };

        result = result
            .checked_mul(62)
            .and_then(|r| r.checked_add(digit))
            .ok_or(DxError::IntegerOverflow)?;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base62_encode() {
        assert_eq!(encode_base62(0), "0");
        assert_eq!(encode_base62(35), "Z"); // Last uppercase letter
        assert_eq!(encode_base62(61), "z"); // Last lowercase letter
        assert_eq!(encode_base62(62), "10");
        assert_eq!(encode_base62(320), "5A"); // 320 = 5*62 + 10
        assert_eq!(encode_base62(540), "8i"); // 540 = 8*62 + 44
        assert_eq!(encode_base62(180), "2u"); // 180 = 2*62 + 56
        assert_eq!(encode_base62(10000), "2bI");
    }

    #[test]
    fn test_base62_decode() {
        assert_eq!(decode_base62("0").unwrap(), 0);
        assert_eq!(decode_base62("Z").unwrap(), 35);
        assert_eq!(decode_base62("z").unwrap(), 61);
        assert_eq!(decode_base62("10").unwrap(), 62);
        assert_eq!(decode_base62("5A").unwrap(), 320);
        assert_eq!(decode_base62("8i").unwrap(), 540);
        assert_eq!(decode_base62("2u").unwrap(), 180);
        assert_eq!(decode_base62("2bI").unwrap(), 10000);
    }

    #[test]
    fn test_base62_round_trip() {
        for n in [0, 1, 61, 62, 100, 320, 540, 1000, 10000, 999999] {
            let encoded = encode_base62(n);
            let decoded = decode_base62(&encoded).unwrap();
            assert_eq!(decoded, n, "Failed for {}: {} -> {}", n, encoded, decoded);
        }
    }

    #[test]
    fn test_base62_savings() {
        // Show byte savings
        assert!(encode_base62(320).len() < "320".len()); // 2 vs 3 bytes
        assert!(encode_base62(10000).len() < "10000".len()); // 3 vs 5 bytes
    }
}
