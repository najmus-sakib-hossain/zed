//! Streaming data with backpressure support.
//!
//! This module provides Node.js-compatible stream implementations including:
//! - `ReadableStream` and `WritableStream` for basic streaming
//! - `DuplexStream` for bidirectional streaming
//! - `TransformStream` for data transformation
//! - `pipeline` for connecting streams with error handling
//! - `finished` for detecting stream completion
//! - Backpressure utilities for flow control

use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

pub mod duplex;
pub mod finished;
pub mod pipeline;
pub mod transform;

pub use duplex::{DuplexStream, ReadableHalf, WritableHalf};
pub use finished::{
    finished, on_finished, pipe_with_backpressure, BackpressureController,
    BackpressuredStream, BackpressuredWriter, Finished, FinishedCallback,
    FinishedOptions, StreamState,
};
pub use pipeline::{
    collect, collect_bytes, from, pipeline, pipeline_callback, pipeline_many,
    pipeline_with_options, AbortController, AbortSignal, BoxedPipelineStream,
    PipelineBuilder, PipelineError, PipelineOptions, PipelineResult,
    PipelineStream, PipelineTransform,
};
pub use transform::{TransformFn, TransformStream, TransformStreamBuilder};

/// Stream error type.
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Closed error
    #[error("Stream closed")]
    Closed,
}

/// Readable stream trait.
pub trait Readable: Stream<Item = Result<Bytes, StreamError>> {
    /// Pause the stream.
    fn pause(&mut self);
    /// Resume the stream.
    fn resume(&mut self);
    /// Check if paused.
    fn is_paused(&self) -> bool;
}

/// Writable stream trait.
pub trait Writable {
    /// Write a chunk.
    fn write(&mut self, chunk: Bytes) -> Result<bool, StreamError>;
    /// End the stream.
    fn end(&mut self) -> Result<(), StreamError>;
}

/// Simple readable stream implementation.
pub struct ReadableStream {
    chunks: Vec<Bytes>,
    index: usize,
    paused: bool,
}

impl ReadableStream {
    /// Create a new readable stream from chunks.
    pub fn from_chunks(chunks: Vec<Bytes>) -> Self {
        Self {
            chunks,
            index: 0,
            paused: false,
        }
    }
}

impl Stream for ReadableStream {
    type Item = Result<Bytes, StreamError>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.paused {
            return Poll::Pending;
        }
        if self.index < self.chunks.len() {
            let chunk = self.chunks[self.index].clone();
            self.index += 1;
            Poll::Ready(Some(Ok(chunk)))
        } else {
            Poll::Ready(None)
        }
    }
}

impl Readable for ReadableStream {
    fn pause(&mut self) {
        self.paused = true;
    }

    fn resume(&mut self) {
        self.paused = false;
    }

    fn is_paused(&self) -> bool {
        self.paused
    }
}

/// Simple writable stream implementation.
pub struct WritableStream {
    chunks: Vec<Bytes>,
    closed: bool,
}

impl WritableStream {
    /// Create a new writable stream.
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            closed: false,
        }
    }

    /// Get collected chunks.
    pub fn chunks(&self) -> &[Bytes] {
        &self.chunks
    }
}

impl Default for WritableStream {
    fn default() -> Self {
        Self::new()
    }
}

impl Writable for WritableStream {
    fn write(&mut self, chunk: Bytes) -> Result<bool, StreamError> {
        if self.closed {
            return Err(StreamError::Closed);
        }
        self.chunks.push(chunk);
        Ok(true)
    }

    fn end(&mut self) -> Result<(), StreamError> {
        self.closed = true;
        Ok(())
    }
}

/// Pipe streams with zero-copy transfer.
pub async fn pipe<R, W>(mut source: R, mut dest: W) -> Result<(), StreamError>
where
    R: Readable + Unpin,
    W: Writable,
{
    use futures::StreamExt;

    while let Some(result) = source.next().await {
        let chunk = result?;
        dest.write(chunk)?;
    }
    dest.end()?;
    Ok(())
}
