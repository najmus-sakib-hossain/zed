//! WASM Client Test Infrastructure
//!
//! This module provides testing utilities for the dx-www WASM client.
//! Since the client is a no_std crate, we test the protocol and data
//! structures here in a standard Rust environment.

pub mod htip_builder;
pub mod mock_host;
