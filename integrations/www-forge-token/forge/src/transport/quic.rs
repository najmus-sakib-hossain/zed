use anyhow::{bail, Result};

pub async fn start_server(_listen_addr: &str) -> Result<()> {
    bail!("QUIC server is not implemented in v0.1; network transport arrives in v0.2")
}

pub async fn connect_client(_remote_addr: &str) -> Result<()> {
    bail!("QUIC client is not implemented in v0.1; network transport arrives in v0.2")
}
