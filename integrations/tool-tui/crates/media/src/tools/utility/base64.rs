//! Base64 encoding and decoding.
//!
//! Encode and decode data using Base64.

use crate::error::{DxError, Result};
use crate::tools::ToolOutput;
use std::path::Path;
use std::process::Command;

/// Encode string to Base64.
///
/// # Example
/// ```no_run
/// use dx_media::tools::utility::base64;
///
/// let result = base64::encode_string("Hello, World!").unwrap();
/// println!("{}", result.message); // SGVsbG8sIFdvcmxkIQ==
/// ```
pub fn encode_string(input: &str) -> Result<ToolOutput> {
    // Try base64 command
    if let Ok(result) = encode_with_command(input.as_bytes()) {
        return Ok(result);
    }

    // Manual implementation
    let encoded = base64_encode_impl(input.as_bytes());
    Ok(ToolOutput::success(encoded.clone()).with_metadata("encoded", encoded))
}

/// Decode Base64 string.
///
/// # Example
/// ```no_run
/// use dx_media::tools::utility::base64;
///
/// let result = base64::decode_string("SGVsbG8sIFdvcmxkIQ==").unwrap();
/// println!("{}", result.message); // Hello, World!
/// ```
pub fn decode_string(input: &str) -> Result<ToolOutput> {
    // Try base64 command
    if let Ok(result) = decode_with_command(input) {
        return Ok(result);
    }

    // Manual implementation
    let decoded = base64_decode_impl(input)?;
    let text = String::from_utf8_lossy(&decoded).to_string();

    Ok(ToolOutput::success(text.clone()).with_metadata("decoded", text))
}

/// Encode file to Base64.
pub fn encode_file<P: AsRef<Path>>(input: P) -> Result<ToolOutput> {
    let input_path = input.as_ref();

    if !input_path.exists() {
        return Err(DxError::FileIo {
            path: input_path.to_path_buf(),
            message: "File not found".to_string(),
            source: None,
        });
    }

    let data = std::fs::read(input_path).map_err(|e| DxError::FileIo {
        path: input_path.to_path_buf(),
        message: format!("Failed to read file: {}", e),
        source: None,
    })?;

    let encoded = base64_encode_impl(&data);

    Ok(ToolOutput::success(encoded.clone())
        .with_metadata("encoded_length", encoded.len().to_string())
        .with_metadata("original_size", data.len().to_string()))
}

/// Decode Base64 to file.
pub fn decode_file<P: AsRef<Path>>(input: P, output: P) -> Result<ToolOutput> {
    let input_path = input.as_ref();
    let output_path = output.as_ref();

    let encoded = std::fs::read_to_string(input_path).map_err(|e| DxError::FileIo {
        path: input_path.to_path_buf(),
        message: format!("Failed to read file: {}", e),
        source: None,
    })?;

    let decoded = base64_decode_impl(encoded.trim())?;

    std::fs::write(output_path, &decoded).map_err(|e| DxError::FileIo {
        path: output_path.to_path_buf(),
        message: format!("Failed to write file: {}", e),
        source: None,
    })?;

    Ok(ToolOutput::success_with_path(
        format!("Decoded {} bytes", decoded.len()),
        output_path,
    ))
}

/// Encode using base64 command.
fn encode_with_command(data: &[u8]) -> Result<ToolOutput> {
    let mut cmd = Command::new("base64");
    cmd.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| DxError::Config {
        message: format!("Failed to run base64: {}", e),
        source: None,
    })?;

    use std::io::Write;
    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(data).map_err(|e| DxError::Config {
            message: format!("Failed to write to stdin: {}", e),
            source: None,
        })?;
    }

    let output = child.wait_with_output().map_err(|e| DxError::Config {
        message: format!("Failed to wait for base64: {}", e),
        source: None,
    })?;

    if !output.status.success() {
        return Err(DxError::Config {
            message: "base64 encoding failed".to_string(),
            source: None,
        });
    }

    let encoded = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(ToolOutput::success(encoded.clone()).with_metadata("encoded", encoded))
}

/// Decode using base64 command.
fn decode_with_command(input: &str) -> Result<ToolOutput> {
    let mut cmd = Command::new("base64");
    cmd.arg("-d")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| DxError::Config {
        message: format!("Failed to run base64: {}", e),
        source: None,
    })?;

    use std::io::Write;
    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(input.as_bytes()).map_err(|e| DxError::Config {
            message: format!("Failed to write to stdin: {}", e),
            source: None,
        })?;
    }

    let output = child.wait_with_output().map_err(|e| DxError::Config {
        message: format!("Failed to wait for base64: {}", e),
        source: None,
    })?;

    if !output.status.success() {
        return Err(DxError::Config {
            message: "base64 decoding failed".to_string(),
            source: None,
        });
    }

    let decoded = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(ToolOutput::success(decoded.clone()).with_metadata("decoded", decoded))
}

/// Base64 encoding alphabet.
const BASE64_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 encode implementation.
fn base64_encode_impl(data: &[u8]) -> String {
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(BASE64_ALPHABET[b0 >> 2] as char);
        result.push(BASE64_ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(BASE64_ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(BASE64_ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}

/// Base64 decode implementation.
fn base64_decode_impl(input: &str) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let input = input.replace(['\n', '\r', ' '], "");

    let decode_char = |c: char| -> Result<u8> {
        match c {
            'A'..='Z' => Ok((c as u8) - b'A'),
            'a'..='z' => Ok((c as u8) - b'a' + 26),
            '0'..='9' => Ok((c as u8) - b'0' + 52),
            '+' => Ok(62),
            '/' => Ok(63),
            '=' => Ok(0),
            _ => Err(DxError::Config {
                message: format!("Invalid base64 character: {}", c),
                source: None,
            }),
        }
    };

    let chars: Vec<char> = input.chars().collect();

    for chunk in chars.chunks(4) {
        if chunk.len() < 4 {
            break;
        }

        let b0 = decode_char(chunk[0])?;
        let b1 = decode_char(chunk[1])?;
        let b2 = decode_char(chunk[2])?;
        let b3 = decode_char(chunk[3])?;

        result.push((b0 << 2) | (b1 >> 4));

        if chunk[2] != '=' {
            result.push((b1 << 4) | (b2 >> 2));
        }

        if chunk[3] != '=' {
            result.push((b2 << 6) | b3);
        }
    }

    Ok(result)
}

/// URL-safe Base64 encode.
pub fn encode_url_safe(input: &str) -> Result<ToolOutput> {
    let encoded = base64_encode_impl(input.as_bytes())
        .replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_string();

    Ok(ToolOutput::success(encoded.clone())
        .with_metadata("encoded", encoded)
        .with_metadata("variant", "url-safe".to_string()))
}

/// URL-safe Base64 decode.
pub fn decode_url_safe(input: &str) -> Result<ToolOutput> {
    let padded = {
        let mut s = input.replace('-', "+").replace('_', "/");
        while s.len() % 4 != 0 {
            s.push('=');
        }
        s
    };

    let decoded = base64_decode_impl(&padded)?;
    let text = String::from_utf8_lossy(&decoded).to_string();

    Ok(ToolOutput::success(text.clone()).with_metadata("decoded", text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let input = "Hello, World!";
        let encoded = base64_encode_impl(input.as_bytes());
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");

        let decoded = base64_decode_impl(&encoded).unwrap();
        assert_eq!(String::from_utf8_lossy(&decoded), input);
    }
}
