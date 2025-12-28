//! Unix signal handling for daemon.
//!
//! Handles SIGTERM and SIGINT for graceful shutdown.

use anyhow::Result;
use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook_tokio::Signals;
use tokio_stream::StreamExt;
use tracing::info;

/// Setup signal handlers and return a future that completes on shutdown signal
pub fn setup_signal_handlers() -> Result<impl std::future::Future<Output = ()>> {
    let mut signals = Signals::new([SIGTERM, SIGINT])?;

    Ok(async move {
        while let Some(signal) = signals.next().await {
            match signal {
                SIGTERM => {
                    info!("Received SIGTERM");
                    break;
                }
                SIGINT => {
                    info!("Received SIGINT (Ctrl+C)");
                    break;
                }
                _ => {}
            }
        }
    })
}
