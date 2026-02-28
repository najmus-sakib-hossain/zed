//! Request pipelining for DCP client.
//!
//! Provides pipelined request handling for improved throughput.

use bytes::Bytes;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};

use super::connection::{MultiplexError, MultiplexedConnection};

/// Pipelined request
pub struct PipelinedRequest {
    /// Request ID
    pub id: u64,
    /// Stream ID
    pub stream_id: u16,
    /// Request data
    pub data: Bytes,
    /// Response channel
    pub response_tx: oneshot::Sender<Result<Bytes, MultiplexError>>,
}

/// Pipelined client for request pipelining
pub struct PipelinedClient {
    /// Underlying multiplexed connection
    connection: Arc<MultiplexedConnection>,
    /// Request ID counter
    request_id: AtomicU64,
    /// Pending requests awaiting responses
    pending: Arc<RwLock<HashMap<u64, PendingRequest>>>,
    /// Maximum concurrent requests
    max_in_flight: usize,
    /// Current in-flight count
    in_flight: Arc<std::sync::atomic::AtomicUsize>,
}

/// Pending request state
struct PendingRequest {
    stream_id: u16,
    response_tx: oneshot::Sender<Result<Bytes, MultiplexError>>,
}

impl PipelinedClient {
    /// Create a new pipelined client
    pub fn new(connection: Arc<MultiplexedConnection>, max_in_flight: usize) -> Self {
        Self {
            connection,
            request_id: AtomicU64::new(1),
            pending: Arc::new(RwLock::new(HashMap::new())),
            max_in_flight,
            in_flight: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    /// Get the underlying connection
    pub fn connection(&self) -> &Arc<MultiplexedConnection> {
        &self.connection
    }

    /// Get current in-flight request count
    pub fn in_flight_count(&self) -> usize {
        self.in_flight.load(Ordering::Relaxed)
    }

    /// Check if we can send more requests
    pub fn can_send(&self) -> bool {
        self.in_flight_count() < self.max_in_flight
    }

    /// Send a pipelined request
    /// Returns a future that resolves when the response is received
    pub async fn send(
        &self,
        data: Bytes,
    ) -> Result<oneshot::Receiver<Result<Bytes, MultiplexError>>, MultiplexError> {
        // Check in-flight limit
        if !self.can_send() {
            return Err(MultiplexError::SendBufferFull);
        }

        // Open a stream for this request
        let stream_id = self.connection.open_stream().await?;

        // Generate request ID
        let request_id = self.request_id.fetch_add(1, Ordering::SeqCst);

        // Create response channel
        let (response_tx, response_rx) = oneshot::channel();

        // Register pending request
        {
            let mut pending = self.pending.write().await;
            pending.insert(
                request_id,
                PendingRequest {
                    stream_id,
                    response_tx,
                },
            );
        }

        // Increment in-flight counter
        self.in_flight.fetch_add(1, Ordering::AcqRel);

        // Send the request
        self.connection.send(stream_id, data).await?;

        Ok(response_rx)
    }

    /// Send multiple requests in a batch
    pub async fn send_batch(
        &self,
        requests: Vec<Bytes>,
    ) -> Result<Vec<oneshot::Receiver<Result<Bytes, MultiplexError>>>, MultiplexError> {
        let mut receivers = Vec::with_capacity(requests.len());

        for data in requests {
            let rx = self.send(data).await?;
            receivers.push(rx);
        }

        Ok(receivers)
    }

    /// Process a response for a stream
    pub async fn process_response(
        &self,
        stream_id: u16,
        data: Bytes,
    ) -> Result<(), MultiplexError> {
        // Find the pending request for this stream
        let pending_request = {
            let mut pending = self.pending.write().await;
            let mut found_id = None;
            for (id, req) in pending.iter() {
                if req.stream_id == stream_id {
                    found_id = Some(*id);
                    break;
                }
            }
            found_id.and_then(|id| pending.remove(&id))
        };

        if let Some(req) = pending_request {
            // Decrement in-flight counter
            self.in_flight.fetch_sub(1, Ordering::AcqRel);

            // Send response
            let _ = req.response_tx.send(Ok(data));

            // Close the stream
            self.connection.close_stream(stream_id).await?;
        }

        Ok(())
    }

    /// Process an error for a stream
    pub async fn process_error(
        &self,
        stream_id: u16,
        error: MultiplexError,
    ) -> Result<(), MultiplexError> {
        // Find the pending request for this stream
        let pending_request = {
            let mut pending = self.pending.write().await;
            let mut found_id = None;
            for (id, req) in pending.iter() {
                if req.stream_id == stream_id {
                    found_id = Some(*id);
                    break;
                }
            }
            found_id.and_then(|id| pending.remove(&id))
        };

        if let Some(req) = pending_request {
            // Decrement in-flight counter
            self.in_flight.fetch_sub(1, Ordering::AcqRel);

            // Send error
            let _ = req.response_tx.send(Err(error));
        }

        Ok(())
    }

    /// Cancel all pending requests
    pub async fn cancel_all(&self) {
        let mut pending = self.pending.write().await;
        for (_, req) in pending.drain() {
            let _ = req.response_tx.send(Err(MultiplexError::ConnectionClosed));
        }
        self.in_flight.store(0, Ordering::Release);
    }

    /// Get pending request count
    pub async fn pending_count(&self) -> usize {
        self.pending.read().await.len()
    }
}

/// Request pipeline for managing multiple concurrent requests
pub struct RequestPipeline {
    /// Pipelined client
    client: Arc<PipelinedClient>,
    /// Request sender channel
    request_tx: mpsc::Sender<PipelinedRequest>,
    /// Request receiver channel
    request_rx: Mutex<mpsc::Receiver<PipelinedRequest>>,
}

impl RequestPipeline {
    /// Create a new request pipeline
    pub fn new(
        connection: Arc<MultiplexedConnection>,
        max_in_flight: usize,
        buffer_size: usize,
    ) -> Self {
        let (request_tx, request_rx) = mpsc::channel(buffer_size);
        let client = Arc::new(PipelinedClient::new(connection, max_in_flight));

        Self {
            client,
            request_tx,
            request_rx: Mutex::new(request_rx),
        }
    }

    /// Get the pipelined client
    pub fn client(&self) -> &Arc<PipelinedClient> {
        &self.client
    }

    /// Submit a request to the pipeline
    pub async fn submit(
        &self,
        data: Bytes,
    ) -> Result<oneshot::Receiver<Result<Bytes, MultiplexError>>, MultiplexError> {
        self.client.send(data).await
    }

    /// Process incoming responses
    /// This should be called in a loop to handle responses
    pub async fn process_incoming(
        &self,
        stream_id: u16,
        data: Bytes,
    ) -> Result<(), MultiplexError> {
        self.client.process_response(stream_id, data).await
    }

    /// Get current in-flight count
    pub fn in_flight_count(&self) -> usize {
        self.client.in_flight_count()
    }

    /// Check if pipeline can accept more requests
    pub fn can_accept(&self) -> bool {
        self.client.can_send()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipelined_client_creation() {
        let conn = Arc::new(MultiplexedConnection::new());
        let client = PipelinedClient::new(conn, 10);

        assert_eq!(client.in_flight_count(), 0);
        assert!(client.can_send());
    }

    #[tokio::test]
    async fn test_pipelined_send() {
        let conn = Arc::new(MultiplexedConnection::new());
        let client = PipelinedClient::new(conn.clone(), 10);

        let rx = client.send(Bytes::from("request1")).await.unwrap();
        assert_eq!(client.in_flight_count(), 1);

        // Simulate response
        let streams = conn.active_streams().await;
        assert!(!streams.is_empty());

        let stream_id = streams[0];
        client.process_response(stream_id, Bytes::from("response1")).await.unwrap();

        // Check response
        let result = rx.await.unwrap();
        assert_eq!(result.unwrap(), Bytes::from("response1"));
        assert_eq!(client.in_flight_count(), 0);
    }

    #[tokio::test]
    async fn test_pipelined_batch() {
        let conn = Arc::new(MultiplexedConnection::new());
        let client = PipelinedClient::new(conn.clone(), 10);

        let requests = vec![
            Bytes::from("req1"),
            Bytes::from("req2"),
            Bytes::from("req3"),
        ];

        let receivers = client.send_batch(requests).await.unwrap();
        assert_eq!(receivers.len(), 3);
        assert_eq!(client.in_flight_count(), 3);
    }

    #[tokio::test]
    async fn test_pipelined_max_in_flight() {
        let conn = Arc::new(MultiplexedConnection::new());
        let client = PipelinedClient::new(conn, 2);

        // Send two requests
        let _rx1 = client.send(Bytes::from("req1")).await.unwrap();
        let _rx2 = client.send(Bytes::from("req2")).await.unwrap();

        assert_eq!(client.in_flight_count(), 2);
        assert!(!client.can_send());

        // Third should fail
        let result = client.send(Bytes::from("req3")).await;
        assert!(matches!(result, Err(MultiplexError::SendBufferFull)));
    }

    #[tokio::test]
    async fn test_pipelined_cancel_all() {
        let conn = Arc::new(MultiplexedConnection::new());
        let client = PipelinedClient::new(conn, 10);

        let rx1 = client.send(Bytes::from("req1")).await.unwrap();
        let rx2 = client.send(Bytes::from("req2")).await.unwrap();

        client.cancel_all().await;

        assert_eq!(client.in_flight_count(), 0);

        // Receivers should get errors
        let result1 = rx1.await.unwrap();
        assert!(matches!(result1, Err(MultiplexError::ConnectionClosed)));

        let result2 = rx2.await.unwrap();
        assert!(matches!(result2, Err(MultiplexError::ConnectionClosed)));
    }

    #[tokio::test]
    async fn test_request_pipeline() {
        let conn = Arc::new(MultiplexedConnection::new());
        let pipeline = RequestPipeline::new(conn, 10, 100);

        assert!(pipeline.can_accept());
        assert_eq!(pipeline.in_flight_count(), 0);
    }
}
