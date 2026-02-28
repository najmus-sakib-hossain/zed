pub mod clock;
pub mod messages;
pub mod protocol;
pub mod remote;

pub use clock::GLOBAL_CLOCK;
pub use messages::SyncMessage;
pub use protocol::SyncManager;

// Real-time sync protocol: in-process broadcast-based sync manager
// Provides a publish/subscribe channel for Operations so the watcher
// and other components can broadcast live operations to subscribers
// (e.g. WebSocket handlers or other peers).
