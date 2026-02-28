//! Tool handler trait and types for zero-copy execution.

use crate::protocol::ToolSchema;
use crate::DCPError;

/// Shared arguments for zero-copy access
pub struct SharedArgs<'a> {
    /// Raw bytes of arguments
    data: &'a [u8],
    /// Argument layout bitfield
    layout: u64,
}

impl<'a> SharedArgs<'a> {
    /// Create new shared args from bytes and layout
    pub fn new(data: &'a [u8], layout: u64) -> Self {
        Self { data, layout }
    }

    /// Get the raw data slice
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Get the argument layout
    pub fn layout(&self) -> u64 {
        self.layout
    }

    /// Read a string at the given offset with bounds checking
    pub fn read_str_at(&self, offset: usize, len: usize) -> Result<&str, DCPError> {
        if offset + len > self.data.len() {
            return Err(DCPError::OutOfBounds);
        }
        std::str::from_utf8(&self.data[offset..offset + len])
            .map_err(|_| DCPError::ValidationFailed)
    }

    /// Read an i32 at the given offset with bounds checking
    pub fn read_i32_at(&self, offset: usize) -> Result<i32, DCPError> {
        if offset + 4 > self.data.len() {
            return Err(DCPError::OutOfBounds);
        }
        let bytes: [u8; 4] =
            self.data[offset..offset + 4].try_into().map_err(|_| DCPError::OutOfBounds)?;
        Ok(i32::from_le_bytes(bytes))
    }

    /// Read an i64 at the given offset with bounds checking
    pub fn read_i64_at(&self, offset: usize) -> Result<i64, DCPError> {
        if offset + 8 > self.data.len() {
            return Err(DCPError::OutOfBounds);
        }
        let bytes: [u8; 8] =
            self.data[offset..offset + 8].try_into().map_err(|_| DCPError::OutOfBounds)?;
        Ok(i64::from_le_bytes(bytes))
    }

    /// Read a u32 at the given offset with bounds checking
    pub fn read_u32_at(&self, offset: usize) -> Result<u32, DCPError> {
        if offset + 4 > self.data.len() {
            return Err(DCPError::OutOfBounds);
        }
        let bytes: [u8; 4] =
            self.data[offset..offset + 4].try_into().map_err(|_| DCPError::OutOfBounds)?;
        Ok(u32::from_le_bytes(bytes))
    }

    /// Read bytes at the given offset with bounds checking
    pub fn read_bytes_at(&self, offset: usize, len: usize) -> Result<&[u8], DCPError> {
        if offset + len > self.data.len() {
            return Err(DCPError::OutOfBounds);
        }
        Ok(&self.data[offset..offset + len])
    }

    /// Read a bool at the given offset
    pub fn read_bool_at(&self, offset: usize) -> Result<bool, DCPError> {
        if offset >= self.data.len() {
            return Err(DCPError::OutOfBounds);
        }
        Ok(self.data[offset] != 0)
    }

    /// Read an f64 at the given offset with bounds checking
    pub fn read_f64_at(&self, offset: usize) -> Result<f64, DCPError> {
        if offset + 8 > self.data.len() {
            return Err(DCPError::OutOfBounds);
        }
        let bytes: [u8; 8] =
            self.data[offset..offset + 8].try_into().map_err(|_| DCPError::OutOfBounds)?;
        Ok(f64::from_le_bytes(bytes))
    }
}

/// Result of tool execution
#[derive(Debug, PartialEq)]
pub enum ToolResult {
    /// Success with binary payload
    Success(Vec<u8>),
    /// Success with no payload
    Empty,
    /// Error result
    Error(DCPError),
}

impl ToolResult {
    /// Create a success result with data
    pub fn success(data: Vec<u8>) -> Self {
        Self::Success(data)
    }

    /// Create an empty success result
    pub fn empty() -> Self {
        Self::Empty
    }

    /// Create an error result
    pub fn error(err: DCPError) -> Self {
        Self::Error(err)
    }

    /// Check if result is success
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_) | Self::Empty)
    }

    /// Get the payload if success
    pub fn payload(&self) -> Option<&[u8]> {
        match self {
            Self::Success(data) => Some(data),
            Self::Empty => Some(&[]),
            Self::Error(_) => None,
        }
    }
}

/// Tool handler trait for executing tools
pub trait ToolHandler: Send + Sync {
    /// Execute the tool with zero-copy arguments
    fn execute(&self, args: &SharedArgs) -> Result<ToolResult, DCPError>;

    /// Get tool schema for validation
    fn schema(&self) -> &ToolSchema;

    /// Get the tool ID
    fn tool_id(&self) -> u16 {
        self.schema().id
    }

    /// Get the tool name
    fn tool_name(&self) -> &'static str {
        self.schema().name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_args_read_i32() {
        let data = [0x01, 0x02, 0x03, 0x04];
        let args = SharedArgs::new(&data, 0);
        assert_eq!(args.read_i32_at(0).unwrap(), 0x04030201);
    }

    #[test]
    fn test_shared_args_read_str() {
        let data = b"hello world";
        let args = SharedArgs::new(data, 0);
        assert_eq!(args.read_str_at(0, 5).unwrap(), "hello");
        assert_eq!(args.read_str_at(6, 5).unwrap(), "world");
    }

    #[test]
    fn test_shared_args_bounds_check() {
        let data = [0x01, 0x02];
        let args = SharedArgs::new(&data, 0);
        assert_eq!(args.read_i32_at(0), Err(DCPError::OutOfBounds));
        assert_eq!(args.read_bytes_at(0, 10), Err(DCPError::OutOfBounds));
    }

    #[test]
    fn test_tool_result() {
        let success = ToolResult::success(vec![1, 2, 3]);
        assert!(success.is_success());
        assert_eq!(success.payload(), Some(&[1, 2, 3][..]));

        let empty = ToolResult::empty();
        assert!(empty.is_success());
        assert_eq!(empty.payload(), Some(&[][..]));

        let error = ToolResult::error(DCPError::ToolNotFound);
        assert!(!error.is_success());
        assert_eq!(error.payload(), None);
    }
}
