//! Binary data handling with zero-copy optimization.
//!
//! This module provides Node.js `Buffer` compatibility with enhanced performance
//! through zero-copy operations.

use bytes::{Bytes, BytesMut};

/// Buffer encoding types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// UTF-8 encoding
    Utf8,
    /// ASCII encoding
    Ascii,
    /// Base64 encoding
    Base64,
    /// Hexadecimal encoding
    Hex,
    /// Latin-1 (ISO-8859-1) encoding
    Latin1,
}

/// Buffer implementation compatible with Node.js Buffer.
#[derive(Clone, Debug)]
pub struct Buffer {
    inner: Bytes,
}

impl Buffer {
    /// Allocate a zero-filled buffer.
    pub fn alloc(size: usize) -> Self {
        Self {
            inner: Bytes::from(vec![0u8; size]),
        }
    }

    /// Allocate an uninitialized buffer (unsafe, but fast).
    pub fn alloc_unsafe(size: usize) -> Self {
        let mut buf = BytesMut::with_capacity(size);
        buf.resize(size, 0);
        Self {
            inner: buf.freeze(),
        }
    }

    /// Create from string with encoding.
    pub fn from_string(s: &str, encoding: Encoding) -> Self {
        let bytes = match encoding {
            Encoding::Utf8 => s.as_bytes().to_vec(),
            Encoding::Ascii => s.bytes().map(|b| b & 0x7f).collect(),
            Encoding::Base64 => {
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s)
                    .unwrap_or_default()
            }
            Encoding::Hex => hex_decode(s).unwrap_or_default(),
            Encoding::Latin1 => s.bytes().collect(),
        };
        Self {
            inner: Bytes::from(bytes),
        }
    }

    /// Create from byte slice (zero-copy when possible).
    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            inner: Bytes::copy_from_slice(data),
        }
    }

    /// Create from Vec<u8> (zero-copy).
    pub fn from_vec(data: Vec<u8>) -> Self {
        Self {
            inner: Bytes::from(data),
        }
    }

    /// Convert to string with encoding.
    pub fn to_string(&self, encoding: Encoding) -> String {
        match encoding {
            Encoding::Utf8 => String::from_utf8_lossy(&self.inner).to_string(),
            Encoding::Ascii => self.inner.iter().map(|&b| (b & 0x7f) as char).collect(),
            Encoding::Base64 => {
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &self.inner)
            }
            Encoding::Hex => hex_encode(&self.inner),
            Encoding::Latin1 => self.inner.iter().map(|&b| b as char).collect(),
        }
    }

    /// Concatenate buffers.
    pub fn concat(buffers: &[Buffer]) -> Self {
        let total_len: usize = buffers.iter().map(|b| b.len()).sum();
        let mut result = BytesMut::with_capacity(total_len);

        for buf in buffers {
            result.extend_from_slice(&buf.inner);
        }

        Self {
            inner: result.freeze(),
        }
    }

    /// Get length.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Slice buffer (zero-copy).
    pub fn slice(&self, start: usize, end: usize) -> Self {
        Self {
            inner: self.inner.slice(start..end),
        }
    }

    /// Get underlying bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }

    /// Convert to Bytes (zero-copy).
    pub fn into_bytes(self) -> Bytes {
        self.inner
    }

    /// Read unsigned 8-bit integer.
    pub fn read_u8(&self, offset: usize) -> Option<u8> {
        self.inner.get(offset).copied()
    }

    /// Read unsigned 16-bit integer (big-endian).
    pub fn read_u16_be(&self, offset: usize) -> Option<u16> {
        if offset + 2 > self.len() {
            return None;
        }
        Some(u16::from_be_bytes([self.inner[offset], self.inner[offset + 1]]))
    }

    /// Read unsigned 32-bit integer (big-endian).
    pub fn read_u32_be(&self, offset: usize) -> Option<u32> {
        if offset + 4 > self.len() {
            return None;
        }
        Some(u32::from_be_bytes([
            self.inner[offset],
            self.inner[offset + 1],
            self.inner[offset + 2],
            self.inner[offset + 3],
        ]))
    }
}

impl AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

impl From<Vec<u8>> for Buffer {
    fn from(data: Vec<u8>) -> Self {
        Self::from_vec(data)
    }
}

impl From<&[u8]> for Buffer {
    fn from(data: &[u8]) -> Self {
        Self::from_slice(data)
    }
}

impl From<Bytes> for Buffer {
    fn from(bytes: Bytes) -> Self {
        Self { inner: bytes }
    }
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }

    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc() {
        let buf = Buffer::alloc(10);
        assert_eq!(buf.len(), 10);
        assert!(buf.as_bytes().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_from_string_utf8() {
        let buf = Buffer::from_string("hello", Encoding::Utf8);
        assert_eq!(buf.to_string(Encoding::Utf8), "hello");
    }

    #[test]
    fn test_from_string_base64() {
        let buf = Buffer::from_string("aGVsbG8=", Encoding::Base64);
        assert_eq!(buf.to_string(Encoding::Utf8), "hello");
    }

    #[test]
    fn test_from_string_hex() {
        let buf = Buffer::from_string("68656c6c6f", Encoding::Hex);
        assert_eq!(buf.to_string(Encoding::Utf8), "hello");
    }

    #[test]
    fn test_concat() {
        let buf1 = Buffer::from_string("hello", Encoding::Utf8);
        let buf2 = Buffer::from_string(" world", Encoding::Utf8);
        let result = Buffer::concat(&[buf1, buf2]);
        assert_eq!(result.to_string(Encoding::Utf8), "hello world");
    }

    #[test]
    fn test_slice() {
        let buf = Buffer::from_string("hello world", Encoding::Utf8);
        let slice = buf.slice(0, 5);
        assert_eq!(slice.to_string(Encoding::Utf8), "hello");
    }
}
