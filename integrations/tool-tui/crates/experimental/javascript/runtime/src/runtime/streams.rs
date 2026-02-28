//! Stream API (Readable, Writable, Transform)

use crate::error::DxResult;
use std::collections::VecDeque;

pub trait Readable {
    fn read(&mut self, size: Option<usize>) -> DxResult<Vec<u8>>;
    fn on_data<F>(&mut self, callback: F)
    where
        F: FnMut(&[u8]) + 'static;
    fn on_end<F>(&mut self, callback: F)
    where
        F: FnOnce() + 'static;
    fn pipe<W: Writable>(&mut self, dest: &mut W) -> DxResult<()>;
}

pub trait Writable {
    fn write(&mut self, chunk: &[u8]) -> DxResult<()>;
    fn end(&mut self) -> DxResult<()>;
    fn on_finish<F>(&mut self, callback: F)
    where
        F: FnOnce() + 'static;
}

pub struct ReadableStream {
    buffer: VecDeque<u8>,
    ended: bool,
}

impl Default for ReadableStream {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadableStream {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
            ended: false,
        }
    }

    pub fn push(&mut self, data: &[u8]) {
        self.buffer.extend(data);
    }

    pub fn push_end(&mut self) {
        self.ended = true;
    }
}

impl Readable for ReadableStream {
    fn read(&mut self, size: Option<usize>) -> DxResult<Vec<u8>> {
        let len = size.unwrap_or(self.buffer.len()).min(self.buffer.len());
        Ok(self.buffer.drain(..len).collect())
    }

    fn on_data<F>(&mut self, mut callback: F)
    where
        F: FnMut(&[u8]) + 'static,
    {
        while !self.buffer.is_empty() {
            let chunk: Vec<u8> = self.buffer.drain(..).collect();
            callback(&chunk);
        }
    }

    fn on_end<F>(&mut self, callback: F)
    where
        F: FnOnce() + 'static,
    {
        if self.ended {
            callback();
        }
    }

    fn pipe<W: Writable>(&mut self, dest: &mut W) -> DxResult<()> {
        while !self.buffer.is_empty() {
            let chunk: Vec<u8> = self.buffer.drain(..).collect();
            dest.write(&chunk)?;
        }
        if self.ended {
            dest.end()?;
        }
        Ok(())
    }
}

pub struct WritableStream {
    buffer: Vec<u8>,
}

impl Default for WritableStream {
    fn default() -> Self {
        Self::new()
    }
}

impl WritableStream {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }
    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
    }
}

impl Writable for WritableStream {
    fn write(&mut self, chunk: &[u8]) -> DxResult<()> {
        self.buffer.extend_from_slice(chunk);
        Ok(())
    }

    fn end(&mut self) -> DxResult<()> {
        Ok(())
    }
    fn on_finish<F>(&mut self, _callback: F)
    where
        F: FnOnce() + 'static,
    {
    }
}

pub struct Transform<F> {
    transform_fn: F,
}

impl<F> Transform<F>
where
    F: FnMut(&[u8]) -> Vec<u8>,
{
    pub fn new(transform_fn: F) -> Self {
        Self { transform_fn }
    }

    pub fn process(&mut self, input: &[u8]) -> Vec<u8> {
        (self.transform_fn)(input)
    }
}
