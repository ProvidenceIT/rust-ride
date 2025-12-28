//! IPC module for daemon-CLI communication.
//!
//! This module provides the Inter-Process Communication protocol for
//! communication between the CLI client and the daemon server using
//! Unix domain sockets with JSON-framed messages.

pub mod messages;
pub mod protocol;

pub use messages::{IpcError, IpcRequest, IpcResponse};
pub use protocol::{IpcClient, IpcServer};
