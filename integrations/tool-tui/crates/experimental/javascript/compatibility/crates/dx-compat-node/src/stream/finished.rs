//! Stream finished and backpressure utilities for Node.js compatibility.
//!
//! This module provides utilities for detecting when streams are finished
//! and handling backpressure in stream pipelines.

use bytes::Bytes;
use futures::{Stream, StreamExt};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use super::{StreamError, Writable};

/// State of a stream for finished detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    /// Stream is open and operational.
    Open,
    /// Stream has ended normally.
    Ended,
    /// Stream was closed.
    Closed,
    /// Stream encountered an error.
    Errored,
}

/// Options for the finished function.
#[derive(Debug, Clone, Default)]
pub struct FinishedOptions {
    /// Check for readable end.
    pub readable: bool,
    /// Check for writable end.
    pub writable: bool,
    /// Error to consider as "finished" (not an error).
    pub cleanup: bool,
}

impl FinishedOptions {
    /// Create new options.
    pub fn new() -> Self {
        Self {
            readable: true,
            writable: true,
            cleanup: false,
        }
    }

    /// Only check readable state.
    pub fn readable_only(mut self) -> Self {
        self.readable = true;
        self.writable = false;
        self
    }

    /// Only check writable state.
    pub fn writable_only(mut self) -> Self {
        self.readable = false;
        self.writable = true;
        self
    }

    /// Enable cleanup mode.
    pub fn cleanup(mut self) -> Self {
        self.cleanup = true;
        self
    }
}

/// A future that resolves when a stream is finished.
pub struct Finished<S> {
    stream: Option<S>,
    state: StreamState,
}

impl<S> Finished<S> {
    /// Create a new Finished future.
    pub fn new(stream: S) -> Self {
        Self {
            stream: Some(stream),
            state: StreamState::Open,
        }
    }
}

impl<S> Future for Finished<S>
where
    S: Stream<Item = Result<Bytes, StreamError>> + Unpin,
{
    type Output = Result<(), StreamError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(ref mut stream) = self.stream {
            match Pin::new(stream).poll_next(cx) {
                Poll::Ready(Some(Ok(_))) => {
                    // Still receiving data, keep polling
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
                Poll::Ready(Some(Err(e))) => {
                    self.stream = None;
                    self.state = StreamState::Errored;
                    Poll::Ready(Err(e))
                }
                Poll::Ready(None) => {
                    self.stream = None;
                    self.state = StreamState::Ended;
                    Poll::Ready(Ok(()))
                }
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

/// Wait for a stream to finish.
pub fn finished<S>(stream: S) -> Finished<S>
where
    S: Stream<Item = Result<Bytes, StreamError>> + Unpin,
{
    Finished::new(stream)
}

/// Callback type for finished notification.
pub type FinishedCallback = Box<dyn FnOnce(Option<StreamError>) + Send>;

/// Register a callback to be called when a stream finishes.
pub fn on_finished<S>(stream: S, callback: FinishedCallback)
where
    S: Stream<Item = Result<Bytes, StreamError>> + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let result = finished(stream).await;
        callback(result.err());
    });
}

/// Backpressure controller for managing flow between streams.
#[derive(Debug)]
pub struct BackpressureController {
    /// High water mark (pause threshold).
    high_water_mark: usize,
    /// Low water mark (resume threshold).
    low_water_mark: usize,
    /// Current buffer size.
    buffer_size: Arc<Mutex<usize>>,
    /// Whether the source is paused.
    paused: Arc<Mutex<bool>>,
    /// Waker to notify when buffer drains.
    waker: Arc<Mutex<Option<Waker>>>,
}

impl BackpressureController {
    /// Create a new backpressure controller.
    pub fn new(high_water_mark: usize) -> Self {
        Self {
            high_water_mark,
            low_water_mark: high_water_mark / 4,
            buffer_size: Arc::new(Mutex::new(0)),
            paused: Arc::new(Mutex::new(false)),
            waker: Arc::new(Mutex::new(None)),
        }
    }

    /// Create with custom low water mark.
    pub fn with_low_water_mark(mut self, low_water_mark: usize) -> Self {
        self.low_water_mark = low_water_mark;
        self
    }

    /// Record data being buffered.
    pub fn buffer(&self, size: usize) -> bool {
        let mut buffer_size = self.buffer_size.lock().unwrap();
        *buffer_size += size;
        
        if *buffer_size >= self.high_water_mark {
            let mut paused = self.paused.lock().unwrap();
            *paused = true;
            false // Signal to pause
        } else {
            true // OK to continue
        }
    }

    /// Record data being consumed.
    pub fn consume(&self, size: usize) {
        let mut buffer_size = self.buffer_size.lock().unwrap();
        *buffer_size = buffer_size.saturating_sub(size);
        
        if *buffer_size <= self.low_water_mark {
            let mut paused = self.paused.lock().unwrap();
            if *paused {
                *paused = false;
                // Wake up the source
                if let Some(waker) = self.waker.lock().unwrap().take() {
                    waker.wake();
                }
            }
        }
    }

    /// Check if currently paused due to backpressure.
    pub fn is_paused(&self) -> bool {
        *self.paused.lock().unwrap()
    }

    /// Get current buffer size.
    pub fn buffer_size(&self) -> usize {
        *self.buffer_size.lock().unwrap()
    }

    /// Register a waker to be notified when buffer drains.
    pub fn register_waker(&self, waker: Waker) {
        *self.waker.lock().unwrap() = Some(waker);
    }

    /// Wait until not paused.
    pub async fn wait_drain(&self) {
        WaitDrain { controller: self }.await
    }
}

impl Clone for BackpressureController {
    fn clone(&self) -> Self {
        Self {
            high_water_mark: self.high_water_mark,
            low_water_mark: self.low_water_mark,
            buffer_size: self.buffer_size.clone(),
            paused: self.paused.clone(),
            waker: self.waker.clone(),
        }
    }
}

/// Future that waits for backpressure to clear.
struct WaitDrain<'a> {
    controller: &'a BackpressureController,
}

impl<'a> Future for WaitDrain<'a> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.controller.is_paused() {
            self.controller.register_waker(cx.waker().clone());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

/// A stream wrapper that applies backpressure.
pub struct BackpressuredStream<S> {
    inner: S,
    controller: BackpressureController,
}

impl<S> BackpressuredStream<S> {
    /// Create a new backpressured stream.
    pub fn new(stream: S, high_water_mark: usize) -> Self {
        Self {
            inner: stream,
            controller: BackpressureController::new(high_water_mark),
        }
    }

    /// Get a reference to the backpressure controller.
    pub fn controller(&self) -> &BackpressureController {
        &self.controller
    }
}

impl<S> Stream for BackpressuredStream<S>
where
    S: Stream<Item = Result<Bytes, StreamError>> + Unpin,
{
    type Item = Result<Bytes, StreamError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // If paused, wait for drain
        if self.controller.is_paused() {
            self.controller.register_waker(cx.waker().clone());
            return Poll::Pending;
        }

        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                let len = chunk.len();
                self.controller.buffer(len);
                Poll::Ready(Some(Ok(chunk)))
            }
            other => other,
        }
    }
}

/// A writable wrapper that signals backpressure.
pub struct BackpressuredWriter<W> {
    inner: W,
    controller: BackpressureController,
}

impl<W> BackpressuredWriter<W>
where
    W: Writable,
{
    /// Create a new backpressured writer.
    pub fn new(writer: W, controller: BackpressureController) -> Self {
        Self {
            inner: writer,
            controller,
        }
    }

    /// Write with backpressure signaling.
    pub fn write(&mut self, chunk: Bytes) -> Result<bool, StreamError> {
        let len = chunk.len();
        let result = self.inner.write(chunk)?;
        self.controller.consume(len);
        Ok(result && !self.controller.is_paused())
    }

    /// End the writer.
    pub fn end(&mut self) -> Result<(), StreamError> {
        self.inner.end()
    }
}

/// Pipe with backpressure handling.
pub async fn pipe_with_backpressure<R, W>(
    source: R,
    dest: W,
    high_water_mark: usize,
) -> Result<(), StreamError>
where
    R: Stream<Item = Result<Bytes, StreamError>> + Unpin,
    W: Writable,
{
    let controller = BackpressureController::new(high_water_mark);
    let mut backpressured = BackpressuredStream::new(source, high_water_mark);
    let mut writer = BackpressuredWriter::new(dest, controller);

    while let Some(result) = backpressured.next().await {
        let chunk = result?;
        let can_continue = writer.write(chunk)?;
        
        if !can_continue {
            // Wait for backpressure to clear
            backpressured.controller().wait_drain().await;
        }
    }
    
    writer.end()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::ReadableStream;

    #[tokio::test]
    async fn test_finished_basic() {
        let stream = ReadableStream::from_chunks(vec![
            Bytes::from("hello"),
            Bytes::from(" world"),
        ]);
        
        let result = finished(stream).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_backpressure_controller() {
        let controller = BackpressureController::new(100);
        
        // Buffer some data
        assert!(controller.buffer(50)); // Under limit
        assert!(!controller.is_paused());
        
        assert!(!controller.buffer(60)); // Over limit
        assert!(controller.is_paused());
        
        // Consume data
        controller.consume(80);
        assert!(!controller.is_paused()); // Below low water mark
    }

    #[test]
    fn test_backpressure_controller_custom_low_water() {
        let controller = BackpressureController::new(100)
            .with_low_water_mark(50);
        
        controller.buffer(100);
        assert!(controller.is_paused());
        
        controller.consume(40); // Still above low water mark
        assert!(controller.is_paused());
        
        controller.consume(20); // Now below low water mark
        assert!(!controller.is_paused());
    }

    #[tokio::test]
    async fn test_backpressured_stream() {
        let source = ReadableStream::from_chunks(vec![
            Bytes::from("hello"),
            Bytes::from(" world"),
        ]);
        
        let mut stream = BackpressuredStream::new(source, 1000);
        
        let chunk1 = stream.next().await.unwrap().unwrap();
        assert_eq!(&chunk1[..], b"hello");
        
        let chunk2 = stream.next().await.unwrap().unwrap();
        assert_eq!(&chunk2[..], b" world");
        
        assert!(stream.next().await.is_none());
    }
}
