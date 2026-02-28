//! Stream pipeline implementation for Node.js compatibility.
//!
//! The pipeline function connects multiple streams together, handling
//! error propagation and cleanup automatically.

use bytes::Bytes;
use futures::{Stream, StreamExt};
use std::pin::Pin;

use super::{StreamError, Writable};

/// Error type for pipeline operations.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    /// Stream error during pipeline execution.
    #[error("Stream error: {0}")]
    Stream(#[from] StreamError),
    /// Pipeline was aborted.
    #[error("Pipeline aborted")]
    Aborted,
    /// Source stream ended unexpectedly.
    #[error("Source ended unexpectedly")]
    UnexpectedEnd,
}

/// Result of a pipeline operation.
pub type PipelineResult<T = ()> = Result<T, PipelineError>;

/// Options for pipeline execution.
#[derive(Debug, Clone, Default)]
pub struct PipelineOptions {
    /// Whether to end the destination when the source ends.
    pub end: bool,
    /// Signal to abort the pipeline.
    pub signal: Option<AbortSignal>,
}

impl PipelineOptions {
    /// Create new options with default values.
    pub fn new() -> Self {
        Self {
            end: true,
            signal: None,
        }
    }

    /// Set whether to end the destination.
    pub fn end(mut self, end: bool) -> Self {
        self.end = end;
        self
    }

    /// Set an abort signal.
    pub fn signal(mut self, signal: AbortSignal) -> Self {
        self.signal = Some(signal);
        self
    }
}

/// A signal that can be used to abort a pipeline.
#[derive(Debug, Clone)]
pub struct AbortSignal {
    aborted: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl AbortSignal {
    /// Create a new abort signal.
    pub fn new() -> (Self, AbortController) {
        let aborted = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        (
            Self { aborted: aborted.clone() },
            AbortController { aborted },
        )
    }

    /// Check if the signal has been aborted.
    pub fn is_aborted(&self) -> bool {
        self.aborted.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Default for AbortSignal {
    fn default() -> Self {
        Self::new().0
    }
}

/// Controller for an abort signal.
#[derive(Debug)]
pub struct AbortController {
    aborted: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl AbortController {
    /// Abort the associated signal.
    pub fn abort(&self) {
        self.aborted.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Connect two streams with automatic error handling and cleanup.
///
/// This is the basic two-stream pipeline function.
pub async fn pipeline<S, D>(source: S, dest: D) -> PipelineResult
where
    S: Stream<Item = Result<Bytes, StreamError>> + Unpin,
    D: Writable,
{
    pipeline_with_options(source, dest, PipelineOptions::new()).await
}

/// Connect two streams with custom options.
pub async fn pipeline_with_options<S, D>(
    mut source: S,
    mut dest: D,
    options: PipelineOptions,
) -> PipelineResult
where
    S: Stream<Item = Result<Bytes, StreamError>> + Unpin,
    D: Writable,
{
    while let Some(result) = source.next().await {
        // Check for abort
        if let Some(ref signal) = options.signal {
            if signal.is_aborted() {
                return Err(PipelineError::Aborted);
            }
        }
        
        let chunk = result?;
        dest.write(chunk)?;
    }
    
    if options.end {
        dest.end()?;
    }
    
    Ok(())
}

/// A trait for streams that can be used in a pipeline.
pub trait PipelineStream: Stream<Item = Result<Bytes, StreamError>> + Unpin {}

impl<T> PipelineStream for T where T: Stream<Item = Result<Bytes, StreamError>> + Unpin {}

/// A boxed pipeline stream for dynamic dispatch.
pub type BoxedPipelineStream = Pin<Box<dyn Stream<Item = Result<Bytes, StreamError>> + Send>>;

/// Connect multiple streams in a pipeline.
///
/// This function takes a source stream and a series of transform/destination
/// streams, connecting them together.
pub async fn pipeline_many<S>(
    source: S,
    transforms: Vec<Box<dyn PipelineTransform + Send>>,
    mut dest: Box<dyn Writable + Send>,
) -> PipelineResult
where
    S: Stream<Item = Result<Bytes, StreamError>> + Unpin + Send + 'static,
{
    let mut current: BoxedPipelineStream = Box::pin(source);
    
    // Apply each transform
    for mut transform in transforms {
        // Collect all data from current stream and write to transform
        while let Some(result) = current.next().await {
            let chunk = result?;
            transform.write(chunk)?;
        }
        transform.end()?;
        
        // Get the transformed output as the new current stream
        current = transform.into_stream();
    }
    
    // Write final output to destination
    while let Some(result) = current.next().await {
        let chunk = result?;
        dest.write(chunk)?;
    }
    dest.end()?;
    
    Ok(())
}

/// A trait for transforms that can be used in a pipeline.
pub trait PipelineTransform: Writable {
    /// Convert this transform into a readable stream.
    fn into_stream(self: Box<Self>) -> BoxedPipelineStream;
}

/// Callback type for pipeline completion.
pub type PipelineCallback = Box<dyn FnOnce(Option<PipelineError>) + Send>;

/// Execute a pipeline with a callback on completion.
pub fn pipeline_callback<S, D>(
    source: S,
    dest: D,
    callback: PipelineCallback,
) where
    S: Stream<Item = Result<Bytes, StreamError>> + Unpin + Send + 'static,
    D: Writable + Send + 'static,
{
    tokio::spawn(async move {
        let result = pipeline(source, dest).await;
        callback(result.err());
    });
}

/// A pipeline builder for fluent API.
pub struct PipelineBuilder<S> {
    source: S,
    options: PipelineOptions,
}

impl<S> PipelineBuilder<S>
where
    S: Stream<Item = Result<Bytes, StreamError>> + Unpin,
{
    /// Create a new pipeline builder with the given source.
    pub fn new(source: S) -> Self {
        Self {
            source,
            options: PipelineOptions::new(),
        }
    }

    /// Set pipeline options.
    pub fn options(mut self, options: PipelineOptions) -> Self {
        self.options = options;
        self
    }

    /// Don't end the destination when source ends.
    pub fn no_end(mut self) -> Self {
        self.options.end = false;
        self
    }

    /// Set an abort signal.
    pub fn abort_signal(mut self, signal: AbortSignal) -> Self {
        self.options.signal = Some(signal);
        self
    }

    /// Execute the pipeline to the given destination.
    pub async fn to<D>(self, dest: D) -> PipelineResult
    where
        D: Writable,
    {
        pipeline_with_options(self.source, dest, self.options).await
    }
}

/// Create a pipeline builder from a source stream.
pub fn from<S>(source: S) -> PipelineBuilder<S>
where
    S: Stream<Item = Result<Bytes, StreamError>> + Unpin,
{
    PipelineBuilder::new(source)
}

/// Utility function to collect a stream into a Vec<Bytes>.
pub async fn collect<S>(mut stream: S) -> PipelineResult<Vec<Bytes>>
where
    S: Stream<Item = Result<Bytes, StreamError>> + Unpin,
{
    let mut chunks = Vec::new();
    while let Some(result) = stream.next().await {
        chunks.push(result?);
    }
    Ok(chunks)
}

/// Utility function to collect a stream into a single Bytes.
pub async fn collect_bytes<S>(stream: S) -> PipelineResult<Bytes>
where
    S: Stream<Item = Result<Bytes, StreamError>> + Unpin,
{
    let chunks = collect(stream).await?;
    let total_len: usize = chunks.iter().map(|c| c.len()).sum();
    let mut result = Vec::with_capacity(total_len);
    for chunk in chunks {
        result.extend_from_slice(&chunk);
    }
    Ok(Bytes::from(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::{ReadableStream, WritableStream};

    #[tokio::test]
    async fn test_pipeline_basic() {
        let source = ReadableStream::from_chunks(vec![
            Bytes::from("hello"),
            Bytes::from(" world"),
        ]);
        let dest = WritableStream::new();
        
        pipeline(source, dest).await.unwrap();
    }

    #[tokio::test]
    async fn test_pipeline_with_abort() {
        let source = ReadableStream::from_chunks(vec![
            Bytes::from("hello"),
            Bytes::from(" world"),
        ]);
        let dest = WritableStream::new();
        
        let (signal, controller) = AbortSignal::new();
        controller.abort();
        
        let result = pipeline_with_options(
            source,
            dest,
            PipelineOptions::new().signal(signal),
        ).await;
        
        assert!(matches!(result, Err(PipelineError::Aborted)));
    }

    #[tokio::test]
    async fn test_pipeline_builder() {
        let source = ReadableStream::from_chunks(vec![
            Bytes::from("test"),
        ]);
        let dest = WritableStream::new();
        
        from(source).to(dest).await.unwrap();
    }

    #[tokio::test]
    async fn test_collect_bytes() {
        let source = ReadableStream::from_chunks(vec![
            Bytes::from("hello"),
            Bytes::from(" world"),
        ]);
        
        let result = collect_bytes(source).await.unwrap();
        assert_eq!(&result[..], b"hello world");
    }

    #[test]
    fn test_abort_signal() {
        let (signal, controller) = AbortSignal::new();
        assert!(!signal.is_aborted());
        
        controller.abort();
        assert!(signal.is_aborted());
    }
}
