//! Integration tests for dx-reactor.
//!
//! These tests verify the full request/response cycle and cross-platform I/O operations.

use dx_reactor::memory::{TeleportBuffer, TeleportReader};
use dx_reactor::middleware::{
    AuthMiddleware, Middleware, RateLimitMiddleware, Request, Response, TimingMiddleware,
    reset_thread_rate_limit,
};
use dx_reactor::protocol::{HbtpFlags, HbtpHeader, HbtpOpcode, HbtpProtocol, ResponseBuffer};
use dx_reactor::{DxReactor, IoBackend, WorkerStrategy};

// ============================================================================
// Reactor Integration Tests
// ============================================================================

#[test]
fn test_reactor_creation_thread_per_core() {
    let reactor = DxReactor::build().workers(WorkerStrategy::ThreadPerCore).build();

    assert!(reactor.num_cores() > 0);
    assert_eq!(reactor.num_cores(), num_cpus::get());
}

#[test]
fn test_reactor_creation_fixed_workers() {
    let reactor = DxReactor::build().workers(WorkerStrategy::Fixed(4)).build();

    assert_eq!(reactor.num_cores(), 4);
}

#[test]
fn test_reactor_builder_fluent_api() {
    let reactor = DxReactor::build()
        .workers(WorkerStrategy::Fixed(2))
        .io_backend(IoBackend::Auto)
        .teleport(true)
        .hbtp(true)
        .buffer_size(4096)
        .buffer_count(512)
        .build();

    assert_eq!(reactor.num_cores(), 2);
}

// ============================================================================
// HBTP Protocol Integration Tests
// ============================================================================

#[test]
fn test_hbtp_full_request_response_cycle() {
    // Create a protocol handler
    let mut protocol = HbtpProtocol::new();

    // Register a handler for RPC calls
    protocol.route(HbtpOpcode::RpcCall, |_header, payload| {
        // Echo the payload back
        Ok(payload.to_vec())
    });

    // Create a request header
    let mut request_bytes = vec![0u8; 16];
    request_bytes[0] = HbtpOpcode::RpcCall as u8;
    request_bytes[1] = HbtpFlags::EXPECTS_RESPONSE.bits();
    request_bytes[2..4].copy_from_slice(&1u16.to_le_bytes()); // sequence
    request_bytes[4..8].copy_from_slice(&8u32.to_le_bytes()); // length
    request_bytes[8..16].copy_from_slice(b"test_rpc");

    // Parse the header
    let header = HbtpHeader::from_bytes(&request_bytes).unwrap();
    assert_eq!(header.opcode, HbtpOpcode::RpcCall as u8);
    let length = header.length;
    assert_eq!(length, 8);

    // Process the request
    let payload = &request_bytes[8..16];
    let handler = protocol.get_handler(header.opcode).unwrap();
    let response = handler(&header, payload).unwrap();

    assert_eq!(response, b"test_rpc");
}

#[test]
fn test_hbtp_response_buffer_lifecycle() {
    let mut buffer = ResponseBuffer::new();

    // Write a pong response
    buffer.write_pong(1);
    assert_eq!(buffer.len(), 8);

    // Reset and reuse
    buffer.reset();
    assert!(buffer.is_empty());

    // Write an RPC response
    buffer.write_rpc_response(2, b"hello world");
    assert_eq!(buffer.len(), 8 + 11);

    // Verify header
    let bytes = buffer.as_bytes();
    let header = HbtpHeader::from_bytes(bytes).unwrap();
    assert_eq!(header.opcode, HbtpOpcode::RpcResponse as u8);
    let sequence = header.sequence;
    let length = header.length;
    assert_eq!(sequence, 2);
    assert_eq!(length, 11);
}

// ============================================================================
// Memory Teleportation Integration Tests
// ============================================================================

#[test]
fn test_teleportation_complex_data() {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct UserData {
        id: u64,
        age: u32,
        active: u8,
        _pad: [u8; 3],
    }

    let user = UserData {
        id: 12345,
        age: 30,
        active: 1,
        _pad: [0; 3],
    };

    let mut buffer = TeleportBuffer::new(256);

    // SAFETY: Creating a byte slice from a #[repr(C)] struct for serialization.
    // - UserData is #[repr(C)] with stable memory layout
    // - The reference is valid for the duration of this call
    // - size_of::<UserData>() gives the correct size
    let user_bytes = unsafe {
        std::slice::from_raw_parts(
            &user as *const UserData as *const u8,
            std::mem::size_of::<UserData>(),
        )
    };

    for byte in user_bytes {
        buffer.write(byte);
    }

    // Read back
    let bytes = buffer.as_bytes();
    let mut reader = TeleportReader::with_string_table(bytes, bytes.len());

    let mut recovered_bytes = Vec::new();
    for _ in 0..std::mem::size_of::<UserData>() {
        if let Ok(b) = reader.read::<u8>() {
            recovered_bytes.push(*b);
        }
    }

    // SAFETY: Reconstructing a #[repr(C)] struct from its byte representation.
    // - UserData is #[repr(C)] with stable memory layout
    // - recovered_bytes contains exactly size_of::<UserData>() bytes
    // - The bytes were originally serialized from a valid UserData instance
    let recovered: UserData =
        unsafe { std::ptr::read(recovered_bytes.as_ptr() as *const UserData) };

    assert_eq!(recovered, user);
}

#[test]
fn test_teleportation_with_strings() {
    let mut buffer = TeleportBuffer::new(512);

    // Write some data
    buffer.write(&42u64);

    // Write strings
    let (offset1, len1) = buffer.write_string("Hello");
    let (offset2, len2) = buffer.write_string("World");

    // Finalize
    let finalized = buffer.finalize();

    // Calculate string table offset
    let string_table_offset = finalized.len() - "Hello".len() - "World".len();

    // Read back
    let reader = TeleportReader::with_string_table(finalized, string_table_offset);

    let s1 = reader.read_string(offset1, len1);
    let s2 = reader.read_string(offset2, len2);

    assert_eq!(s1.unwrap(), "Hello");
    assert_eq!(s2.unwrap(), "World");
}

// ============================================================================
// Middleware Integration Tests
// ============================================================================

#[test]
fn test_middleware_full_chain() {
    reset_thread_rate_limit();

    let mut req = Request::new("/api/test".to_string(), "POST".to_string());
    req.set_header("Authorization", "Bearer valid_token_123");

    let mut res = Response::new();

    // Execute middleware chain manually
    // Before hooks in order
    TimingMiddleware::before(&mut req).unwrap();
    AuthMiddleware::before(&mut req).unwrap();
    RateLimitMiddleware::before(&mut req).unwrap();

    // Verify auth worked
    assert_eq!(req.extension("authenticated"), Some("true"));
    assert_eq!(req.extension("token"), Some("valid_token_123"));

    // After hooks in reverse order
    RateLimitMiddleware::after(&req, &mut res);
    AuthMiddleware::after(&req, &mut res);
    TimingMiddleware::after(&req, &mut res);

    // Verify timing header
    assert!(res.header("X-Response-Time").is_some());

    // Verify rate limit headers
    assert!(res.header("X-RateLimit-Limit").is_some());
    assert!(res.header("X-RateLimit-Remaining").is_some());
}

#[test]
fn test_middleware_auth_rejection() {
    let mut req = Request::new("/api/test".to_string(), "GET".to_string());
    // No Authorization header

    let result = AuthMiddleware::before(&mut req);
    assert!(result.is_err());

    // Invalid format
    req.set_header("Authorization", "Basic invalid");
    let result = AuthMiddleware::before(&mut req);
    assert!(result.is_err());
}

#[test]
fn test_middleware_rate_limiting() {
    reset_thread_rate_limit();

    // Make many requests
    for i in 0..1000 {
        let mut req = Request::new("/api/test".to_string(), "GET".to_string());
        let result = RateLimitMiddleware::before(&mut req);
        assert!(result.is_ok(), "Request {} should succeed", i);
    }

    // Next request should be rate limited
    let mut req = Request::new("/api/test".to_string(), "GET".to_string());
    let result = RateLimitMiddleware::before(&mut req);
    assert!(result.is_err(), "Request should be rate limited");
}

// ============================================================================
// Cross-Component Integration Tests
// ============================================================================

#[test]
fn test_hbtp_with_teleportation() {
    // Create teleported data
    let mut teleport_buffer = TeleportBuffer::new(256);
    teleport_buffer.write(&100u64);
    teleport_buffer.write(&200u32);
    let teleported_data = teleport_buffer.as_bytes().to_vec();

    // Create HBTP response with teleported data
    let mut response_buffer = ResponseBuffer::new();
    response_buffer.write_rpc_response(1, &teleported_data);

    // Parse the response
    let response_bytes = response_buffer.as_bytes();
    let header = HbtpHeader::from_bytes(response_bytes).unwrap();

    assert_eq!(header.opcode, HbtpOpcode::RpcResponse as u8);
    assert_eq!(header.length as usize, teleported_data.len());

    // Extract and read teleported data
    let payload = &response_bytes[8..];
    let mut reader = TeleportReader::with_string_table(payload, payload.len());

    let v1 = reader.read::<u64>().unwrap();
    let v2 = reader.read::<u32>().unwrap();

    assert_eq!(*v1, 100);
    assert_eq!(*v2, 200);
}
