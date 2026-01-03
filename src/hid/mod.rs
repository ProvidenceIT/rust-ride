//! USB HID Device Integration
//!
//! Provides support for USB HID devices like Stream Deck and USB buttons.

pub mod actions;
pub mod device;
pub mod executor;
pub mod generic;
pub mod input;
pub mod mapping;
pub mod persistence;
pub mod streamdeck;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// Re-export main types
pub use actions::{ActionContext, ActionError, ActionExecutor, ActionInfo, ActionResult, ButtonAction};
pub use device::{DefaultHidDeviceManager, HidDevice, HidDeviceManager, HidDeviceStatus};
pub use executor::{
    AppContext, DefaultActionExecutor, ExecutorEvent, LapMarker, NavigationTarget,
};
pub use generic::{
    detect_report_format, find_generic_device_profile, GenericDeviceConfig, GenericDeviceProfile,
    GenericHidParser, GenericReportFormat, KNOWN_GENERIC_DEVICES,
};
pub use input::{HidDeviceType, HidInputReader, OpenDeviceInfo};
pub use mapping::{
    ButtonActionEvent, ButtonInputHandler, ButtonMapping, DefaultButtonInputHandler,
    RawButtonEvent,
};
pub use persistence::{
    config_to_stored_device, config_to_stored_mapping, load_hid_config_from_db,
    new_device_config, new_device_config_without_defaults, save_hid_config_to_db,
    stored_device_to_config, stored_mapping_to_config,
};
pub use streamdeck::{StreamDeckModel, StreamDeckParser};

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
    /// Delay in milliseconds before attempting to reconnect a device
    #[serde(default = "default_reconnect_delay_ms")]
    pub reconnect_delay_ms: u64,
    /// Whether auto-reconnect is enabled
    #[serde(default = "default_auto_reconnect")]
    pub auto_reconnect: bool,
}

fn default_reconnect_delay_ms() -> u64 {
    1000
}

fn default_auto_reconnect() -> bool {
    true
}

impl Default for HidConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            devices: Vec::new(),
            reconnect_delay_ms: default_reconnect_delay_ms(),
            auto_reconnect: default_auto_reconnect(),
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
    /// Device automatically reconnected and opened
    DeviceReconnected(Uuid),
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

/// Default mappings for Stream Deck (15 buttons, 3x5 grid)
/// Layout optimized for cycling workout control
const STREAM_DECK_15_DEFAULTS: &[(u8, ButtonAction)] = &[
    // Top row - Primary ride controls
    (0, ButtonAction::PauseResume),      // Pause/Resume ride
    (1, ButtonAction::AddLapMarker),     // Mark a lap
    (2, ButtonAction::SkipInterval),     // Skip to next interval
    // Row 2 - Workout controls
    (5, ButtonAction::RestartInterval),  // Restart current interval
    // Row 3 - Audio controls
    (10, ButtonAction::VolumeUp),        // Volume up
    (11, ButtonAction::VolumeDown),      // Volume down
    (12, ButtonAction::MuteToggle),      // Mute/unmute
    // Row 4 - Fan controls
    (13, ButtonAction::FanSpeedUp),      // Increase fan speed
    (14, ButtonAction::FanSpeedDown),    // Decrease fan speed
];

/// Default mappings for Stream Deck Mini (6 buttons, 2x3 grid)
/// Compact layout with most essential controls
const STREAM_DECK_MINI_DEFAULTS: &[(u8, ButtonAction)] = &[
    // Top row
    (0, ButtonAction::PauseResume),      // Pause/Resume ride
    (1, ButtonAction::AddLapMarker),     // Mark a lap
    (2, ButtonAction::SkipInterval),     // Skip to next interval
    // Bottom row
    (3, ButtonAction::VolumeUp),         // Volume up
    (4, ButtonAction::VolumeDown),       // Volume down
    (5, ButtonAction::FanToggle),        // Toggle fan on/off
];

/// Default mappings for Stream Deck XL (32 buttons, 4x8 grid)
/// Extended layout with full control suite
const STREAM_DECK_XL_DEFAULTS: &[(u8, ButtonAction)] = &[
    // Top row - Primary ride controls
    (0, ButtonAction::PauseResume),      // Pause/Resume ride
    (1, ButtonAction::AddLapMarker),     // Mark a lap
    (2, ButtonAction::SkipInterval),     // Skip to next interval
    (3, ButtonAction::RestartInterval),  // Restart current interval
    // Row 2 - Navigation
    (8, ButtonAction::ShowMetrics),      // Show metrics view
    (9, ButtonAction::ShowMap),          // Show map view
    (10, ButtonAction::ShowWorkout),     // Show workout view
    (11, ButtonAction::ToggleFullscreen),// Toggle fullscreen
    // Row 3 - Audio controls
    (16, ButtonAction::VolumeUp),        // Volume up
    (17, ButtonAction::VolumeDown),      // Volume down
    (18, ButtonAction::MuteToggle),      // Mute/unmute
    // Row 4 - Fan controls
    (24, ButtonAction::FanSpeedUp),      // Increase fan speed
    (25, ButtonAction::FanSpeedDown),    // Decrease fan speed
    (26, ButtonAction::FanToggle),       // Toggle fan on/off
];

/// Default mappings for Stream Deck Pedal (3 buttons - foot pedal)
/// Hands-free controls for intense workout moments
const STREAM_DECK_PEDAL_DEFAULTS: &[(u8, ButtonAction)] = &[
    (0, ButtonAction::AddLapMarker),     // Left pedal - Mark a lap
    (1, ButtonAction::PauseResume),      // Center pedal - Pause/Resume
    (2, ButtonAction::SkipInterval),     // Right pedal - Skip interval
];

/// Known devices for automatic detection
pub const KNOWN_DEVICES: &[KnownDevice] = &[
    KnownDevice {
        vendor_id: 0x0FD9,
        product_id: 0x0060,
        name: "Elgato Stream Deck",
        button_count: 15,
        default_mappings: STREAM_DECK_15_DEFAULTS,
    },
    KnownDevice {
        vendor_id: 0x0FD9,
        product_id: 0x006C,
        name: "Elgato Stream Deck Mini",
        button_count: 6,
        default_mappings: STREAM_DECK_MINI_DEFAULTS,
    },
    KnownDevice {
        vendor_id: 0x0FD9,
        product_id: 0x006D,
        name: "Elgato Stream Deck XL",
        button_count: 32,
        default_mappings: STREAM_DECK_XL_DEFAULTS,
    },
    KnownDevice {
        vendor_id: 0x0FD9,
        product_id: 0x0080,
        name: "Elgato Stream Deck MK.2",
        button_count: 15,
        default_mappings: STREAM_DECK_15_DEFAULTS,
    },
    KnownDevice {
        vendor_id: 0x0FD9,
        product_id: 0x0086,
        name: "Elgato Stream Deck Pedal",
        button_count: 3,
        default_mappings: STREAM_DECK_PEDAL_DEFAULTS,
    },
];

/// Find known device by VID/PID
pub fn find_known_device(vendor_id: u16, product_id: u16) -> Option<&'static KnownDevice> {
    KNOWN_DEVICES
        .iter()
        .find(|d| d.vendor_id == vendor_id && d.product_id == product_id)
}

/// Get default button mappings for a known device as ButtonMappingConfig.
///
/// Returns an empty Vec if the device is unknown or has no default mappings.
/// This is used to populate mappings for devices when first connected.
pub fn get_default_mappings(vendor_id: u16, product_id: u16) -> Vec<ButtonMappingConfig> {
    find_known_device(vendor_id, product_id)
        .map(|known| {
            known
                .default_mappings
                .iter()
                .map(|(button_code, action)| ButtonMappingConfig {
                    button_code: *button_code,
                    action: action.clone(),
                    label: Some(action.display_name().to_string()),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Check if a device has default mappings configured.
pub fn has_default_mappings(vendor_id: u16, product_id: u16) -> bool {
    find_known_device(vendor_id, product_id)
        .map(|known| !known.default_mappings.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = HidConfig::default();
        assert!(config.enabled);
        assert!(config.devices.is_empty());
        assert!(config.auto_reconnect);
        assert_eq!(config.reconnect_delay_ms, 1000);
    }

    #[test]
    fn test_find_known_device() {
        let device = find_known_device(0x0FD9, 0x0060);
        assert!(device.is_some());
        assert_eq!(device.unwrap().name, "Elgato Stream Deck");

        let unknown = find_known_device(0x1234, 0x5678);
        assert!(unknown.is_none());
    }

    #[test]
    fn test_stream_deck_has_default_mappings() {
        // Stream Deck Original should have defaults
        let device = find_known_device(0x0FD9, 0x0060).unwrap();
        assert!(!device.default_mappings.is_empty());
        assert_eq!(device.default_mappings.len(), 9); // 9 default mappings

        // First mapping should be PauseResume on button 0
        assert_eq!(device.default_mappings[0].0, 0);
        assert_eq!(device.default_mappings[0].1, ButtonAction::PauseResume);
    }

    #[test]
    fn test_stream_deck_mini_defaults() {
        let device = find_known_device(0x0FD9, 0x006C).unwrap();
        assert_eq!(device.default_mappings.len(), 6); // All 6 buttons mapped
    }

    #[test]
    fn test_stream_deck_xl_defaults() {
        let device = find_known_device(0x0FD9, 0x006D).unwrap();
        assert_eq!(device.default_mappings.len(), 14); // Extended mappings
    }

    #[test]
    fn test_stream_deck_pedal_defaults() {
        let device = find_known_device(0x0FD9, 0x0086).unwrap();
        assert_eq!(device.default_mappings.len(), 3); // All 3 pedals mapped

        // Verify foot pedal mapping makes sense (hands-free controls)
        assert_eq!(device.default_mappings[0].1, ButtonAction::AddLapMarker); // Left
        assert_eq!(device.default_mappings[1].1, ButtonAction::PauseResume);  // Center
        assert_eq!(device.default_mappings[2].1, ButtonAction::SkipInterval); // Right
    }

    #[test]
    fn test_get_default_mappings_known_device() {
        let mappings = get_default_mappings(0x0FD9, 0x0060);
        assert!(!mappings.is_empty());

        // Check first mapping
        let first = &mappings[0];
        assert_eq!(first.button_code, 0);
        assert_eq!(first.action, ButtonAction::PauseResume);
        assert_eq!(first.label, Some("Pause/Resume".to_string()));
    }

    #[test]
    fn test_get_default_mappings_unknown_device() {
        let mappings = get_default_mappings(0x1234, 0x5678);
        assert!(mappings.is_empty());
    }

    #[test]
    fn test_has_default_mappings() {
        // Known devices have defaults
        assert!(has_default_mappings(0x0FD9, 0x0060)); // Stream Deck
        assert!(has_default_mappings(0x0FD9, 0x006C)); // Stream Deck Mini
        assert!(has_default_mappings(0x0FD9, 0x0086)); // Stream Deck Pedal

        // Unknown devices don't have defaults
        assert!(!has_default_mappings(0x1234, 0x5678));
    }
}
