//! GPU compute shader support

use crate::error::{DxError, DxResult};

pub struct GpuDevice {
    available: bool,
}

impl GpuDevice {
    pub fn new() -> Self {
        Self { available: false }
    }

    pub fn is_available(&self) -> bool {
        self.available
    }

    pub fn create_buffer(&self, size: usize) -> DxResult<GpuBuffer> {
        if !self.available {
            return Err(DxError::RuntimeError("GPU not available".to_string()));
        }
        Ok(GpuBuffer {
            data: vec![0; size],
            size,
        })
    }
}

impl Default for GpuDevice {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GpuBuffer {
    data: Vec<u8>,
    size: usize,
}

impl GpuBuffer {
    pub fn write(&mut self, offset: usize, data: &[u8]) -> DxResult<()> {
        if offset + data.len() > self.size {
            return Err(DxError::RuntimeError("Buffer write out of bounds".to_string()));
        }
        self.data[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }

    pub fn read(&self, offset: usize, len: usize) -> DxResult<&[u8]> {
        self.data
            .get(offset..offset + len)
            .ok_or_else(|| DxError::RuntimeError("Buffer read out of bounds".to_string()))
    }
}

pub struct ComputeShader {
    /// Shader source code - reserved for GPU compute implementation
    #[allow(dead_code)]
    source: String,
}

impl ComputeShader {
    pub fn new(source: String) -> Self {
        Self { source }
    }

    pub fn dispatch(&self, _workgroups: [u32; 3]) -> DxResult<()> {
        Ok(())
    }
}
