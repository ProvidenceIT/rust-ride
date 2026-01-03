//! Sensor conflict resolution dialog.
//!
//! T009-4.4: UI dialog to resolve sensor conflicts. Shows both sensors,
//! allows user to select primary, with option to remember choice.
//!
//! This dialog appears when multiple sensors provide the same data type
//! (e.g., two power meters or a power meter and trainer both providing power).

use egui::{Align, Color32, Layout, RichText, Sense, Stroke, Ui, Vec2};

use crate::sensors::conflict::{DataSource, DataType, SensorConflict};
use crate::sensors::types::{SensorProtocol, SensorType};

/// State for the sensor conflict dialog.
#[derive(Debug, Clone)]
pub struct SensorConflictDialogState {
    /// Whether the dialog is visible.
    pub visible: bool,
    /// The conflict being resolved.
    pub conflict: Option<SensorConflict>,
    /// The currently selected sensor device ID.
    pub selected_device_id: Option<String>,
    /// Whether to remember this choice for future sessions.
    pub remember_choice: bool,
    /// Hover state for sensor cards.
    hovered_device_id: Option<String>,
}

impl Default for SensorConflictDialogState {
    fn default() -> Self {
        Self::new()
    }
}

impl SensorConflictDialogState {
    /// Create a new dialog state.
    pub fn new() -> Self {
        Self {
            visible: false,
            conflict: None,
            selected_device_id: None,
            remember_choice: true, // Default to remembering
            hovered_device_id: None,
        }
    }

    /// Open the dialog with a conflict to resolve.
    pub fn open(&mut self, conflict: SensorConflict) {
        // Pre-select the current primary if set
        let selected = conflict.primary_device_id.clone();
        self.conflict = Some(conflict);
        self.selected_device_id = selected;
        self.remember_choice = true;
        self.visible = true;
    }

    /// Close the dialog.
    pub fn close(&mut self) {
        self.visible = false;
        self.conflict = None;
        self.selected_device_id = None;
        self.hovered_device_id = None;
    }

    /// Check if a sensor is selected.
    pub fn is_selected(&self, device_id: &str) -> bool {
        self.selected_device_id.as_deref() == Some(device_id)
    }

    /// Select a sensor.
    pub fn select(&mut self, device_id: &str) {
        self.selected_device_id = Some(device_id.to_string());
    }

    /// Get the selected device ID.
    pub fn selected(&self) -> Option<&str> {
        self.selected_device_id.as_deref()
    }
}

/// Action from the conflict resolution dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictResolutionAction {
    /// User selected a primary sensor.
    SelectPrimary {
        /// The data type being resolved.
        data_type: DataType,
        /// The selected primary device ID.
        device_id: String,
        /// Whether to remember this choice.
        remember: bool,
    },
    /// User cancelled the dialog.
    Cancel,
    /// No action (dialog still open).
    None,
}

/// Response from showing the sensor conflict dialog.
#[derive(Debug)]
pub struct SensorConflictDialogResponse {
    /// The action taken by the user.
    pub action: ConflictResolutionAction,
}

/// Sensor conflict resolution dialog.
///
/// Displays a modal dialog allowing the user to choose which sensor
/// should be the primary source for a conflicted data type.
pub struct SensorConflictDialog<'a> {
    state: &'a mut SensorConflictDialogState,
}

impl<'a> SensorConflictDialog<'a> {
    /// Create a new sensor conflict dialog.
    pub fn new(state: &'a mut SensorConflictDialogState) -> Self {
        Self { state }
    }

    /// Show the dialog and return any action taken.
    pub fn show(&mut self, ui: &mut Ui) -> SensorConflictDialogResponse {
        let mut action = ConflictResolutionAction::None;

        if !self.state.visible {
            return SensorConflictDialogResponse { action };
        }

        let conflict = match &self.state.conflict {
            Some(c) => c.clone(),
            None => {
                return SensorConflictDialogResponse { action };
            }
        };

        egui::Window::new("Sensor Conflict")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_min_size(Vec2::new(420.0, 300.0));

                ui.vertical(|ui| {
                    // Header section
                    self.render_header(ui, &conflict);

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(16.0);

                    // Sensor selection section
                    ui.label(
                        RichText::new("Select which sensor to use as the primary source:")
                            .weak(),
                    );
                    ui.add_space(12.0);

                    // Sensor cards
                    for source in &conflict.sources {
                        let is_selected = self.state.is_selected(&source.device_id);
                        let is_hovered = self.state.hovered_device_id.as_deref() == Some(&source.device_id);

                        if self.render_sensor_option(ui, source, is_selected, is_hovered) {
                            self.state.select(&source.device_id);
                        }
                        ui.add_space(8.0);
                    }

                    ui.add_space(8.0);

                    // Remember choice checkbox
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.state.remember_choice, "Remember this choice");
                        ui.label(RichText::new("(applies to future sessions)").weak().small());
                    });

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(12.0);

                    // Action buttons
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            action = ConflictResolutionAction::Cancel;
                            self.state.close();
                        }

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let can_confirm = self.state.selected_device_id.is_some();

                            if ui
                                .add_enabled(
                                    can_confirm,
                                    egui::Button::new(
                                        RichText::new("Set as Primary")
                                            .color(Color32::WHITE),
                                    )
                                    .fill(Color32::from_rgb(66, 133, 244)),
                                )
                                .clicked()
                            {
                                if let Some(device_id) = self.state.selected_device_id.clone() {
                                    action = ConflictResolutionAction::SelectPrimary {
                                        data_type: conflict.data_type,
                                        device_id,
                                        remember: self.state.remember_choice,
                                    };
                                    self.state.close();
                                }
                            }

                            if !can_confirm {
                                ui.label(RichText::new("Select a sensor").weak().small());
                            }
                        });
                    });
                });
            });

        SensorConflictDialogResponse { action }
    }

    /// Render the dialog header.
    fn render_header(&self, ui: &mut Ui, conflict: &SensorConflict) {
        ui.vertical_centered(|ui| {
            // Conflict icon
            ui.label(RichText::new(data_type_icon(conflict.data_type)).size(40.0));

            ui.add_space(8.0);

            // Title
            ui.label(
                RichText::new(format!("{} Conflict Detected", conflict.data_type))
                    .size(20.0)
                    .strong(),
            );

            ui.add_space(4.0);

            // Subtitle
            let subtitle = format!(
                "{} sensors provide {} data. Choose which one to use.",
                conflict.sources.len(),
                conflict.data_type.display_name().to_lowercase()
            );
            ui.label(RichText::new(subtitle).weak());
        });
    }

    /// Render a sensor option card.
    ///
    /// Returns true if the card was clicked.
    fn render_sensor_option(
        &mut self,
        ui: &mut Ui,
        source: &DataSource,
        is_selected: bool,
        is_hovered: bool,
    ) -> bool {
        let mut clicked = false;

        // Determine card styling based on state
        let (bg_color, border_color, border_width) = if is_selected {
            (
                Color32::from_rgba_unmultiplied(66, 133, 244, 40),
                Color32::from_rgb(66, 133, 244),
                2.0,
            )
        } else if is_hovered {
            (
                ui.visuals().widgets.hovered.bg_fill,
                ui.visuals().widgets.hovered.bg_stroke.color,
                1.0,
            )
        } else {
            (
                ui.visuals().faint_bg_color,
                ui.visuals().widgets.noninteractive.bg_stroke.color,
                1.0,
            )
        };

        let frame = egui::Frame::new()
            .fill(bg_color)
            .stroke(Stroke::new(border_width, border_color))
            .inner_margin(12.0)
            .corner_radius(8.0);

        let response = frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width() - 4.0);

            ui.horizontal(|ui| {
                // Selection indicator
                if is_selected {
                    ui.label(
                        RichText::new("✓")
                            .size(18.0)
                            .color(Color32::from_rgb(66, 133, 244)),
                    );
                } else {
                    ui.label(RichText::new("○").size(18.0).weak());
                }

                ui.add_space(8.0);

                // Sensor icon
                ui.label(RichText::new(sensor_type_icon(source.sensor_type)).size(28.0));

                ui.add_space(8.0);

                // Sensor info
                ui.vertical(|ui| {
                    // Name and protocol badge
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&source.name).strong());
                        ui.add_space(8.0);
                        ui.label(protocol_badge(source.protocol));
                    });

                    // Sensor type and connection status
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{}", source.sensor_type)).weak().small());

                        ui.add_space(8.0);

                        if source.is_connected {
                            ui.label(
                                RichText::new("● Connected")
                                    .small()
                                    .color(Color32::from_rgb(52, 168, 83)),
                            );
                        } else {
                            ui.label(
                                RichText::new("○ Not connected")
                                    .small()
                                    .color(Color32::GRAY),
                            );
                        }
                    });
                });

                // Primary badge if this was previously selected
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if source.is_primary {
                        ui.label(
                            RichText::new("CURRENT PRIMARY")
                                .small()
                                .color(Color32::from_rgb(102, 187, 106)),
                        );
                    }
                });
            });
        });

        // Handle interaction
        let rect = response.response.rect;
        let response = ui.interact(rect, ui.id().with(&source.device_id), Sense::click());

        if response.hovered() {
            self.state.hovered_device_id = Some(source.device_id.clone());
        }

        if response.clicked() {
            clicked = true;
        }

        clicked
    }
}

/// Get an icon for a data type.
fn data_type_icon(data_type: DataType) -> &'static str {
    match data_type {
        DataType::Power => "⚡",
        DataType::HeartRate => "❤",
        DataType::Cadence => "🔄",
        DataType::Speed => "💨",
        DataType::TrainerControl => "🎮",
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

/// Get a protocol badge.
fn protocol_badge(protocol: SensorProtocol) -> RichText {
    match protocol {
        SensorProtocol::Ble => RichText::new("BLE")
            .small()
            .color(Color32::from_rgb(0, 122, 255))
            .background_color(Color32::from_gray(40)),
        SensorProtocol::AntPlus => RichText::new("ANT+")
            .small()
            .color(Color32::from_rgb(255, 102, 0))
            .background_color(Color32::from_gray(40)),
    }
}

/// Conflict notification banner for showing unresolved conflicts.
pub struct ConflictNotificationBanner<'a> {
    conflicts: &'a [SensorConflict],
}

impl<'a> ConflictNotificationBanner<'a> {
    /// Create a new conflict notification banner.
    pub fn new(conflicts: &'a [SensorConflict]) -> Self {
        Self { conflicts }
    }

    /// Show the notification banner.
    ///
    /// Returns the data type that was clicked for resolution, if any.
    pub fn show(&self, ui: &mut Ui) -> Option<DataType> {
        if self.conflicts.is_empty() {
            return None;
        }

        let mut clicked_conflict: Option<DataType> = None;

        let warning_bg = Color32::from_rgba_unmultiplied(251, 188, 4, 30);
        let warning_border = Color32::from_rgb(251, 188, 4);

        egui::Frame::new()
            .fill(warning_bg)
            .stroke(Stroke::new(1.0, warning_border))
            .inner_margin(12.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚠").size(20.0).color(warning_border));
                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Sensor Conflicts Detected")
                                .strong()
                                .color(warning_border),
                        );

                        let conflict_count = self.conflicts.len();
                        let text = if conflict_count == 1 {
                            "Multiple sensors provide the same data type. Click to resolve."
                        } else {
                            "Multiple sensors provide the same data types. Click to resolve."
                        };
                        ui.label(RichText::new(text).weak().small());

                        ui.add_space(8.0);

                        // Show conflict chips
                        ui.horizontal_wrapped(|ui| {
                            for conflict in self.conflicts {
                                let chip_text = format!(
                                    "{} {} ({})",
                                    data_type_icon(conflict.data_type),
                                    conflict.data_type,
                                    conflict.sensor_count()
                                );

                                let chip = egui::Button::new(
                                    RichText::new(chip_text).small(),
                                )
                                .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 20))
                                .corner_radius(12.0);

                                if ui.add(chip).clicked() {
                                    clicked_conflict = Some(conflict.data_type);
                                }
                            }
                        });
                    });
                });
            });

        clicked_conflict
    }
}

/// Compact conflict indicator for status bar.
pub struct ConflictIndicator {
    /// Number of unresolved conflicts.
    conflict_count: usize,
}

impl ConflictIndicator {
    /// Create a new conflict indicator.
    pub fn new(conflict_count: usize) -> Self {
        Self { conflict_count }
    }

    /// Show the indicator.
    ///
    /// Returns true if clicked.
    pub fn show(&self, ui: &mut Ui) -> bool {
        if self.conflict_count == 0 {
            return false;
        }

        let label = if self.conflict_count == 1 {
            "⚠ 1 conflict".to_string()
        } else {
            format!("⚠ {} conflicts", self.conflict_count)
        };

        let response = ui.add(
            egui::Button::new(RichText::new(label).color(Color32::from_rgb(251, 188, 4)))
                .fill(Color32::from_rgba_unmultiplied(251, 188, 4, 30)),
        );

        response.clicked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_data_source(device_id: &str, name: &str, sensor_type: SensorType) -> DataSource {
        DataSource {
            device_id: device_id.to_string(),
            name: name.to_string(),
            sensor_type,
            protocol: SensorProtocol::Ble,
            is_connected: true,
            data_type: DataType::Power,
            is_primary: false,
        }
    }

    fn make_conflict() -> SensorConflict {
        let sources = vec![
            make_data_source("power_meter_1", "Stages Power", SensorType::PowerMeter),
            make_data_source("trainer_1", "KICKR Core", SensorType::Trainer),
        ];
        SensorConflict::new(DataType::Power, sources)
    }

    #[test]
    fn test_dialog_state_new() {
        let state = SensorConflictDialogState::new();
        assert!(!state.visible);
        assert!(state.conflict.is_none());
        assert!(state.selected_device_id.is_none());
        assert!(state.remember_choice);
    }

    #[test]
    fn test_dialog_state_open() {
        let mut state = SensorConflictDialogState::new();
        let conflict = make_conflict();

        state.open(conflict.clone());

        assert!(state.visible);
        assert!(state.conflict.is_some());
        assert!(state.remember_choice);
    }

    #[test]
    fn test_dialog_state_close() {
        let mut state = SensorConflictDialogState::new();
        let conflict = make_conflict();

        state.open(conflict);
        state.select("power_meter_1");
        state.close();

        assert!(!state.visible);
        assert!(state.conflict.is_none());
        assert!(state.selected_device_id.is_none());
    }

    #[test]
    fn test_dialog_state_selection() {
        let mut state = SensorConflictDialogState::new();
        let conflict = make_conflict();

        state.open(conflict);

        assert!(!state.is_selected("power_meter_1"));
        assert!(!state.is_selected("trainer_1"));

        state.select("power_meter_1");

        assert!(state.is_selected("power_meter_1"));
        assert!(!state.is_selected("trainer_1"));
        assert_eq!(state.selected(), Some("power_meter_1"));
    }

    #[test]
    fn test_dialog_state_preselects_existing_primary() {
        let mut state = SensorConflictDialogState::new();
        let mut conflict = make_conflict();
        conflict.set_primary("trainer_1");

        state.open(conflict);

        // Should pre-select the existing primary
        assert!(state.is_selected("trainer_1"));
    }

    #[test]
    fn test_data_type_icons() {
        assert_eq!(data_type_icon(DataType::Power), "⚡");
        assert_eq!(data_type_icon(DataType::HeartRate), "❤");
        assert_eq!(data_type_icon(DataType::Cadence), "🔄");
        assert_eq!(data_type_icon(DataType::Speed), "💨");
        assert_eq!(data_type_icon(DataType::TrainerControl), "🎮");
    }

    #[test]
    fn test_sensor_type_icons() {
        assert_eq!(sensor_type_icon(SensorType::PowerMeter), "⚡");
        assert_eq!(sensor_type_icon(SensorType::HeartRate), "❤");
        assert_eq!(sensor_type_icon(SensorType::Trainer), "🚴");
    }

    #[test]
    fn test_conflict_resolution_action_eq() {
        let action1 = ConflictResolutionAction::SelectPrimary {
            data_type: DataType::Power,
            device_id: "test".to_string(),
            remember: true,
        };
        let action2 = ConflictResolutionAction::SelectPrimary {
            data_type: DataType::Power,
            device_id: "test".to_string(),
            remember: true,
        };
        let action3 = ConflictResolutionAction::Cancel;

        assert_eq!(action1, action2);
        assert_ne!(action1, action3);
        assert_eq!(action3, ConflictResolutionAction::Cancel);
    }

    #[test]
    fn test_conflict_indicator() {
        let indicator = ConflictIndicator::new(0);
        assert_eq!(indicator.conflict_count, 0);

        let indicator = ConflictIndicator::new(2);
        assert_eq!(indicator.conflict_count, 2);
    }
}
