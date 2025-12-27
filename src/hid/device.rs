//! HID Device Management
//!
//! Handles detection and management of USB HID devices.

use super::{HidConfig, HidDeviceEvent, HidError, KNOWN_DEVICES};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// Represents a USB HID device
#[derive(Debug, Clone)]
pub struct HidDevice {
    /// Unique identifier
    pub id: Uuid,
    /// USB vendor ID
    pub vendor_id: u16,
    /// USB product ID
    pub product_id: u16,
    /// Device name
    pub name: String,
    /// Serial number if available
    pub serial_number: Option<String>,
    /// Number of buttons (if known)
    pub button_count: Option<u8>,
    /// Current status
    pub status: HidDeviceStatus,
    /// Whether this is a known/supported device
    pub is_known: bool,
}

/// Device status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HidDeviceStatus {
    /// Device detected but not opened
    Detected,
    /// Device is opening
    Opening,
    /// Device is open and ready
    Open,
    /// Device has an error
    Error(String),
    /// Device was disconnected
    Disconnected,
}

impl HidDevice {
    /// Create a new HID device
    pub fn new(vendor_id: u16, product_id: u16, name: String) -> Self {
        let known = KNOWN_DEVICES
            .iter()
            .find(|d| d.vendor_id == vendor_id && d.product_id == product_id);

        Self {
            id: Uuid::new_v4(),
            vendor_id,
            product_id,
            name,
            serial_number: None,
            button_count: known.map(|d| d.button_count),
            status: HidDeviceStatus::Detected,
            is_known: known.is_some(),
        }
    }

    /// Check if device is open
    pub fn is_open(&self) -> bool {
        matches!(self.status, HidDeviceStatus::Open)
    }

    /// Get device path for display
    pub fn display_path(&self) -> String {
        format!("{:04X}:{:04X}", self.vendor_id, self.product_id)
    }
}

/// Trait for HID device management
pub trait HidDeviceManager: Send + Sync {
    /// Scan for connected HID devices
    fn scan_devices(&self) -> Vec<HidDevice>;

    /// Start monitoring for device connect/disconnect
    fn start_monitoring(&self) -> impl std::future::Future<Output = Result<(), HidError>> + Send;

    /// Stop monitoring
    fn stop_monitoring(&self);

    /// Get device by ID
    fn get_device(&self, device_id: &Uuid) -> Option<HidDevice>;

    /// Open device for input
    fn open_device(
        &self,
        device_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<(), HidError>> + Send;

    /// Close device
    fn close_device(
        &self,
        device_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<(), HidError>> + Send;

    /// Check if device is open
    fn is_open(&self, device_id: &Uuid) -> bool;

    /// Enable/disable device
    fn set_enabled(&self, device_id: &Uuid, enabled: bool);

    /// Subscribe to device events
    fn subscribe_events(&self) -> broadcast::Receiver<HidDeviceEvent>;
}

/// Default HID device manager implementation
pub struct DefaultHidDeviceManager {
    devices: Arc<RwLock<Vec<HidDevice>>>,
    event_tx: broadcast::Sender<HidDeviceEvent>,
    is_monitoring: Arc<RwLock<bool>>,
    _config: HidConfig,
}

impl DefaultHidDeviceManager {
    /// Create a new device manager
    pub fn new(config: HidConfig) -> Self {
        let (event_tx, _) = broadcast::channel(100);

        Self {
            devices: Arc::new(RwLock::new(Vec::new())),
            event_tx,
            is_monitoring: Arc::new(RwLock::new(false)),
            _config: config,
        }
    }
}

impl HidDeviceManager for DefaultHidDeviceManager {
    fn scan_devices(&self) -> Vec<HidDevice> {
        tracing::info!("Scanning for HID devices...");

        let mut found = Vec::new();

        // TODO: Use hidapi to enumerate devices
        // let api = HidApi::new().unwrap();
        // for device in api.device_list() {
        //     if let Some(known) = find_known_device(device.vendor_id(), device.product_id()) {
        //         found.push(HidDevice::new(
        //             device.vendor_id(),
        //             device.product_id(),
        //             known.name.to_string(),
        //         ));
        //     }
        // }

        tracing::debug!("Found {} HID devices", found.len());

        found
    }

    async fn start_monitoring(&self) -> Result<(), HidError> {
        *self.is_monitoring.write().await = true;

        tracing::info!("Started HID device monitoring");

        // TODO: Start background task to monitor for device changes
        // This would use platform-specific APIs or polling

        Ok(())
    }

    fn stop_monitoring(&self) {
        if let Ok(mut monitoring) = self.is_monitoring.try_write() {
            *monitoring = false;
        }

        tracing::info!("Stopped HID device monitoring");
    }

    fn get_device(&self, device_id: &Uuid) -> Option<HidDevice> {
        self.devices
            .try_read()
            .ok()?
            .iter()
            .find(|d| &d.id == device_id)
            .cloned()
    }

    async fn open_device(&self, device_id: &Uuid) -> Result<(), HidError> {
        let mut devices = self.devices.write().await;

        let device = devices
            .iter_mut()
            .find(|d| &d.id == device_id)
            .ok_or(HidError::DeviceNotFound(*device_id))?;

        device.status = HidDeviceStatus::Opening;

        tracing::info!("Opening HID device: {}", device.name);

        // TODO: Actually open the device using hidapi
        // let api = HidApi::new()?;
        // let handle = api.open(device.vendor_id, device.product_id)?;

        device.status = HidDeviceStatus::Open;

        let _ = self.event_tx.send(HidDeviceEvent::DeviceOpened(*device_id));

        Ok(())
    }

    async fn close_device(&self, device_id: &Uuid) -> Result<(), HidError> {
        let mut devices = self.devices.write().await;

        let device = devices
            .iter_mut()
            .find(|d| &d.id == device_id)
            .ok_or(HidError::DeviceNotFound(*device_id))?;

        tracing::info!("Closing HID device: {}", device.name);

        // TODO: Close the device handle

        device.status = HidDeviceStatus::Detected;

        let _ = self.event_tx.send(HidDeviceEvent::DeviceClosed(*device_id));

        Ok(())
    }

    fn is_open(&self, device_id: &Uuid) -> bool {
        self.devices
            .try_read()
            .ok()
            .and_then(|d| {
                d.iter()
                    .find(|dev| &dev.id == device_id)
                    .map(|dev| dev.is_open())
            })
            .unwrap_or(false)
    }

    fn set_enabled(&self, _device_id: &Uuid, _enabled: bool) {
        // TODO: Update device enabled state in config
    }

    fn subscribe_events(&self) -> broadcast::Receiver<HidDeviceEvent> {
        self.event_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_creation() {
        let device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());

        assert!(device.is_known);
        assert_eq!(device.button_count, Some(15));
        assert!(!device.is_open());
    }

    #[test]
    fn test_unknown_device() {
        let device = HidDevice::new(0x1234, 0x5678, "Unknown Device".to_string());

        assert!(!device.is_known);
        assert!(device.button_count.is_none());
    }

    #[test]
    fn test_display_path() {
        let device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
        assert_eq!(device.display_path(), "0FD9:0060");
    }
}
