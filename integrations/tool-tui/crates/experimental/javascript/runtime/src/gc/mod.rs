//! Garbage Collection Module
//!
//! This module provides memory management for JavaScript heap objects.
//! It implements a generational mark-and-sweep garbage collector with:
//! - Young generation (nursery) for short-lived objects
//! - Old generation for long-lived objects
//! - Write barriers for tracking old-to-young pointers
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                     GcHeap                               │
//! ├─────────────────────────────────────────────────────────┤
//! │  Young Generation (Nursery)    │  Old Generation        │
//! │  - Fast allocation             │  - Promoted objects    │
//! │  - Frequent collection         │  - Infrequent GC       │
//! │  - Copy collection             │  - Mark-sweep          │
//! └─────────────────────────────────────────────────────────┘
//! ```

mod gc_ref;
mod header;
mod heap;

pub use gc_ref::{GcObject, GcRef};
pub use header::GcHeader;
pub use heap::{GcConfig, GcHeap, GcStats, NodeMemoryUsage, OomError};
