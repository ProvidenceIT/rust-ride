//! Sensor manager for BLE device discovery and connection.
//!
//! T030: Implement SensorManager struct with btleplug adapter initialization
//! T031: Implement start_discovery() with FTMS/CPS/HRS service UUID filtering
//! T032: Implement stop_discovery()
//! T033: Implement connect() with characteristic subscription
//! T034: Implement disconnect()
//! T035: Implement event channel for SensorEvent streaming

use crate::sensors::ftms::{
    parse_cycling_power_measurement, parse_heart_rate_measurement, parse_indoor_bike_data,
    CYCLING_POWER_MEASUREMENT_UUID, CYCLING_POWER_SERVICE_UUID, FTMS_SERVICE_UUID,
    HEART_RATE_MEASUREMENT_UUID, HEART_RATE_SERVICE_UUID, INDOOR_BIKE_DATA_UUID,
};
use crate::sensors::types::{
    ConnectionState, DiscoveredSensor, Protocol, SensorConfig, SensorError, SensorEvent,
    SensorReading, SensorState, SensorType,
};
use btleplug::api::{
    Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use crossbeam::channel::{Receiver, Sender};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Manages BLE sensor discovery, connection, and data streaming.
pub struct SensorManager {
    /// Configuration
    config: SensorConfig,
    /// BLE adapter
    adapter: Option<Adapter>,
    /// Channel for sending sensor events
    event_tx: Option<Sender<SensorEvent>>,
    /// Discovered sensors (device_id -> DiscoveredSensor)
    discovered: Arc<Mutex<HashMap<String, DiscoveredSensor>>>,
    /// Connected peripherals (device_id -> Peripheral)
    connected: Arc<Mutex<HashMap<String, Peripheral>>>,
    /// Sensor states (device_id -> SensorState)
    sensor_states: Arc<Mutex<HashMap<String, SensorState>>>,
    /// Whether currently scanning
    is_scanning: Arc<Mutex<bool>>,
}

impl SensorManager {
    /// Create a new sensor manager.
    pub fn new(config: SensorConfig) -> Self {
        Self {
            config,
            adapter: None,
            event_tx: None,
            discovered: Arc::new(Mutex::new(HashMap::new())),
            connected: Arc::new(Mutex::new(HashMap::new())),
            sensor_states: Arc::new(Mutex::new(HashMap::new())),
            is_scanning: Arc::new(Mutex::new(false)),
        }
    }

    /// Create a new sensor manager with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(SensorConfig::default())
    }

    /// Initialize the BLE adapter.
    ///
    /// This must be called before any sensor operations.
    pub async fn initialize(&mut self) -> Result<(), SensorError> {
        tracing::info!("Initializing SensorManager");

        let manager = Manager::new()
            .await
            .map_err(|e| SensorError::BleError(e.to_string()))?;

        let adapters = manager
            .adapters()
            .await
            .map_err(|e| SensorError::BleError(e.to_string()))?;

        let adapter = adapters
            .into_iter()
            .next()
            .ok_or(SensorError::AdapterNotFound)?;

        tracing::info!("BLE adapter initialized");
        self.adapter = Some(adapter);

        Ok(())
    }

    /// Get an event receiver for sensor events.
    pub fn event_receiver(&mut self) -> Receiver<SensorEvent> {
        let (tx, rx) = crossbeam::channel::unbounded();
        self.event_tx = Some(tx);
        rx
    }

    /// Send an event if the channel is available.
    fn send_event(&self, event: SensorEvent) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(event);
        }
    }

    /// Start scanning for BLE sensors.
    pub async fn start_discovery(&mut self) -> Result<(), SensorError> {
        let adapter = self
            .adapter
            .as_ref()
            .ok_or(SensorError::AdapterNotFound)?;

        {
            let mut is_scanning = self.is_scanning.lock().await;
            if *is_scanning {
                return Ok(()); // Already scanning
            }
            *is_scanning = true;
        }

        tracing::info!("Starting sensor discovery");

        // Clear previous discoveries
        self.discovered.lock().await.clear();

        // Create scan filter for fitness services
        let scan_filter = ScanFilter {
            services: vec![
                FTMS_SERVICE_UUID,
                CYCLING_POWER_SERVICE_UUID,
                HEART_RATE_SERVICE_UUID,
            ],
        };

        adapter
            .start_scan(scan_filter)
            .await
            .map_err(|e| SensorError::ScanFailed(e.to_string()))?;

        self.send_event(SensorEvent::ScanStarted);

        // Start event processing in background
        let adapter_clone = adapter.clone();
        let discovered = self.discovered.clone();
        let event_tx = self.event_tx.clone();
        let is_scanning = self.is_scanning.clone();

        tokio::spawn(async move {
            Self::process_discovery_events(adapter_clone, discovered, event_tx, is_scanning).await;
        });

        Ok(())
    }

    /// Process discovery events from the adapter.
    async fn process_discovery_events(
        adapter: Adapter,
        discovered: Arc<Mutex<HashMap<String, DiscoveredSensor>>>,
        event_tx: Option<Sender<SensorEvent>>,
        is_scanning: Arc<Mutex<bool>>,
    ) {
        use futures::stream::StreamExt;

        let mut events = match adapter.events().await {
            Ok(events) => events,
            Err(e) => {
                tracing::error!("Failed to get adapter events: {}", e);
                return;
            }
        };

        while let Some(event) = events.next().await {
            // Check if still scanning
            if !*is_scanning.lock().await {
                break;
            }

            if let CentralEvent::DeviceDiscovered(id) = event {
                let peripherals = match adapter.peripherals().await {
                    Ok(p) => p,
                    Err(_) => continue,
                };

                for peripheral in peripherals {
                    if peripheral.id() == id {
                        if let Some(sensor) = Self::classify_peripheral(&peripheral).await {
                            let device_id = peripheral.id().to_string();

                            // Store discovered sensor
                            discovered.lock().await.insert(device_id.clone(), sensor.clone());

                            // Send discovery event
                            if let Some(tx) = &event_tx {
                                let _ = tx.send(SensorEvent::Discovered(sensor));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Classify a peripheral based on its advertised services.
    async fn classify_peripheral(peripheral: &Peripheral) -> Option<DiscoveredSensor> {
        let properties = peripheral.properties().await.ok()??;

        let name = properties
            .local_name
            .unwrap_or_else(|| "Unknown Sensor".to_string());

        let services = properties.services;

        // Determine sensor type and protocol from services
        let (sensor_type, protocol) = if services.contains(&FTMS_SERVICE_UUID) {
            (SensorType::Trainer, Protocol::BleFtms)
        } else if services.contains(&CYCLING_POWER_SERVICE_UUID) {
            (SensorType::PowerMeter, Protocol::BleCyclingPower)
        } else if services.contains(&HEART_RATE_SERVICE_UUID) {
            (SensorType::HeartRate, Protocol::BleHeartRate)
        } else {
            return None; // Not a supported sensor
        };

        let signal_strength = properties.rssi;

        Some(DiscoveredSensor {
            device_id: peripheral.id().to_string(),
            name,
            sensor_type,
            protocol,
            signal_strength,
            last_seen: Instant::now(),
        })
    }

    /// Stop scanning for BLE sensors.
    pub async fn stop_discovery(&mut self) -> Result<(), SensorError> {
        let adapter = self.adapter.as_ref().ok_or(SensorError::AdapterNotFound)?;

        {
            let mut is_scanning = self.is_scanning.lock().await;
            if !*is_scanning {
                return Ok(()); // Not scanning
            }
            *is_scanning = false;
        }

        tracing::info!("Stopping sensor discovery");

        adapter
            .stop_scan()
            .await
            .map_err(|e| SensorError::ScanFailed(e.to_string()))?;

        self.send_event(SensorEvent::ScanStopped);

        Ok(())
    }

    /// Connect to a sensor by device ID.
    pub async fn connect(&mut self, device_id: &str) -> Result<(), SensorError> {
        let adapter = self.adapter.as_ref().ok_or(SensorError::AdapterNotFound)?;

        tracing::info!("Connecting to sensor: {}", device_id);

        // Send connecting state
        self.send_event(SensorEvent::ConnectionChanged {
            device_id: device_id.to_string(),
            state: ConnectionState::Connecting,
        });

        // Find the peripheral
        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|e| SensorError::BleError(e.to_string()))?;

        let peripheral = peripherals
            .into_iter()
            .find(|p| p.id().to_string() == device_id)
            .ok_or_else(|| SensorError::SensorNotFound(device_id.to_string()))?;

        // Connect
        peripheral
            .connect()
            .await
            .map_err(|e| SensorError::ConnectionFailed(e.to_string()))?;

        // Discover services
        peripheral
            .discover_services()
            .await
            .map_err(|e| SensorError::ConnectionFailed(e.to_string()))?;

        // Subscribe to relevant characteristics
        self.subscribe_to_characteristics(&peripheral).await?;

        // Store connected peripheral
        self.connected
            .lock()
            .await
            .insert(device_id.to_string(), peripheral.clone());

        // Create sensor state
        let discovered = self.discovered.lock().await;
        if let Some(disc_sensor) = discovered.get(device_id) {
            let state = SensorState {
                id: Uuid::new_v4(),
                device_id: device_id.to_string(),
                name: disc_sensor.name.clone(),
                sensor_type: disc_sensor.sensor_type,
                protocol: disc_sensor.protocol,
                connection_state: ConnectionState::Connected,
                signal_strength: disc_sensor.signal_strength,
                battery_level: None,
                last_data_at: None,
                is_primary: false,
            };

            self.sensor_states
                .lock()
                .await
                .insert(device_id.to_string(), state);
        }

        // Send connected state
        self.send_event(SensorEvent::ConnectionChanged {
            device_id: device_id.to_string(),
            state: ConnectionState::Connected,
        });

        // Start notification handler
        let event_tx = self.event_tx.clone();
        let sensor_states = self.sensor_states.clone();
        let device_id_clone = device_id.to_string();

        tokio::spawn(async move {
            Self::handle_notifications(peripheral, event_tx, sensor_states, device_id_clone).await;
        });

        tracing::info!("Connected to sensor: {}", device_id);

        Ok(())
    }

    /// Subscribe to sensor data characteristics.
    async fn subscribe_to_characteristics(&self, peripheral: &Peripheral) -> Result<(), SensorError> {
        let characteristics = peripheral.characteristics();

        for char in characteristics {
            let char_uuid = char.uuid;

            // Subscribe to relevant characteristics
            if char_uuid == INDOOR_BIKE_DATA_UUID
                || char_uuid == CYCLING_POWER_MEASUREMENT_UUID
                || char_uuid == HEART_RATE_MEASUREMENT_UUID
            {
                peripheral
                    .subscribe(&char)
                    .await
                    .map_err(|e| SensorError::SubscriptionFailed(e.to_string()))?;

                tracing::debug!("Subscribed to characteristic: {}", char_uuid);
            }
        }

        Ok(())
    }

    /// Handle notifications from a connected peripheral.
    async fn handle_notifications(
        peripheral: Peripheral,
        event_tx: Option<Sender<SensorEvent>>,
        sensor_states: Arc<Mutex<HashMap<String, SensorState>>>,
        device_id: String,
    ) {
        use futures::stream::StreamExt;

        let mut notification_stream = match peripheral.notifications().await {
            Ok(stream) => stream,
            Err(e) => {
                tracing::error!("Failed to get notification stream: {}", e);
                return;
            }
        };

        while let Some(notification) = notification_stream.next().await {
            let char_uuid = notification.uuid;
            let data = notification.value;

            // Parse the data based on characteristic
            let reading = if char_uuid == INDOOR_BIKE_DATA_UUID {
                Self::parse_ftms_notification(&data, &device_id)
            } else if char_uuid == CYCLING_POWER_MEASUREMENT_UUID {
                Self::parse_power_notification(&data, &device_id)
            } else if char_uuid == HEART_RATE_MEASUREMENT_UUID {
                Self::parse_hr_notification(&data, &device_id)
            } else {
                None
            };

            if let Some(reading) = reading {
                // Update last data time
                if let Some(state) = sensor_states.lock().await.get_mut(&device_id) {
                    state.last_data_at = Some(Instant::now());
                }

                // Send data event
                if let Some(tx) = &event_tx {
                    let _ = tx.send(SensorEvent::Data(reading));
                }
            }
        }

        // Stream ended - peripheral disconnected
        if let Some(tx) = &event_tx {
            let _ = tx.send(SensorEvent::ConnectionChanged {
                device_id,
                state: ConnectionState::Disconnected,
            });
        }
    }

    /// Parse FTMS Indoor Bike Data notification.
    fn parse_ftms_notification(data: &[u8], device_id: &str) -> Option<SensorReading> {
        let parsed = parse_indoor_bike_data(data)?;

        Some(SensorReading {
            sensor_id: Uuid::nil(), // Will be set properly later
            timestamp: Instant::now(),
            power_watts: parsed.power_watts.map(|p| p as u16),
            cadence_rpm: parsed.cadence_rpm.map(|c| c as u8),
            heart_rate_bpm: parsed.heart_rate_bpm,
            speed_kmh: parsed.speed_kmh,
            distance_delta_m: None, // Would need to calculate from total distance
        })
    }

    /// Parse Cycling Power Measurement notification.
    fn parse_power_notification(data: &[u8], device_id: &str) -> Option<SensorReading> {
        let parsed = parse_cycling_power_measurement(data)?;

        Some(SensorReading {
            sensor_id: Uuid::nil(),
            timestamp: Instant::now(),
            power_watts: Some(parsed.power_watts as u16),
            cadence_rpm: None, // Would need crank revolution data
            heart_rate_bpm: None,
            speed_kmh: None,
            distance_delta_m: None,
        })
    }

    /// Parse Heart Rate Measurement notification.
    fn parse_hr_notification(data: &[u8], device_id: &str) -> Option<SensorReading> {
        let parsed = parse_heart_rate_measurement(data)?;

        Some(SensorReading {
            sensor_id: Uuid::nil(),
            timestamp: Instant::now(),
            power_watts: None,
            cadence_rpm: None,
            heart_rate_bpm: Some(parsed.heart_rate_bpm as u8),
            speed_kmh: None,
            distance_delta_m: None,
        })
    }

    /// Disconnect from a sensor.
    pub async fn disconnect(&mut self, device_id: &str) -> Result<(), SensorError> {
        tracing::info!("Disconnecting from sensor: {}", device_id);

        let mut connected = self.connected.lock().await;

        if let Some(peripheral) = connected.remove(device_id) {
            peripheral
                .disconnect()
                .await
                .map_err(|e| SensorError::BleError(e.to_string()))?;
        }

        // Update sensor state
        if let Some(state) = self.sensor_states.lock().await.get_mut(device_id) {
            state.connection_state = ConnectionState::Disconnected;
        }

        // Send disconnected event
        self.send_event(SensorEvent::ConnectionChanged {
            device_id: device_id.to_string(),
            state: ConnectionState::Disconnected,
        });

        Ok(())
    }

    /// Set target power for ERG mode.
    pub async fn set_target_power(
        &self,
        device_id: &str,
        target_watts: u16,
    ) -> Result<(), SensorError> {
        let connected = self.connected.lock().await;

        let peripheral = connected
            .get(device_id)
            .ok_or_else(|| SensorError::SensorNotFound(device_id.to_string()))?;

        // Find FTMS Control Point characteristic
        let characteristics = peripheral.characteristics();
        let control_point = characteristics
            .iter()
            .find(|c| c.uuid == crate::sensors::ftms::FTMS_CONTROL_POINT_UUID)
            .ok_or(SensorError::Unsupported)?;

        // Build and send the command
        let cmd = crate::sensors::ftms::build_set_target_power(target_watts);

        peripheral
            .write(control_point, &cmd, WriteType::WithResponse)
            .await
            .map_err(|e| SensorError::WriteFailed(e.to_string()))?;

        tracing::debug!("Set target power to {}W", target_watts);

        Ok(())
    }

    /// Get list of discovered sensors.
    pub async fn get_discovered(&self) -> Vec<DiscoveredSensor> {
        self.discovered.lock().await.values().cloned().collect()
    }

    /// Get list of connected sensor states.
    pub async fn get_connected(&self) -> Vec<SensorState> {
        self.sensor_states
            .lock()
            .await
            .values()
            .filter(|s| s.connection_state == ConnectionState::Connected)
            .cloned()
            .collect()
    }

    /// Check if currently scanning.
    pub async fn is_scanning(&self) -> bool {
        *self.is_scanning.lock().await
    }

    /// Shutdown the sensor manager.
    pub async fn shutdown(&mut self) {
        tracing::info!("Shutting down SensorManager");

        // Stop scanning
        let _ = self.stop_discovery().await;

        // Disconnect all sensors
        let device_ids: Vec<String> = self.connected.lock().await.keys().cloned().collect();

        for device_id in device_ids {
            let _ = self.disconnect(&device_id).await;
        }
    }
}
