//! HID Device Management
//!
//! Handles detection and management of USB HID devices.

use super::{find_known_device, HidConfig, HidDeviceEvent, HidError, KNOWN_DEVICES};
use hidapi::HidApi;
use std::collections::HashMap;
use std::ffi::CString;
use std::sync::{Arc, Mutex};
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
    /// Device path for opening (platform-specific)
    pub device_path: Option<String>,
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
            device_path: None,
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
    /// Open device handles keyed by device UUID
    device_handles: Arc<Mutex<HashMap<Uuid, hidapi::HidDevice>>>,
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
            device_handles: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            is_monitoring: Arc::new(RwLock::new(false)),
            _config: config,
        }
    }

    /// Get a reference to the device handles for input reading
    pub fn device_handles(&self) -> Arc<Mutex<HashMap<Uuid, hidapi::HidDevice>>> {
        Arc::clone(&self.device_handles)
    }
}

impl HidDeviceManager for DefaultHidDeviceManager {
    fn scan_devices(&self) -> Vec<HidDevice> {
        tracing::info!("Scanning for HID devices...");

        let mut found = Vec::new();

        // Initialize the HID API
        let api = match HidApi::new() {
            Ok(api) => api,
            Err(e) => {
                tracing::error!("Failed to initialize HID API: {}", e);
                return found;
            }
        };

        // Enumerate all connected HID devices
        for device_info in api.device_list() {
            let vendor_id = device_info.vendor_id();
            let product_id = device_info.product_id();

            // Check if this is a known/supported device
            if let Some(known) = find_known_device(vendor_id, product_id) {
                // Get the serial number if available
                let serial_number = device_info
                    .serial_number()
                    .map(|s| s.to_string());

                // Get the device path for later opening
                let device_path = device_info.path().to_string_lossy().to_string();

                // Create the HID device struct
                let mut device = HidDevice::new(
                    vendor_id,
                    product_id,
                    known.name.to_string(),
                );
                device.serial_number = serial_number;
                device.device_path = Some(device_path);

                tracing::debug!(
                    "Found known HID device: {} (VID:{:04X} PID:{:04X}{})",
                    known.name,
                    vendor_id,
                    product_id,
                    device.serial_number
                        .as_ref()
                        .map(|s| format!(" SN:{}", s))
                        .unwrap_or_default()
                );

                // Avoid adding duplicate devices (same VID:PID:Serial)
                let is_duplicate = found.iter().any(|d: &HidDevice| {
                    d.vendor_id == vendor_id
                        && d.product_id == product_id
                        && d.serial_number == device.serial_number
                });

                if !is_duplicate {
                    found.push(device);
                }
            }
        }

        tracing::info!("Found {} HID device(s)", found.len());

        // Update our internal device list
        if let Ok(mut devices) = self.devices.try_write() {
            *devices = found.clone();
        }

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
        // Check if device is already open
        {
            let handles = self.device_handles.lock().map_err(|e| {
                HidError::OpenFailed(format!("Failed to lock device handles: {}", e))
            })?;
            if handles.contains_key(device_id) {
                return Err(HidError::DeviceInUse);
            }
        }

        // Get device info for opening
        let (vendor_id, product_id, device_path, device_name) = {
            let mut devices = self.devices.write().await;
            let device = devices
                .iter_mut()
                .find(|d| &d.id == device_id)
                .ok_or(HidError::DeviceNotFound(*device_id))?;

            device.status = HidDeviceStatus::Opening;

            (
                device.vendor_id,
                device.product_id,
                device.device_path.clone(),
                device.name.clone(),
            )
        };

        tracing::info!("Opening HID device: {} (VID:{:04X} PID:{:04X})",
            device_name, vendor_id, product_id);

        // Open the device using hidapi
        let device_id_copy = *device_id;
        let handles = Arc::clone(&self.device_handles);

        let open_result = tokio::task::spawn_blocking(move || {
            // Initialize the HID API
            let api = HidApi::new().map_err(|e| {
                HidError::HidApiError(format!("Failed to initialize HID API: {}", e))
            })?;

            // Try to open by path first (more specific), fall back to VID/PID
            let handle = if let Some(path) = device_path {
                let c_path = CString::new(path.clone()).map_err(|e| {
                    HidError::OpenFailed(format!("Invalid device path: {}", e))
                })?;
                api.open_path(&c_path).map_err(|e| {
                    HidError::OpenFailed(format!(
                        "Failed to open device by path '{}': {}. \
                         This may be a permissions issue. On Linux, you may need udev rules. \
                         On Windows, the device may be in use by another application.",
                        path, e
                    ))
                })?
            } else {
                // Fall back to opening by VID/PID (may open any matching device)
                api.open(vendor_id, product_id).map_err(|e| {
                    HidError::OpenFailed(format!(
                        "Failed to open device {:04X}:{:04X}: {}. \
                         This may be a permissions issue or the device may be disconnected.",
                        vendor_id, product_id, e
                    ))
                })?
            };

            // Store the handle
            let mut handles_guard = handles.lock().map_err(|e| {
                HidError::OpenFailed(format!("Failed to lock device handles: {}", e))
            })?;
            handles_guard.insert(device_id_copy, handle);

            Ok::<(), HidError>(())
        })
        .await
        .map_err(|e| HidError::OpenFailed(format!("Task join error: {}", e)))?;

        // Update device status based on result
        {
            let mut devices = self.devices.write().await;
            if let Some(device) = devices.iter_mut().find(|d| &d.id == device_id) {
                match &open_result {
                    Ok(()) => {
                        device.status = HidDeviceStatus::Open;
                        tracing::info!("Successfully opened HID device: {}", device.name);
                    }
                    Err(e) => {
                        device.status = HidDeviceStatus::Error(e.to_string());
                        tracing::error!("Failed to open HID device {}: {}", device.name, e);
                    }
                }
            }
        }

        if open_result.is_ok() {
            let _ = self.event_tx.send(HidDeviceEvent::DeviceOpened(*device_id));
        }

        open_result
    }

    async fn close_device(&self, device_id: &Uuid) -> Result<(), HidError> {
        // Get device name for logging
        let device_name = {
            let devices = self.devices.read().await;
            devices
                .iter()
                .find(|d| &d.id == device_id)
                .map(|d| d.name.clone())
                .ok_or(HidError::DeviceNotFound(*device_id))?
        };

        tracing::info!("Closing HID device: {}", device_name);

        // Remove the handle from storage (dropping it closes the device)
        let was_open = {
            let mut handles = self.device_handles.lock().map_err(|e| {
                HidError::OpenFailed(format!("Failed to lock device handles: {}", e))
            })?;
            handles.remove(device_id).is_some()
        };

        if !was_open {
            tracing::warn!("Device {} was not open", device_name);
            return Err(HidError::DeviceNotOpen);
        }

        // Update device status
        {
            let mut devices = self.devices.write().await;
            if let Some(device) = devices.iter_mut().find(|d| &d.id == device_id) {
                device.status = HidDeviceStatus::Detected;
            }
        }

        tracing::info!("Successfully closed HID device: {}", device_name);
        let _ = self.event_tx.send(HidDeviceEvent::DeviceClosed(*device_id));

        Ok(())
    }

    fn is_open(&self, device_id: &Uuid) -> bool {
        // Check actual device handles for authoritative answer
        if let Ok(handles) = self.device_handles.lock() {
            return handles.contains_key(device_id);
        }
        // Fall back to status check if lock fails
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
