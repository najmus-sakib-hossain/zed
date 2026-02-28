//! # DX Binary Codec
//!
//! Custom binary serialization replacing bincode.
//! Uses length-prefixed encoding for variable-length data.
//!
//! ## Format
//! - Strings: u32 length + UTF-8 bytes
//! - Arrays: u32 count + elements
//! - Enums: u8 variant + payload
//! - Structs: fields in order

use crate::{DxBinaryError, Result};

/// Binary encoder for HTIP payload
pub struct BinaryEncoder {
    buffer: Vec<u8>,
}

impl BinaryEncoder {
    /// Create new encoder with capacity hint
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Write u8
    #[inline]
    pub fn write_u8(&mut self, value: u8) {
        self.buffer.push(value);
    }

    /// Write u16 (little-endian)
    #[inline]
    pub fn write_u16(&mut self, value: u16) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Write u32 (little-endian)
    #[inline]
    pub fn write_u32(&mut self, value: u32) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Write i64 (little-endian)
    #[inline]
    pub fn write_i64(&mut self, value: i64) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Write f64 (little-endian)
    #[inline]
    pub fn write_f64(&mut self, value: f64) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Write bool
    #[inline]
    pub fn write_bool(&mut self, value: bool) {
        self.buffer.push(if value { 1 } else { 0 });
    }

    /// Write string (length-prefixed)
    #[inline]
    pub fn write_string(&mut self, value: &str) {
        let bytes = value.as_bytes();
        self.write_u32(bytes.len() as u32);
        self.buffer.extend_from_slice(bytes);
    }

    /// Write bytes (length-prefixed)
    #[inline]
    pub fn write_bytes(&mut self, value: &[u8]) {
        self.write_u32(value.len() as u32);
        self.buffer.extend_from_slice(value);
    }

    /// Write array of strings
    #[inline]
    pub fn write_string_array(&mut self, values: &[String]) {
        self.write_u32(values.len() as u32);
        for s in values {
            self.write_string(s);
        }
    }

    /// Write array of u8
    #[inline]
    pub fn write_u8_array(&mut self, values: &[u8]) {
        self.write_u32(values.len() as u32);
        self.buffer.extend_from_slice(values);
    }

    /// Write array of u32
    #[inline]
    pub fn write_u32_array(&mut self, values: &[u32]) {
        self.write_u32(values.len() as u32);
        for v in values {
            self.write_u32(*v);
        }
    }

    /// Finish and return buffer
    pub fn finish(self) -> Vec<u8> {
        self.buffer
    }

    /// Get current length
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

/// Binary decoder for HTIP payload
pub struct BinaryDecoder<'a> {
    buffer: &'a [u8],
    position: usize,
}

impl<'a> BinaryDecoder<'a> {
    /// Create new decoder
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer,
            position: 0,
        }
    }

    /// Check if we have enough bytes
    #[inline]
    fn check_remaining(&self, needed: usize) -> Result<()> {
        if self.position + needed > self.buffer.len() {
            return Err(DxBinaryError::IoError(format!(
                "Buffer underflow: need {} bytes, have {}",
                needed,
                self.buffer.len() - self.position
            )));
        }
        Ok(())
    }

    /// Read u8
    #[inline]
    pub fn read_u8(&mut self) -> Result<u8> {
        self.check_remaining(1)?;
        let value = self.buffer[self.position];
        self.position += 1;
        Ok(value)
    }

    /// Read u16 (little-endian)
    #[inline]
    pub fn read_u16(&mut self) -> Result<u16> {
        self.check_remaining(2)?;
        let bytes = &self.buffer[self.position..self.position + 2];
        self.position += 2;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// Read u32 (little-endian)
    #[inline]
    pub fn read_u32(&mut self) -> Result<u32> {
        self.check_remaining(4)?;
        let bytes = &self.buffer[self.position..self.position + 4];
        self.position += 4;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Read i64 (little-endian)
    #[inline]
    pub fn read_i64(&mut self) -> Result<i64> {
        self.check_remaining(8)?;
        let bytes = &self.buffer[self.position..self.position + 8];
        self.position += 8;
        Ok(i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    /// Read f64 (little-endian)
    #[inline]
    pub fn read_f64(&mut self) -> Result<f64> {
        self.check_remaining(8)?;
        let bytes = &self.buffer[self.position..self.position + 8];
        self.position += 8;
        Ok(f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    /// Read bool
    #[inline]
    pub fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read_u8()? != 0)
    }

    /// Read string (length-prefixed)
    #[inline]
    pub fn read_string(&mut self) -> Result<String> {
        let len = self.read_u32()? as usize;
        self.check_remaining(len)?;
        let bytes = &self.buffer[self.position..self.position + len];
        self.position += len;
        String::from_utf8(bytes.to_vec())
            .map_err(|e| DxBinaryError::IoError(format!("Invalid UTF-8: {}", e)))
    }

    /// Read bytes (length-prefixed)
    #[inline]
    pub fn read_bytes(&mut self) -> Result<Vec<u8>> {
        let len = self.read_u32()? as usize;
        self.check_remaining(len)?;
        let bytes = self.buffer[self.position..self.position + len].to_vec();
        self.position += len;
        Ok(bytes)
    }

    /// Read array of strings
    #[inline]
    pub fn read_string_array(&mut self) -> Result<Vec<String>> {
        let count = self.read_u32()? as usize;
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(self.read_string()?);
        }
        Ok(result)
    }

    /// Read array of u8
    #[inline]
    pub fn read_u8_array(&mut self) -> Result<Vec<u8>> {
        self.read_bytes()
    }

    /// Read array of u32
    #[inline]
    pub fn read_u32_array(&mut self) -> Result<Vec<u32>> {
        let count = self.read_u32()? as usize;
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(self.read_u32()?);
        }
        Ok(result)
    }

    /// Get remaining bytes
    pub fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.position)
    }

    /// Get current position
    pub fn position(&self) -> usize {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u8_roundtrip() {
        let mut encoder = BinaryEncoder::new(16);
        encoder.write_u8(42);
        encoder.write_u8(255);
        let bytes = encoder.finish();

        let mut decoder = BinaryDecoder::new(&bytes);
        assert_eq!(decoder.read_u8().unwrap(), 42);
        assert_eq!(decoder.read_u8().unwrap(), 255);
    }

    #[test]
    fn test_u16_roundtrip() {
        let mut encoder = BinaryEncoder::new(16);
        encoder.write_u16(12345);
        encoder.write_u16(65535);
        let bytes = encoder.finish();

        let mut decoder = BinaryDecoder::new(&bytes);
        assert_eq!(decoder.read_u16().unwrap(), 12345);
        assert_eq!(decoder.read_u16().unwrap(), 65535);
    }

    #[test]
    fn test_u32_roundtrip() {
        let mut encoder = BinaryEncoder::new(16);
        encoder.write_u32(0x12345678);
        let bytes = encoder.finish();

        let mut decoder = BinaryDecoder::new(&bytes);
        assert_eq!(decoder.read_u32().unwrap(), 0x12345678);
    }

    #[test]
    fn test_string_roundtrip() {
        let mut encoder = BinaryEncoder::new(64);
        encoder.write_string("Hello, World!");
        encoder.write_string("");
        encoder.write_string("🦀 Rust");
        let bytes = encoder.finish();

        let mut decoder = BinaryDecoder::new(&bytes);
        assert_eq!(decoder.read_string().unwrap(), "Hello, World!");
        assert_eq!(decoder.read_string().unwrap(), "");
        assert_eq!(decoder.read_string().unwrap(), "🦀 Rust");
    }

    #[test]
    fn test_string_array_roundtrip() {
        let mut encoder = BinaryEncoder::new(128);
        let strings = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        encoder.write_string_array(&strings);
        let bytes = encoder.finish();

        let mut decoder = BinaryDecoder::new(&bytes);
        let decoded = decoder.read_string_array().unwrap();
        assert_eq!(decoded, strings);
    }

    #[test]
    fn test_bool_roundtrip() {
        let mut encoder = BinaryEncoder::new(16);
        encoder.write_bool(true);
        encoder.write_bool(false);
        let bytes = encoder.finish();

        let mut decoder = BinaryDecoder::new(&bytes);
        assert!(decoder.read_bool().unwrap());
        assert!(!decoder.read_bool().unwrap());
    }

    #[test]
    fn test_f64_roundtrip() {
        let mut encoder = BinaryEncoder::new(16);
        encoder.write_f64(3.14159265359);
        encoder.write_f64(-273.15);
        let bytes = encoder.finish();

        let mut decoder = BinaryDecoder::new(&bytes);
        assert!((decoder.read_f64().unwrap() - 3.14159265359).abs() < 1e-10);
        assert!((decoder.read_f64().unwrap() - (-273.15)).abs() < 1e-10);
    }

    #[test]
    fn test_buffer_underflow() {
        let bytes = [0u8; 2];
        let mut decoder = BinaryDecoder::new(&bytes);
        assert!(decoder.read_u32().is_err());
    }
}
