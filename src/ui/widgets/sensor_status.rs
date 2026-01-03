//! Sensor status indicator widget.
//!
//! T047: Implement connection status indicators
//! T009-3.4: Add connection quality indicator support

use egui::{Color32, RichText, Ui, Vec2};

use crate::sensors::quality::{QualityLevel, QualityStats};
use crate::sensors::types::{ConnectionState, SensorState, SensorType};

use super::connection_quality::ConnectionQualityIndicator;

/// A compact sensor status indicator for the status bar.
pub struct SensorStatusIndicator<'a> {
    sensors: &'a [SensorState],
}

impl<'a> SensorStatusIndicator<'a> {
    /// Create a new sensor status indicator.
    pub fn new(sensors: &'a [SensorState]) -> Self {
        Self { sensors }
    }

    /// Render the sensor status indicator.
    pub fn show(self, ui: &mut Ui) {
        if self.sensors.is_empty() {
            ui.label(RichText::new("No sensors").weak());
            return;
        }

        ui.horizontal(|ui| {
            for sensor in self.sensors {
                self.render_sensor_badge(ui, sensor);
            }
        });
    }

    /// Render a single sensor badge.
    fn render_sensor_badge(&self, ui: &mut Ui, sensor: &SensorState) {
        let (icon, color) = match sensor.connection_state {
            ConnectionState::Connected => (
                sensor_icon(sensor.sensor_type),
                Color32::from_rgb(52, 168, 83),
            ),
            ConnectionState::Connecting | ConnectionState::Reconnecting => (
                sensor_icon(sensor.sensor_type),
                Color32::from_rgb(251, 188, 4),
            ),
            ConnectionState::Disconnected => (
                sensor_icon(sensor.sensor_type),
                Color32::from_rgb(160, 160, 170),
            ),
        };

        ui.label(RichText::new(icon).color(color));
    }
}

/// Get an icon for a sensor type.
fn sensor_icon(sensor_type: SensorType) -> &'static str {
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

/// A detailed sensor connection card.
pub struct SensorConnectionCard<'a> {
    sensor: &'a SensorState,
    show_details: bool,
    quality_stats: Option<&'a QualityStats>,
}

impl<'a> SensorConnectionCard<'a> {
    /// Create a new sensor connection card.
    pub fn new(sensor: &'a SensorState) -> Self {
        Self {
            sensor,
            show_details: true,
            quality_stats: None,
        }
    }

    /// Set whether to show detailed information.
    pub fn with_details(mut self, show: bool) -> Self {
        self.show_details = show;
        self
    }

    /// Set quality stats to display.
    pub fn with_quality(mut self, stats: &'a QualityStats) -> Self {
        self.quality_stats = Some(stats);
        self
    }

    /// Render the sensor connection card.
    pub fn show(self, ui: &mut Ui) {
        // Check if this connection has poor quality
        let is_poor_quality = self
            .quality_stats
            .map(|q| q.level == QualityLevel::Poor)
            .unwrap_or(false);

        // Use different background color based on connection state and quality
        let bg_color = if is_poor_quality {
            Color32::from_rgba_unmultiplied(234, 67, 53, 30) // Red tint for poor connections
        } else {
            match self.sensor.connection_state {
                ConnectionState::Connected => Color32::from_rgba_unmultiplied(52, 168, 83, 30),
                ConnectionState::Connecting | ConnectionState::Reconnecting => {
                    Color32::from_rgba_unmultiplied(251, 188, 4, 30)
                }
                ConnectionState::Disconnected => Color32::from_rgba_unmultiplied(160, 160, 170, 20),
            }
        };

        egui::Frame::new()
            .fill(bg_color)
            .inner_margin(12.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(200.0, 60.0));

                ui.horizontal(|ui| {
                    // Icon
                    ui.label(RichText::new(sensor_icon(self.sensor.sensor_type)).size(32.0));

                    ui.vertical(|ui| {
                        // Name with warning icon if poor quality
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&self.sensor.name).strong());
                            if is_poor_quality {
                                ui.label(
                                    RichText::new("⚠")
                                        .color(Color32::from_rgb(234, 67, 53)),
                                );
                            }
                        });

                        // Status
                        let status_text = match self.sensor.connection_state {
                            ConnectionState::Connected => "Connected",
                            ConnectionState::Connecting => "Connecting...",
                            ConnectionState::Reconnecting => "Reconnecting...",
                            ConnectionState::Disconnected => "Disconnected",
                        };

                        let status_color = match self.sensor.connection_state {
                            ConnectionState::Connected => Color32::from_rgb(52, 168, 83),
                            ConnectionState::Connecting | ConnectionState::Reconnecting => {
                                Color32::from_rgb(251, 188, 4)
                            }
                            ConnectionState::Disconnected => Color32::from_rgb(160, 160, 170),
                        };

                        ui.label(RichText::new(status_text).color(status_color).small());

                        // Details
                        if self.show_details {
                            ui.horizontal(|ui| {
                                if let Some(battery) = self.sensor.battery_level {
                                    ui.label(
                                        RichText::new(format!("🔋 {}%", battery)).weak().small(),
                                    );
                                }

                                // Show quality indicator if available
                                if let Some(stats) = self.quality_stats {
                                    ConnectionQualityIndicator::new()
                                        .with_stats(stats.clone())
                                        .compact()
                                        .show(ui);

                                    ui.add_space(4.0);

                                    // Show quality level text
                                    let quality_color = match stats.level {
                                        QualityLevel::Excellent => Color32::from_rgb(52, 168, 83),
                                        QualityLevel::Good => Color32::from_rgb(102, 187, 106),
                                        QualityLevel::Fair => Color32::from_rgb(251, 188, 4),
                                        QualityLevel::Poor => Color32::from_rgb(234, 67, 53),
                                    };
                                    ui.label(
                                        RichText::new(format!("{}", stats.level))
                                            .small()
                                            .color(quality_color),
                                    );
                                } else if let Some(rssi) = self.sensor.signal_strength {
                                    // Fallback to RSSI display if no quality stats
                                    ui.label(
                                        RichText::new(format!("📶 {} dBm", rssi)).weak().small(),
                                    );
                                }
                            });
                        }
                    });
                });
            });
    }
}

/// Aggregate sensor status for quick overview.
#[derive(Debug, Clone, Default)]
pub struct SensorStatusSummary {
    /// Total number of sensors
    pub total: usize,
    /// Number of connected sensors
    pub connected: usize,
    /// Number of sensors with data
    pub active: usize,
    /// Has power data
    pub has_power: bool,
    /// Has heart rate data
    pub has_heart_rate: bool,
    /// Has cadence data
    pub has_cadence: bool,
    /// Number of sensors with poor quality
    pub poor_quality_count: usize,
    /// Number of sensors with fair quality
    pub fair_quality_count: usize,
}

impl SensorStatusSummary {
    /// Create a summary from a list of sensor states.
    pub fn from_sensors(sensors: &[SensorState]) -> Self {
        let mut summary = Self {
            total: sensors.len(),
            ..Default::default()
        };

        for sensor in sensors {
            if sensor.connection_state == ConnectionState::Connected {
                summary.connected += 1;

                match sensor.sensor_type {
                    SensorType::Trainer | SensorType::PowerMeter => {
                        summary.has_power = true;
                    }
                    SensorType::HeartRate => {
                        summary.has_heart_rate = true;
                    }
                    SensorType::Cadence | SensorType::SpeedCadence => {
                        summary.has_cadence = true;
                    }
                    _ => {}
                }
            }
        }

        summary
    }

    /// Create a summary from sensor states with quality information.
    pub fn from_sensors_with_quality(
        sensors: &[SensorState],
        quality_stats: &std::collections::HashMap<String, QualityStats>,
    ) -> Self {
        let mut summary = Self::from_sensors(sensors);

        for sensor in sensors {
            if sensor.connection_state == ConnectionState::Connected {
                if let Some(quality) = quality_stats.get(&sensor.device_id) {
                    match quality.level {
                        QualityLevel::Poor => summary.poor_quality_count += 1,
                        QualityLevel::Fair => summary.fair_quality_count += 1,
                        _ => {}
                    }
                }
            }
        }

        summary
    }

    /// Check if all required sensors are connected for a ride.
    pub fn is_ride_ready(&self) -> bool {
        self.has_power
    }

    /// Check if any sensor has quality issues.
    pub fn has_quality_issues(&self) -> bool {
        self.poor_quality_count > 0
    }

    /// Check if any sensor has degraded quality.
    pub fn has_degraded_quality(&self) -> bool {
        self.poor_quality_count > 0 || self.fair_quality_count > 0
    }

    /// Get a status message.
    pub fn status_message(&self) -> &'static str {
        if self.total == 0 {
            "No sensors paired"
        } else if self.connected == 0 {
            "No sensors connected"
        } else if !self.has_power {
            "No power source"
        } else if self.poor_quality_count > 0 {
            "Connection issues detected"
        } else {
            "Ready to ride"
        }
    }
}
