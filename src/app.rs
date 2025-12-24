//! Main application state and egui integration.
//!
//! T042: Create App struct with egui state
//! T044: Implement screen navigation state machine
//! T050: Wire sensor data to UI via crossbeam channel

use eframe::egui;

use crate::metrics::MetricsCalculator;
use crate::recording::RideRecorder;
use crate::sensors::types::{ConnectionState, SensorEvent};
use crate::sensors::SensorManager;
use crate::storage::config::{AppConfig, UserProfile};
use crate::ui::screens::{HomeScreen, RideScreen, Screen, SensorSetupScreen};
use crate::ui::theme::Theme;
use crate::workouts::WorkoutEngine;
use crossbeam::channel::Receiver;
use std::time::Instant;

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
    sensor_manager: SensorManager,
    /// Workout engine
    _workout_engine: WorkoutEngine,
    /// Ride recorder
    _ride_recorder: RideRecorder,
    /// Metrics calculator
    metrics_calculator: MetricsCalculator,
    /// Sensor setup screen state
    sensor_setup_screen: SensorSetupScreen,
    /// Ride screen state
    ride_screen: RideScreen,
    /// Sensor event receiver
    sensor_event_rx: Option<Receiver<SensorEvent>>,
    /// Last UI update time
    last_update: Instant,
    /// Sensor status for status bar
    sensor_status: String,
    /// Number of connected sensors
    connected_sensor_count: usize,
}

impl RustRideApp {
    /// Create a new application instance.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load configuration
        let config = crate::storage::config::load_config().unwrap_or_default();

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

        Self {
            current_screen: Screen::Home,
            theme,
            profile,
            _config: config,
            sensor_manager,
            _workout_engine: workout_engine,
            _ride_recorder: ride_recorder,
            metrics_calculator,
            sensor_setup_screen: SensorSetupScreen::new(),
            ride_screen: RideScreen::new(),
            sensor_event_rx,
            last_update: Instant::now(),
            sensor_status: "No sensors connected".to_string(),
            connected_sensor_count: 0,
        }
    }

    /// Process pending sensor events from the channel.
    fn process_sensor_events(&mut self) {
        if let Some(rx) = &self.sensor_event_rx {
            // Process all available events without blocking
            while let Ok(event) = rx.try_recv() {
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
                                    if self.connected_sensor_count == 1 { "" } else { "s" }
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
                                        if self.connected_sensor_count == 1 { "" } else { "s" }
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
                        self.sensor_setup_screen.update_connection_state(&device_id, state);
                    }
                    SensorEvent::Data(reading) => {
                        // Only process data if we're on the ride screen and recording
                        if self.current_screen == Screen::Ride && !self.ride_screen.is_paused {
                            // Process the reading through the metrics calculator
                            self.metrics_calculator.process(&reading);

                            // Update ride screen metrics
                            self.ride_screen.metrics = self.metrics_calculator.get_aggregated();
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
    }

    /// Update elapsed time on ride screen.
    fn update_ride_time(&mut self) {
        if self.current_screen == Screen::Ride
            && !self.ride_screen.is_paused
            && self.ride_screen.recording_status == crate::recording::types::RecordingStatus::Recording
        {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_update);
            if elapsed.as_secs() >= 1 {
                self.ride_screen.elapsed_seconds += 1;
                self.last_update = now;
            }
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
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.current_screen != Screen::Home {
                self.navigate(Screen::Home);
            }
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
                    if self.ride_screen.recording_status == crate::recording::types::RecordingStatus::Idle {
                        self.ride_screen.start_free_ride();
                    }

                    if let Some(next) = self.ride_screen.show(ui) {
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
                    ui.heading("Settings");
                    ui.label("Settings - coming soon");
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
    }
}
