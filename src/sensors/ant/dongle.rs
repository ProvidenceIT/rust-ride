//! ANT+ USB Dongle Management
//!
//! Handles detection, initialization, and management of ANT+ USB dongles.

use super::{AntConfig, AntError, AntEvent};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// Known ANT+ dongle vendor/product IDs
pub const KNOWN_DONGLES: &[(u16, u16, &str)] = &[
    (0x0FCF, 0x1008, "Garmin USB ANT Stick"),
    (0x0FCF, 0x1009, "Garmin USB2 ANT Stick"),
    (0x0FCF, 0x1004, "Dynastream USB ANT Stick"),
    (0x0FCF, 0x1006, "Dynastream USB ANT Stick 2"),
];

/// Status of an ANT+ dongle
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DongleStatus {
    /// Dongle detected but not initialized
    Detected,
    /// Dongle is initializing
    Initializing,
    /// Dongle is ready for use
    Ready,
    /// Dongle has an error
    Error(String),
    /// Dongle was disconnected
    Disconnected,
}

/// Represents an ANT+ USB dongle
#[derive(Debug, Clone)]
pub struct AntDongle {
    /// Unique identifier for this dongle instance
    pub id: Uuid,
    /// USB vendor ID
    pub vendor_id: u16,
    /// USB product ID
    pub product_id: u16,
    /// Product name
    pub name: String,
    /// Serial number if available
    pub serial_number: Option<String>,
    /// Current status
    pub status: DongleStatus,
    /// Number of available channels (typically 8)
    pub total_channels: u8,
    /// Number of channels currently in use
    pub used_channels: u8,
}

impl AntDongle {
    /// Create a new dongle instance
    pub fn new(vendor_id: u16, product_id: u16, name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            vendor_id,
            product_id,
            name,
            serial_number: None,
            status: DongleStatus::Detected,
            total_channels: 8,
            used_channels: 0,
        }
    }

    /// Check if dongle is ready for use
    pub fn is_ready(&self) -> bool {
        matches!(self.status, DongleStatus::Ready)
    }

    /// Get number of available channels
    pub fn available_channels(&self) -> u8 {
        self.total_channels.saturating_sub(self.used_channels)
    }
}

/// Trait for managing ANT+ USB dongles
pub trait AntDongleManager: Send + Sync {
    /// Scan for connected ANT+ dongles
    fn scan_dongles(&self) -> Vec<AntDongle>;

    /// Initialize a specific dongle for use
    fn initialize_dongle(
        &self,
        dongle_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<(), AntError>> + Send;

    /// Release a dongle and close all channels
    fn release_dongle(
        &self,
        dongle_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<(), AntError>> + Send;

    /// Get current status of a dongle
    fn get_dongle_status(&self, dongle_id: &Uuid) -> Option<DongleStatus>;

    /// Get all known dongles
    fn get_dongles(&self) -> Vec<AntDongle>;

    /// Subscribe to dongle events
    fn subscribe_events(&self) -> broadcast::Receiver<AntEvent>;
}

/// Default implementation of dongle manager
pub struct DefaultDongleManager {
    dongles: Arc<RwLock<Vec<AntDongle>>>,
    event_tx: broadcast::Sender<AntEvent>,
    _config: AntConfig,
}

impl DefaultDongleManager {
    /// Create a new dongle manager
    pub fn new(config: AntConfig) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            dongles: Arc::new(RwLock::new(Vec::new())),
            event_tx,
            _config: config,
        }
    }
}

impl AntDongleManager for DefaultDongleManager {
    fn scan_dongles(&self) -> Vec<AntDongle> {
        // TODO: Implement actual USB scanning using libusb
        // For now, return empty list
        tracing::info!("Scanning for ANT+ dongles...");

        let mut found = Vec::new();

        // This would use libusb to enumerate USB devices
        // and match against KNOWN_DONGLES
        for (vid, pid, name) in KNOWN_DONGLES {
            // Placeholder: In real implementation, check if device exists
            tracing::debug!(
                "Would check for dongle: {} (VID: {:04X}, PID: {:04X})",
                name,
                vid,
                pid
            );
        }

        found
    }

    async fn initialize_dongle(&self, dongle_id: &Uuid) -> Result<(), AntError> {
        let mut dongles = self.dongles.write().await;

        if let Some(dongle) = dongles.iter_mut().find(|d| &d.id == dongle_id) {
            tracing::info!("Initializing dongle: {}", dongle.name);
            dongle.status = DongleStatus::Initializing;

            // TODO: Actual initialization sequence:
            // 1. Open USB device
            // 2. Reset ANT+ chip
            // 3. Set network key
            // 4. Verify response

            dongle.status = DongleStatus::Ready;

            let _ = self.event_tx.send(AntEvent::DongleConnected {
                dongle_id: *dongle_id,
            });

            Ok(())
        } else {
            Err(AntError::DeviceNotFound(*dongle_id))
        }
    }

    async fn release_dongle(&self, dongle_id: &Uuid) -> Result<(), AntError> {
        let mut dongles = self.dongles.write().await;

        if let Some(dongle) = dongles.iter_mut().find(|d| &d.id == dongle_id) {
            tracing::info!("Releasing dongle: {}", dongle.name);

            // TODO: Close all channels and release USB device

            dongle.status = DongleStatus::Disconnected;

            let _ = self.event_tx.send(AntEvent::DongleDisconnected {
                dongle_id: *dongle_id,
            });

            Ok(())
        } else {
            Err(AntError::DeviceNotFound(*dongle_id))
        }
    }

    fn get_dongle_status(&self, dongle_id: &Uuid) -> Option<DongleStatus> {
        // Use try_read to avoid blocking
        if let Ok(dongles) = self.dongles.try_read() {
            dongles
                .iter()
                .find(|d| &d.id == dongle_id)
                .map(|d| d.status.clone())
        } else {
            None
        }
    }

    fn get_dongles(&self) -> Vec<AntDongle> {
        if let Ok(dongles) = self.dongles.try_read() {
            dongles.clone()
        } else {
            Vec::new()
        }
    }

    fn subscribe_events(&self) -> broadcast::Receiver<AntEvent> {
        self.event_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dongle_creation() {
        let dongle = AntDongle::new(0x0FCF, 0x1008, "Test Dongle".to_string());
        assert_eq!(dongle.vendor_id, 0x0FCF);
        assert_eq!(dongle.product_id, 0x1008);
        assert_eq!(dongle.status, DongleStatus::Detected);
        assert_eq!(dongle.available_channels(), 8);
    }

    #[test]
    fn test_known_dongles() {
        assert!(!KNOWN_DONGLES.is_empty());
        for (vid, pid, name) in KNOWN_DONGLES {
            assert_eq!(*vid, 0x0FCF); // All Dynastream/Garmin dongles
            assert!(!name.is_empty());
        }
    }
}
