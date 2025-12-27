//! ANT+ Protocol Support
//!
//! This module provides ANT+ sensor connectivity for legacy fitness devices.
//! Requires an ANT+ USB dongle for communication.

pub mod channels;
pub mod dongle;
pub mod duplex;
pub mod profiles;

use std::sync::Arc;
use thiserror::Error;
use tokio::sync::broadcast;
use uuid::Uuid;

// Re-export main types
pub use channels::{AntChannel, AntChannelManager, ChannelStatus};
pub use dongle::{AntDongle, AntDongleManager, DongleStatus};
pub use duplex::{DualProtocolBinding, DualProtocolDetector};

/// Errors that can occur during ANT+ operations
#[derive(Debug, Error)]
pub enum AntError {
    #[error("No ANT+ dongle found")]
    NoDongleFound,

    #[error("Dongle initialization failed: {0}")]
    DongleInitFailed(String),

    #[error("USB error: {0}")]
    UsbError(String),

    #[error("Channel allocation failed: {0}")]
    ChannelAllocationFailed(String),

    #[error("Search timeout for device type {0}")]
    SearchTimeout(u8),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(Uuid),
}

/// ANT+ device types supported by this implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AntDeviceType {
    /// Heart Rate Monitor (Device Type 120)
    HeartRate,
    /// Power Meter (Device Type 11)
    Power,
    /// Speed/Cadence Sensor (Device Type 121, 122, 123)
    SpeedCadence,
    /// Fitness Equipment (FE-C) (Device Type 17)
    FitnessEquipment,
    /// Unknown device type
    Unknown(u8),
}

impl AntDeviceType {
    /// Get the ANT+ device type number
    pub fn device_type_number(&self) -> u8 {
        match self {
            AntDeviceType::HeartRate => 120,
            AntDeviceType::Power => 11,
            AntDeviceType::SpeedCadence => 121, // Combined S&C
            AntDeviceType::FitnessEquipment => 17,
            AntDeviceType::Unknown(n) => *n,
        }
    }

    /// Create from device type number
    pub fn from_number(n: u8) -> Self {
        match n {
            120 => AntDeviceType::HeartRate,
            11 => AntDeviceType::Power,
            121 | 122 | 123 => AntDeviceType::SpeedCadence,
            17 => AntDeviceType::FitnessEquipment,
            _ => AntDeviceType::Unknown(n),
        }
    }
}

/// Events from the ANT+ subsystem
#[derive(Debug, Clone)]
pub enum AntEvent {
    /// Dongle connected
    DongleConnected { dongle_id: Uuid },
    /// Dongle disconnected
    DongleDisconnected { dongle_id: Uuid },
    /// Device discovered during search
    DeviceDiscovered {
        device_id: u16,
        device_type: AntDeviceType,
        transmission_type: u8,
    },
    /// Device paired successfully
    DevicePaired { device_id: u16, channel: u8 },
    /// Device lost connection
    DeviceLost { device_id: u16 },
    /// Data received from device
    DataReceived {
        device_id: u16,
        device_type: AntDeviceType,
        data: Vec<u8>,
    },
    /// Error occurred
    Error { message: String },
}

/// Configuration for ANT+ subsystem
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AntConfig {
    /// Whether ANT+ is enabled
    pub enabled: bool,
    /// Auto-reconnect on disconnect
    pub auto_reconnect: bool,
    /// Search timeout in seconds
    pub search_timeout_secs: u32,
    /// Device types to search for
    pub enabled_device_types: Vec<AntDeviceType>,
}

impl Default for AntConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_reconnect: true,
            search_timeout_secs: 30,
            enabled_device_types: vec![
                AntDeviceType::HeartRate,
                AntDeviceType::Power,
                AntDeviceType::FitnessEquipment,
                AntDeviceType::SpeedCadence,
            ],
        }
    }
}
