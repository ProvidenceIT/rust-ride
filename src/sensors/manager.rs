//! Sensor manager for BLE and ANT+ device discovery and connection.
//!
//! T030: Implement SensorManager struct with btleplug adapter initialization
//! T031: Implement start_discovery() with FTMS/CPS/HRS service UUID filtering
//! T032: Implement stop_discovery()
//! T033: Implement connect() with characteristic subscription
//! T034: Implement disconnect()
//! T035: Implement event channel for SensorEvent streaming

use crate::sensors::ant::dongle::{AntDongle, AntDongleManager, DefaultDongleManager};
use crate::sensors::ant::{AntConfig, AntDeviceType, AntEvent};
use crate::sensors::cache::SensorCache;
use crate::sensors::connection_queue::{ConnectionQueue, ConnectionQueueEntry, SensorPriority};
use crate::sensors::ftms::{
    parse_cycling_power_measurement, parse_heart_rate_measurement, parse_indoor_bike_data,
    CYCLING_POWER_MEASUREMENT_UUID, CYCLING_POWER_SERVICE_UUID, FTMS_SERVICE_UUID,
    HEART_RATE_MEASUREMENT_UUID, HEART_RATE_SERVICE_UUID, INDOOR_BIKE_DATA_UUID,
};
use crate::sensors::types::{
    ConnectionState, DiscoveredSensor, DiscoveryPhase, DiscoveryProgress, ParallelDiscoveryResult,
    ProgressiveTimeoutConfig, ProgressiveTimeoutState, Protocol, SensorConfig, SensorError,
    SensorEvent, SensorReading, SensorState, SensorType, StopReason, TimeoutDecision,
};
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use crossbeam::channel::{Receiver, Sender};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Context for handling notifications from a connected peripheral.
/// Groups related parameters to reduce function argument count.
#[allow(dead_code)]
struct NotificationContext {
    event_tx: Option<Sender<SensorEvent>>,
    sensor_states: Arc<Mutex<HashMap<String, SensorState>>>,
    device_id: String,
    reconnect_attempts: Arc<Mutex<HashMap<String, u32>>>,
    max_reconnect_attempts: u32,
    reconnect_delay_secs: u64,
    auto_reconnect: bool,
}

/// Manages BLE and ANT+ sensor discovery, connection, and data streaming.
pub struct SensorManager {
    /// Configuration
    config: SensorConfig,
    /// BLE adapter
    adapter: Option<Adapter>,
    /// ANT+ dongle manager
    ant_manager: Option<Arc<DefaultDongleManager>>,
    /// Whether ANT+ scanning is enabled
    ant_enabled: Arc<Mutex<bool>>,
    /// Detected ANT+ dongles
    ant_dongles: Arc<Mutex<Vec<AntDongle>>>,
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
    /// Discovery timeout handle (for cancellation)
    discovery_timeout_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Reconnection attempts (device_id -> attempt count)
    reconnect_attempts: Arc<Mutex<HashMap<String, u32>>>,
    /// Cache of previously connected sensors for fast reconnection
    sensor_cache: Arc<Mutex<SensorCache>>,
    /// Progressive timeout state for current discovery
    progressive_timeout_state: Arc<Mutex<Option<ProgressiveTimeoutState>>>,
    /// Priority-based connection queue for discovered sensors
    connection_queue: Arc<Mutex<ConnectionQueue>>,
}

impl SensorManager {
    /// Create a new sensor manager.
    pub fn new(config: SensorConfig) -> Self {
        // Load sensor cache from disk
        let sensor_cache = SensorCache::load();
        tracing::debug!(
            "Loaded {} cached sensors for fast reconnection",
            sensor_cache.len()
        );

        Self {
            config,
            adapter: None,
            ant_manager: None,
            ant_enabled: Arc::new(Mutex::new(false)),
            ant_dongles: Arc::new(Mutex::new(Vec::new())),
            event_tx: None,
            discovered: Arc::new(Mutex::new(HashMap::new())),
            connected: Arc::new(Mutex::new(HashMap::new())),
            sensor_states: Arc::new(Mutex::new(HashMap::new())),
            is_scanning: Arc::new(Mutex::new(false)),
            discovery_timeout_handle: Arc::new(Mutex::new(None)),
            reconnect_attempts: Arc::new(Mutex::new(HashMap::new())),
            sensor_cache: Arc::new(Mutex::new(sensor_cache)),
            progressive_timeout_state: Arc::new(Mutex::new(None)),
            connection_queue: Arc::new(Mutex::new(ConnectionQueue::new())),
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

    /// Initialize ANT+ dongle support.
    ///
    /// This scans for available ANT+ USB dongles and initializes the manager.
    pub async fn initialize_ant(&mut self) -> Result<(), SensorError> {
        tracing::info!("Initializing ANT+ support");

        let ant_config = AntConfig::default();
        let manager = DefaultDongleManager::new(ant_config);

        // Scan for available dongles (synchronous operation)
        let dongles = manager.scan_dongles();

        if dongles.is_empty() {
            tracing::info!("No ANT+ dongles found");
        } else {
            tracing::info!("Found {} ANT+ dongle(s)", dongles.len());
            for dongle in &dongles {
                tracing::debug!(
                    "ANT+ dongle: {} (serial: {:?})",
                    dongle.name,
                    dongle.serial_number
                );
            }
        }

        *self.ant_dongles.lock().await = dongles;
        self.ant_manager = Some(Arc::new(manager));

        Ok(())
    }

    /// Get the list of detected ANT+ dongles.
    pub async fn get_ant_dongles(&self) -> Vec<AntDongle> {
        self.ant_dongles.lock().await.clone()
    }

    /// Check if ANT+ is available (at least one dongle detected).
    pub async fn is_ant_available(&self) -> bool {
        !self.ant_dongles.lock().await.is_empty()
    }

    /// Enable or disable ANT+ scanning.
    ///
    /// When enabled, discovery will also scan for ANT+ sensors alongside BLE.
    pub async fn set_ant_enabled(&self, enabled: bool) {
        *self.ant_enabled.lock().await = enabled;
        tracing::info!(
            "ANT+ scanning {}",
            if enabled { "enabled" } else { "disabled" }
        );
    }

    /// Check if ANT+ scanning is enabled.
    pub async fn is_ant_enabled(&self) -> bool {
        *self.ant_enabled.lock().await
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

    /// Start scanning for BLE and ANT+ sensors concurrently.
    ///
    /// When both BLE and ANT+ are enabled, both protocols scan simultaneously
    /// to reduce total discovery time. This is significantly faster than
    /// sequential scanning.
    pub async fn start_discovery(&mut self) -> Result<(), SensorError> {
        let adapter = self.adapter.as_ref().ok_or(SensorError::AdapterNotFound)?;

        {
            let mut is_scanning = self.is_scanning.lock().await;
            if *is_scanning {
                return Ok(()); // Already scanning
            }
            *is_scanning = true;
        }

        tracing::info!("Starting sensor discovery (concurrent BLE/ANT+ scanning)");

        // Clear previous discoveries
        self.discovered.lock().await.clear();

        // Check if ANT+ should be enabled for concurrent scanning
        let ant_enabled = *self.ant_enabled.lock().await;

        // Run BLE and ANT+ discovery concurrently using tokio::join!
        // This reduces total discovery time compared to sequential scanning
        let ble_result = self.start_ble_discovery(adapter.clone()).await;

        if ant_enabled {
            // Start ANT+ discovery concurrently with BLE event processing
            let ant_result = self.start_ant_discovery().await;
            if let Err(e) = ant_result {
                tracing::warn!("ANT+ discovery failed to start: {}", e);
                // Continue with BLE-only discovery
            }
        }

        // Check BLE result
        ble_result?;

        self.send_event(SensorEvent::ScanStarted);

        // Start discovery timeout (T030)
        let timeout_secs = self.config.discovery_timeout_secs;
        let is_scanning_timeout = self.is_scanning.clone();
        let event_tx_timeout = self.event_tx.clone();
        let adapter_timeout = adapter.clone();
        let timeout_handle = self.discovery_timeout_handle.clone();

        let handle = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(timeout_secs)).await;

            // Check if still scanning
            let mut is_scanning = is_scanning_timeout.lock().await;
            if *is_scanning {
                tracing::info!(
                    "Discovery timeout reached ({}s), stopping scan",
                    timeout_secs
                );
                *is_scanning = false;
                drop(is_scanning);

                // Stop the scan
                if let Err(e) = adapter_timeout.stop_scan().await {
                    tracing::warn!("Failed to stop scan on timeout: {}", e);
                }

                // Send scan stopped event
                if let Some(tx) = &event_tx_timeout {
                    let _ = tx.send(SensorEvent::ScanStopped);
                }
            }
        });

        *timeout_handle.lock().await = Some(handle);

        Ok(())
    }

    /// Start BLE sensor discovery.
    ///
    /// Internal helper that starts BLE scanning and event processing.
    async fn start_ble_discovery(&self, adapter: Adapter) -> Result<(), SensorError> {
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

        tracing::debug!("BLE scan started");

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

    /// Start truly concurrent BLE and ANT+ discovery using tokio::join!
    ///
    /// This method ensures both protocols start scanning at exactly the same time,
    /// maximizing the parallelism and reducing total discovery time.
    pub async fn start_concurrent_discovery(&mut self) -> Result<ParallelDiscoveryResult, SensorError> {
        let adapter = self.adapter.as_ref().ok_or(SensorError::AdapterNotFound)?;

        {
            let mut is_scanning = self.is_scanning.lock().await;
            if *is_scanning {
                return Ok(ParallelDiscoveryResult {
                    ble_started: false,
                    ant_started: false,
                    ble_error: None,
                    ant_error: None,
                }); // Already scanning
            }
            *is_scanning = true;
        }

        let start_time = Instant::now();
        tracing::info!("Starting concurrent BLE/ANT+ discovery");

        // Clear previous discoveries
        self.discovered.lock().await.clear();

        let ant_enabled = *self.ant_enabled.lock().await;

        // Create futures for both discovery types
        let ble_future = self.start_ble_discovery(adapter.clone());

        // Run both discoveries concurrently
        let result = if ant_enabled {
            // Create ANT+ discovery future
            let ant_future = self.prepare_ant_discovery();

            // Use tokio::join! to run both concurrently
            let (ble_result, ant_result) = tokio::join!(ble_future, ant_future);

            let ble_started = ble_result.is_ok();
            let ant_started = ant_result.is_ok();

            ParallelDiscoveryResult {
                ble_started,
                ant_started,
                ble_error: ble_result.err().map(|e| e.to_string()),
                ant_error: ant_result.err().map(|e| e.to_string()),
            }
        } else {
            // BLE only
            let ble_result = ble_future.await;
            ParallelDiscoveryResult {
                ble_started: ble_result.is_ok(),
                ant_started: false,
                ble_error: ble_result.err().map(|e| e.to_string()),
                ant_error: None,
            }
        };

        let elapsed = start_time.elapsed();
        tracing::info!(
            "Concurrent discovery started in {:?} (BLE: {}, ANT+: {})",
            elapsed,
            if result.ble_started { "OK" } else { "Failed" },
            if result.ant_started { "OK" } else { "Disabled/Failed" }
        );

        self.send_event(SensorEvent::ScanStarted);

        // Start discovery timeout
        self.start_discovery_timeout(adapter.clone()).await;

        // Return error only if both protocols failed
        if !result.ble_started && !result.ant_started && ant_enabled {
            return Err(SensorError::ScanFailed(
                "Both BLE and ANT+ discovery failed to start".to_string()
            ));
        }

        Ok(result)
    }

    /// Prepare ANT+ discovery for concurrent execution.
    ///
    /// This is a wrapper that can be used with tokio::join! for parallel scanning.
    async fn prepare_ant_discovery(&self) -> Result<(), SensorError> {
        self.start_ant_discovery().await
    }

    /// Start discovery timeout task with progressive timeout support.
    async fn start_discovery_timeout(&self, adapter: Adapter) {
        let timeout_secs = self.config.discovery_timeout_secs;
        let progressive_config = self.config.progressive_timeout.clone();
        let is_scanning_timeout = self.is_scanning.clone();
        let event_tx_timeout = self.event_tx.clone();
        let timeout_handle = self.discovery_timeout_handle.clone();
        let progressive_state = self.progressive_timeout_state.clone();
        let discovered = self.discovered.clone();

        // Initialize progressive timeout state
        *progressive_state.lock().await = Some(ProgressiveTimeoutState::new());

        let handle = tokio::spawn(async move {
            Self::run_progressive_timeout(
                adapter,
                progressive_config,
                timeout_secs,
                is_scanning_timeout,
                event_tx_timeout,
                progressive_state,
                discovered,
            )
            .await;
        });

        *timeout_handle.lock().await = Some(handle);
    }

    /// Run the progressive timeout logic.
    ///
    /// This monitors sensor discovery activity and adjusts the timeout:
    /// - Initial aggressive 10s scan
    /// - Extends if sensors are still being found
    /// - Stops early if idle for too long
    /// - Maximum 30s total scan time
    async fn run_progressive_timeout(
        adapter: Adapter,
        config: ProgressiveTimeoutConfig,
        fallback_timeout_secs: u64,
        is_scanning: Arc<Mutex<bool>>,
        event_tx: Option<Sender<SensorEvent>>,
        state: Arc<Mutex<Option<ProgressiveTimeoutState>>>,
        discovered: Arc<Mutex<HashMap<String, DiscoveredSensor>>>,
    ) {
        // Check interval for progressive timeout decisions
        const CHECK_INTERVAL_MS: u64 = 500;

        let mut last_discovered_count = 0usize;

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(CHECK_INTERVAL_MS)).await;

            // Check if still scanning
            if !*is_scanning.lock().await {
                tracing::debug!("Progressive timeout: scanning stopped externally");
                break;
            }

            // Update state with new discoveries
            {
                let current_count = discovered.lock().await.len();
                let mut state_guard = state.lock().await;

                if let Some(ref mut timeout_state) = *state_guard {
                    // Record new discoveries
                    while last_discovered_count < current_count {
                        timeout_state.record_discovery();
                        last_discovered_count += 1;
                        tracing::debug!(
                            "Progressive timeout: recorded discovery #{}, phase: {}",
                            timeout_state.sensors_discovered,
                            timeout_state.phase
                        );
                    }

                    // Calculate decision
                    let decision = timeout_state.calculate_decision(&config);

                    match decision {
                        TimeoutDecision::Continue => {
                            // Keep scanning
                            continue;
                        }
                        TimeoutDecision::Extend => {
                            timeout_state.apply_extension();
                            tracing::info!(
                                "Progressive timeout: extending scan (extension #{}, {} sensors found)",
                                timeout_state.extensions_count,
                                timeout_state.sensors_discovered
                            );
                        }
                        TimeoutDecision::Stop { reason } => {
                            timeout_state.mark_completed();
                            let elapsed = timeout_state.elapsed();
                            let sensors = timeout_state.sensors_discovered;

                            tracing::info!(
                                "Progressive timeout: stopping scan ({:?}), elapsed: {:?}, found: {} sensors",
                                reason,
                                elapsed,
                                sensors
                            );

                            // Stop scanning
                            drop(state_guard);
                            Self::stop_discovery_internal(
                                &adapter,
                                &is_scanning,
                                &event_tx,
                                reason,
                            )
                            .await;
                            return;
                        }
                    }
                } else {
                    // No progressive state - use fallback timeout
                    let elapsed = std::time::Instant::now();
                    if elapsed.elapsed() >= std::time::Duration::from_secs(fallback_timeout_secs) {
                        drop(state_guard);
                        Self::stop_discovery_internal(
                            &adapter,
                            &is_scanning,
                            &event_tx,
                            StopReason::MaxTimeReached,
                        )
                        .await;
                        return;
                    }
                }
            }
        }
    }

    /// Internal helper to stop discovery.
    async fn stop_discovery_internal(
        adapter: &Adapter,
        is_scanning: &Arc<Mutex<bool>>,
        event_tx: &Option<Sender<SensorEvent>>,
        reason: StopReason,
    ) {
        let mut scanning = is_scanning.lock().await;
        if *scanning {
            *scanning = false;
            drop(scanning);

            tracing::info!("Discovery stopped: {:?}", reason);

            if let Err(e) = adapter.stop_scan().await {
                tracing::warn!("Failed to stop scan: {}", e);
            }

            if let Some(tx) = event_tx {
                let _ = tx.send(SensorEvent::ScanStopped);
            }
        }
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
                            discovered
                                .lock()
                                .await
                                .insert(device_id.clone(), sensor.clone());

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

    /// Start ANT+ sensor discovery.
    async fn start_ant_discovery(&self) -> Result<(), SensorError> {
        let _manager = match &self.ant_manager {
            Some(m) => m.clone(),
            None => {
                tracing::warn!("ANT+ not initialized, skipping ANT+ discovery");
                return Ok(());
            }
        };

        let dongles = self.ant_dongles.lock().await;
        if dongles.is_empty() {
            tracing::info!("No ANT+ dongles available for discovery");
            return Ok(());
        }

        // Use the first available dongle
        let dongle = &dongles[0];
        tracing::info!("Starting ANT+ discovery using dongle: {}", dongle.name);

        // Open the dongle for scanning
        let dongle_clone = dongle.clone();
        let discovered = self.discovered.clone();
        let event_tx = self.event_tx.clone();
        let is_scanning = self.is_scanning.clone();

        tokio::spawn(async move {
            Self::process_ant_discovery(dongle_clone, discovered, event_tx, is_scanning).await;
        });

        Ok(())
    }

    /// Process ANT+ device discovery.
    #[allow(unused_variables)]
    async fn process_ant_discovery(
        dongle: AntDongle,
        discovered: Arc<Mutex<HashMap<String, DiscoveredSensor>>>,
        event_tx: Option<Sender<SensorEvent>>,
        is_scanning: Arc<Mutex<bool>>,
    ) {
        // Device types to search for
        let device_types = vec![
            AntDeviceType::HeartRate,
            AntDeviceType::Power,
            AntDeviceType::SpeedCadence,
            AntDeviceType::FitnessEquipment,
        ];

        // Simulate ANT+ discovery - in real implementation this would
        // use the ANT+ channel manager to open search channels
        for device_type in device_types {
            // Check if still scanning
            if !*is_scanning.lock().await {
                break;
            }

            // For now, log that we're searching for this type
            // Real implementation would open ANT+ channels and wait for broadcasts
            tracing::debug!(
                "Searching for ANT+ device type: {:?} (type {})",
                device_type,
                device_type.device_type_number()
            );

            // Small delay between channel openings
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    /// Convert ANT+ device type to SensorType.
    fn ant_device_type_to_sensor_type(device_type: AntDeviceType) -> SensorType {
        match device_type {
            AntDeviceType::HeartRate => SensorType::HeartRate,
            AntDeviceType::Power => SensorType::PowerMeter,
            AntDeviceType::SpeedCadence => SensorType::SpeedCadence,
            AntDeviceType::FitnessEquipment => SensorType::SmartTrainer,
            AntDeviceType::Unknown(_) => SensorType::Trainer,
        }
    }

    /// Create an ANT+ device ID from device number and type.
    fn create_ant_device_id(device_number: u16, device_type: AntDeviceType) -> String {
        format!(
            "ant+:{}:{}",
            device_type.device_type_number(),
            device_number
        )
    }

    /// Get the protocol for an ANT+ device type.
    fn ant_device_type_to_protocol(device_type: AntDeviceType) -> Protocol {
        match device_type {
            AntDeviceType::HeartRate => Protocol::AntHeartRate,
            AntDeviceType::Power => Protocol::AntPower,
            AntDeviceType::SpeedCadence => Protocol::AntSpeedCadence,
            AntDeviceType::FitnessEquipment => Protocol::AntFec,
            AntDeviceType::Unknown(_) => Protocol::AntFec,
        }
    }

    /// Handle an ANT+ device discovery event.
    /// Called when an ANT+ device broadcast is received.
    #[allow(dead_code)]
    async fn handle_ant_device_found(
        &self,
        device_number: u16,
        device_type: AntDeviceType,
        name: Option<String>,
    ) {
        let device_id = Self::create_ant_device_id(device_number, device_type);
        let sensor_type = Self::ant_device_type_to_sensor_type(device_type);
        let protocol = Self::ant_device_type_to_protocol(device_type);

        let sensor = DiscoveredSensor {
            device_id: device_id.clone(),
            name: name.unwrap_or_else(|| format!("ANT+ {:?} {}", device_type, device_number)),
            sensor_type,
            protocol,
            signal_strength: None, // ANT+ doesn't typically provide RSSI
            last_seen: Instant::now(),
        };

        // Store discovered sensor
        self.discovered
            .lock()
            .await
            .insert(device_id, sensor.clone());

        // Send discovery event
        self.send_event(SensorEvent::Discovered(sensor));
    }

    /// Handle ANT+ heart rate data.
    /// Called when heart rate data page is received from an ANT+ HR sensor.
    #[allow(dead_code)]
    fn handle_ant_heart_rate_data(&self, device_number: u16, heart_rate_bpm: u8) {
        let device_id = Self::create_ant_device_id(device_number, AntDeviceType::HeartRate);

        let reading = SensorReading {
            sensor_id: Uuid::nil(),
            timestamp: Instant::now(),
            power_watts: None,
            cadence_rpm: None,
            heart_rate_bpm: Some(heart_rate_bpm),
            speed_kmh: None,
            distance_delta_m: None,
        };

        self.send_event(SensorEvent::Data(reading));

        tracing::trace!("ANT+ HR data from {}: {} bpm", device_id, heart_rate_bpm);
    }

    /// Handle ANT+ power meter data.
    /// Called when power data page is received from an ANT+ power meter.
    #[allow(dead_code)]
    fn handle_ant_power_data(&self, device_number: u16, power_watts: u16, cadence_rpm: Option<u8>) {
        let device_id = Self::create_ant_device_id(device_number, AntDeviceType::Power);

        let reading = SensorReading {
            sensor_id: Uuid::nil(),
            timestamp: Instant::now(),
            power_watts: Some(power_watts),
            cadence_rpm,
            heart_rate_bpm: None,
            speed_kmh: None,
            distance_delta_m: None,
        };

        self.send_event(SensorEvent::Data(reading));

        tracing::trace!(
            "ANT+ Power data from {}: {}W, {:?}rpm",
            device_id,
            power_watts,
            cadence_rpm
        );
    }

    /// Handle ANT+ FE-C (Fitness Equipment) data.
    /// Called when general FE data page is received from a smart trainer.
    #[allow(dead_code)]
    fn handle_ant_fec_data(
        &self,
        device_number: u16,
        power_watts: Option<u16>,
        cadence_rpm: Option<u8>,
        speed_kmh: Option<f32>,
    ) {
        let device_id = Self::create_ant_device_id(device_number, AntDeviceType::FitnessEquipment);

        let reading = SensorReading {
            sensor_id: Uuid::nil(),
            timestamp: Instant::now(),
            power_watts,
            cadence_rpm,
            heart_rate_bpm: None,
            speed_kmh,
            distance_delta_m: None,
        };

        self.send_event(SensorEvent::Data(reading));

        tracing::trace!(
            "ANT+ FE-C data from {}: {:?}W, {:?}rpm, {:?}km/h",
            device_id,
            power_watts,
            cadence_rpm,
            speed_kmh
        );
    }

    /// Handle ANT+ speed/cadence sensor data.
    #[allow(dead_code)]
    fn handle_ant_speed_cadence_data(
        &self,
        device_number: u16,
        speed_kmh: Option<f32>,
        cadence_rpm: Option<u8>,
    ) {
        let device_id = Self::create_ant_device_id(device_number, AntDeviceType::SpeedCadence);

        let reading = SensorReading {
            sensor_id: Uuid::nil(),
            timestamp: Instant::now(),
            power_watts: None,
            cadence_rpm,
            heart_rate_bpm: None,
            speed_kmh,
            distance_delta_m: None,
        };

        self.send_event(SensorEvent::Data(reading));

        tracing::trace!(
            "ANT+ S/C data from {}: {:?}km/h, {:?}rpm",
            device_id,
            speed_kmh,
            cadence_rpm
        );
    }

    /// Subscribe to ANT+ events from the dongle manager.
    /// Returns a receiver for ANT+ events.
    #[allow(dead_code)]
    pub fn subscribe_ant_events(&self) -> Option<tokio::sync::broadcast::Receiver<AntEvent>> {
        self.ant_manager.as_ref().map(|m| m.subscribe_events())
    }

    /// Stop scanning for BLE and ANT+ sensors.
    pub async fn stop_discovery(&mut self) -> Result<(), SensorError> {
        let adapter = self.adapter.as_ref().ok_or(SensorError::AdapterNotFound)?;

        {
            let mut is_scanning = self.is_scanning.lock().await;
            if !*is_scanning {
                return Ok(()); // Not scanning
            }
            *is_scanning = false;
        }

        // Cancel the timeout task
        if let Some(handle) = self.discovery_timeout_handle.lock().await.take() {
            handle.abort();
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

        // Create sensor state and cache the sensor
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

            // Cache the sensor for fast reconnection
            let mut cache = self.sensor_cache.lock().await;
            cache.cache_sensor(
                device_id.to_string(),
                disc_sensor.name.clone(),
                disc_sensor.sensor_type,
                disc_sensor.protocol,
            );
            if let Err(e) = cache.save() {
                tracing::warn!("Failed to save sensor cache: {}", e);
            }
        }

        // Send connected state
        self.send_event(SensorEvent::ConnectionChanged {
            device_id: device_id.to_string(),
            state: ConnectionState::Connected,
        });

        // Start notification handler with auto-reconnect support (T029)
        let ctx = NotificationContext {
            event_tx: self.event_tx.clone(),
            sensor_states: self.sensor_states.clone(),
            device_id: device_id.to_string(),
            reconnect_attempts: self.reconnect_attempts.clone(),
            max_reconnect_attempts: self.config.max_reconnect_attempts,
            reconnect_delay_secs: self.config.reconnect_delay_secs,
            auto_reconnect: self.config.auto_reconnect,
        };

        tokio::spawn(async move {
            Self::handle_notifications(peripheral, ctx).await;
        });

        tracing::info!("Connected to sensor: {}", device_id);

        Ok(())
    }

    /// Subscribe to sensor data characteristics.
    async fn subscribe_to_characteristics(
        &self,
        peripheral: &Peripheral,
    ) -> Result<(), SensorError> {
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
    async fn handle_notifications(peripheral: Peripheral, ctx: NotificationContext) {
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
                Self::parse_ftms_notification(&data, &ctx.device_id)
            } else if char_uuid == CYCLING_POWER_MEASUREMENT_UUID {
                Self::parse_power_notification(&data, &ctx.device_id)
            } else if char_uuid == HEART_RATE_MEASUREMENT_UUID {
                Self::parse_hr_notification(&data, &ctx.device_id)
            } else {
                None
            };

            if let Some(reading) = reading {
                // Update last data time
                if let Some(state) = ctx.sensor_states.lock().await.get_mut(&ctx.device_id) {
                    state.last_data_at = Some(Instant::now());
                }

                // Reset reconnect attempts on successful data
                ctx.reconnect_attempts.lock().await.remove(&ctx.device_id);

                // Send data event
                if let Some(tx) = &ctx.event_tx {
                    let _ = tx.send(SensorEvent::Data(reading));
                }
            }
        }

        // Stream ended - peripheral disconnected
        tracing::warn!(
            "Sensor {} notification stream ended (disconnected)",
            ctx.device_id
        );

        // Update sensor state to disconnected
        if let Some(state) = ctx.sensor_states.lock().await.get_mut(&ctx.device_id) {
            state.connection_state = ConnectionState::Disconnected;
        }

        // Check if we should attempt auto-reconnect (T029)
        if ctx.auto_reconnect {
            let mut attempts = ctx.reconnect_attempts.lock().await;
            let attempt_count = attempts.entry(ctx.device_id.clone()).or_insert(0);

            if *attempt_count < ctx.max_reconnect_attempts {
                *attempt_count += 1;
                let current_attempt = *attempt_count;
                drop(attempts);

                tracing::info!(
                    "Auto-reconnect attempt {}/{} for sensor {}",
                    current_attempt,
                    ctx.max_reconnect_attempts,
                    ctx.device_id
                );

                // Send reconnecting state
                if let Some(tx) = &ctx.event_tx {
                    let _ = tx.send(SensorEvent::ConnectionChanged {
                        device_id: ctx.device_id.clone(),
                        state: ConnectionState::Reconnecting,
                    });
                }

                // Update sensor state
                if let Some(state) = ctx.sensor_states.lock().await.get_mut(&ctx.device_id) {
                    state.connection_state = ConnectionState::Reconnecting;
                }

                // Wait before reconnect attempt
                tokio::time::sleep(std::time::Duration::from_secs(ctx.reconnect_delay_secs)).await;

                // Try to reconnect
                if let Err(e) = peripheral.connect().await {
                    tracing::warn!("Reconnection failed for {}: {}", ctx.device_id, e);

                    // Send final disconnected state if all attempts exhausted
                    if current_attempt >= ctx.max_reconnect_attempts {
                        if let Some(tx) = &ctx.event_tx {
                            let _ = tx.send(SensorEvent::ConnectionChanged {
                                device_id: ctx.device_id.clone(),
                                state: ConnectionState::Disconnected,
                            });
                            let _ = tx.send(SensorEvent::Error(format!(
                                "Failed to reconnect to {} after {} attempts",
                                ctx.device_id, ctx.max_reconnect_attempts
                            )));
                        }
                    }
                } else {
                    tracing::info!("Reconnected to sensor {}", ctx.device_id);

                    // Rediscover services and resubscribe
                    if let Err(e) = peripheral.discover_services().await {
                        tracing::error!("Failed to rediscover services: {}", e);
                        return;
                    }

                    // Resubscribe to characteristics
                    for char in peripheral.characteristics() {
                        let char_uuid = char.uuid;
                        if char_uuid == INDOOR_BIKE_DATA_UUID
                            || char_uuid == CYCLING_POWER_MEASUREMENT_UUID
                            || char_uuid == HEART_RATE_MEASUREMENT_UUID
                        {
                            if let Err(e) = peripheral.subscribe(&char).await {
                                tracing::warn!("Failed to resubscribe to {}: {}", char_uuid, e);
                            }
                        }
                    }

                    // Update state to connected
                    if let Some(state) = ctx.sensor_states.lock().await.get_mut(&ctx.device_id) {
                        state.connection_state = ConnectionState::Connected;
                    }

                    if let Some(tx) = &ctx.event_tx {
                        let _ = tx.send(SensorEvent::ConnectionChanged {
                            device_id: ctx.device_id.clone(),
                            state: ConnectionState::Connected,
                        });
                    }

                    // Reset attempts on successful reconnect
                    ctx.reconnect_attempts.lock().await.remove(&ctx.device_id);

                    // Recursively handle notifications again
                    Box::pin(Self::handle_notifications(peripheral, ctx)).await;
                    return;
                }
            } else {
                // Max attempts reached
                tracing::warn!(
                    "Max reconnect attempts ({}) reached for sensor {}",
                    ctx.max_reconnect_attempts,
                    ctx.device_id
                );
            }
        }

        // Send final disconnected event
        if let Some(tx) = &ctx.event_tx {
            let _ = tx.send(SensorEvent::ConnectionChanged {
                device_id: ctx.device_id,
                state: ConnectionState::Disconnected,
            });
        }
    }

    /// Parse FTMS Indoor Bike Data notification.
    fn parse_ftms_notification(data: &[u8], _device_id: &str) -> Option<SensorReading> {
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
    fn parse_power_notification(data: &[u8], _device_id: &str) -> Option<SensorReading> {
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
    fn parse_hr_notification(data: &[u8], _device_id: &str) -> Option<SensorReading> {
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

    /// Get the current discovery phase.
    pub async fn get_discovery_phase(&self) -> Option<DiscoveryPhase> {
        self.progressive_timeout_state
            .lock()
            .await
            .as_ref()
            .map(|s| s.phase)
    }

    /// Get detailed discovery progress information.
    pub async fn get_discovery_progress(&self) -> Option<DiscoveryProgress> {
        let state = self.progressive_timeout_state.lock().await;
        state.as_ref().map(|s| DiscoveryProgress {
            phase: s.phase,
            elapsed: s.elapsed(),
            sensors_discovered: s.sensors_discovered,
            extensions_count: s.extensions_count,
            is_active: s.phase != DiscoveryPhase::Completed,
        })
    }

    /// Get the progressive timeout configuration.
    pub fn get_progressive_timeout_config(&self) -> &ProgressiveTimeoutConfig {
        &self.config.progressive_timeout
    }

    /// Set the progressive timeout configuration.
    ///
    /// Note: This only affects future discovery scans, not the current one.
    pub fn set_progressive_timeout_config(&mut self, config: ProgressiveTimeoutConfig) {
        self.config.progressive_timeout = config;
    }

    /// Get all sensor states (connected and recently seen).
    pub async fn get_sensor_states(&self) -> Vec<SensorState> {
        self.sensor_states.lock().await.values().cloned().collect()
    }

    /// Check if a controllable trainer is connected (FTMS support).
    pub async fn has_controllable_trainer(&self) -> bool {
        let states = self.sensor_states.lock().await;
        states.values().any(|s| {
            s.sensor_type == SensorType::Trainer
                && s.connection_state == ConnectionState::Connected
                && s.protocol == Protocol::BleFtms
        })
    }

    /// Set simulation mode grade on a trainer.
    pub async fn set_simulation_grade(
        &self,
        device_id: &str,
        grade_percent: f32,
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

        // Build and send the simulation parameters command
        let cmd = crate::sensors::ftms::build_set_simulation_grade(grade_percent);

        peripheral
            .write(control_point, &cmd, WriteType::WithResponse)
            .await
            .map_err(|e| SensorError::WriteFailed(e.to_string()))?;

        tracing::debug!("Set simulation grade to {}%", grade_percent);

        Ok(())
    }

    // =========================================================================
    // Sensor Cache Methods for Fast Reconnection
    // =========================================================================

    /// Get cached sensors for fast reconnection.
    ///
    /// Returns sensors sorted by reconnection priority:
    /// 1. Preferred sensors first
    /// 2. Then by connection count (most used)
    /// 3. Then by last connected (most recent)
    pub async fn get_cached_sensors(&self) -> Vec<crate::sensors::cache::CachedSensor> {
        let cache = self.sensor_cache.lock().await;
        cache.reconnection_priority().into_iter().cloned().collect()
    }

    /// Get cached sensors of a specific type.
    pub async fn get_cached_sensors_of_type(
        &self,
        sensor_type: SensorType,
    ) -> Vec<crate::sensors::cache::CachedSensor> {
        let cache = self.sensor_cache.lock().await;
        cache.sensors_of_type(sensor_type).into_iter().cloned().collect()
    }

    /// Check if a sensor is in the cache.
    pub async fn is_sensor_cached(&self, device_id: &str) -> bool {
        let cache = self.sensor_cache.lock().await;
        cache.contains(device_id)
    }

    /// Attempt fast reconnection to cached sensors.
    ///
    /// Tries to connect to sensors that were previously connected without
    /// requiring a full discovery scan. This is significantly faster for
    /// known sensors.
    ///
    /// Returns a list of device IDs that were successfully found and added
    /// to discovered sensors (ready for connection).
    pub async fn fast_reconnect_cached(&mut self) -> Result<Vec<String>, SensorError> {
        let adapter = self.adapter.as_ref().ok_or(SensorError::AdapterNotFound)?;

        let cached_sensors: Vec<_> = {
            let cache = self.sensor_cache.lock().await;
            cache.reconnection_priority().into_iter().cloned().collect()
        };

        if cached_sensors.is_empty() {
            tracing::debug!("No cached sensors for fast reconnection");
            return Ok(Vec::new());
        }

        tracing::info!(
            "Attempting fast reconnection to {} cached sensors",
            cached_sensors.len()
        );

        let start = Instant::now();
        let mut found_sensors = Vec::new();

        // Get all currently visible peripherals
        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|e| SensorError::BleError(e.to_string()))?;

        for peripheral in peripherals {
            let peripheral_id = peripheral.id().to_string();

            // Check if this peripheral matches any cached sensor
            for cached in &cached_sensors {
                if cached.device_id == peripheral_id {
                    // Found a cached sensor! Add it to discovered
                    let sensor = DiscoveredSensor {
                        device_id: cached.device_id.clone(),
                        name: cached.name.clone(),
                        sensor_type: cached.sensor_type,
                        protocol: cached.protocol,
                        signal_strength: None,
                        last_seen: Instant::now(),
                    };

                    self.discovered
                        .lock()
                        .await
                        .insert(cached.device_id.clone(), sensor.clone());

                    self.send_event(SensorEvent::Discovered(sensor));
                    found_sensors.push(cached.device_id.clone());

                    tracing::info!(
                        "Fast reconnect: found cached sensor {} ({})",
                        cached.display_name(),
                        cached.device_id
                    );
                    break;
                }
            }
        }

        let elapsed = start.elapsed();
        tracing::info!(
            "Fast reconnection found {}/{} cached sensors in {:?}",
            found_sensors.len(),
            cached_sensors.len(),
            elapsed
        );

        Ok(found_sensors)
    }

    /// Try to connect to all preferred cached sensors.
    ///
    /// This method attempts fast reconnection and then connects to any
    /// preferred sensors that were found.
    pub async fn connect_preferred_sensors(&mut self) -> Result<Vec<String>, SensorError> {
        // First try fast reconnection
        let found = self.fast_reconnect_cached().await?;

        // Get preferred sensors that were found
        let preferred_ids: Vec<String> = {
            let cache = self.sensor_cache.lock().await;
            cache
                .preferred_sensors()
                .iter()
                .filter(|s| found.contains(&s.device_id))
                .map(|s| s.device_id.clone())
                .collect()
        };

        // Connect to each preferred sensor
        let mut connected_ids = Vec::new();
        for device_id in preferred_ids {
            match self.connect(&device_id).await {
                Ok(()) => {
                    connected_ids.push(device_id);
                }
                Err(e) => {
                    tracing::warn!("Failed to connect to preferred sensor {}: {}", device_id, e);
                }
            }
        }

        Ok(connected_ids)
    }

    /// Set a sensor as preferred for auto-reconnection.
    pub async fn set_sensor_preferred(&self, device_id: &str, preferred: bool) -> bool {
        let mut cache = self.sensor_cache.lock().await;
        let result = cache.set_preferred(device_id, preferred);
        if result {
            if let Err(e) = cache.save() {
                tracing::warn!("Failed to save sensor cache: {}", e);
            }
        }
        result
    }

    /// Set a nickname for a cached sensor.
    pub async fn set_sensor_nickname(&self, device_id: &str, nickname: Option<String>) -> bool {
        let mut cache = self.sensor_cache.lock().await;
        let result = cache.set_nickname(device_id, nickname);
        if result {
            if let Err(e) = cache.save() {
                tracing::warn!("Failed to save sensor cache: {}", e);
            }
        }
        result
    }

    /// Remove a sensor from the cache.
    pub async fn remove_cached_sensor(&self, device_id: &str) -> bool {
        let mut cache = self.sensor_cache.lock().await;
        let removed = cache.remove(device_id).is_some();
        if removed {
            if let Err(e) = cache.save() {
                tracing::warn!("Failed to save sensor cache: {}", e);
            }
        }
        removed
    }

    /// Clear all cached sensors.
    pub async fn clear_sensor_cache(&self) {
        let mut cache = self.sensor_cache.lock().await;
        cache.clear();
        if let Err(e) = cache.save() {
            tracing::warn!("Failed to save sensor cache: {}", e);
        }
    }

    /// Prune stale sensors from the cache.
    ///
    /// Returns the number of sensors removed.
    pub async fn prune_stale_sensors(&self) -> usize {
        let mut cache = self.sensor_cache.lock().await;
        let count = cache.prune_stale();
        if count > 0 {
            if let Err(e) = cache.save() {
                tracing::warn!("Failed to save sensor cache: {}", e);
            }
        }
        count
    }

    /// Get the number of cached sensors.
    pub async fn cached_sensor_count(&self) -> usize {
        let cache = self.sensor_cache.lock().await;
        cache.len()
    }

    // =========================================================================
    // Priority-Based Connection Queue Methods
    // =========================================================================

    /// Add a discovered sensor to the connection queue.
    ///
    /// The sensor will be automatically prioritized based on its type:
    /// - Primary (trainers, power meters) connect first
    /// - Secondary (HR, cadence) connect after
    pub async fn queue_sensor_for_connection(&self, sensor: DiscoveredSensor) {
        let mut queue = self.connection_queue.lock().await;
        queue.enqueue(sensor);
    }

    /// Add a preferred sensor to the connection queue (highest priority within its level).
    pub async fn queue_preferred_sensor(&self, sensor: DiscoveredSensor) {
        let mut queue = self.connection_queue.lock().await;
        queue.enqueue_preferred(sensor);
    }

    /// Add all discovered sensors to the connection queue.
    ///
    /// Sensors are automatically prioritized by type.
    pub async fn queue_all_discovered(&self) {
        let discovered = self.discovered.lock().await.clone();
        let cache = self.sensor_cache.lock().await;
        let mut queue = self.connection_queue.lock().await;

        for sensor in discovered.into_values() {
            // Check if this sensor is preferred in the cache
            if cache.get(&sensor.device_id).map_or(false, |c| c.is_preferred) {
                queue.enqueue_preferred(sensor);
            } else {
                queue.enqueue(sensor);
            }
        }
    }

    /// Connect to the next sensor in the priority queue.
    ///
    /// Returns the device ID of the sensor that connection was attempted for,
    /// or None if the queue is empty.
    pub async fn connect_next_in_queue(&mut self) -> Result<Option<String>, SensorError> {
        let entry = {
            let mut queue = self.connection_queue.lock().await;
            queue.dequeue()
        };

        match entry {
            Some(entry) => {
                let device_id = entry.sensor.device_id.clone();
                tracing::info!(
                    "Connecting to {} sensor: {} ({})",
                    entry.priority,
                    entry.name(),
                    device_id
                );
                self.connect(&device_id).await?;
                Ok(Some(device_id))
            }
            None => Ok(None),
        }
    }

    /// Connect to all sensors in the queue in priority order.
    ///
    /// Primary sensors (trainers, power meters) are connected first,
    /// then secondary sensors (HR, cadence).
    ///
    /// Returns a list of device IDs that were successfully connected.
    pub async fn connect_all_in_queue(&mut self) -> Vec<String> {
        let entries = {
            let mut queue = self.connection_queue.lock().await;
            queue.drain_in_order()
        };

        let mut connected = Vec::new();

        for entry in entries {
            let device_id = entry.sensor.device_id.clone();
            tracing::info!(
                "Connecting to {} sensor: {} ({})",
                entry.priority,
                entry.name(),
                device_id
            );

            match self.connect(&device_id).await {
                Ok(()) => {
                    connected.push(device_id);
                }
                Err(e) => {
                    tracing::warn!("Failed to connect to {}: {}", device_id, e);
                    // Continue with other sensors
                }
            }
        }

        if !connected.is_empty() {
            tracing::info!(
                "Connected to {} sensors in priority order",
                connected.len()
            );
        }

        connected
    }

    /// Connect to discovered sensors in priority order.
    ///
    /// This is a convenience method that:
    /// 1. Queues all discovered sensors
    /// 2. Connects to them in priority order (primary first)
    ///
    /// Returns a list of device IDs that were successfully connected.
    pub async fn connect_discovered_by_priority(&mut self) -> Vec<String> {
        // Queue all discovered sensors
        self.queue_all_discovered().await;

        // Connect in priority order
        self.connect_all_in_queue().await
    }

    /// Get the number of sensors in the connection queue.
    pub async fn connection_queue_len(&self) -> usize {
        self.connection_queue.lock().await.len()
    }

    /// Check if the connection queue is empty.
    pub async fn is_connection_queue_empty(&self) -> bool {
        self.connection_queue.lock().await.is_empty()
    }

    /// Get the count of primary and secondary sensors in the queue.
    pub async fn connection_queue_counts(&self) -> (usize, usize) {
        self.connection_queue.lock().await.count_by_priority()
    }

    /// Clear the connection queue.
    pub async fn clear_connection_queue(&self) {
        self.connection_queue.lock().await.clear();
    }

    /// Remove a sensor from the connection queue.
    pub async fn remove_from_connection_queue(&self, device_id: &str) -> bool {
        self.connection_queue.lock().await.remove(device_id)
    }

    /// Peek at the next sensor in the queue without removing it.
    pub async fn peek_connection_queue(&self) -> Option<(String, SensorPriority)> {
        self.connection_queue.lock().await.peek().map(|entry| {
            (entry.sensor.device_id.clone(), entry.priority)
        })
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

        // Save sensor cache
        let mut cache = self.sensor_cache.lock().await;
        if let Err(e) = cache.save() {
            tracing::warn!("Failed to save sensor cache on shutdown: {}", e);
        }
    }
}
