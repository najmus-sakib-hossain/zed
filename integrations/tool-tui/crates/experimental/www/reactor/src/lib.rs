//! # dx-reactor
//!
//! Binary Dawn - Cross-platform I/O reactor with thread-per-core architecture.
//!
//! This crate provides a unified I/O abstraction layer that automatically selects
//! the best platform-specific backend:
//!
//! - **Linux 5.1+**: io_uring with SQPOLL for zero-syscall I/O
//! - **Linux (older)**: epoll fallback
//! - **macOS/BSD**: kqueue
//! - **Windows**: IOCP (I/O Completion Ports)
//!
//! ## Features
//!
//! - Thread-per-core architecture with CPU pinning
//! - Zero-copy I/O operations
//! - HBTP binary protocol support
//! - Memory teleportation for WASM interop

// Allow certain clippy lints for this low-level I/O crate
// These are intentional design decisions for performance and safety
#![allow(clippy::missing_const_for_fn)] // Many functions can't be const due to runtime checks
#![allow(clippy::must_use_candidate)] // Builder pattern functions don't need must_use
#![allow(clippy::missing_panics_doc)] // Panics are documented in safety sections
#![allow(clippy::missing_errors_doc)] // Error conditions are clear from Result types
#![allow(clippy::needless_pass_by_value)] // Arc cloning is intentional for thread safety
#![allow(clippy::expect_used)] // Expect is used for unrecoverable errors in worker threads
#![allow(clippy::redundant_clone)] // Some clones are needed for ownership transfer
#![allow(clippy::uninlined_format_args)] // Readability preference
#![allow(clippy::cast_possible_truncation)] // Intentional for u32 sizes in protocol
#![allow(clippy::cast_possible_wrap)] // Intentional for signed/unsigned conversions
#![allow(clippy::cast_sign_loss)] // Intentional for signed/unsigned conversions
#![allow(clippy::doc_markdown)] // Allow technical terms without backticks
#![allow(clippy::return_self_not_must_use)] // Builder pattern doesn't need must_use
#![allow(clippy::use_self)] // Explicit type names improve readability
#![allow(clippy::ptr_as_ptr)] // Raw pointer casts are intentional for FFI
#![allow(clippy::ref_as_ptr)] // Reference to pointer casts are intentional
#![allow(clippy::borrow_as_ptr)] // Borrow as pointer is intentional for FFI
#![allow(clippy::map_unwrap_or)] // map().unwrap_or() is clearer in some cases
#![allow(clippy::redundant_closure_for_method_calls)] // Explicit closures improve readability
#![allow(clippy::pub_underscore_fields)] // Padding fields are intentionally public
#![allow(clippy::bool_to_int_with_if)] // Clearer than u8::from(bool)
#![allow(clippy::imprecise_flops)] // Manual sqrt is intentional for clarity
#![allow(clippy::duplicated_attributes)] // Platform-specific cfg attributes are intentional
#![allow(clippy::needless_continue)] // Continue in match arms improves readability
#![allow(unsafe_code)] // This crate requires unsafe for low-level I/O operations

pub mod io;
pub mod memory;
pub mod middleware;
pub mod protocol;

mod core_state;
mod reactor;

pub use core_state::CoreState;
pub use io::{Completion, Interest, IoHandle, PlatformReactor, Reactor, ReactorConfig};
pub use reactor::{DxReactor, IoBackend, ReactorBuilder, WorkerStrategy};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{
        Completion, DxReactor, Interest, IoBackend, Reactor, ReactorBuilder, ReactorConfig,
        WorkerStrategy,
    };
}
