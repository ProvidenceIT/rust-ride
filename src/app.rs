//! Main application state and egui integration.
//!
//! T042: Create App struct with egui state
//! T044: Implement screen navigation state machine
//! T050: Wire sensor data to UI via crossbeam channel
//! T157: Implement crash recovery prompt on startup
//! T043: Integrate achievement tracking into ride completion flow

use eframe::egui;

use crossbeam::channel::Receiver;
use rustride::accessibility::FocusManager;
use rustride::achievements::{
    AchievementTracker, AllCheckers, CumulativeStats, DefaultAchievementTracker, NotificationQueue,
    RideMetrics,
};
use rustride::audio::{AudioEngine, DefaultAudioEngine};
use rustride::hid::{
    load_hid_config_from_db, save_hid_config_to_db, ButtonInputHandler, ButtonMapping,
    DefaultButtonInputHandler, DefaultHidDeviceManager, ExecutorEvent, HidConfig, NavigationTarget,
};
use rustride::integrations::mqtt::{
    DefaultFanController, DefaultMqttClient, FanController, FanProfile, MqttClient, MqttConfig,
};
use rustride::integrations::streaming::{
    DefaultPinAuthenticator, DefaultStreamingServer, PinAuthenticator, StreamingConfig,
    StreamingMetrics, StreamingServer,
};
use rustride::metrics::MetricsCalculator;
use rustride::onboarding::OnboardingState;
use rustride::recording::RideRecorder;
use rustride::sensors::types::{ConnectionState, SensorEvent};
use rustride::sensors::{
    CadenceFusion, DefaultInclineController, FusionMode, InclineConfig, InclineController,
    SensorFusion, SensorFusionConfig, SensorManager,
};
use rustride::storage::config::{get_data_dir, AppConfig, UserProfile};
use rustride::storage::{Database, HardwareStore};
use rustride::ui::screens::{
    AnalyticsScreen, AvatarScreen, HomeScreen, OnboardingScreen, RideScreen, RideView, Screen,
    SensorSetupScreen, SettingsScreen, WorldSelectScreen,
};
use rustride::ui::theme::Theme;
use rustride::ui::widgets::AchievementNotificationWidget;
use rustride::workouts::WorkoutEngine;
use rustride::world::physics::GradientController;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex as TokioMutex;
use uuid::Uuid;

/// Crash recovery dialog state.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RecoveryState {
    /// No recovery data or already handled
    None,
    /// Recovery data found, showing prompt
    Pending {
        /// Timestamp of the recovered ride
        timestamp: String,
        /// Duration of the recovered ride
        duration: String,
        /// Number of samples in the recovered ride
        sample_count: usize,
    },
    /// User chose to recover
    Recovering,
    /// User chose to discard
    Discarding,
}

/// Main application state.
pub struct RustRideApp {
    /// Current screen
    current_screen: Screen,
    /// UI theme
    theme: Theme,
    /// User profile
    profile: UserProfile,
    /// Application configuration
    _config: AppConfig,
    /// Sensor manager (wrapped for async access)
    sensor_manager: Arc<TokioMutex<SensorManager>>,
    /// Tokio runtime for async operations
    tokio_runtime: Arc<tokio::runtime::Runtime>,
    /// Workout engine
    _workout_engine: WorkoutEngine,
    /// Ride recorder
    _ride_recorder: RideRecorder,
    /// Metrics calculator
    metrics_calculator: MetricsCalculator,
    /// Audio engine for voice alerts and sound effects (Hardware Integration)
    audio_engine: Arc<DefaultAudioEngine>,
    /// Sensor setup screen state
    sensor_setup_screen: SensorSetupScreen,
    /// Ride screen state
    ride_screen: RideScreen,
    /// World selection screen state
    world_select_screen: WorldSelectScreen,
    /// Avatar customization screen state
    avatar_screen: AvatarScreen,
    /// Analytics screen state
    analytics_screen: AnalyticsScreen,
    /// Settings screen state
    settings_screen: SettingsScreen,
    /// T043: Incline/slope mode controller
    incline_controller: DefaultInclineController,
    /// T043: Gradient controller for route-based resistance
    gradient_controller: GradientController,
    /// T071: MQTT client for smart home integration (reserved for future use)
    #[allow(dead_code)]
    mqtt_client: Arc<DefaultMqttClient>,
    /// T071: Fan controller for zone-based fan speed control
    fan_controller: Arc<DefaultFanController<DefaultMqttClient>>,
    /// T071: MQTT configuration
    mqtt_config: MqttConfig,
    /// T080: Streaming server for external displays
    streaming_server: Arc<DefaultStreamingServer>,
    /// T080: Streaming configuration
    streaming_config: StreamingConfig,
    /// T091: HID device manager for USB buttons/Stream Deck (reserved for future use)
    #[allow(dead_code)]
    hid_device_manager: Arc<DefaultHidDeviceManager>,
    /// T091: Button input handler for mapping (reserved for future use)
    #[allow(dead_code)]
    button_input_handler: Arc<DefaultButtonInputHandler>,
    /// T091: Executor event receiver for UI navigation actions
    executor_event_rx: Option<tokio::sync::broadcast::Receiver<ExecutorEvent>>,
    /// Sensor event receiver
    sensor_event_rx: Option<Receiver<SensorEvent>>,
    /// Last UI update time
    last_update: Instant,
    /// Sensor status for status bar
    sensor_status: String,
    /// Number of connected sensors
    connected_sensor_count: usize,
    /// Crash recovery state
    recovery_state: RecoveryState,
    /// T135: Cadence sensor fusion for multi-source cadence
    cadence_fusion: CadenceFusion,
    /// T135: Track primary cadence sensor ID
    primary_cadence_sensor: Option<uuid::Uuid>,
    /// T135: Track secondary cadence sensor ID
    secondary_cadence_sensor: Option<uuid::Uuid>,
    /// T029: Focus manager for keyboard navigation
    focus_manager: FocusManager,
    /// T059: Onboarding screen for first-time user experience
    onboarding_screen: OnboardingScreen,
    /// T043: Achievement tracker for gamification
    achievement_tracker: DefaultAchievementTracker,
    /// T043: Achievement notification queue
    achievement_notification_queue: NotificationQueue,
    /// T043: Achievement notification widget
    achievement_notification_widget: AchievementNotificationWidget,
    /// T043: Achievement checker for ride completion
    achievement_checker: AllCheckers,
    /// T043: Cumulative stats for lifetime achievements
    cumulative_stats: CumulativeStats,
    /// T043: User ID for achievement tracking
    user_id: Uuid,
    /// T091: Database connection for persistent storage
    database: Option<Database>,
    /// T071/4.3: Pending MQTT connection test task handle
    pending_mqtt_test:
        Option<tokio::task::JoinHandle<rustride::integrations::mqtt::MqttTestResult>>,
}

impl RustRideApp {
    /// Create a new application instance.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load configuration
        let config = rustride::storage::config::load_config().unwrap_or_default();

        // Create default profile
        let profile = UserProfile::default();

        // T043: Generate or load user ID for achievement tracking
        // In a full implementation, this would be loaded from the database
        let user_id = Uuid::new_v4();

        // Set up theme
        let theme = Theme::Dark;
        cc.egui_ctx.set_visuals(theme.visuals());

        // Note: Using default egui fonts for now
        // Custom fonts can be configured later if needed

        // Create tokio runtime for async operations (BLE, ANT+, etc.)
        let tokio_runtime = Arc::new(
            tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime for async operations"),
        );

        // Create and initialize sensor manager
        let mut sensor_manager = SensorManager::with_defaults();
        let sensor_event_rx = Some(sensor_manager.event_receiver());

        // Initialize BLE adapter asynchronously
        let rt = tokio_runtime.clone();
        let init_result = rt.block_on(async { sensor_manager.initialize().await });

        if let Err(e) = init_result {
            tracing::error!(
                "Failed to initialize BLE adapter: {}. Bluetooth sensors will not be available.",
                e
            );
            tracing::info!("Please check that Bluetooth is enabled on your system and that the application has permission to access it.");
        } else {
            tracing::info!("BLE adapter initialized successfully");

            // Try to initialize ANT+ support (optional)
            if let Err(e) = rt.block_on(async { sensor_manager.initialize_ant().await }) {
                tracing::warn!(
                    "Failed to initialize ANT+ support: {}. ANT+ sensors will not be available.",
                    e
                );
            }
        }

        let sensor_manager = Arc::new(TokioMutex::new(sensor_manager));

        let workout_engine = WorkoutEngine::new();
        let ride_recorder = RideRecorder::with_defaults();
        let metrics_calculator = MetricsCalculator::new(profile.ftp);

        // Initialize audio engine (Hardware Integration)
        // Use audio config from loaded AppConfig for persistence
        let audio_engine = Arc::new(DefaultAudioEngine::new(config.audio.clone()));
        if let Err(e) = audio_engine.initialize() {
            tracing::warn!("Failed to initialize audio engine: {}", e);
        }

        // Check for crash recovery data
        let recovery_state = if ride_recorder.has_recovery_data() {
            // In a real implementation, we'd get the actual recovery data here
            tracing::info!("Found crash recovery data from previous session");
            RecoveryState::Pending {
                timestamp: "Unknown".to_string(),
                duration: "Unknown".to_string(),
                sample_count: 0,
            }
        } else {
            RecoveryState::None
        };

        // Initialize incline controller with default config
        let incline_config = InclineConfig {
            rider_weight_kg: profile.weight_kg,
            ..InclineConfig::default()
        };
        let incline_controller = DefaultInclineController::new(incline_config.clone());
        let gradient_controller = GradientController::new();

        // T071: Initialize MQTT client and fan controller
        let mqtt_config = MqttConfig::default();
        let mqtt_client = Arc::new(DefaultMqttClient::new());
        let fan_controller = Arc::new(DefaultFanController::new(mqtt_client.clone()));

        // Load default fan profile
        let default_fan_profile = FanProfile::default();
        fan_controller.configure(vec![default_fan_profile]);

        // T080: Initialize streaming server for external displays
        let streaming_config = StreamingConfig::default();
        let pin_auth: Arc<dyn PinAuthenticator> = Arc::new(DefaultPinAuthenticator::new(
            streaming_config.pin_expiry_minutes,
        ));
        let streaming_server = Arc::new(DefaultStreamingServer::new(pin_auth));

        // T091: Initialize database for HID config persistence
        let db_path = get_data_dir().join("rustride.db");
        let database = match Database::open(&db_path) {
            Ok(db) => {
                tracing::info!("Opened database at {:?}", db_path);
                Some(db)
            }
            Err(e) => {
                tracing::error!("Failed to open database: {}. HID config will not persist.", e);
                None
            }
        };

        // T091: Initialize HID device manager and button input handler
        // Load HID config from database if available
        let hid_config = if let Some(ref db) = database {
            let store = HardwareStore::new(db.connection());
            load_hid_config_from_db(&store, &user_id)
        } else {
            HidConfig::default()
        };

        let hid_device_manager = Arc::new(DefaultHidDeviceManager::new(hid_config.clone()));
        let button_input_handler = Arc::new(DefaultButtonInputHandler::new());

        // T091: Register loaded button mappings with the handler
        for device_config in &hid_config.devices {
            if device_config.enabled {
                let mappings: Vec<ButtonMapping> = device_config
                    .mappings
                    .iter()
                    .map(|m| ButtonMapping::new(device_config.device_id, m.button_code, m.action.clone()))
                    .collect();
                if !mappings.is_empty() {
                    button_input_handler.register_mappings(&device_config.device_id, mappings);
                    tracing::info!(
                        "Loaded {} button mappings for device {}",
                        device_config.mappings.len(),
                        device_config.name
                    );
                }
            }
        }

        // T091: Create executor event channel for UI navigation actions
        // The executor_event_rx will be set when the action executor is created
        // For now, create a placeholder channel that can receive navigation events
        let (executor_event_tx, executor_event_rx) =
            tokio::sync::broadcast::channel::<ExecutorEvent>(100);
        drop(executor_event_tx); // Drop sender for now - will be replaced with actual executor integration

        // T135: Initialize cadence sensor fusion
        let fusion_config = SensorFusionConfig::default();
        let cadence_fusion = CadenceFusion::with_config(fusion_config);

        // T029: Initialize focus manager for keyboard navigation
        let focus_manager = FocusManager::new();

        // T059: Initialize onboarding screen and check if it should be shown
        // In a real implementation, we'd load the onboarding state from storage
        let onboarding_state = load_onboarding_state();
        let onboarding_screen = OnboardingScreen::from_wizard(
            rustride::onboarding::OnboardingWizard::from_state(onboarding_state.clone()),
        );

        // Determine starting screen based on onboarding state
        let start_screen = if onboarding_screen.should_show() {
            Screen::Onboarding
        } else {
            Screen::Home
        };

        // Initialize settings screen with profile and HID config
        let mut settings_screen = SettingsScreen::new(profile.clone());
        settings_screen.set_incline_config(incline_config);
        settings_screen.set_hid_config(hid_config);

        Self {
            current_screen: start_screen,
            theme,
            profile,
            _config: config,
            sensor_manager,
            tokio_runtime,
            _workout_engine: workout_engine,
            _ride_recorder: ride_recorder,
            metrics_calculator,
            audio_engine,
            sensor_setup_screen: SensorSetupScreen::new(),
            ride_screen: RideScreen::new(),
            world_select_screen: WorldSelectScreen::new(),
            avatar_screen: AvatarScreen::new(),
            analytics_screen: AnalyticsScreen::new(),
            settings_screen,
            incline_controller,
            gradient_controller,
            mqtt_client,
            fan_controller,
            mqtt_config,
            streaming_server,
            streaming_config,
            hid_device_manager,
            button_input_handler,
            executor_event_rx: Some(executor_event_rx),
            sensor_event_rx,
            last_update: Instant::now(),
            sensor_status: "No sensors connected".to_string(),
            connected_sensor_count: 0,
            recovery_state,
            cadence_fusion,
            primary_cadence_sensor: None,
            secondary_cadence_sensor: None,
            focus_manager,
            onboarding_screen,
            // T043: Achievement system initialization
            achievement_tracker: DefaultAchievementTracker::new(user_id),
            achievement_notification_queue: NotificationQueue::default(),
            achievement_notification_widget: AchievementNotificationWidget::default(),
            achievement_checker: AllCheckers::new(),
            cumulative_stats: CumulativeStats::default(),
            user_id,
            database,
            pending_mqtt_test: None,
        }
    }

    /// Start sensor discovery (BLE and ANT+).
    fn start_sensor_discovery(&mut self) {
        let sensor_manager = self.sensor_manager.clone();
        let rt = self.tokio_runtime.clone();

        rt.spawn(async move {
            let mut sm = sensor_manager.lock().await;
            if let Err(e) = sm.start_discovery().await {
                tracing::error!("Failed to start sensor discovery: {}", e);
            } else {
                tracing::info!("Sensor discovery started");
            }
        });

        self.sensor_setup_screen.set_scanning(true);
    }

    /// Stop sensor discovery.
    fn stop_sensor_discovery(&mut self) {
        let sensor_manager = self.sensor_manager.clone();
        let rt = self.tokio_runtime.clone();

        rt.spawn(async move {
            let mut sm = sensor_manager.lock().await;
            if let Err(e) = sm.stop_discovery().await {
                tracing::error!("Failed to stop sensor discovery: {}", e);
            } else {
                tracing::info!("Sensor discovery stopped");
            }
        });

        self.sensor_setup_screen.set_scanning(false);
    }

    /// Connect to a sensor by device ID.
    #[allow(dead_code)]
    fn connect_to_sensor(&mut self, device_id: String) {
        let sensor_manager = self.sensor_manager.clone();
        let rt = self.tokio_runtime.clone();

        rt.spawn(async move {
            let mut sm = sensor_manager.lock().await;
            if let Err(e) = sm.connect(&device_id).await {
                tracing::error!("Failed to connect to sensor {}: {}", device_id, e);
            } else {
                tracing::info!("Connected to sensor: {}", device_id);
            }
        });
    }

    /// Disconnect from a sensor by device ID.
    #[allow(dead_code)]
    fn disconnect_from_sensor(&mut self, device_id: String) {
        let sensor_manager = self.sensor_manager.clone();
        let rt = self.tokio_runtime.clone();

        rt.spawn(async move {
            let mut sm = sensor_manager.lock().await;
            if let Err(e) = sm.disconnect(&device_id).await {
                tracing::error!("Failed to disconnect from sensor {}: {}", device_id, e);
            } else {
                tracing::info!("Disconnected from sensor: {}", device_id);
            }
        });
    }

    /// Process pending sensor events from the channel.
    fn process_sensor_events(&mut self) {
        // Collect events first to avoid borrow conflict
        let events: Vec<SensorEvent> = if let Some(rx) = &self.sensor_event_rx {
            let mut collected = Vec::new();
            while let Ok(event) = rx.try_recv() {
                collected.push(event);
            }
            collected
        } else {
            return;
        };

        // Now process collected events
        for event in events {
            match event {
                SensorEvent::Discovered(sensor) => {
                    tracing::debug!("Discovered sensor: {} ({})", sensor.name, sensor.device_id);
                    // Update sensor setup screen with discovered sensors
                    self.sensor_setup_screen.add_discovered_sensor(sensor);
                }
                SensorEvent::ConnectionChanged { device_id, state } => {
                    tracing::info!("Sensor {} connection state: {:?}", device_id, state);
                    match state {
                        ConnectionState::Connected => {
                            self.connected_sensor_count += 1;
                            self.sensor_status = format!(
                                "{} sensor{} connected",
                                self.connected_sensor_count,
                                if self.connected_sensor_count == 1 {
                                    ""
                                } else {
                                    "s"
                                }
                            );
                        }
                        ConnectionState::Disconnected => {
                            if self.connected_sensor_count > 0 {
                                self.connected_sensor_count -= 1;
                            }
                            if self.connected_sensor_count == 0 {
                                self.sensor_status = "No sensors connected".to_string();
                            } else {
                                self.sensor_status = format!(
                                    "{} sensor{} connected",
                                    self.connected_sensor_count,
                                    if self.connected_sensor_count == 1 {
                                        ""
                                    } else {
                                        "s"
                                    }
                                );
                            }
                        }
                        ConnectionState::Connecting => {
                            self.sensor_status = "Connecting...".to_string();
                        }
                        ConnectionState::Reconnecting => {
                            self.sensor_status = "Reconnecting...".to_string();
                        }
                    }
                    self.sensor_setup_screen
                        .update_connection_state(&device_id, state);
                }
                SensorEvent::Data(reading) => {
                    // Only process data if we're on the ride screen and recording
                    if self.current_screen == Screen::Ride && !self.ride_screen.is_paused {
                        // T135: Update cadence fusion with data from this sensor
                        let cadence_f32 = reading.cadence_rpm.map(|c| c as f32);
                        self.update_cadence_fusion(&reading.sensor_id, cadence_f32);

                        // Get fused cadence if available
                        let fused_cadence = self.get_fused_cadence();

                        // Create a modified reading with fused cadence if available
                        let reading_to_process = if fused_cadence.is_some() {
                            let mut modified = reading.clone();
                            modified.cadence_rpm = fused_cadence;
                            modified
                        } else {
                            reading.clone()
                        };

                        // Process the reading through the metrics calculator
                        self.metrics_calculator.process(&reading_to_process);

                        // Update ride screen metrics
                        self.ride_screen.metrics = self.metrics_calculator.get_aggregated();

                        // T071: Update fan controller with current metrics
                        let aggregated = self.metrics_calculator.get_aggregated();
                        let power = aggregated.power_instant.unwrap_or(0);
                        let hr = aggregated.heart_rate;
                        let power_zone = aggregated.power_zone.unwrap_or(1);
                        let hr_zone = aggregated.hr_zone;
                        self.update_fan_controller(power, hr, power_zone, hr_zone);

                        // T080: Broadcast metrics to external displays
                        self.broadcast_streaming_metrics(&aggregated);
                    }
                }
                SensorEvent::ScanStarted => {
                    tracing::debug!("Sensor scan started");
                    self.sensor_setup_screen.set_scanning(true);
                }
                SensorEvent::ScanStopped => {
                    tracing::debug!("Sensor scan stopped");
                    self.sensor_setup_screen.set_scanning(false);
                }
                SensorEvent::Error(err) => {
                    tracing::error!("Sensor error: {}", err);
                    self.sensor_status = format!("Error: {}", err);
                }
            }
        }
    }

    /// Update elapsed time on ride screen.
    fn update_ride_time(&mut self) {
        if self.current_screen == Screen::Ride
            && !self.ride_screen.is_paused
            && self.ride_screen.recording_status
                == rustride::recording::types::RecordingStatus::Recording
        {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_update);
            if elapsed.as_secs() >= 1 {
                self.ride_screen.elapsed_seconds += 1;
                self.last_update = now;
            }
        }
    }

    /// Update fan controller with current metrics (T071).
    ///
    /// Sends power and HR zone data to the fan controller for zone-based speed adjustment.
    fn update_fan_controller(
        &self,
        power: u16,
        hr: Option<u8>,
        power_zone: u8,
        hr_zone: Option<u8>,
    ) {
        if self.mqtt_config.enabled {
            self.fan_controller
                .update_metrics(power, hr, power_zone, hr_zone);
        }
    }

    /// Start MQTT connection and fan controller when a ride begins (T071/4.1).
    ///
    /// If MQTT is enabled in the configuration, this method:
    /// 1. Connects to the MQTT broker asynchronously
    /// 2. Starts the fan controller for zone-based fan speed control
    ///
    /// This should be called when a ride starts (free ride or workout).
    fn start_mqtt_fan_control(&self) {
        if !self.mqtt_config.enabled {
            tracing::debug!("MQTT not enabled, skipping fan control initialization");
            return;
        }

        let mqtt_client = self.mqtt_client.clone();
        let fan_controller = self.fan_controller.clone();
        let mqtt_config = self.mqtt_config.clone();

        self.tokio_runtime.spawn(async move {
            // Connect to MQTT broker
            tracing::info!("Starting MQTT connection for fan control");
            match mqtt_client.connect(&mqtt_config).await {
                Ok(()) => {
                    tracing::info!("MQTT connection initiated successfully");

                    // Start the fan controller
                    if let Err(e) = fan_controller.start().await {
                        tracing::error!("Failed to start fan controller: {}", e);
                    } else {
                        tracing::info!("Fan controller started");
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to connect to MQTT broker: {}", e);
                }
            }
        });
    }

    /// Stop MQTT connection and fan controller when a ride ends (T071/4.2).
    ///
    /// If MQTT is enabled in the configuration, this method:
    /// 1. Stops the fan controller (turns off all fans)
    /// 2. Disconnects from the MQTT broker
    ///
    /// This should be called when a ride ends (completion, cancel, or navigation away).
    fn stop_mqtt_fan_control(&self) {
        if !self.mqtt_config.enabled {
            tracing::debug!("MQTT not enabled, skipping fan control shutdown");
            return;
        }

        let mqtt_client = self.mqtt_client.clone();
        let fan_controller = self.fan_controller.clone();

        self.tokio_runtime.spawn(async move {
            // Stop the fan controller (turns off all fans)
            tracing::info!("Stopping fan controller");
            if let Err(e) = fan_controller.stop().await {
                tracing::error!("Failed to stop fan controller: {}", e);
            } else {
                tracing::info!("Fan controller stopped, fans turned off");
            }

            // Disconnect from MQTT broker
            tracing::info!("Disconnecting from MQTT broker");
            if let Err(e) = mqtt_client.disconnect().await {
                tracing::error!("Failed to disconnect from MQTT broker: {}", e);
            } else {
                tracing::info!("MQTT broker disconnected");
            }
        });
    }

    /// Update MQTT connection status on ride screen (T071/4.4).
    ///
    /// Gets the current connection state from the MQTT client and updates
    /// the ride screen so it can display the fan control status.
    fn update_mqtt_status_on_ride_screen(&mut self) {
        let connection_state = self.mqtt_client.connection_state();
        self.ride_screen
            .update_mqtt_state(connection_state, self.mqtt_config.enabled);
    }

    /// Update streaming server with current metrics (T080).
    ///
    /// Broadcasts metrics to all connected external displays.
    fn broadcast_streaming_metrics(
        &self,
        aggregated: &rustride::metrics::calculator::AggregatedMetrics,
    ) {
        if self.streaming_config.enabled && self.streaming_server.is_running() {
            let metrics = StreamingMetrics {
                timestamp_ms: aggregated.elapsed_time.as_millis() as u64,
                power: aggregated.power_instant,
                heart_rate: aggregated.heart_rate,
                cadence: aggregated.cadence,
                speed: aggregated.speed,
                distance: Some(aggregated.distance as f32),
                elapsed_time: aggregated.elapsed_time,
                current_interval: None, // TODO: Get from workout engine
                zone_name: aggregated.power_zone.map(|z| format!("Zone {}", z)),
                gradient: None, // TODO: Get from gradient controller
                left_right_balance: None,
                calories: Some(aggregated.calories),
                normalized_power: aggregated.normalized_power,
                intensity_factor: aggregated.intensity_factor,
            };
            self.streaming_server.broadcast_metrics(&metrics);
        }
    }

    /// Update cadence fusion with a new reading from a sensor (T135).
    ///
    /// Automatically assigns sensors to primary/secondary roles based on
    /// connection order. First cadence-capable sensor becomes primary.
    fn update_cadence_fusion(&mut self, sensor_id: &uuid::Uuid, cadence: Option<f32>) {
        // Skip if no cadence data
        if cadence.is_none() {
            return;
        }

        // Assign sensor roles if not yet assigned
        if self.primary_cadence_sensor.is_none() {
            self.primary_cadence_sensor = Some(*sensor_id);
            tracing::info!("Assigned primary cadence sensor: {}", sensor_id);
        } else if self.secondary_cadence_sensor.is_none()
            && self.primary_cadence_sensor != Some(*sensor_id)
        {
            self.secondary_cadence_sensor = Some(*sensor_id);
            tracing::info!("Assigned secondary cadence sensor: {}", sensor_id);
        }

        // Feed data to fusion based on sensor role
        if self.primary_cadence_sensor == Some(*sensor_id) {
            self.cadence_fusion.update(cadence, None);
        } else if self.secondary_cadence_sensor == Some(*sensor_id) {
            // Get current primary value to pass along
            let diag = self.cadence_fusion.get_diagnostics();
            self.cadence_fusion.update(diag.primary_value, cadence);
        }
    }

    /// Get the fused cadence value if available (T135).
    ///
    /// Returns the fused cadence only when both sensors are active.
    /// Falls back to single sensor if fusion is not in dual-sensor mode.
    fn get_fused_cadence(&self) -> Option<u8> {
        let diag = self.cadence_fusion.get_diagnostics();

        // Only use fused value when we have meaningful fusion data
        match diag.mode {
            FusionMode::DualSensor | FusionMode::Inconsistent => {
                diag.fused_value.map(|v| v.round() as u8)
            }
            FusionMode::PrimaryOnly | FusionMode::SecondaryOnly => {
                // In fallback mode, still use the fused/smoothed value
                diag.fused_value.map(|v| v.round() as u8)
            }
            FusionMode::NoData => None,
        }
    }

    /// Get cadence fusion diagnostics for display (T135).
    #[allow(dead_code)]
    pub fn get_cadence_fusion_diagnostics(&self) -> rustride::sensors::FusionDiagnostics {
        self.cadence_fusion.get_diagnostics()
    }

    /// Reset cadence fusion state (T135).
    ///
    /// Called when ending a ride to clear sensor assignments.
    fn reset_cadence_fusion(&mut self) {
        self.cadence_fusion.reset();
        self.primary_cadence_sensor = None;
        self.secondary_cadence_sensor = None;
        tracing::debug!("Cadence fusion state reset");
    }

    /// T091: Poll for learned button from HID input handler.
    ///
    /// Called each frame to check if a button was learned and update the settings screen.
    fn poll_learned_button(&mut self) {
        // Only poll when in learning mode and on the Settings screen
        if self.current_screen != Screen::Settings {
            return;
        }

        // Check if the button input handler has a learned button
        if self.button_input_handler.is_learning() {
            if let Some(button_code) = self.button_input_handler.get_learned_button() {
                tracing::info!("Learned button code: {}", button_code);
                // Pass the learned button code to the settings screen
                self.settings_screen.set_learned_button(button_code);
                // Stop learning mode in the handler (button was captured)
                self.button_input_handler.stop_learning_mode();
            }
        }
    }

    /// T071/4.3: Poll for MQTT connection test result.
    ///
    /// Called each frame to check if a pending MQTT test has completed.
    fn poll_mqtt_test(&mut self) {
        // Only poll when on the Settings screen
        if self.current_screen != Screen::Settings {
            return;
        }

        // Check if we have a pending test
        if let Some(handle) = &mut self.pending_mqtt_test {
            // Check if the task has completed (non-blocking)
            if handle.is_finished() {
                // Take ownership of the handle
                if let Some(handle) = self.pending_mqtt_test.take() {
                    // Block on the result (it's already finished, so this is instant)
                    match self.tokio_runtime.block_on(handle) {
                        Ok(result) => {
                            tracing::info!(
                                "MQTT test completed: success={}, message={}",
                                result.success,
                                result.message
                            );
                            self.settings_screen
                                .set_mqtt_test_result(result.success, result.message);
                        }
                        Err(e) => {
                            tracing::error!("MQTT test task panicked: {}", e);
                            self.settings_screen.set_mqtt_test_result(
                                false,
                                format!("Internal error: {}", e),
                            );
                        }
                    }
                }
            }
        }
    }

    /// T091: Process pending executor events for UI navigation.
    ///
    /// Called each frame to handle navigation and fullscreen events from HID button actions.
    fn process_executor_events(&mut self) {
        let Some(rx) = &mut self.executor_event_rx else {
            return;
        };

        // Process all pending events
        while let Ok(event) = rx.try_recv() {
            match event {
                ExecutorEvent::NavigationRequest(target) => {
                    // Only handle navigation when on the Ride screen
                    if self.current_screen == Screen::Ride {
                        match target {
                            NavigationTarget::Metrics => {
                                tracing::info!("HID action: switching to metrics view");
                                self.ride_screen.set_view(RideView::Metrics);
                            }
                            NavigationTarget::Map => {
                                tracing::info!("HID action: switching to map view");
                                self.ride_screen.set_view(RideView::Map);
                            }
                            NavigationTarget::Workout => {
                                tracing::info!("HID action: switching to workout view");
                                self.ride_screen.show_workout_view();
                            }
                        }
                    }
                }
                ExecutorEvent::FullscreenToggle => {
                    // Handle fullscreen toggle when on the Ride screen
                    if self.current_screen == Screen::Ride {
                        tracing::info!("HID action: toggling fullscreen mode");
                        self.ride_screen.toggle_fullscreen();
                    }
                }
                ExecutorEvent::ActionExecuted(result) => {
                    // Log action results for debugging
                    tracing::debug!(
                        "Action executed: {:?}, success: {}",
                        result.action,
                        result.success
                    );
                }
                ExecutorEvent::LapMarked(lap) => {
                    tracing::info!("Lap {} marked at {}s", lap.lap_number, lap.elapsed_seconds);
                }
                ExecutorEvent::CustomAction { command } => {
                    tracing::info!("Custom action: {}", command);
                }
            }
        }
    }

    /// T043: Check achievements after ride completion.
    ///
    /// This analyzes the completed ride metrics and cumulative stats
    /// to determine which achievements have been earned.
    fn check_ride_achievements(&mut self) {
        // Build ride metrics from the completed ride
        let ride_id = Uuid::new_v4();
        let metrics = self.ride_screen.metrics.clone();

        let ride_metrics = RideMetrics {
            ride_id,
            distance_km: metrics.distance / 1000.0, // Convert m to km
            duration_secs: self.ride_screen.elapsed_seconds,
            elevation_gain_m: 0.0, // Would come from gradient controller if available
            avg_power: self.metrics_calculator.average_power(),
            normalized_power: metrics.normalized_power,
            max_power: self.metrics_calculator.max_power(),
            avg_hr: metrics.heart_rate,
            max_hr: metrics.heart_rate, // Use current HR as max (simplified)
            avg_cadence: metrics.cadence,
            calories: Some(metrics.calories),
            workout_completed: self.ride_screen.workout.is_some()
                && self.ride_screen.workout_status
                    == rustride::workouts::types::WorkoutStatus::Completed,
            workout_id: self.ride_screen.workout.as_ref().map(|_| Uuid::new_v4()),
            tss: metrics.tss,
            intensity_factor: metrics.intensity_factor,
            ..Default::default()
        };

        // Update cumulative stats
        self.cumulative_stats.total_distance_km += ride_metrics.distance_km;
        self.cumulative_stats.total_time_secs += ride_metrics.duration_secs as u64;
        self.cumulative_stats.total_rides += 1;
        if ride_metrics.workout_completed {
            self.cumulative_stats.total_workouts += 1;
        }

        // Check for earned achievements
        let earned = self
            .achievement_checker
            .check_all(&ride_metrics, &self.cumulative_stats);

        // Award achievements and queue notifications
        for achievement in earned {
            if let Some(earned_achievement) =
                self.achievement_tracker.award(&achievement, Some(ride_id))
            {
                tracing::info!(
                    "Achievement unlocked: {} (+{} XP)",
                    achievement.name,
                    earned_achievement.xp_awarded
                );

                // Queue notification
                let notification = rustride::achievements::AchievementNotification::new(
                    self.user_id,
                    achievement.title.clone(),
                    &achievement.description,
                    achievement.category,
                    achievement.tier,
                    earned_achievement.xp_awarded,
                );
                self.achievement_notification_queue.push(notification);
            }
        }
    }

    /// Update incline controller with route gradient (T043).
    ///
    /// This is called during World3D rides to send gradient commands to the trainer.
    fn update_incline_from_gradient(&mut self, gradient_percent: f32, delta_time: f32) {
        if !self.incline_controller.is_enabled() {
            return;
        }

        // Update the incline controller with the route gradient
        self.incline_controller.set_gradient(gradient_percent);

        // Update gradient smoothing
        self.incline_controller.update_smoothing();

        // Use the gradient controller for rate-limiting and FTMS command generation
        if let Some(smoothed_gradient) = self.gradient_controller.update(
            self.incline_controller.get_state().smoothed_gradient,
            delta_time,
        ) {
            // Build and send FTMS command (in a real implementation)
            let _ftms_command = self.gradient_controller.build_ftms_command();
            tracing::debug!(
                "Sending gradient to trainer: {:.1}% (raw: {:.1}%)",
                smoothed_gradient,
                gradient_percent
            );
        }
    }

    /// Navigate to a different screen.
    fn navigate(&mut self, screen: Screen) {
        tracing::debug!("Navigating from {:?} to {:?}", self.current_screen, screen);

        // Populate available voices when entering Settings screen
        if matches!(screen, Screen::Settings) {
            let voices = self.audio_engine.tts_provider().get_voices();
            self.settings_screen.set_available_voices(voices);
        }

        self.current_screen = screen;
    }

    /// Toggle the theme between dark and light.
    fn toggle_theme(&mut self, ctx: &egui::Context) {
        self.theme = match self.theme {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        };
        ctx.set_visuals(self.theme.visuals());
    }

    /// Render the crash recovery dialog.
    fn render_recovery_dialog(&mut self, ctx: &egui::Context) {
        if let RecoveryState::Pending {
            timestamp,
            duration,
            sample_count,
        } = &self.recovery_state
        {
            let timestamp = timestamp.clone();
            let duration = duration.clone();
            let sample_count = *sample_count;

            egui::Window::new("Recover Previous Ride?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_min_width(400.0);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("⚠")
                                    .size(24.0)
                                    .color(egui::Color32::from_rgb(251, 188, 4)),
                            );
                            ui.label(
                                egui::RichText::new("Unsaved Ride Detected")
                                    .size(18.0)
                                    .strong(),
                            );
                        });

                        ui.add_space(12.0);

                        ui.label(
                            "It looks like RustRide closed unexpectedly during your last ride.",
                        );
                        ui.label("Would you like to recover your ride data?");

                        ui.add_space(12.0);

                        // Recovery data details
                        ui.group(|ui| {
                            ui.set_min_width(ui.available_width() - 8.0);

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Started:").strong());
                                ui.label(&timestamp);
                            });

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Duration:").strong());
                                ui.label(&duration);
                            });

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Data points:").strong());
                                ui.label(format!("{}", sample_count));
                            });
                        });

                        ui.add_space(16.0);

                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Button::new("Discard")
                                        .fill(egui::Color32::from_rgb(160, 160, 170)),
                                )
                                .clicked()
                            {
                                tracing::info!("User discarded crash recovery data");
                                // TODO: Actually discard the recovery data
                                self.recovery_state = RecoveryState::None;
                            }

                            ui.add_space(16.0);

                            if ui
                                .add(
                                    egui::Button::new("Recover Ride")
                                        .fill(egui::Color32::from_rgb(52, 168, 83)),
                                )
                                .clicked()
                            {
                                tracing::info!("User chose to recover crash data");
                                // TODO: Actually recover the data and show ride summary
                                self.recovery_state = RecoveryState::None;
                                // Navigate to ride summary with recovered data
                                self.current_screen = Screen::RideSummary;
                            }
                        });
                    });
                });
        }
    }
}

impl eframe::App for RustRideApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process sensor events each frame
        self.process_sensor_events();

        // T091: Process HID executor events for UI navigation
        self.process_executor_events();

        // T091: Poll for learned buttons from HID input handler
        self.poll_learned_button();

        // T071/4.3: Poll for MQTT connection test result
        self.poll_mqtt_test();

        // Update ride time if recording
        self.update_ride_time();

        // Request repaint to keep UI responsive (for sensor updates, HID learning mode, MQTT test)
        if self.current_screen == Screen::Ride
            || self.current_screen == Screen::SensorSetup
            || (self.current_screen == Screen::Settings
                && (self.button_input_handler.is_learning() || self.pending_mqtt_test.is_some()))
        {
            ctx.request_repaint();
        }

        // T029: Clear focus manager widgets at start of each frame
        self.focus_manager.clear_widgets();

        // T029: Handle focus navigation (Tab/Shift+Tab)
        self.focus_manager.handle_keyboard_input(ctx);

        // Handle keyboard shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.current_screen != Screen::Home {
            self.navigate(Screen::Home);
        }

        // Top panel with navigation
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("RustRide");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Theme toggle
                    let theme_icon = match self.theme {
                        Theme::Dark => "🌙",
                        Theme::Light => "☀",
                    };
                    if ui.button(theme_icon).clicked() {
                        self.toggle_theme(ctx);
                    }

                    // Profile
                    ui.label(&self.profile.name);
                    ui.label(format!("FTP: {}W", self.profile.ftp));
                });
            });
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_screen {
                Screen::Onboarding => {
                    // T059: Show onboarding wizard on first launch
                    if self.onboarding_screen.show(ui) {
                        // Onboarding complete - save state and navigate to home
                        save_onboarding_complete();

                        // Apply profile data from onboarding
                        let profile_data = self.onboarding_screen.get_profile_data();
                        if !profile_data.name.is_empty() {
                            self.profile.name = profile_data.name;
                        }
                        self.profile.weight_kg = profile_data.weight_kg as f32;
                        self.profile.ftp = profile_data.ftp;
                        if let Some(max_hr) = profile_data.max_hr {
                            self.profile.max_hr = Some(max_hr);
                        }

                        // Update metrics calculator with new FTP
                        self.metrics_calculator = MetricsCalculator::new(self.profile.ftp);

                        // Update settings screen with new profile
                        self.settings_screen = SettingsScreen::new(self.profile.clone());

                        self.navigate(Screen::Home);
                    }
                }
                Screen::Home => {
                    if let Some(next) = HomeScreen::show(ui) {
                        self.navigate(next);
                    }
                }
                Screen::SensorSetup => {
                    // Store previous scanning state to detect changes
                    let was_scanning = self.sensor_setup_screen.is_scanning;

                    // Show the sensor setup screen and handle navigation
                    let should_navigate = self.sensor_setup_screen.show(ui);

                    // Check if scanning state changed via UI
                    let is_scanning_now = self.sensor_setup_screen.is_scanning;
                    if !was_scanning && is_scanning_now {
                        // User clicked "Start Scanning"
                        self.start_sensor_discovery();
                    } else if was_scanning && !is_scanning_now {
                        // User clicked "Stop Scanning"
                        self.stop_sensor_discovery();
                    }

                    // Handle navigation after processing scanning state
                    if let Some(next) = should_navigate {
                        self.navigate(next);
                    }
                }
                Screen::WorkoutLibrary => {
                    ui.heading("Workout Library");
                    ui.label("Workout library - coming soon");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::Ride => {
                    // Start free ride if coming from home
                    if self.ride_screen.recording_status
                        == rustride::recording::types::RecordingStatus::Idle
                    {
                        self.ride_screen.start_free_ride();
                        // T071/4.1: Start MQTT connection and fan controller when ride begins
                        self.start_mqtt_fan_control();
                    }

                    // T071/4.4: Update MQTT connection status for display
                    self.update_mqtt_status_on_ride_screen();

                    // T043: Update incline controller with current gradient in World3D mode
                    if self.ride_screen.mode == rustride::ui::screens::ride::RideMode::World3D
                        && !self.ride_screen.is_paused
                    {
                        let delta_time = self.ride_screen.get_delta_time();
                        let gradient = self.ride_screen.get_gradient();
                        self.update_incline_from_gradient(gradient, delta_time);
                    }

                    if let Some(next) = self.ride_screen.show(ui) {
                        // T043: Check achievements before resetting ride screen
                        if next == Screen::RideSummary {
                            self.check_ride_achievements();
                        }
                        // T071/4.2: Stop MQTT fan control when ride ends
                        self.stop_mqtt_fan_control();
                        // Reset gradient controller when leaving ride
                        self.gradient_controller.reset();
                        // T135: Reset cadence fusion when ending ride
                        self.reset_cadence_fusion();
                        // Reset ride screen when leaving
                        self.ride_screen = RideScreen::new();
                        self.navigate(next);
                    }
                }
                Screen::RideSummary => {
                    ui.heading("Ride Summary");
                    ui.label("Ride summary - coming soon");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::RideHistory => {
                    ui.heading("Ride History");
                    ui.label("Ride history - coming soon");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::RideDetail => {
                    ui.heading("Ride Detail");
                    ui.label("Ride detail - coming soon");
                    if ui.button("Back").clicked() {
                        self.navigate(Screen::RideHistory);
                    }
                }
                Screen::Settings => {
                    use rustride::ui::screens::SettingsAction;

                    match self.settings_screen.show(ui) {
                        SettingsAction::Save => {
                            // Apply incline config changes to the controller
                            let incline_config = self.settings_screen.get_incline_config().clone();
                            self.incline_controller.set_config(incline_config.clone());
                            self.incline_controller.set_enabled(incline_config.enabled);

                            // Update gradient controller limits
                            self.gradient_controller = GradientController::with_settings(
                                incline_config.max_gradient,
                                incline_config.min_gradient,
                                0.3, // smoothing
                                0.5, // update interval
                            );

                            // T091: Save HID config to database
                            let hid_config = self.settings_screen.get_hid_config();
                            if let Some(ref db) = self.database {
                                let store = HardwareStore::new(db.connection());
                                if let Err(e) = save_hid_config_to_db(&store, &self.user_id, &hid_config) {
                                    tracing::error!("Failed to save HID config to database: {}", e);
                                } else {
                                    tracing::info!("HID config saved to database");
                                }

                                // Update button mappings in the handler
                                for device_config in &hid_config.devices {
                                    // Clear existing mappings for this device
                                    self.button_input_handler.clear_mappings(&device_config.device_id);

                                    // Register new mappings if device is enabled
                                    if device_config.enabled && !device_config.mappings.is_empty() {
                                        let mappings: Vec<ButtonMapping> = device_config
                                            .mappings
                                            .iter()
                                            .map(|m| ButtonMapping::new(device_config.device_id, m.button_code, m.action.clone()))
                                            .collect();
                                        self.button_input_handler.register_mappings(&device_config.device_id, mappings);
                                    }
                                }
                            }

                            tracing::info!(
                                "Settings saved. Incline mode: {}",
                                incline_config.enabled
                            );
                            self.navigate(Screen::Home);
                        }
                        SettingsAction::Cancel => {
                            // Reset settings screen to original values
                            self.settings_screen.reset();
                            self.navigate(Screen::Home);
                        }
                        SettingsAction::TestVoice(settings) => {
                            // Preview the selected voice with current settings
                            let tts = self.audio_engine.tts_provider();

                            // Apply the test settings
                            if let Some(ref voice_id) = settings.voice_id {
                                if let Err(e) = tts.set_voice(voice_id) {
                                    tracing::warn!("Failed to set voice for preview: {}", e);
                                }
                            }
                            tts.set_volume(settings.volume);
                            tts.set_rate(settings.rate);

                            // Speak the preview phrase
                            const PREVIEW_PHRASE: &str = "This is how your voice alerts will sound.";
                            if let Err(e) = tts.speak(PREVIEW_PHRASE) {
                                tracing::warn!("Failed to preview voice: {}", e);
                            }
                        }
                        SettingsAction::ScanHidDevices => {
                            // T091: Scan for HID devices and update the settings screen
                            tracing::info!("Scanning for HID devices...");
                            let devices = self.hid_device_manager.scan_devices();
                            tracing::info!("Found {} HID device(s)", devices.len());
                            self.settings_screen.set_hid_devices(devices);
                        }
                        SettingsAction::StartLearningMode(device_id) => {
                            // T091: Start button learning mode for the specified device
                            tracing::info!("Starting button learning mode for device {:?}", device_id);
                            self.button_input_handler.start_learning_mode(&device_id);
                        }
                        SettingsAction::StopLearningMode => {
                            // T091: Stop button learning mode
                            tracing::info!("Stopping button learning mode");
                            self.button_input_handler.stop_learning_mode();
                        }
                        SettingsAction::TestMqttConnection(config) => {
                            // T071/4.3: Test MQTT broker connection
                            tracing::info!(
                                "Testing MQTT connection to {}:{}",
                                config.broker_host,
                                config.broker_port
                            );

                            // Spawn async task to test the connection
                            let handle = self.tokio_runtime.spawn(async move {
                                rustride::integrations::mqtt::test_mqtt_connection(&config).await
                            });
                            self.pending_mqtt_test = Some(handle);
                        }
                        SettingsAction::None => {}
                    }
                }
                Screen::WorldSelect => {
                    if let Some((next, _selection)) = self.world_select_screen.show(ui) {
                        // TODO: Pass selection to ride screen when starting 3D ride
                        self.navigate(next);
                    }
                }
                Screen::Avatar => {
                    if let Some((next, _config)) = self.avatar_screen.show(ui) {
                        // TODO: Save avatar config to database
                        self.navigate(next);
                    }
                }
                Screen::Analytics => {
                    self.analytics_screen.show(ui);
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::RouteImport => {
                    ui.heading("Import Route");
                    ui.label("Route import - coming soon");
                    ui.label("Supported formats: GPX, FIT, TCX");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::RouteBrowser => {
                    ui.heading("Route Library");
                    ui.label("Route browser - coming soon");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::GroupRide => {
                    ui.heading("Group Ride");
                    ui.label("LAN Group Rides - coming soon");
                    ui.label("Discover and join other riders on your local network.");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::Leaderboard => {
                    ui.heading("Leaderboards");
                    ui.label("Segment leaderboards - coming soon");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::Challenges => {
                    ui.heading("Challenges");
                    ui.label("Training challenges - coming soon");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::ActivityFeed => {
                    ui.heading("Activity Feed");
                    ui.label("Activity feed from LAN peers - coming soon");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::Clubs => {
                    ui.heading("Clubs");
                    ui.label("Club management - coming soon");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::RaceLobby => {
                    ui.heading("Virtual Racing");
                    ui.label("Virtual race events - coming soon");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::RiderProfile => {
                    ui.heading("My Profile");
                    ui.label("Rider profile - coming soon");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::Streaming => {
                    ui.heading("External Display");
                    ui.label("Streaming screen - requires full integration");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::Achievements => {
                    ui.heading("Achievements");
                    ui.label("Achievement gallery - coming soon");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::PowerProfile => {
                    ui.heading("Power Profile");
                    ui.label("4D Power profiling - coming soon");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::Career => {
                    ui.heading("Career Progress");
                    ui.label("Career progression with level unlocks - coming soon");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::Rewards => {
                    ui.heading("Rewards Gallery");
                    ui.label("Cosmetic rewards gallery - coming soon");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
                Screen::TrainingPlans => {
                    ui.heading("Training Plans");
                    ui.label("Training plans browser - coming soon");
                    if ui.button("Back to Home").clicked() {
                        self.navigate(Screen::Home);
                    }
                }
            }
        });

        // Status bar at bottom
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
                ui.separator();
                ui.label(&self.sensor_status);
            });
        });

        // T043: Achievement notification overlay (shown on top of other UI)
        egui::Area::new(egui::Id::new("achievement_notification_area"))
            .anchor(egui::Align2::RIGHT_TOP, [-16.0, 60.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                self.achievement_notification_widget
                    .show(ui, &mut self.achievement_notification_queue);
            });

        // Request repaint if notifications are animating
        if self.achievement_notification_queue.current().is_some() {
            ctx.request_repaint();
        }

        // Crash recovery dialog (shown on top of everything)
        self.render_recovery_dialog(ctx);
    }
}

/// T059: Load onboarding state from storage.
///
/// In a real implementation, this would read from the database.
/// For now, returns default state which triggers onboarding on first launch.
fn load_onboarding_state() -> OnboardingState {
    // Check if a marker file exists to determine if onboarding was completed
    let data_dir = rustride::storage::config::get_data_dir();
    let onboarding_marker = data_dir.join("onboarding_complete");

    if onboarding_marker.exists() {
        OnboardingState {
            completed: true,
            current_step: rustride::onboarding::OnboardingStep::Complete,
            skipped: false,
            completed_steps: rustride::onboarding::OnboardingStep::all().to_vec(),
        }
    } else {
        OnboardingState::default()
    }
}

/// T059: Save onboarding state to storage.
///
/// Marks onboarding as complete by creating a marker file.
fn save_onboarding_complete() {
    let data_dir = rustride::storage::config::get_data_dir();
    if std::fs::create_dir_all(&data_dir).is_ok() {
        let marker = data_dir.join("onboarding_complete");
        let _ = std::fs::write(marker, "1");
    }
}
