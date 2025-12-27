//! Participant list widget for group rides.
//!
//! Displays connected riders with their live metrics.

use egui::{Color32, RichText, Ui, Vec2};
use std::collections::HashMap;
use uuid::Uuid;

use crate::networking::protocol::RiderMetrics;
use crate::networking::session::Participant;

/// Participant display configuration.
#[derive(Debug, Clone)]
pub struct ParticipantListConfig {
    /// Show power in watts.
    pub show_power: bool,
    /// Show heart rate.
    pub show_heart_rate: bool,
    /// Show cadence.
    pub show_cadence: bool,
    /// Show speed.
    pub show_speed: bool,
    /// Show distance.
    pub show_distance: bool,
    /// Compact mode (less spacing).
    pub compact: bool,
}

impl Default for ParticipantListConfig {
    fn default() -> Self {
        Self {
            show_power: true,
            show_heart_rate: true,
            show_cadence: true,
            show_speed: true,
            show_distance: false,
            compact: false,
        }
    }
}

/// Participant list widget.
pub struct ParticipantList {
    config: ParticipantListConfig,
}

impl Default for ParticipantList {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticipantList {
    /// Create a new participant list widget.
    pub fn new() -> Self {
        Self {
            config: ParticipantListConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: ParticipantListConfig) -> Self {
        Self { config }
    }

    /// Render the participant list.
    pub fn show(
        &self,
        ui: &mut Ui,
        participants: &[Participant],
        metrics: &HashMap<Uuid, RiderMetrics>,
        local_rider_id: Uuid,
    ) {
        let spacing = if self.config.compact { 2.0 } else { 5.0 };

        for participant in participants {
            let is_local = participant.rider_id == local_rider_id;
            self.show_participant(
                ui,
                participant,
                metrics.get(&participant.rider_id),
                is_local,
            );
            ui.add_space(spacing);
        }
    }

    /// Render a single participant.
    fn show_participant(
        &self,
        ui: &mut Ui,
        participant: &Participant,
        metrics: Option<&RiderMetrics>,
        is_local: bool,
    ) {
        let bg_color = if is_local {
            Color32::from_rgb(40, 60, 80)
        } else {
            Color32::from_rgb(30, 30, 30)
        };

        egui::Frame::new()
            .fill(bg_color)
            .inner_margin(8.0)
            .outer_margin(1.0)
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.horizontal(|ui| {
                    // Avatar placeholder
                    let avatar_size = if self.config.compact { 24.0 } else { 32.0 };
                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::splat(avatar_size), egui::Sense::hover());
                    ui.painter().circle_filled(
                        rect.center(),
                        avatar_size / 2.0,
                        Color32::from_rgb(100, 100, 100),
                    );

                    ui.add_space(8.0);

                    // Name and status
                    ui.vertical(|ui| {
                        let mut name_text = RichText::new(&participant.rider_name);
                        if participant.is_host {
                            name_text = name_text.color(Color32::GOLD);
                        }
                        if is_local {
                            name_text = name_text.strong();
                        }
                        ui.label(name_text);

                        if participant.is_host {
                            ui.label(RichText::new("Host").small().color(Color32::GRAY));
                        } else if is_local {
                            ui.label(RichText::new("You").small().color(Color32::GRAY));
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(m) = metrics {
                            self.show_metrics(ui, m);
                        } else {
                            ui.label(RichText::new("No data").italics().color(Color32::GRAY));
                        }
                    });
                });
            });
    }

    /// Render metrics for a participant.
    fn show_metrics(&self, ui: &mut Ui, metrics: &RiderMetrics) {
        // Render right to left
        if self.config.show_distance {
            ui.label(format!("{:.1}km", metrics.distance_m / 1000.0));
            ui.add_space(10.0);
        }

        if self.config.show_speed {
            ui.label(format!("{:.1}km/h", metrics.speed_kmh));
            ui.add_space(10.0);
        }

        if self.config.show_cadence {
            if let Some(cadence) = metrics.cadence_rpm {
                ui.label(format!("{}rpm", cadence));
            } else {
                ui.label(RichText::new("--rpm").color(Color32::GRAY));
            }
            ui.add_space(10.0);
        }

        if self.config.show_heart_rate {
            if let Some(hr) = metrics.heart_rate_bpm {
                ui.label(
                    RichText::new(format!("{}bpm", hr)).color(Color32::from_rgb(255, 100, 100)),
                );
            } else {
                ui.label(RichText::new("--bpm").color(Color32::GRAY));
            }
            ui.add_space(10.0);
        }

        if self.config.show_power {
            let power_color = power_zone_color(metrics.power_watts);
            ui.label(
                RichText::new(format!("{}W", metrics.power_watts))
                    .color(power_color)
                    .strong(),
            );
        }
    }
}

/// Get color based on power zone (rough estimate without FTP).
fn power_zone_color(watts: u16) -> Color32 {
    match watts {
        0..=100 => Color32::from_rgb(128, 128, 128), // Recovery
        101..=150 => Color32::from_rgb(100, 149, 237), // Endurance
        151..=200 => Color32::from_rgb(50, 205, 50), // Tempo
        201..=250 => Color32::from_rgb(255, 215, 0), // Threshold
        251..=300 => Color32::from_rgb(255, 140, 0), // VO2max
        _ => Color32::from_rgb(255, 69, 0),          // Anaerobic
    }
}

/// Compact participant row for overlay display.
pub struct CompactParticipantRow;

impl CompactParticipantRow {
    /// Show a compact participant row (for overlay during ride).
    pub fn show(ui: &mut Ui, name: &str, metrics: Option<&RiderMetrics>, rank: Option<u32>) {
        ui.horizontal(|ui| {
            // Rank if provided
            if let Some(r) = rank {
                ui.label(RichText::new(format!("#{}", r)).strong());
                ui.add_space(5.0);
            }

            // Name (truncated)
            let display_name: String = name.chars().take(12).collect();
            ui.label(&display_name);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(m) = metrics {
                    ui.label(format!("{:.1}km/h", m.speed_kmh));
                    ui.label(format!("{}W", m.power_watts));
                }
            });
        });
    }
}
