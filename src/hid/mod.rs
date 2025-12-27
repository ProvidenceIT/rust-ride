//! USB HID Device Integration
//!
//! Provides support for USB HID devices like Stream Deck and USB buttons.

pub mod actions;
pub mod device;
pub mod mapping;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::broadcast;
use uuid::Uuid;

// Re-export main types
pub use actions::{ActionExecutor, ButtonAction};
pub use device::{DefaultHidDeviceManager, HidDevice, HidDeviceManager, HidDeviceStatus};
pub use mapping::{ButtonInputHandler, ButtonMapping, DefaultButtonInputHandler};

/// HID-related errors
#[derive(Debug, Error)]
pub enum HidError {
    #[error("Device not found: {0}")]
    DeviceNotFound(Uuid),

    #[error("Device not open")]
    DeviceNotOpen,

    #[error("Failed to open device: {0}")]
    OpenFailed(String),

    #[error("Read error: {0}")]
    ReadError(String),

    #[error("HID API error: {0}")]
    HidApiError(String),

    #[error("Device already in use")]
    DeviceInUse,

    #[error("Unsupported device")]
    UnsupportedDevice,
}

/// HID configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HidConfig {
    /// Whether HID support is enabled
    pub enabled: bool,
    /// Device configurations
    pub devices: Vec<HidDeviceConfig>,
}

impl Default for HidConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            devices: Vec::new(),
        }
    }
}

/// Per-device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HidDeviceConfig {
    /// Device ID
    pub device_id: Uuid,
    /// USB vendor ID
    pub vendor_id: u16,
    /// USB product ID
    pub product_id: u16,
    /// Display name
    pub name: String,
    /// Whether device is enabled
    pub enabled: bool,
    /// Button mappings
    pub mappings: Vec<ButtonMappingConfig>,
}

/// Saved button mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonMappingConfig {
    /// Button code from device
    pub button_code: u8,
    /// Action to execute
    pub action: ButtonAction,
    /// Optional label for the button
    pub label: Option<String>,
}

/// HID device events
#[derive(Debug, Clone)]
pub enum HidDeviceEvent {
    /// Device connected
    DeviceConnected(HidDevice),
    /// Device disconnected
    DeviceDisconnected(Uuid),
    /// Device opened for input
    DeviceOpened(Uuid),
    /// Device closed
    DeviceClosed(Uuid),
    /// Error occurred
    Error {
        device_id: Option<Uuid>,
        error: String,
    },
}

/// Known device profiles for automatic configuration
#[derive(Debug, Clone)]
pub struct KnownDevice {
    /// USB vendor ID
    pub vendor_id: u16,
    /// USB product ID
    pub product_id: u16,
    /// Device name
    pub name: &'static str,
    /// Number of buttons
    pub button_count: u8,
    /// Default button mappings
    pub default_mappings: &'static [(u8, ButtonAction)],
}

/// Known devices for automatic detection
pub const KNOWN_DEVICES: &[KnownDevice] = &[
    KnownDevice {
        vendor_id: 0x0FD9,
        product_id: 0x0060,
        name: "Elgato Stream Deck",
        button_count: 15,
        default_mappings: &[],
    },
    KnownDevice {
        vendor_id: 0x0FD9,
        product_id: 0x006C,
        name: "Elgato Stream Deck Mini",
        button_count: 6,
        default_mappings: &[],
    },
    KnownDevice {
        vendor_id: 0x0FD9,
        product_id: 0x006D,
        name: "Elgato Stream Deck XL",
        button_count: 32,
        default_mappings: &[],
    },
    KnownDevice {
        vendor_id: 0x0FD9,
        product_id: 0x0080,
        name: "Elgato Stream Deck MK.2",
        button_count: 15,
        default_mappings: &[],
    },
    KnownDevice {
        vendor_id: 0x0FD9,
        product_id: 0x0086,
        name: "Elgato Stream Deck Pedal",
        button_count: 3,
        default_mappings: &[],
    },
];

/// Find known device by VID/PID
pub fn find_known_device(vendor_id: u16, product_id: u16) -> Option<&'static KnownDevice> {
    KNOWN_DEVICES
        .iter()
        .find(|d| d.vendor_id == vendor_id && d.product_id == product_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = HidConfig::default();
        assert!(config.enabled);
        assert!(config.devices.is_empty());
    }

    #[test]
    fn test_find_known_device() {
        let device = find_known_device(0x0FD9, 0x0060);
        assert!(device.is_some());
        assert_eq!(device.unwrap().name, "Elgato Stream Deck");

        let unknown = find_known_device(0x1234, 0x5678);
        assert!(unknown.is_none());
    }
}
