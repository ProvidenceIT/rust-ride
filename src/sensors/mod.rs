//! Sensor module for BLE device communication.

pub mod ftms;
pub mod manager;
pub mod types;

pub use manager::SensorManager;
pub use types::{
    ConnectionState, DiscoveredSensor, Protocol, SensorConfig, SensorError, SensorEvent,
    SensorReading, SensorState, SensorType,
};
