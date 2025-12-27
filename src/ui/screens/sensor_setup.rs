//! Sensor setup screen implementation.
//!
//! T045: Implement sensor discovery list widget
//! T046: Implement sensor pairing confirmation dialog

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};

use crate::sensors::ant::dongle::{AntDongle, DongleStatus};
use crate::sensors::types::{ConnectionState, DiscoveredSensor, Protocol, SensorState, SensorType};

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
    /// ANT+ enabled status
    pub ant_enabled: bool,
    /// Detected ANT+ dongles
    pub ant_dongles: Vec<AntDongle>,
    /// Show protocol choice dialog (for dual-protocol sensors)
    pub show_protocol_dialog: bool,
    /// Sensor for protocol choice (device_id, ble_sensor, ant_sensor)
    pub protocol_choice_sensor:
        Option<(String, Option<DiscoveredSensor>, Option<DiscoveredSensor>)>,
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
            self.connected_sensors.retain(|s| s.device_id != device_id);
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

    /// Update the list of ANT+ dongles.
    pub fn set_ant_dongles(&mut self, dongles: Vec<AntDongle>) {
        self.ant_dongles = dongles;
    }

    /// Set ANT+ enabled state.
    pub fn set_ant_enabled(&mut self, enabled: bool) {
        self.ant_enabled = enabled;
    }

    /// Check if a sensor with the same name exists with a different protocol.
    /// Returns Some((device_id, ble_sensor, ant_sensor)) if dual-protocol detected.
    pub fn find_dual_protocol_sensor(
        &self,
        sensor: &DiscoveredSensor,
    ) -> Option<(String, DiscoveredSensor, DiscoveredSensor)> {
        // Check if there's another sensor with same name but different protocol
        for existing in &self.discovered_sensors {
            if existing.name == sensor.name && existing.device_id != sensor.device_id {
                let is_ble = matches!(
                    sensor.protocol,
                    Protocol::BleFtms
                        | Protocol::BleCyclingPower
                        | Protocol::BleHeartRate
                        | Protocol::BleCsc
                );
                let existing_is_ble = matches!(
                    existing.protocol,
                    Protocol::BleFtms
                        | Protocol::BleCyclingPower
                        | Protocol::BleHeartRate
                        | Protocol::BleCsc
                );

                // One is BLE, one is ANT+
                if is_ble != existing_is_ble {
                    let (ble, ant) = if is_ble {
                        (sensor.clone(), existing.clone())
                    } else {
                        (existing.clone(), sensor.clone())
                    };
                    return Some((sensor.name.clone(), ble, ant));
                }
            }
        }
        None
    }

    /// Show protocol choice dialog for dual-protocol sensors.
    pub fn show_protocol_choice(
        &mut self,
        base_name: String,
        ble: DiscoveredSensor,
        ant: DiscoveredSensor,
    ) {
        self.protocol_choice_sensor = Some((base_name, Some(ble), Some(ant)));
        self.show_protocol_dialog = true;
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

            // Scanning controls and ANT+ status
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

                ui.separator();

                // ANT+ toggle
                let ant_available = !self.ant_dongles.is_empty();
                ui.add_enabled_ui(ant_available, |ui| {
                    if ui.checkbox(&mut self.ant_enabled, "ANT+").changed() {
                        // TODO: Toggle ANT+ scanning
                    }
                });

                // ANT+ dongle status indicator
                if ant_available {
                    let dongle = &self.ant_dongles[0];
                    let status_text = match &dongle.status {
                        DongleStatus::Detected => "Detected",
                        DongleStatus::Initializing => "Initializing...",
                        DongleStatus::Ready => "Ready",
                        DongleStatus::Error(e) => "Error",
                        DongleStatus::Disconnected => "Disconnected",
                    };
                    let status_color = match &dongle.status {
                        DongleStatus::Ready => Color32::from_rgb(52, 168, 83),
                        DongleStatus::Initializing | DongleStatus::Detected => {
                            Color32::from_rgb(251, 188, 4)
                        }
                        DongleStatus::Error(_) | DongleStatus::Disconnected => {
                            Color32::from_rgb(234, 67, 53)
                        }
                    };
                    ui.label(
                        RichText::new(format!("📡 {}", status_text))
                            .color(status_color)
                            .small(),
                    );
                } else {
                    ui.label(
                        RichText::new("📡 No ANT+ dongle")
                            .color(Color32::GRAY)
                            .small(),
                    );
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
                            ui.label(
                                RichText::new("Start scanning to discover nearby sensors").weak(),
                            );
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

        // Protocol choice dialog for dual-protocol sensors
        if self.show_protocol_dialog {
            if let Some((name, ble, ant)) = &self.protocol_choice_sensor.clone() {
                self.render_protocol_choice_dialog(ui, name, ble.as_ref(), ant.as_ref());
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
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&sensor.name).strong());
                        // Protocol badge
                        ui.label(protocol_badge(sensor.protocol));
                    });
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
                            .add(egui::Button::new("Connect").fill(Color32::from_rgb(66, 133, 244)))
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

    /// Render protocol choice dialog for dual-protocol sensors.
    fn render_protocol_choice_dialog(
        &mut self,
        ui: &mut Ui,
        name: &str,
        ble: Option<&DiscoveredSensor>,
        ant: Option<&DiscoveredSensor>,
    ) {
        egui::Window::new("Choose Protocol")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_min_size(Vec2::new(350.0, 200.0));

                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);

                    ui.label(RichText::new("🔄").size(32.0));
                    ui.add_space(8.0);
                    ui.label(RichText::new(name).size(18.0).strong());
                    ui.add_space(4.0);
                    ui.label("This sensor is available via both BLE and ANT+.");
                    ui.label("Which protocol would you like to use?");

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        ui.add_space(20.0);

                        // BLE option
                        ui.vertical(|ui| {
                            ui.set_min_width(120.0);
                            let ble_button =
                                egui::Button::new(RichText::new("BLE (Bluetooth)").size(14.0))
                                    .fill(Color32::from_rgb(0, 122, 255));

                            if ui.add(ble_button).clicked() {
                                if let Some(sensor) = ble {
                                    // Find index of BLE sensor
                                    if let Some(idx) = self
                                        .discovered_sensors
                                        .iter()
                                        .position(|s| s.device_id == sensor.device_id)
                                    {
                                        self.selected_sensor = Some(idx);
                                        self.show_pairing_dialog = true;
                                    }
                                }
                                self.show_protocol_dialog = false;
                                self.protocol_choice_sensor = None;
                            }
                            ui.add_space(4.0);
                            ui.label(RichText::new("Better compatibility").small().weak());
                        });

                        ui.add_space(20.0);

                        // ANT+ option
                        ui.vertical(|ui| {
                            ui.set_min_width(120.0);
                            let ant_button = egui::Button::new(RichText::new("ANT+").size(14.0))
                                .fill(Color32::from_rgb(255, 102, 0));

                            if ui.add(ant_button).clicked() {
                                if let Some(sensor) = ant {
                                    // Find index of ANT+ sensor
                                    if let Some(idx) = self
                                        .discovered_sensors
                                        .iter()
                                        .position(|s| s.device_id == sensor.device_id)
                                    {
                                        self.selected_sensor = Some(idx);
                                        self.show_pairing_dialog = true;
                                    }
                                }
                                self.show_protocol_dialog = false;
                                self.protocol_choice_sensor = None;
                            }
                            ui.add_space(4.0);
                            ui.label(RichText::new("Lower latency").small().weak());
                        });
                    });

                    ui.add_space(16.0);

                    if ui.button("Cancel").clicked() {
                        self.show_protocol_dialog = false;
                        self.protocol_choice_sensor = None;
                    }
                });
            });
    }
}

/// Get an icon for a sensor type.
fn sensor_type_icon(sensor_type: SensorType) -> &'static str {
    match sensor_type {
        SensorType::Trainer | SensorType::SmartTrainer => "🚴",
        SensorType::PowerMeter => "⚡",
        SensorType::HeartRate => "❤",
        SensorType::Cadence | SensorType::CadenceSensor => "🔄",
        SensorType::Speed => "💨",
        SensorType::SpeedCadence => "📊",
        SensorType::SmO2 => "🩸",
        SensorType::Imu => "📐",
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

    RichText::new(format!("{} {}%", icon, level))
        .color(color)
        .small()
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

/// Get a protocol badge for BLE/ANT+.
fn protocol_badge(protocol: Protocol) -> RichText {
    let (text, color) = match protocol {
        Protocol::BleFtms
        | Protocol::BleCyclingPower
        | Protocol::BleHeartRate
        | Protocol::BleCsc => {
            ("BLE", Color32::from_rgb(0, 122, 255)) // Blue for BLE
        }
        Protocol::AntHeartRate
        | Protocol::AntPower
        | Protocol::AntFec
        | Protocol::AntSpeedCadence => {
            ("ANT+", Color32::from_rgb(255, 102, 0)) // Orange for ANT+
        }
    };

    RichText::new(text)
        .small()
        .color(color)
        .background_color(Color32::from_gray(40))
}
