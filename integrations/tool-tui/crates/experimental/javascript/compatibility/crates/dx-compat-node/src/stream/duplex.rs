//! Duplex stream implementation for Node.js compatibility.
//!
//! A Duplex stream is both readable and writable, allowing bidirectional
//! data flow. This is commonly used for network sockets and other
//! bidirectional communication channels.

use bytes::Bytes;
use futures::Stream;
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use super::{Readable, StreamError, Writable};

/// Internal state shared between readable and writable halves.
#[derive(Debug)]
struct DuplexState {
    /// Read buffer (data available to read)
    read_buffer: VecDeque<Bytes>,
    /// Write buffer (data written but not consumed)
    write_buffer: VecDeque<Bytes>,
    /// Whether the readable side is closed
    read_closed: bool,
    /// Whether the writable side is closed
    write_closed: bool,
    /// Whether the readable side is paused
    read_paused: bool,
    /// Waker for the readable side
    read_waker: Option<Waker>,
    /// Waker for the writable side
    write_waker: Option<Waker>,
    /// High water mark for backpressure
    high_water_mark: usize,
    /// Current buffer size
    buffer_size: usize,
}

impl DuplexState {
    fn new(high_water_mark: usize) -> Self {
        Self {
            read_buffer: VecDeque::new(),
            write_buffer: VecDeque::new(),
            read_closed: false,
            write_closed: false,
            read_paused: false,
            read_waker: None,
            write_waker: None,
            high_water_mark,
            buffer_size: 0,
        }
    }
}

/// A duplex stream that supports both reading and writing.
///
/// This implements Node.js-style Duplex stream semantics with
/// backpressure support.
#[derive(Debug, Clone)]
pub struct DuplexStream {
    state: Arc<Mutex<DuplexState>>,
}

impl DuplexStream {
    /// Create a new duplex stream with default high water mark (16KB).
    pub fn new() -> Self {
        Self::with_high_water_mark(16 * 1024)
    }

    /// Create a new duplex stream with custom high water mark.
    pub fn with_high_water_mark(high_water_mark: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(DuplexState::new(high_water_mark))),
        }
    }

    /// Push data to the read buffer (for the readable side to consume).
    pub fn push(&self, chunk: Bytes) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.read_closed {
            return false;
        }
        state.buffer_size += chunk.len();
        state.read_buffer.push_back(chunk);
        
        // Wake up any pending readers
        if let Some(waker) = state.read_waker.take() {
            waker.wake();
        }
        
        // Return false if we've exceeded high water mark (backpressure)
        state.buffer_size < state.high_water_mark
    }

    /// Push EOF to signal end of readable data.
    pub fn push_eof(&self) {
        let mut state = self.state.lock().unwrap();
        state.read_closed = true;
        if let Some(waker) = state.read_waker.take() {
            waker.wake();
        }
    }

    /// Check if the stream is readable.
    pub fn is_readable(&self) -> bool {
        let state = self.state.lock().unwrap();
        !state.read_closed || !state.read_buffer.is_empty()
    }

    /// Check if the stream is writable.
    pub fn is_writable(&self) -> bool {
        let state = self.state.lock().unwrap();
        !state.write_closed
    }

    /// Get the current buffer size.
    pub fn buffer_size(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.buffer_size
    }

    /// Destroy the stream, closing both sides.
    pub fn destroy(&self) {
        let mut state = self.state.lock().unwrap();
        state.read_closed = true;
        state.write_closed = true;
        state.read_buffer.clear();
        state.write_buffer.clear();
        state.buffer_size = 0;
        
        if let Some(waker) = state.read_waker.take() {
            waker.wake();
        }
        if let Some(waker) = state.write_waker.take() {
            waker.wake();
        }
    }

    /// Split into readable and writable halves.
    pub fn split(self) -> (ReadableHalf, WritableHalf) {
        (
            ReadableHalf { state: self.state.clone() },
            WritableHalf { state: self.state },
        )
    }
}

impl Default for DuplexStream {
    fn default() -> Self {
        Self::new()
    }
}

/// The readable half of a duplex stream.
#[derive(Debug, Clone)]
pub struct ReadableHalf {
    state: Arc<Mutex<DuplexState>>,
}

impl ReadableHalf {
    /// Read a chunk from the stream.
    pub fn read(&self) -> Option<Bytes> {
        let mut state = self.state.lock().unwrap();
        if let Some(chunk) = state.read_buffer.pop_front() {
            state.buffer_size = state.buffer_size.saturating_sub(chunk.len());
            
            // Wake up writers if we were at high water mark
            if let Some(waker) = state.write_waker.take() {
                waker.wake();
            }
            Some(chunk)
        } else {
            None
        }
    }
}

/// The writable half of a duplex stream.
#[derive(Debug, Clone)]
pub struct WritableHalf {
    state: Arc<Mutex<DuplexState>>,
}

impl WritableHalf {
    /// Write a chunk to the stream.
    pub fn write(&self, chunk: Bytes) -> Result<bool, StreamError> {
        let mut state = self.state.lock().unwrap();
        if state.write_closed {
            return Err(StreamError::Closed);
        }
        state.write_buffer.push_back(chunk);
        Ok(true)
    }

    /// End the writable side.
    pub fn end(&self) -> Result<(), StreamError> {
        let mut state = self.state.lock().unwrap();
        state.write_closed = true;
        Ok(())
    }
}

// Implement Stream trait for DuplexStream
impl Stream for DuplexStream {
    type Item = Result<Bytes, StreamError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut state = self.state.lock().unwrap();
        
        if state.read_paused {
            state.read_waker = Some(cx.waker().clone());
            return Poll::Pending;
        }
        
        if let Some(chunk) = state.read_buffer.pop_front() {
            state.buffer_size = state.buffer_size.saturating_sub(chunk.len());
            Poll::Ready(Some(Ok(chunk)))
        } else if state.read_closed {
            Poll::Ready(None)
        } else {
            state.read_waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

// Implement Readable trait for DuplexStream
impl Readable for DuplexStream {
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

// Implement Writable trait for DuplexStream
impl Writable for DuplexStream {
    fn write(&mut self, chunk: Bytes) -> Result<bool, StreamError> {
        let mut state = self.state.lock().unwrap();
        if state.write_closed {
            return Err(StreamError::Closed);
        }
        state.write_buffer.push_back(chunk);
        Ok(state.buffer_size < state.high_water_mark)
    }

    fn end(&mut self) -> Result<(), StreamError> {
        let mut state = self.state.lock().unwrap();
        state.write_closed = true;
        Ok(())
    }
}

// Implement AsyncRead for DuplexStream
impl AsyncRead for DuplexStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let mut state = self.state.lock().unwrap();
        
        if let Some(chunk) = state.read_buffer.pop_front() {
            let len = std::cmp::min(chunk.len(), buf.remaining());
            buf.put_slice(&chunk[..len]);
            
            // If we didn't consume the whole chunk, put the rest back
            if len < chunk.len() {
                state.read_buffer.push_front(chunk.slice(len..));
            } else {
                state.buffer_size = state.buffer_size.saturating_sub(chunk.len());
            }
            
            Poll::Ready(Ok(()))
        } else if state.read_closed {
            Poll::Ready(Ok(())) // EOF
        } else {
            state.read_waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

// Implement AsyncWrite for DuplexStream
impl AsyncWrite for DuplexStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let mut state = self.state.lock().unwrap();
        
        if state.write_closed {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Stream closed",
            )));
        }
        
        let chunk = Bytes::copy_from_slice(buf);
        state.write_buffer.push_back(chunk);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let mut state = self.state.lock().unwrap();
        state.write_closed = true;
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_duplex_basic_read_write() {
        let duplex = DuplexStream::new();
        
        // Push some data
        assert!(duplex.push(Bytes::from("hello")));
        assert!(duplex.push(Bytes::from(" world")));
        duplex.push_eof();
        
        // Read it back
        let mut duplex = duplex;
        let chunk1 = duplex.next().await.unwrap().unwrap();
        assert_eq!(&chunk1[..], b"hello");
        
        let chunk2 = duplex.next().await.unwrap().unwrap();
        assert_eq!(&chunk2[..], b" world");
        
        // Should be EOF
        assert!(duplex.next().await.is_none());
    }

    #[test]
    fn test_duplex_split() {
        let duplex = DuplexStream::new();
        let (readable, writable) = duplex.split();
        
        // Write to writable half
        writable.write(Bytes::from("test")).unwrap();
        writable.end().unwrap();
        
        // The write buffer should have data
        // (In a real implementation, this would be connected)
    }

    #[test]
    fn test_duplex_backpressure() {
        let duplex = DuplexStream::with_high_water_mark(10);
        
        // Push data up to high water mark
        assert!(duplex.push(Bytes::from("12345"))); // 5 bytes, under limit
        assert!(!duplex.push(Bytes::from("67890"))); // 10 bytes, at limit
        assert!(!duplex.push(Bytes::from("abc"))); // 13 bytes, over limit
    }

    #[test]
    fn test_duplex_destroy() {
        let duplex = DuplexStream::new();
        duplex.push(Bytes::from("data"));
        
        assert!(duplex.is_readable());
        assert!(duplex.is_writable());
        
        duplex.destroy();
        
        assert!(!duplex.is_readable());
        assert!(!duplex.is_writable());
    }
}
