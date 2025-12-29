//! IPC client for CLI commands.
//!
//! Connects to the daemon socket and sends commands.

use anyhow::Result;
use std::path::Path;

use crate::ipc::messages::{IpcRequest, IpcResponse};
use crate::ipc::protocol::IpcClient;

/// Connect to the daemon and send a command
pub async fn send_command(socket_path: &Path, request: IpcRequest) -> Result<IpcResponse> {
    let mut client = IpcClient::connect(socket_path).await?;
    let response = client.send(request).await?;
    Ok(response)
}

/// Check if the daemon is running by attempting to connect
pub async fn is_daemon_running(socket_path: &Path) -> bool {
    IpcClient::connect(socket_path).await.is_ok()
}
