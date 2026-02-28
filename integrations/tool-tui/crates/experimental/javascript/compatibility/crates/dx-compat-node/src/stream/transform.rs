//! Transform stream implementation for Node.js compatibility.
//!
//! A Transform stream is a Duplex stream where the output is computed
//! from the input. This is commonly used for data transformation like
//! compression, encryption, or parsing.

use bytes::Bytes;
use futures::Stream;
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use super::{Readable, StreamError, Writable};

/// Transform function type that converts input chunks to output chunks.
pub type TransformFn = Box<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>;

/// Internal state for transform stream.
#[derive(Debug)]
struct TransformState {
    /// Input buffer (data written to the stream)
    input_buffer: VecDeque<Bytes>,
    /// Output buffer (transformed data ready to read)
    output_buffer: VecDeque<Bytes>,
    /// Whether the writable side is closed
    write_closed: bool,
    /// Whether the readable side is closed
    read_closed: bool,
    /// Whether the readable side is paused
    read_paused: bool,
    /// Waker for the readable side
    read_waker: Option<Waker>,
    /// High water mark for backpressure
    high_water_mark: usize,
    /// Current output buffer size
    buffer_size: usize,
}

impl TransformState {
    fn new(high_water_mark: usize) -> Self {
        Self {
            input_buffer: VecDeque::new(),
            output_buffer: VecDeque::new(),
            write_closed: false,
            read_closed: false,
            read_paused: false,
            read_waker: None,
            high_water_mark,
            buffer_size: 0,
        }
    }
}

/// A transform stream that applies a transformation function to data.
///
/// Data written to the stream is transformed and made available for reading.
pub struct TransformStream {
    state: Arc<Mutex<TransformState>>,
    transform: Arc<TransformFn>,
}

impl TransformStream {
    /// Create a new transform stream with the given transformation function.
    pub fn new<F>(transform: F) -> Self
    where
        F: Fn(&[u8]) -> Vec<u8> + Send + Sync + 'static,
    {
        Self::with_high_water_mark(transform, 16 * 1024)
    }

    /// Create a new transform stream with custom high water mark.
    pub fn with_high_water_mark<F>(transform: F, high_water_mark: usize) -> Self
    where
        F: Fn(&[u8]) -> Vec<u8> + Send + Sync + 'static,
    {
        Self {
            state: Arc::new(Mutex::new(TransformState::new(high_water_mark))),
            transform: Arc::new(Box::new(transform)),
        }
    }

    /// Create a passthrough transform (identity function).
    pub fn passthrough() -> Self {
        Self::new(|data| data.to_vec())
    }

    /// Create a transform that converts to uppercase.
    pub fn to_uppercase() -> Self {
        Self::new(|data| {
            String::from_utf8_lossy(data).to_uppercase().into_bytes()
        })
    }

    /// Create a transform that converts to lowercase.
    pub fn to_lowercase() -> Self {
        Self::new(|data| {
            String::from_utf8_lossy(data).to_lowercase().into_bytes()
        })
    }

    /// Process pending input through the transform function.
    fn process_pending(&self) {
        let mut state = self.state.lock().unwrap();
        
        while let Some(chunk) = state.input_buffer.pop_front() {
            let transformed = (self.transform)(&chunk);
            if !transformed.is_empty() {
                state.buffer_size += transformed.len();
                state.output_buffer.push_back(Bytes::from(transformed));
            }
        }
        
        // Wake up readers if we have data
        if !state.output_buffer.is_empty() {
            if let Some(waker) = state.read_waker.take() {
                waker.wake();
            }
        }
    }

    /// Check if the stream is readable.
    pub fn is_readable(&self) -> bool {
        let state = self.state.lock().unwrap();
        !state.read_closed || !state.output_buffer.is_empty()
    }

    /// Check if the stream is writable.
    pub fn is_writable(&self) -> bool {
        let state = self.state.lock().unwrap();
        !state.write_closed
    }

    /// Get the current output buffer size.
    pub fn buffer_size(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.buffer_size
    }

    /// Destroy the stream.
    pub fn destroy(&self) {
        let mut state = self.state.lock().unwrap();
        state.read_closed = true;
        state.write_closed = true;
        state.input_buffer.clear();
        state.output_buffer.clear();
        state.buffer_size = 0;
        
        if let Some(waker) = state.read_waker.take() {
            waker.wake();
        }
    }

    /// Flush any remaining data through the transform.
    pub fn flush(&self) -> Result<(), StreamError> {
        self.process_pending();
        Ok(())
    }
}

impl Clone for TransformStream {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            transform: self.transform.clone(),
        }
    }
}

// Implement Stream trait for TransformStream
impl Stream for TransformStream {
    type Item = Result<Bytes, StreamError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // First, process any pending input
        self.process_pending();
        
        let mut state = self.state.lock().unwrap();
        
        if state.read_paused {
            state.read_waker = Some(cx.waker().clone());
            return Poll::Pending;
        }
        
        if let Some(chunk) = state.output_buffer.pop_front() {
            state.buffer_size = state.buffer_size.saturating_sub(chunk.len());
            Poll::Ready(Some(Ok(chunk)))
        } else if state.write_closed && state.input_buffer.is_empty() {
            state.read_closed = true;
            Poll::Ready(None)
        } else {
            state.read_waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

// Implement Readable trait for TransformStream
impl Readable for TransformStream {
    fn pause(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.read_paused = true;
    }

    fn resume(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.read_paused = false;
        if let Some(waker) = state.read_waker.take() {
            waker.wake();
        }
    }

    fn is_paused(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.read_paused
    }
}

// Implement Writable trait for TransformStream
impl Writable for TransformStream {
    fn write(&mut self, chunk: Bytes) -> Result<bool, StreamError> {
        {
            let mut state = self.state.lock().unwrap();
            if state.write_closed {
                return Err(StreamError::Closed);
            }
            state.input_buffer.push_back(chunk);
        }
        
        // Process the input immediately
        self.process_pending();
        
        let state = self.state.lock().unwrap();
        Ok(state.buffer_size < state.high_water_mark)
    }

    fn end(&mut self) -> Result<(), StreamError> {
        // Process any remaining input
        self.process_pending();
        
        let mut state = self.state.lock().unwrap();
        state.write_closed = true;
        
        // Wake up readers to signal EOF
        if let Some(waker) = state.read_waker.take() {
            waker.wake();
        }
        
        Ok(())
    }
}

/// A builder for creating transform streams with custom options.
pub struct TransformStreamBuilder {
    transform: Option<TransformFn>,
    high_water_mark: usize,
    flush_fn: Option<Box<dyn Fn() -> Vec<u8> + Send + Sync>>,
}

impl TransformStreamBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            transform: None,
            high_water_mark: 16 * 1024,
            flush_fn: None,
        }
    }

    /// Set the transform function.
    pub fn transform<F>(mut self, f: F) -> Self
    where
        F: Fn(&[u8]) -> Vec<u8> + Send + Sync + 'static,
    {
        self.transform = Some(Box::new(f));
        self
    }

    /// Set the high water mark.
    pub fn high_water_mark(mut self, hwm: usize) -> Self {
        self.high_water_mark = hwm;
        self
    }

    /// Set a flush function called when the stream ends.
    pub fn flush<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Vec<u8> + Send + Sync + 'static,
    {
        self.flush_fn = Some(Box::new(f));
        self
    }

    /// Build the transform stream.
    pub fn build(self) -> TransformStream {
        let transform = self.transform.unwrap_or_else(|| Box::new(|data: &[u8]| data.to_vec()));
        TransformStream {
            state: Arc::new(Mutex::new(TransformState::new(self.high_water_mark))),
            transform: Arc::new(transform),
        }
    }
}

impl Default for TransformStreamBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_transform_passthrough() {
        let mut transform = TransformStream::passthrough();
        
        transform.write(Bytes::from("hello")).unwrap();
        transform.write(Bytes::from(" world")).unwrap();
        transform.end().unwrap();
        
        let chunk1 = transform.next().await.unwrap().unwrap();
        assert_eq!(&chunk1[..], b"hello");
        
        let chunk2 = transform.next().await.unwrap().unwrap();
        assert_eq!(&chunk2[..], b" world");
        
        assert!(transform.next().await.is_none());
    }

    #[tokio::test]
    async fn test_transform_uppercase() {
        let mut transform = TransformStream::to_uppercase();
        
        transform.write(Bytes::from("hello")).unwrap();
        transform.end().unwrap();
        
        let chunk = transform.next().await.unwrap().unwrap();
        assert_eq!(&chunk[..], b"HELLO");
    }

    #[tokio::test]
    async fn test_transform_custom() {
        let mut transform = TransformStream::new(|data| {
            data.iter().map(|b| b.wrapping_add(1)).collect()
        });
        
        transform.write(Bytes::from(vec![0, 1, 2])).unwrap();
        transform.end().unwrap();
        
        let chunk = transform.next().await.unwrap().unwrap();
        assert_eq!(&chunk[..], &[1, 2, 3]);
    }

    #[test]
    fn test_transform_builder() {
        let transform = TransformStreamBuilder::new()
            .transform(|data| data.to_vec())
            .high_water_mark(1024)
            .build();
        
        assert!(transform.is_writable());
        assert!(!transform.is_readable()); // No data yet
    }

    #[test]
    fn test_transform_closed_write() {
        let mut transform = TransformStream::passthrough();
        transform.end().unwrap();
        
        let result = transform.write(Bytes::from("test"));
        assert!(result.is_err());
    }
}
