//! mDNS service advertisement for companion server discovery.
//!
//! Advertises the companion server on the local network using mDNS/DNS-SD
//! so that mobile companion apps can automatically discover the server.
//!
//! The service is advertised as `_rustride._tcp.local.` with TXT records
//! containing the port and protocol version.

use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::server::get_local_ip;

/// mDNS service type for RustRide companion server.
/// Uses TCP since WebSocket runs over TCP.
pub const COMPANION_SERVICE_TYPE: &str = "_rustride._tcp.local.";

/// Protocol version for companion API.
/// Increment this when making breaking changes to the API.
pub const COMPANION_PROTOCOL_VERSION: &str = "1";

/// Errors that can occur during mDNS operations.
#[derive(Debug, thiserror::Error)]
pub enum MdnsError {
    /// Failed to create mDNS daemon.
    #[error("Failed to create mDNS daemon: {0}")]
    DaemonCreationFailed(String),

    /// Failed to register service.
    #[error("Failed to register service: {0}")]
    RegistrationFailed(String),

    /// Failed to unregister service.
    #[error("Failed to unregister service: {0}")]
    UnregistrationFailed(String),

    /// Service not running.
    #[error("mDNS service not running")]
    NotRunning,
}

/// mDNS service advertiser for the companion server.
///
/// Handles registration and unregistration of the companion server
/// as an mDNS service for automatic discovery by mobile apps.
pub struct CompanionMdnsAdvertiser {
    /// The mDNS daemon instance.
    daemon: Arc<RwLock<Option<ServiceDaemon>>>,
    /// The registered service name.
    registered_name: Arc<RwLock<Option<String>>>,
    /// Whether the advertiser is currently active.
    is_running: Arc<RwLock<bool>>,
}

impl CompanionMdnsAdvertiser {
    /// Create a new mDNS advertiser.
    pub fn new() -> Self {
        Self {
            daemon: Arc::new(RwLock::new(None)),
            registered_name: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start advertising the companion server.
    ///
    /// # Arguments
    ///
    /// * `port` - The port the WebSocket server is listening on.
    /// * `instance_name` - Optional custom instance name. If None, generates one.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the service was successfully advertised.
    pub async fn start(&self, port: u16, instance_name: Option<&str>) -> Result<(), MdnsError> {
        // Check if already running
        if *self.is_running.read().await {
            tracing::debug!("mDNS advertiser already running");
            return Ok(());
        }

        // Create mDNS daemon
        let daemon = ServiceDaemon::new()
            .map_err(|e| MdnsError::DaemonCreationFailed(e.to_string()))?;

        // Generate service name
        let service_name = instance_name
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("RustRide-{}", &uuid::Uuid::new_v4().to_string()[..8]));

        // Get local hostname (use a sensible default)
        let hostname = get_hostname();

        // Get local IP for the service
        let local_ip = get_local_ip();

        // Build TXT record properties
        let mut properties = HashMap::new();
        properties.insert("port".to_string(), port.to_string());
        properties.insert("version".to_string(), COMPANION_PROTOCOL_VERSION.to_string());
        properties.insert("protocol".to_string(), "websocket".to_string());

        // Create service info
        let service_info = ServiceInfo::new(
            COMPANION_SERVICE_TYPE,
            &service_name,
            &hostname,
            &local_ip,
            port,
            properties,
        )
        .map_err(|e| MdnsError::RegistrationFailed(e.to_string()))?;

        // Register the service
        daemon
            .register(service_info)
            .map_err(|e| MdnsError::RegistrationFailed(e.to_string()))?;

        // Store state
        *self.daemon.write().await = Some(daemon);
        *self.registered_name.write().await = Some(service_name.clone());
        *self.is_running.write().await = true;

        tracing::info!(
            "Companion server advertised via mDNS: {}.{} on port {}",
            service_name,
            COMPANION_SERVICE_TYPE,
            port
        );

        Ok(())
    }

    /// Stop advertising the companion server.
    ///
    /// Unregisters the service from mDNS and shuts down the daemon.
    pub async fn stop(&self) -> Result<(), MdnsError> {
        if !*self.is_running.read().await {
            return Ok(());
        }

        // Get the daemon and service name
        let daemon = self.daemon.write().await.take();
        let service_name = self.registered_name.write().await.take();

        if let (Some(daemon), Some(name)) = (daemon, service_name) {
            // Build full service name for unregistration
            let full_name = format!("{}.{}", name, COMPANION_SERVICE_TYPE);

            // Attempt to unregister
            if let Err(e) = daemon.unregister(&full_name) {
                tracing::warn!("Failed to unregister mDNS service: {}", e);
            }

            // Shutdown the daemon
            if let Err(e) = daemon.shutdown() {
                tracing::warn!("Failed to shutdown mDNS daemon: {}", e);
            }

            tracing::info!("Companion server mDNS advertisement stopped");
        }

        *self.is_running.write().await = false;

        Ok(())
    }

    /// Check if the advertiser is currently running.
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Get the service type being advertised.
    pub fn service_type(&self) -> &'static str {
        COMPANION_SERVICE_TYPE
    }

    /// Get the protocol version being advertised.
    pub fn protocol_version(&self) -> &'static str {
        COMPANION_PROTOCOL_VERSION
    }
}

impl Default for CompanionMdnsAdvertiser {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CompanionMdnsAdvertiser {
    fn drop(&mut self) {
        // Attempt synchronous cleanup
        // Note: This may not fully work in async context, but we try our best
        if let Ok(mut is_running) = self.is_running.try_write() {
            if *is_running {
                if let Ok(mut daemon) = self.daemon.try_write() {
                    if let Some(d) = daemon.take() {
                        if let Ok(mut name) = self.registered_name.try_write() {
                            if let Some(n) = name.take() {
                                let full_name = format!("{}.{}", n, COMPANION_SERVICE_TYPE);
                                let _ = d.unregister(&full_name);
                            }
                        }
                        let _ = d.shutdown();
                    }
                }
                *is_running = false;
            }
        }
    }
}

/// Get the local hostname for mDNS registration.
fn get_hostname() -> String {
    // Try to get the system hostname using gethostname
    #[cfg(unix)]
    {
        use std::ffi::CStr;
        let mut buf = [0i8; 256];
        unsafe {
            if libc::gethostname(buf.as_mut_ptr(), buf.len()) == 0 {
                if let Ok(name) = CStr::from_ptr(buf.as_ptr()).to_str() {
                    let clean_name = name.trim_end_matches('.');
                    if !clean_name.is_empty() {
                        return format!("{}.local.", clean_name);
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        // On Windows, use environment variable
        if let Ok(name) = std::env::var("COMPUTERNAME") {
            let clean_name = name.trim_end_matches('.');
            if !clean_name.is_empty() {
                return format!("{}.local.", clean_name);
            }
        }
    }

    // Fallback to a generated hostname
    "rustride-companion.local.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_type() {
        assert_eq!(COMPANION_SERVICE_TYPE, "_rustride._tcp.local.");
    }

    #[test]
    fn test_protocol_version() {
        assert_eq!(COMPANION_PROTOCOL_VERSION, "1");
    }

    #[test]
    fn test_advertiser_creation() {
        let advertiser = CompanionMdnsAdvertiser::new();
        assert_eq!(advertiser.service_type(), COMPANION_SERVICE_TYPE);
        assert_eq!(advertiser.protocol_version(), COMPANION_PROTOCOL_VERSION);
    }

    #[test]
    fn test_get_hostname() {
        let hostname = get_hostname();
        assert!(hostname.ends_with(".local."));
        assert!(!hostname.is_empty());
    }

    #[tokio::test]
    async fn test_advertiser_not_running_initially() {
        let advertiser = CompanionMdnsAdvertiser::new();
        assert!(!advertiser.is_running().await);
    }

    #[tokio::test]
    async fn test_stop_when_not_running() {
        let advertiser = CompanionMdnsAdvertiser::new();
        // Should not error when stopping a non-running advertiser
        let result = advertiser.stop().await;
        assert!(result.is_ok());
    }
}
