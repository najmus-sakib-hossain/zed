#[cfg(not(windows))]
use std::path::PathBuf;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::codec::{WorkerEnvelope, decode_envelope, encode_envelope};

pub trait AsyncIo: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T> AsyncIo for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

type IoBox = Box<dyn AsyncIo>;

#[derive(Debug, Clone)]
pub enum IpcEndpoint {
    #[cfg(windows)]
    NamedPipe(String),
    #[cfg(not(windows))]
    UnixSocket(PathBuf),
}

impl IpcEndpoint {
    pub fn local(worker_id: &str) -> Self {
        #[cfg(windows)]
        {
            Self::NamedPipe(format!(r"\\.\pipe\dx-agent-{}", worker_id))
        }
        #[cfg(not(windows))]
        {
            Self::UnixSocket(std::env::temp_dir().join(format!("dx-agent-{}.sock", worker_id)))
        }
    }
}

pub struct WorkerConnection {
    io: IoBox,
}

impl WorkerConnection {
    pub fn new(io: IoBox) -> Self {
        Self { io }
    }

    pub async fn send(&mut self, envelope: &WorkerEnvelope) -> anyhow::Result<()> {
        let bytes = encode_envelope(envelope)?;
        self.io.write_u32(bytes.len() as u32).await?;
        self.io.write_all(&bytes).await?;
        self.io.flush().await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> anyhow::Result<WorkerEnvelope> {
        let len = self.io.read_u32().await? as usize;
        let mut buf = vec![0u8; len];
        self.io.read_exact(&mut buf).await?;
        decode_envelope(&buf)
    }
}

pub enum IpcServer {
    #[cfg(windows)]
    NamedPipe { name: String },
    #[cfg(not(windows))]
    Unix(tokio::net::UnixListener),
}

impl IpcServer {
    pub async fn bind(endpoint: &IpcEndpoint) -> anyhow::Result<Self> {
        #[cfg(windows)]
        {
            match endpoint {
                IpcEndpoint::NamedPipe(name) => Ok(Self::NamedPipe { name: name.clone() }),
            }
        }

        #[cfg(not(windows))]
        {
            match endpoint {
                IpcEndpoint::UnixSocket(path) => {
                    if path.exists() {
                        let _ = std::fs::remove_file(path);
                    }
                    let listener = tokio::net::UnixListener::bind(path)?;
                    Ok(Self::Unix(listener))
                }
            }
        }
    }

    pub async fn accept(&self) -> anyhow::Result<WorkerConnection> {
        #[cfg(windows)]
        {
            use tokio::net::windows::named_pipe::ServerOptions;
            let Self::NamedPipe { name } = self;
            let server = ServerOptions::new().create(name)?;
            server.connect().await?;
            Ok(WorkerConnection::new(Box::new(server)))
        }

        #[cfg(not(windows))]
        {
            let Self::Unix(listener) = self;
            let (stream, _) = listener.accept().await?;
            Ok(WorkerConnection::new(Box::new(stream)))
        }
    }
}

pub async fn connect(endpoint: &IpcEndpoint) -> anyhow::Result<WorkerConnection> {
    #[cfg(windows)]
    {
        use tokio::net::windows::named_pipe::ClientOptions;
        let IpcEndpoint::NamedPipe(name) = endpoint;
        loop {
            match ClientOptions::new().open(name) {
                Ok(client) => return Ok(WorkerConnection::new(Box::new(client))),
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
            }
        }
    }

    #[cfg(not(windows))]
    {
        let IpcEndpoint::UnixSocket(path) = endpoint;
        let stream = tokio::net::UnixStream::connect(path).await?;
        Ok(WorkerConnection::new(Box::new(stream)))
    }
}
