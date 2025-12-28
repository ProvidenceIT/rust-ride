//! IPC protocol implementation.
//!
//! Provides length-prefixed JSON framing for Unix socket communication.

use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::RwLock;
use anyhow::Result;
use tracing::{debug, error, info};

use super::messages::{IpcRequest, IpcResponse};
use crate::daemon::handler::handle_request;
use crate::daemon::state::DaemonState;

/// Length prefix size in bytes (big-endian u32)
const LENGTH_PREFIX_SIZE: usize = 4;

/// Maximum message size (1MB)
const MAX_MESSAGE_SIZE: u32 = 1024 * 1024;

/// Write a length-prefixed JSON message
pub async fn write_message<W: AsyncWriteExt + Unpin, T: serde::Serialize>(
    writer: &mut W,
    message: &T,
) -> Result<()> {
    let json = serde_json::to_vec(message)?;
    let len = json.len() as u32;

    // Write length prefix (big-endian)
    writer.write_all(&len.to_be_bytes()).await?;
    // Write JSON payload
    writer.write_all(&json).await?;
    writer.flush().await?;

    Ok(())
}

/// Read a length-prefixed JSON message
pub async fn read_message<R: AsyncReadExt + Unpin, T: serde::de::DeserializeOwned>(
    reader: &mut R,
) -> Result<T> {
    // Read length prefix
    let mut len_buf = [0u8; LENGTH_PREFIX_SIZE];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf);

    // Validate length
    if len > MAX_MESSAGE_SIZE {
        anyhow::bail!("Message too large: {} bytes", len);
    }

    // Read JSON payload
    let mut json_buf = vec![0u8; len as usize];
    reader.read_exact(&mut json_buf).await?;

    // Deserialize
    let message = serde_json::from_slice(&json_buf)?;
    Ok(message)
}

/// IPC server that listens on a Unix socket
pub struct IpcServer {
    listener: UnixListener,
    state: Arc<RwLock<DaemonState>>,
}

impl IpcServer {
    /// Create a new IPC server
    pub async fn new(socket_path: &Path, state: Arc<RwLock<DaemonState>>) -> Result<Self> {
        // Remove existing socket file if present
        if socket_path.exists() {
            std::fs::remove_file(socket_path)?;
        }

        let listener = UnixListener::bind(socket_path)?;
        info!("IPC server listening on {:?}", socket_path);

        Ok(Self { listener, state })
    }

    /// Run the server, accepting connections until shutdown
    pub async fn run(self) -> Result<()> {
        loop {
            match self.listener.accept().await {
                Ok((stream, _addr)) => {
                    let state = self.state.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, state).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }
}

/// Handle a single client connection
async fn handle_connection(
    mut stream: UnixStream,
    state: Arc<RwLock<DaemonState>>,
) -> Result<()> {
    debug!("New client connection");

    // Read request
    let request: IpcRequest = read_message(&mut stream).await?;
    debug!("Received request: {:?}", request);

    // Handle request
    let response = handle_request(request, state).await;

    // Write response
    write_message(&mut stream, &response).await?;
    debug!("Sent response");

    Ok(())
}

/// IPC client for connecting to the daemon
pub struct IpcClient {
    stream: UnixStream,
}

impl IpcClient {
    /// Connect to the daemon socket
    pub async fn connect(socket_path: &Path) -> Result<Self> {
        let stream = UnixStream::connect(socket_path).await?;
        Ok(Self { stream })
    }

    /// Send a request and receive a response
    pub async fn send(&mut self, request: IpcRequest) -> Result<IpcResponse> {
        write_message(&mut self.stream, &request).await?;
        let response = read_message(&mut self.stream).await?;
        Ok(response)
    }
}
