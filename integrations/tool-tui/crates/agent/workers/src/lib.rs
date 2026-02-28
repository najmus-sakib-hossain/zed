//! DX Agent workers: cross-platform IPC + worker process manager + health monitoring.

pub mod codec;
pub mod health;
pub mod ipc;
pub mod process;

pub use codec::{WorkerEnvelope, WorkerMessageKind, decode_envelope, encode_envelope};
pub use health::{HealthConfig, HealthMonitor};
pub use ipc::{IpcEndpoint, IpcServer, WorkerConnection};
pub use process::{RestartPolicy, WorkerManager, WorkerSpec, WorkerStatus};
