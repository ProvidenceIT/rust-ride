//! Muscle Oxygen (SmO2) Sensor Support
//!
//! T111: SmO2Reading and MuscleLocation types
//! T112: SmO2Provider trait implementation
//! T113: Moxy BLE GATT service parsing

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// SmO2-related errors
#[derive(Debug, Error)]
pub enum SmO2Error {
    #[error("Sensor not found: {0}")]
    SensorNotFound(Uuid),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Reading failed: {0}")]
    ReadingFailed(String),

    #[error("Sensor not connected")]
    NotConnected,

    #[error("Unsupported sensor: {0}")]
    UnsupportedSensor(String),
}

/// T111: Muscle location for SmO2 sensor placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum MuscleLocation {
    /// Left quadriceps
    #[default]
    LeftQuad,
    /// Right quadriceps
    RightQuad,
    /// Left calf/gastrocnemius
    LeftCalf,
    /// Right calf/gastrocnemius
    RightCalf,
    /// Left vastus lateralis (outer quad)
    LeftVastusLateralis,
    /// Right vastus lateralis (outer quad)
    RightVastusLateralis,
    /// Left glute
    LeftGlute,
    /// Right glute
    RightGlute,
    /// Other/custom location
    Other,
}

impl MuscleLocation {
    /// Get display name for the location.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::LeftQuad => "Left Quad",
            Self::RightQuad => "Right Quad",
            Self::LeftCalf => "Left Calf",
            Self::RightCalf => "Right Calf",
            Self::LeftVastusLateralis => "Left V. Lateralis",
            Self::RightVastusLateralis => "Right V. Lateralis",
            Self::LeftGlute => "Left Glute",
            Self::RightGlute => "Right Glute",
            Self::Other => "Other",
        }
    }

    /// Get short name for compact display.
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::LeftQuad => "L.Quad",
            Self::RightQuad => "R.Quad",
            Self::LeftCalf => "L.Calf",
            Self::RightCalf => "R.Calf",
            Self::LeftVastusLateralis => "L.VL",
            Self::RightVastusLateralis => "R.VL",
            Self::LeftGlute => "L.Glu",
            Self::RightGlute => "R.Glu",
            Self::Other => "Oth",
        }
    }
}

/// T111: Muscle oxygen reading from a SmO2 sensor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmO2Reading {
    /// Sensor ID
    pub sensor_id: Uuid,
    /// Muscle location
    pub location: MuscleLocation,
    /// Muscle oxygen saturation percentage (0-100)
    pub smo2_percent: f32,
    /// Total hemoglobin concentration (optional, g/dL)
    pub thb: Option<f32>,
    /// Oxygenated hemoglobin (optional, g/dL)
    pub hbo2: Option<f32>,
    /// Deoxygenated hemoglobin (optional, g/dL)
    pub hhb: Option<f32>,
    /// Reading timestamp
    pub timestamp: DateTime<Utc>,
    /// Signal quality (0-100, if available)
    pub signal_quality: Option<u8>,
}

impl SmO2Reading {
    /// Create a new SmO2 reading.
    pub fn new(sensor_id: Uuid, location: MuscleLocation, smo2_percent: f32) -> Self {
        Self {
            sensor_id,
            location,
            smo2_percent,
            thb: None,
            hbo2: None,
            hhb: None,
            timestamp: Utc::now(),
            signal_quality: None,
        }
    }

    /// Set total hemoglobin.
    pub fn with_thb(mut self, thb: f32) -> Self {
        self.thb = Some(thb);
        self
    }

    /// Set signal quality.
    pub fn with_signal_quality(mut self, quality: u8) -> Self {
        self.signal_quality = Some(quality.min(100));
        self
    }

    /// Check if reading is valid.
    pub fn is_valid(&self) -> bool {
        (0.0..=100.0).contains(&self.smo2_percent) && self.signal_quality.map_or(true, |q| q >= 20)
    }
}

/// SmO2 sensor connection status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmO2Status {
    /// Sensor detected but not connected
    Detected,
    /// Connecting to sensor
    Connecting,
    /// Connected and receiving data
    Connected,
    /// Lost connection
    Disconnected,
    /// Error state
    Error(String),
}

/// SmO2 sensor information
#[derive(Debug, Clone)]
pub struct SmO2Sensor {
    /// Unique sensor ID
    pub id: Uuid,
    /// Sensor name/model
    pub name: String,
    /// BLE address
    pub address: String,
    /// Assigned muscle location
    pub location: MuscleLocation,
    /// Current status
    pub status: SmO2Status,
    /// Last reading
    pub last_reading: Option<SmO2Reading>,
    /// Battery level (if available)
    pub battery_level: Option<u8>,
}

impl SmO2Sensor {
    /// Create a new sensor.
    pub fn new(name: String, address: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            address,
            location: MuscleLocation::default(),
            status: SmO2Status::Detected,
            last_reading: None,
            battery_level: None,
        }
    }

    /// Set muscle location.
    pub fn with_location(mut self, location: MuscleLocation) -> Self {
        self.location = location;
        self
    }

    /// Check if sensor is connected.
    pub fn is_connected(&self) -> bool {
        self.status == SmO2Status::Connected
    }
}

/// T112: Trait for SmO2 sensor providers.
pub trait SmO2Provider: Send + Sync {
    /// Discover available SmO2 sensors.
    fn discover_smo2_sensors(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<SmO2Sensor>, SmO2Error>> + Send;

    /// Connect to a sensor.
    fn connect(
        &self,
        sensor_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<(), SmO2Error>> + Send;

    /// Disconnect from a sensor.
    fn disconnect(
        &self,
        sensor_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<(), SmO2Error>> + Send;

    /// Get current reading from a sensor.
    fn get_current_reading(
        &self,
        sensor_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<SmO2Reading, SmO2Error>> + Send;

    /// Get all connected sensors.
    fn get_connected_sensors(&self) -> Vec<SmO2Sensor>;

    /// Subscribe to readings.
    fn subscribe_readings(&self) -> broadcast::Receiver<SmO2Reading>;
}

/// Default SmO2 provider implementation.
pub struct DefaultSmO2Provider {
    sensors: Arc<RwLock<HashMap<Uuid, SmO2Sensor>>>,
    reading_tx: broadcast::Sender<SmO2Reading>,
}

impl Default for DefaultSmO2Provider {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultSmO2Provider {
    /// Create a new SmO2 provider.
    pub fn new() -> Self {
        let (reading_tx, _) = broadcast::channel(100);
        Self {
            sensors: Arc::new(RwLock::new(HashMap::new())),
            reading_tx,
        }
    }

    /// T113: Parse Moxy BLE service data.
    ///
    /// Moxy sensors use a custom BLE GATT service with UUID:
    /// `00001814-0000-1000-8000-00805f9b34fb` (similar to CSC)
    ///
    /// The data format includes:
    /// - SmO2 percentage (1 byte, 0-100)
    /// - THb (optional, 2 bytes, scaled by 100)
    /// - Signal quality (optional, 1 byte)
    pub fn parse_moxy_data(data: &[u8], sensor: &SmO2Sensor) -> Option<SmO2Reading> {
        if data.is_empty() {
            return None;
        }

        // First byte is SmO2 percentage
        let smo2 = data[0] as f32;

        let mut reading = SmO2Reading::new(sensor.id, sensor.location, smo2);

        // Parse THb if available (bytes 1-2)
        if data.len() >= 3 {
            let thb_raw = u16::from_le_bytes([data[1], data[2]]);
            reading.thb = Some(thb_raw as f32 / 100.0);
        }

        // Parse signal quality if available (byte 3)
        if data.len() >= 4 {
            reading.signal_quality = Some(data[3].min(100));
        }

        Some(reading)
    }

    /// Simulate a reading for testing.
    pub async fn simulate_reading(&self, sensor_id: &Uuid) -> Result<SmO2Reading, SmO2Error> {
        let sensors = self.sensors.read().await;
        let sensor = sensors
            .get(sensor_id)
            .ok_or(SmO2Error::SensorNotFound(*sensor_id))?;

        if sensor.status != SmO2Status::Connected {
            return Err(SmO2Error::NotConnected);
        }

        // Simulate realistic SmO2 values (typically 40-80% at rest, lower during exercise)
        let base_smo2 = 65.0;
        let noise = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| (d.as_millis() % 100) as f32 / 10.0)
            .unwrap_or(0.0))
            - 5.0;

        let reading = SmO2Reading::new(sensor.id, sensor.location, base_smo2 + noise)
            .with_thb(12.5)
            .with_signal_quality(90);

        let _ = self.reading_tx.send(reading.clone());

        Ok(reading)
    }
}

impl SmO2Provider for DefaultSmO2Provider {
    async fn discover_smo2_sensors(&self) -> Result<Vec<SmO2Sensor>, SmO2Error> {
        // In a real implementation, this would scan for BLE devices
        // with the Moxy service UUID
        tracing::info!("Scanning for SmO2 sensors...");

        // Return empty list - actual BLE scanning would happen here
        Ok(Vec::new())
    }

    async fn connect(&self, sensor_id: &Uuid) -> Result<(), SmO2Error> {
        let mut sensors = self.sensors.write().await;
        let sensor = sensors
            .get_mut(sensor_id)
            .ok_or(SmO2Error::SensorNotFound(*sensor_id))?;

        sensor.status = SmO2Status::Connecting;
        tracing::info!("Connecting to SmO2 sensor: {}", sensor.name);

        // In a real implementation, this would establish BLE connection
        sensor.status = SmO2Status::Connected;

        Ok(())
    }

    async fn disconnect(&self, sensor_id: &Uuid) -> Result<(), SmO2Error> {
        let mut sensors = self.sensors.write().await;
        let sensor = sensors
            .get_mut(sensor_id)
            .ok_or(SmO2Error::SensorNotFound(*sensor_id))?;

        sensor.status = SmO2Status::Disconnected;
        tracing::info!("Disconnected from SmO2 sensor: {}", sensor.name);

        Ok(())
    }

    async fn get_current_reading(&self, sensor_id: &Uuid) -> Result<SmO2Reading, SmO2Error> {
        self.simulate_reading(sensor_id).await
    }

    fn get_connected_sensors(&self) -> Vec<SmO2Sensor> {
        self.sensors
            .try_read()
            .map(|s| {
                s.values()
                    .filter(|sensor| sensor.is_connected())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    fn subscribe_readings(&self) -> broadcast::Receiver<SmO2Reading> {
        self.reading_tx.subscribe()
    }
}

/// Known SmO2 sensor manufacturers and models
pub mod known_sensors {
    /// Moxy Monitor
    pub const MOXY_SERVICE_UUID: &str = "00001814-0000-1000-8000-00805f9b34fb";
    /// Train.Red sensor
    pub const TRAINRED_SERVICE_UUID: &str = "6e400001-b5a3-f393-e0a9-e50e24dcca9e";
    /// BSX Insight (discontinued)
    pub const BSX_SERVICE_UUID: &str = "00002a5d-0000-1000-8000-00805f9b34fb";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_muscle_location_display() {
        assert_eq!(MuscleLocation::LeftQuad.display_name(), "Left Quad");
        assert_eq!(MuscleLocation::RightVastusLateralis.short_name(), "R.VL");
    }

    #[test]
    fn test_smo2_reading_creation() {
        let sensor_id = Uuid::new_v4();
        let reading = SmO2Reading::new(sensor_id, MuscleLocation::LeftQuad, 65.0);

        assert_eq!(reading.smo2_percent, 65.0);
        assert!(reading.is_valid());
        assert!(reading.thb.is_none());
    }

    #[test]
    fn test_smo2_reading_with_extras() {
        let sensor_id = Uuid::new_v4();
        let reading = SmO2Reading::new(sensor_id, MuscleLocation::RightCalf, 58.0)
            .with_thb(12.5)
            .with_signal_quality(85);

        assert_eq!(reading.thb, Some(12.5));
        assert_eq!(reading.signal_quality, Some(85));
        assert!(reading.is_valid());
    }

    #[test]
    fn test_invalid_reading() {
        let sensor_id = Uuid::new_v4();
        let reading = SmO2Reading::new(sensor_id, MuscleLocation::LeftQuad, 150.0);
        assert!(!reading.is_valid());

        let low_quality =
            SmO2Reading::new(sensor_id, MuscleLocation::LeftQuad, 50.0).with_signal_quality(10);
        assert!(!low_quality.is_valid());
    }

    #[test]
    fn test_sensor_creation() {
        let sensor = SmO2Sensor::new("Moxy".to_string(), "00:11:22:33:44:55".to_string());
        assert_eq!(sensor.location, MuscleLocation::LeftQuad);
        assert!(!sensor.is_connected());
    }

    #[test]
    fn test_parse_moxy_data() {
        let sensor = SmO2Sensor::new("Moxy".to_string(), "00:11:22:33:44:55".to_string());

        // SmO2 only
        let data = [65u8];
        let reading = DefaultSmO2Provider::parse_moxy_data(&data, &sensor);
        assert!(reading.is_some());
        assert_eq!(reading.unwrap().smo2_percent, 65.0);

        // SmO2 + THb
        let data_thb = [70u8, 0xE8, 0x03]; // 70%, THb = 1000 / 100 = 10.0
        let reading_thb = DefaultSmO2Provider::parse_moxy_data(&data_thb, &sensor);
        assert!(reading_thb.is_some());
        let r = reading_thb.unwrap();
        assert_eq!(r.smo2_percent, 70.0);
        assert_eq!(r.thb, Some(10.0));

        // Full data with signal quality
        let full_data = [55u8, 0xDC, 0x05, 95]; // 55%, THb = 1500/100 = 15.0, quality = 95
        let full_reading = DefaultSmO2Provider::parse_moxy_data(&full_data, &sensor);
        assert!(full_reading.is_some());
        let fr = full_reading.unwrap();
        assert_eq!(fr.smo2_percent, 55.0);
        assert_eq!(fr.thb, Some(15.0));
        assert_eq!(fr.signal_quality, Some(95));
    }

    #[test]
    fn test_provider_creation() {
        let provider = DefaultSmO2Provider::new();
        assert!(provider.get_connected_sensors().is_empty());
    }
}
