//! Basic example of using DxReactor.
//!
//! This example demonstrates how to create and configure a DxReactor
//! with the Binary Dawn architecture.

use dx_reactor::{DxReactor, IoBackend, WorkerStrategy};

fn main() {
    println!("Binary Dawn - DxReactor Example");
    println!("================================\n");

    // Create a reactor with thread-per-core architecture
    let reactor = DxReactor::build()
        .workers(WorkerStrategy::ThreadPerCore)
        .io_backend(IoBackend::Auto)
        .teleport(true)
        .hbtp(true)
        .buffer_size(8192)
        .buffer_count(1024)
        .build();

    println!("Reactor created with {} cores", reactor.num_cores());
    println!("I/O Backend: Auto-detected");
    println!("Teleportation: Enabled");
    println!("HBTP Protocol: Enabled");

    // In a real application, you would call reactor.ignite() to start
    // the event loop. For this example, we just demonstrate configuration.

    println!("\nReactor is ready to ignite!");
    println!("Call reactor.ignite() to start the event loop.");
}
