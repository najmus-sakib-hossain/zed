//! # Gateway Protocol for DX Platform Apps

pub mod actix_main;
pub mod actix_server;
// pub mod actix_ws;  // Deprecated - uses old actix actor model
// pub mod bridge;  // Deprecated - replaced by nodejs bridge
pub mod daemon;
pub mod discovery;
// pub mod http_server;  // Deprecated - replaced by actix_server
pub mod mdns;
pub mod middleware;
pub mod pairing;
pub mod platform_pairing;
pub mod protocol;
pub mod rate_limiter;
pub mod rest_api;
pub mod rpc;
pub mod server;
pub mod ws_handler;
// pub mod ws_server;  // Deprecated - replaced by actix_server

pub use discovery::{DeviceType, DiscoveredDevice, DiscoveryService};
pub use pairing::{PairingCode, PairingManager};
pub use protocol::{GatewayMessage, GatewayRequest, GatewayResponse};
pub use rate_limiter::RateLimiter;
pub use rpc::{RpcMethod, RpcRegistry};
pub use server::GatewayServer;
