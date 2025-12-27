//! Main application state and egui integration.
//!
//! T042: Create App struct with egui state
//! T044: Implement screen navigation state machine
//! T050: Wire sensor data to UI via crossbeam channel
//! T157: Implement crash recovery prompt on startup

use eframe::egui;

use crossbeam::channel::Receiver;
use rustride::audio::{AudioConfig, AudioEngine, DefaultAudioEngine};
use rustride::hid::{
    ButtonAction, ButtonInputHandler, DefaultButtonInputHandler, DefaultHidDeviceManager,
    HidConfig, HidDeviceManager,
};
use rustride::integrations::mqtt::{
    DefaultFanController, DefaultMqttClient, FanController, FanProfile, MqttConfig,
};
use rustride::integrations::streaming::{
    DefaultPinAuthenticator, DefaultStreamingServer, PinAuthenticator, StreamingConfig,
    StreamingMetrics, StreamingServer,
};
use rustride::metrics::MetricsCalculator;
use rustride::recording::RideRecorder;
use rustride::sensors::types::{ConnectionState, SensorEvent};
use rustride::sensors::{
    CadenceFusion, DefaultInclineController, FusionMode, InclineConfig, InclineController,
    SensorFusion, SensorFusionConfig, SensorManager,
};
use rustride::storage::config::{AppConfig, UserProfile};
use rustride::ui::screens::{
    AnalyticsScreen, AvatarScreen, HomeScreen, RideScreen, Screen, SensorSetupScreen,
    SettingsScreen, WorldSelectScreen,
};
use rustride::ui::theme::Theme;
use rustride::workouts::WorkoutEngine;
use rustride::world::physics::GradientController;
use std::sync::Arc;
use std::time::Instant;

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
    /// Sensor manager
    _sensor_manager: SensorManager,
    /// Workout engine
    _workout_engine: WorkoutEngine,
    /// Ride recorder
    _ride_recorder: RideRecorder,
    /// Metrics calculator
    metrics_calculator: MetricsCalculator,
    /// Audio engine for voice alerts and sound effects (Hardware Integration)
    _audio_engine: Arc<DefaultAudioEngine>,
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
    /// T071: MQTT client for smart home integration
    mqtt_client: Arc<DefaultMqttClient>,
    /// T071: Fan controller for zone-based fan speed control
    fan_controller: Arc<DefaultFanController<DefaultMqttClient>>,
    /// T071: MQTT configuration
    mqtt_config: MqttConfig,
    /// T080: Streaming server for external displays
    streaming_server: Arc<DefaultStreamingServer>,
    /// T080: Streaming configuration
    streaming_config: StreamingConfig,
    /// T091: HID device manager for USB buttons/Stream Deck
    hid_device_manager: Arc<DefaultHidDeviceManager>,
    /// T091: Button input handler for mapping
    button_input_handler: Arc<DefaultButtonInputHandler>,
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
}

impl RustRideApp {
    /// Create a new application instance.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load configuration
        let config = rustride::storage::config::load_config().unwrap_or_default();

        // Create default profile
        let profile = UserProfile::default();

        // Set up theme
        let theme = Theme::Dark;
        cc.egui_ctx.set_visuals(theme.visuals());

        // Note: Using default egui fonts for now
        // Custom fonts can be configured later if needed

        // Create managers
        let mut sensor_manager = SensorManager::with_defaults();
        let sensor_event_rx = Some(sensor_manager.event_receiver());
        let workout_engine = WorkoutEngine::new();
        let ride_recorder = RideRecorder::with_defaults();
        let metrics_calculator = MetricsCalculator::new(profile.ftp);

        // Initialize audio engine (Hardware Integration)
        let audio_config = AudioConfig::default();
        let audio_engine = Arc::new(DefaultAudioEngine::new(audio_config));
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

        // T091: Initialize HID device manager and button input handler
        let hid_config = HidConfig::default();
        let hid_device_manager = Arc::new(DefaultHidDeviceManager::new(hid_config));
        let button_input_handler = Arc::new(DefaultButtonInputHandler::new());

        // T135: Initialize cadence sensor fusion
        let fusion_config = SensorFusionConfig::default();
        let cadence_fusion = CadenceFusion::with_config(fusion_config);

        // Initialize settings screen with profile
        let mut settings_screen = SettingsScreen::new(profile.clone());
        settings_screen.set_incline_config(incline_config);

        Self {
            current_screen: Screen::Home,
            theme,
            profile,
            _config: config,
            _sensor_manager: sensor_manager,
            _workout_engine: workout_engine,
            _ride_recorder: ride_recorder,
            metrics_calculator,
            _audio_engine: audio_engine,
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
            sensor_event_rx,
            last_update: Instant::now(),
            sensor_status: "No sensors connected".to_string(),
            connected_sensor_count: 0,
            recovery_state,
            cadence_fusion,
            primary_cadence_sensor: None,
            secondary_cadence_sensor: None,
        }
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
                calories: Some(aggregated.calories as u32),
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

        // Update ride time if recording
        self.update_ride_time();

        // Request repaint to keep UI responsive (for sensor updates)
        if self.current_screen == Screen::Ride || self.current_screen == Screen::SensorSetup {
            ctx.request_repaint();
        }

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
                Screen::Home => {
                    if let Some(next) = HomeScreen::show(ui) {
                        self.navigate(next);
                    }
                }
                Screen::SensorSetup => {
                    if let Some(next) = self.sensor_setup_screen.show(ui) {
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
                    }

                    // T043: Update incline controller with current gradient in World3D mode
                    if self.ride_screen.mode == rustride::ui::screens::ride::RideMode::World3D
                        && !self.ride_screen.is_paused
                    {
                        let delta_time = self.ride_screen.get_delta_time();
                        let gradient = self.ride_screen.get_gradient();
                        self.update_incline_from_gradient(gradient, delta_time);
                    }

                    if let Some(next) = self.ride_screen.show(ui) {
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

        // Crash recovery dialog (shown on top of everything)
        self.render_recovery_dialog(ctx);
    }
}
