//! Async I/O integration tests

use dx_py_reactor::{
    Reactor, ReactorFeature, IoBuffer, IoOperation,
    CompletionFlags, PyFuture,
};

#[test]
fn test_io_buffer_creation() {
    let buffer = IoBuffer::new(4096);
    assert_eq!(buffer.capacity(), 4096);
    assert_eq!(buffer.len(), 0);
}

#[test]
fn test_io_buffer_write_read() {
    let mut buffer = IoBuffer::new(1024);
    
    let data = b"Hello, World!";
    buffer.write(data);
    
    assert_eq!(buffer.len(), data.len());
    assert_eq!(buffer.as_slice(), data);
}

#[test]
fn test_io_operation_types() {
    // Test that all operation types are defined
    let _read = IoOperation::Read {
        fd: 0,
        buffer: IoBuffer::new(1024),
        offset: 0,
    };
    
    let _write = IoOperation::Write {
        fd: 0,
        buffer: IoBuffer::new(1024),
        offset: 0,
    };
    
    let _accept = IoOperation::Accept {
        fd: 0,
    };
    
    let _connect = IoOperation::Connect {
        fd: 0,
        addr: std::net::SocketAddr::from(([127, 0, 0, 1], 8080)),
    };
}

#[test]
fn test_completion_flags() {
    let flags = CompletionFlags::empty();
    assert!(!flags.contains(CompletionFlags::ERROR));
    assert!(!flags.contains(CompletionFlags::CANCELLED));
    
    let error_flags = CompletionFlags::ERROR;
    assert!(error_flags.contains(CompletionFlags::ERROR));
}

#[test]
fn test_py_future_creation() {
    let future = PyFuture::new();
    
    assert!(!future.is_complete());
    assert!(!future.is_cancelled());
    assert!(future.error().is_none());
}

#[test]
fn test_py_future_completion() {
    let future = PyFuture::new();
    
    // Complete the future
    future.complete(vec![1, 2, 3, 4]);
    
    assert!(future.is_complete());
    assert_eq!(future.result(), Some(vec![1, 2, 3, 4]));
}

#[test]
fn test_py_future_cancellation() {
    let future = PyFuture::new();
    
    future.cancel();
    
    assert!(future.is_cancelled());
    assert!(future.is_complete());
}

#[test]
fn test_py_future_error() {
    let future = PyFuture::new();
    
    future.set_error("Connection refused".to_string());
    
    assert!(future.is_complete());
    assert_eq!(future.error(), Some("Connection refused".to_string()));
}

#[test]
fn test_reactor_features() {
    // Test feature detection
    let features = [
        ReactorFeature::FileRead,
        ReactorFeature::FileWrite,
        ReactorFeature::NetworkAccept,
        ReactorFeature::NetworkConnect,
        ReactorFeature::NetworkSend,
        ReactorFeature::NetworkRecv,
        ReactorFeature::DnsResolve,
    ];
    
    for feature in features {
        // Just ensure the enum variants exist
        let _ = format!("{:?}", feature);
    }
}

#[test]
fn test_io_buffer_clear() {
    let mut buffer = IoBuffer::new(1024);
    buffer.write(b"test data");
    
    assert!(buffer.len() > 0);
    
    buffer.clear();
    assert_eq!(buffer.len(), 0);
}

#[test]
fn test_io_buffer_resize() {
    let mut buffer = IoBuffer::new(64);
    
    // Write more than initial capacity
    let large_data = vec![0u8; 128];
    buffer.write(&large_data);
    
    assert!(buffer.capacity() >= 128);
    assert_eq!(buffer.len(), 128);
}
