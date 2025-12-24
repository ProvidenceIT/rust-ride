//! Sensor setup screen implementation.
//!
//! T045: Implement sensor discovery list widget
//! T046: Implement sensor pairing confirmation dialog

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};

use crate::sensors::types::{ConnectionState, DiscoveredSensor, SensorState, SensorType};

use super::Screen;

/// Sensor setup screen state.
#[derive(Default)]
pub struct SensorSetupScreen {
    /// Whether discovery is active
    pub is_scanning: bool,
    /// List of discovered sensors
    pub discovered_sensors: Vec<DiscoveredSensor>,
    /// List of connected sensors
    pub connected_sensors: Vec<SensorState>,
    /// Selected sensor for pairing dialog
    pub selected_sensor: Option<usize>,
    /// Show pairing confirmation dialog
    pub show_pairing_dialog: bool,
}

impl SensorSetupScreen {
    /// Create a new sensor setup screen.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a newly discovered sensor to the list.
    pub fn add_discovered_sensor(&mut self, sensor: DiscoveredSensor) {
        // Check if sensor already exists (by device_id)
        if !self
            .discovered_sensors
            .iter()
            .any(|s| s.device_id == sensor.device_id)
        {
            self.discovered_sensors.push(sensor);
        }
    }

    /// Update connection state for a sensor.
    pub fn update_connection_state(&mut self, device_id: &str, state: ConnectionState) {
        // If connecting/connected, move from discovered to connected list
        if state == ConnectionState::Connected {
            if let Some(idx) = self
                .discovered_sensors
                .iter()
                .position(|s| s.device_id == device_id)
            {
                let sensor = self.discovered_sensors.remove(idx);
                let sensor_state = SensorState {
                    id: uuid::Uuid::new_v4(),
                    device_id: sensor.device_id,
                    name: sensor.name,
                    sensor_type: sensor.sensor_type,
                    protocol: sensor.protocol,
                    connection_state: state,
                    signal_strength: sensor.signal_strength,
                    battery_level: None,
                    last_data_at: None,
                    is_primary: self.connected_sensors.is_empty(), // First sensor is primary
                };
                self.connected_sensors.push(sensor_state);
            }
        } else if state == ConnectionState::Disconnected {
            // Remove from connected list
            self.connected_sensors
                .retain(|s| s.device_id != device_id);
        } else {
            // Update state of existing connected sensor
            if let Some(sensor) = self
                .connected_sensors
                .iter_mut()
                .find(|s| s.device_id == device_id)
            {
                sensor.connection_state = state;
            }
        }
    }

    /// Set whether scanning is active.
    pub fn set_scanning(&mut self, scanning: bool) {
        self.is_scanning = scanning;
    }

    /// Render the sensor setup screen.
    pub fn show(&mut self, ui: &mut Ui) -> Option<Screen> {
        let mut next_screen = None;

        ui.vertical(|ui| {
            // Header
            ui.horizontal(|ui| {
                if ui.button("← Back").clicked() {
                    next_screen = Some(Screen::Home);
                }
                ui.heading("Sensor Setup");
            });

            ui.add_space(16.0);

            // Scanning controls
            ui.horizontal(|ui| {
                if self.is_scanning {
                    if ui.button("Stop Scanning").clicked() {
                        self.is_scanning = false;
                    }
                    ui.spinner();
                    ui.label("Scanning for sensors...");
                } else if ui.button("Start Scanning").clicked() {
                    self.is_scanning = true;
                    // TODO: Trigger actual BLE scan
                }
            });

            ui.add_space(16.0);
            ui.separator();

            // Two-column layout: Discovered | Connected
            ui.columns(2, |columns| {
                // Left column: Discovered sensors
                columns[0].vertical(|ui| {
                    ui.heading("Discovered Sensors");
                    ui.add_space(8.0);

                    if self.discovered_sensors.is_empty() {
                        if self.is_scanning {
                            ui.label(RichText::new("Searching...").weak());
                        } else {
                            ui.label(RichText::new("No sensors found").weak());
                            ui.label(RichText::new("Start scanning to discover nearby sensors").weak());
                        }

                        // T154: Troubleshooting tips
                        ui.add_space(16.0);
                        ui.group(|ui| {
                            ui.label(RichText::new("Troubleshooting Tips").size(14.0).strong());
                            ui.add_space(4.0);
                            ui.label("• Make sure Bluetooth is enabled on your device");
                            ui.label("• Ensure your trainer/sensors are powered on");
                            ui.label("• Keep sensors within 10 meters of your computer");
                            ui.label("• Wake up your sensors by moving/pedaling");
                            ui.label("• Check that no other app is connected to the sensor");
                            ui.label("• Try restarting the sensor if it won't appear");
                        });
                    } else {
                        // Clone sensors to avoid borrow conflict with mutable self
                        let sensors: Vec<_> = self.discovered_sensors.clone();
                        for (i, sensor) in sensors.iter().enumerate() {
                            self.render_discovered_sensor(ui, sensor, i);
                        }
                    }
                });

                // Right column: Connected sensors
                columns[1].vertical(|ui| {
                    ui.heading("Connected Sensors");
                    ui.add_space(8.0);

                    if self.connected_sensors.is_empty() {
                        ui.label(RichText::new("No sensors connected").weak());
                    } else {
                        for sensor in &self.connected_sensors {
                            self.render_connected_sensor(ui, sensor);
                        }
                    }
                });
            });
        });

        // Pairing confirmation dialog
        if self.show_pairing_dialog {
            if let Some(idx) = self.selected_sensor {
                if idx < self.discovered_sensors.len() {
                    // Clone to avoid borrow conflict
                    let sensor = self.discovered_sensors[idx].clone();
                    self.render_pairing_dialog(ui, &sensor);
                }
            }
        }

        next_screen
    }

    /// Render a discovered sensor item.
    fn render_discovered_sensor(&mut self, ui: &mut Ui, sensor: &DiscoveredSensor, index: usize) {
        let frame = egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(12.0)
            .corner_radius(4.0);

        frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                // Sensor icon
                let icon = sensor_type_icon(sensor.sensor_type);
                ui.label(RichText::new(icon).size(24.0));

                ui.vertical(|ui| {
                    ui.label(RichText::new(&sensor.name).strong());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{}", sensor.sensor_type)).weak());
                        if let Some(rssi) = sensor.signal_strength {
                            ui.label(signal_indicator(rssi));
                        }
                    });
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Connect").clicked() {
                        self.selected_sensor = Some(index);
                        self.show_pairing_dialog = true;
                    }
                });
            });
        });

        ui.add_space(4.0);
    }

    /// Render a connected sensor item.
    fn render_connected_sensor(&self, ui: &mut Ui, sensor: &SensorState) {
        let frame = egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(12.0)
            .corner_radius(4.0);

        frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                // Sensor icon
                let icon = sensor_type_icon(sensor.sensor_type);
                ui.label(RichText::new(icon).size(24.0));

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&sensor.name).strong());
                        if sensor.is_primary {
                            ui.label(
                                RichText::new("PRIMARY")
                                    .small()
                                    .color(Color32::from_rgb(52, 168, 83)),
                            );
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(connection_status_label(sensor.connection_state));
                        if let Some(battery) = sensor.battery_level {
                            ui.label(battery_indicator(battery));
                        }
                    });
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Disconnect").clicked() {
                        // TODO: Disconnect sensor
                    }
                });
            });
        });

        ui.add_space(4.0);
    }

    /// Render the pairing confirmation dialog.
    fn render_pairing_dialog(&mut self, ui: &mut Ui, sensor: &DiscoveredSensor) {
        egui::Window::new("Connect Sensor")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_min_size(Vec2::new(300.0, 150.0));

                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);

                    let icon = sensor_type_icon(sensor.sensor_type);
                    ui.label(RichText::new(icon).size(48.0));

                    ui.add_space(8.0);
                    ui.label(RichText::new(&sensor.name).size(18.0).strong());
                    ui.label(format!("{}", sensor.sensor_type));

                    ui.add_space(16.0);
                    ui.label("Connect to this sensor?");

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_pairing_dialog = false;
                            self.selected_sensor = None;
                        }

                        ui.add_space(16.0);

                        if ui
                            .add(
                                egui::Button::new("Connect")
                                    .fill(Color32::from_rgb(66, 133, 244)),
                            )
                            .clicked()
                        {
                            // TODO: Actually connect to the sensor
                            self.show_pairing_dialog = false;
                            self.selected_sensor = None;
                        }
                    });
                });
            });
    }
}

/// Get an icon for a sensor type.
fn sensor_type_icon(sensor_type: SensorType) -> &'static str {
    match sensor_type {
        SensorType::Trainer => "🚴",
        SensorType::PowerMeter => "⚡",
        SensorType::HeartRate => "❤",
        SensorType::Cadence => "🔄",
        SensorType::Speed => "💨",
        SensorType::SpeedCadence => "📊",
    }
}

/// Get a signal strength indicator.
fn signal_indicator(rssi: i16) -> RichText {
    let (bars, color) = if rssi > -50 {
        ("●●●●", Color32::from_rgb(52, 168, 83)) // Excellent
    } else if rssi > -60 {
        ("●●●○", Color32::from_rgb(52, 168, 83)) // Good
    } else if rssi > -70 {
        ("●●○○", Color32::from_rgb(251, 188, 4)) // Fair
    } else {
        ("●○○○", Color32::from_rgb(234, 67, 53)) // Weak
    };

    RichText::new(bars).color(color).small()
}

/// Get a battery indicator.
fn battery_indicator(level: u8) -> RichText {
    let (icon, color) = if level > 80 {
        ("🔋", Color32::from_rgb(52, 168, 83))
    } else if level > 40 {
        ("🔋", Color32::from_rgb(251, 188, 4))
    } else if level > 20 {
        ("🪫", Color32::from_rgb(255, 128, 0))
    } else {
        ("🪫", Color32::from_rgb(234, 67, 53))
    };

    RichText::new(format!("{} {}%", icon, level)).color(color).small()
}

/// Get a connection status label.
fn connection_status_label(state: ConnectionState) -> RichText {
    match state {
        ConnectionState::Connected => {
            RichText::new("● Connected").color(Color32::from_rgb(52, 168, 83))
        }
        ConnectionState::Connecting => {
            RichText::new("◐ Connecting...").color(Color32::from_rgb(251, 188, 4))
        }
        ConnectionState::Reconnecting => {
            RichText::new("◐ Reconnecting...").color(Color32::from_rgb(255, 128, 0))
        }
        ConnectionState::Disconnected => {
            RichText::new("○ Disconnected").color(Color32::from_rgb(160, 160, 170))
        }
    }
}
