//! WHATWG Streams API.

use bytes::Bytes;

/// ReadableStream implementation.
pub struct ReadableStream {
    chunks: Vec<Bytes>,
    index: usize,
}

impl ReadableStream {
    /// Create a new readable stream.
    pub fn new(chunks: Vec<Bytes>) -> Self {
        Self { chunks, index: 0 }
    }

    /// Get a reader.
    pub fn get_reader(&self) -> ReadableStreamReader {
        ReadableStreamReader {
            chunks: self.chunks.clone(),
            index: 0,
        }
    }

    /// Pipe to a writable stream.
    pub async fn pipe_to(&mut self, dest: &mut WritableStream) -> Result<(), StreamError> {
        while self.index < self.chunks.len() {
            let chunk = self.chunks[self.index].clone();
            dest.write(chunk)?;
            self.index += 1;
        }
        dest.close()?;
        Ok(())
    }

    /// Tee the stream into two.
    pub fn tee(self) -> (ReadableStream, ReadableStream) {
        let chunks = self.chunks.clone();
        (ReadableStream::new(chunks.clone()), ReadableStream::new(chunks))
    }
}

/// ReadableStreamReader.
pub struct ReadableStreamReader {
    chunks: Vec<Bytes>,
    index: usize,
}

impl ReadableStreamReader {
    /// Read the next chunk.
    pub async fn read(&mut self) -> Option<Bytes> {
        if self.index < self.chunks.len() {
            let chunk = self.chunks[self.index].clone();
            self.index += 1;
            Some(chunk)
        } else {
            None
        }
    }

    /// Cancel reading.
    pub fn cancel(&mut self) {
        self.index = self.chunks.len();
    }
}

/// WritableStream implementation.
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

    /// Get a writer.
    pub fn get_writer(&mut self) -> WritableStreamWriter<'_> {
        WritableStreamWriter { stream: self }
    }

    /// Write a chunk.
    pub fn write(&mut self, chunk: Bytes) -> Result<(), StreamError> {
        if self.closed {
            return Err(StreamError::Closed);
        }
        self.chunks.push(chunk);
        Ok(())
    }

    /// Close the stream.
    pub fn close(&mut self) -> Result<(), StreamError> {
        self.closed = true;
        Ok(())
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

/// WritableStreamWriter.
pub struct WritableStreamWriter<'a> {
    stream: &'a mut WritableStream,
}

impl WritableStreamWriter<'_> {
    /// Write a chunk.
    pub async fn write(&mut self, chunk: Bytes) -> Result<(), StreamError> {
        self.stream.write(chunk)
    }

    /// Close the stream.
    pub async fn close(&mut self) -> Result<(), StreamError> {
        self.stream.close()
    }
}

/// TransformStream for data transformation.
pub struct TransformStream {
    /// Readable side
    pub readable: ReadableStream,
    /// Writable side
    pub writable: WritableStream,
}

/// Stream error type.
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    /// Stream is closed
    #[error("Stream is closed")]
    Closed,
    /// Read error
    #[error("Read error: {0}")]
    Read(String),
    /// Write error
    #[error("Write error: {0}")]
    Write(String),
}
