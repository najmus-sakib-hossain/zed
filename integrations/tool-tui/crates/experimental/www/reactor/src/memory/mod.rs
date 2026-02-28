//! Memory teleportation for zero-copy serialization.
//!
//! This module provides zero-copy data transfer between Rust server and WASM client
//! by ensuring identical memory layouts on both sides.

mod teleport;

pub use teleport::{
    TeleportBuffer, TeleportLayout, TeleportReader, Teleportable, TeleportablePoint,
    TeleportableTimestamp, TeleportableUser,
};
