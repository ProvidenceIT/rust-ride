//! Power meter calibration dialog.
//!
//! T009-5.4: Guided UI flow for power meter zero-offset calibration.
//! Supports both manual and automatic calibration commands.
//!
//! This dialog provides a step-by-step calibration wizard that guides
//! users through the calibration process with clear instructions,
//! progress feedback, and result display.

use egui::{Align, Color32, Layout, RichText, Sense, Stroke, Ui, Vec2};

use crate::sensors::calibration::{
    CalibrationProcess, CalibrationResult, CalibrationStep, CalibrationType,
};
use crate::sensors::types::{Protocol, SensorProtocol, SensorType};

/// State for the calibration dialog.
#[derive(Debug, Clone)]
pub struct CalibrationDialogState {
    /// Whether the dialog is visible.
    pub visible: bool,
    /// The current calibration process (if any).
    pub process: Option<CalibrationProcess>,
    /// User notes input buffer.
    pub notes_buffer: String,
    /// Selected calibration type for new calibration.
    pub selected_type: CalibrationType,
    /// Whether to show the calibration type selection.
    pub show_type_selection: bool,
}

impl Default for CalibrationDialogState {
    fn default() -> Self {
        Self::new()
    }
}

impl CalibrationDialogState {
    /// Create a new dialog state.
    pub fn new() -> Self {
        Self {
            visible: false,
            process: None,
            notes_buffer: String::new(),
            selected_type: CalibrationType::ZeroOffset,
            show_type_selection: false,
        }
    }

    /// Open the dialog to start a new calibration.
    pub fn open(
        &mut self,
        device_id: String,
        device_name: String,
        protocol: Protocol,
    ) {
        self.open_with_type(device_id, device_name, protocol, CalibrationType::ZeroOffset);
    }

    /// Open the dialog with a specific calibration type.
    pub fn open_with_type(
        &mut self,
        device_id: String,
        device_name: String,
        protocol: Protocol,
        calibration_type: CalibrationType,
    ) {
        self.process = Some(CalibrationProcess::new(
            device_id,
            device_name,
            protocol,
            calibration_type,
        ));
        self.selected_type = calibration_type;
        self.notes_buffer.clear();
        self.show_type_selection = false;
        self.visible = true;
    }

    /// Open the dialog with calibration type selection.
    pub fn open_with_selection(
        &mut self,
        device_id: String,
        device_name: String,
        protocol: Protocol,
    ) {
        self.process = Some(CalibrationProcess::new(
            device_id,
            device_name,
            protocol,
            CalibrationType::ZeroOffset,
        ));
        self.selected_type = CalibrationType::ZeroOffset;
        self.notes_buffer.clear();
        self.show_type_selection = true;
        self.visible = true;
    }

    /// Close the dialog.
    pub fn close(&mut self) {
        self.visible = false;
        self.process = None;
        self.notes_buffer.clear();
        self.show_type_selection = false;
    }

    /// Get the current process.
    pub fn current_process(&self) -> Option<&CalibrationProcess> {
        self.process.as_ref()
    }

    /// Get the current process mutably.
    pub fn current_process_mut(&mut self) -> Option<&mut CalibrationProcess> {
        self.process.as_mut()
    }

    /// Check if a calibration is in progress.
    pub fn is_calibrating(&self) -> bool {
        self.process.as_ref().map_or(false, |p| p.is_in_progress())
    }

    /// Check if the calibration is finished.
    pub fn is_finished(&self) -> bool {
        self.process.as_ref().map_or(false, |p| p.is_finished())
    }

    /// Advance to the next step.
    pub fn advance_step(&mut self) {
        if let Some(process) = &mut self.process {
            process.advance_step();
        }
    }

    /// Mark calibration as completed.
    pub fn complete(&mut self, offset_value: Option<i32>) {
        if let Some(process) = &mut self.process {
            process.complete(offset_value);
        }
    }

    /// Mark calibration as failed.
    pub fn fail(&mut self, error_message: String) {
        if let Some(process) = &mut self.process {
            process.fail(error_message);
        }
    }

    /// Get the user notes (if any).
    pub fn get_notes(&self) -> Option<String> {
        if self.notes_buffer.trim().is_empty() {
            None
        } else {
            Some(self.notes_buffer.clone())
        }
    }

    /// Change the calibration type (restarts the process).
    pub fn change_calibration_type(&mut self, calibration_type: CalibrationType) {
        if let Some(process) = &self.process {
            let device_id = process.device_id.clone();
            let device_name = process.device_name.clone();
            let protocol = process.protocol;

            self.process = Some(CalibrationProcess::new(
                device_id,
                device_name,
                protocol,
                calibration_type,
            ));
        }
        self.selected_type = calibration_type;
    }
}

/// Action from the calibration dialog.
#[derive(Debug, Clone, PartialEq)]
pub enum CalibrationDialogAction {
    /// User wants to start calibration (send command to power meter).
    StartCalibration {
        /// Device ID to calibrate.
        device_id: String,
        /// Type of calibration to perform.
        calibration_type: CalibrationType,
    },
    /// User cancelled the calibration.
    Cancel,
    /// User acknowledged the result and wants to close.
    Close {
        /// Whether to record the calibration in history.
        record_calibration: bool,
        /// User notes (if any).
        notes: Option<String>,
    },
    /// Retry a failed calibration.
    Retry {
        /// Device ID to retry.
        device_id: String,
        /// Type of calibration to retry.
        calibration_type: CalibrationType,
    },
    /// No action (dialog still open).
    None,
}

/// Response from showing the calibration dialog.
#[derive(Debug)]
pub struct CalibrationDialogResponse {
    /// The action taken by the user.
    pub action: CalibrationDialogAction,
}

/// Power meter calibration dialog.
///
/// Provides a step-by-step wizard for calibrating power meters with:
/// - Clear instructions for each calibration type
/// - Progress indicator during calibration
/// - Result display with success/failure feedback
/// - Optional notes field for recording conditions
pub struct CalibrationDialog<'a> {
    state: &'a mut CalibrationDialogState,
}

impl<'a> CalibrationDialog<'a> {
    /// Create a new calibration dialog.
    pub fn new(state: &'a mut CalibrationDialogState) -> Self {
        Self { state }
    }

    /// Show the dialog and return any action taken.
    pub fn show(&mut self, ui: &mut Ui) -> CalibrationDialogResponse {
        let mut action = CalibrationDialogAction::None;

        if !self.state.visible {
            return CalibrationDialogResponse { action };
        }

        let process = match &self.state.process {
            Some(p) => p.clone(),
            None => return CalibrationDialogResponse { action },
        };

        let window_title = if self.state.show_type_selection {
            "Calibrate Power Meter"
        } else {
            &process.instructions.title
        };

        egui::Window::new(window_title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_min_size(Vec2::new(450.0, 350.0));

                ui.vertical(|ui| {
                    // Header with device info
                    self.render_header(ui, &process);

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(12.0);

                    // Calibration type selection (if enabled)
                    if self.state.show_type_selection && process.current_step == CalibrationStep::Instructions {
                        action = self.render_type_selection(ui);
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);
                    }

                    // Main content based on current step
                    let step_action = match process.current_step {
                        CalibrationStep::Instructions => self.render_instructions(ui, &process),
                        CalibrationStep::Preparing => self.render_preparing(ui, &process),
                        CalibrationStep::SendingCommand => self.render_sending_command(ui, &process),
                        CalibrationStep::WaitingForResult => self.render_waiting(ui, &process),
                        CalibrationStep::Completed => self.render_completed(ui, &process),
                        CalibrationStep::Failed => self.render_failed(ui, &process),
                    };

                    if step_action != CalibrationDialogAction::None {
                        action = step_action;
                    }
                });
            });

        CalibrationDialogResponse { action }
    }

    /// Render the dialog header with device info.
    fn render_header(&self, ui: &mut Ui, process: &CalibrationProcess) {
        ui.horizontal(|ui| {
            // Power meter icon
            ui.label(RichText::new("⚡").size(32.0));

            ui.add_space(12.0);

            ui.vertical(|ui| {
                // Device name
                ui.label(RichText::new(&process.device_name).size(18.0).strong());

                // Protocol badge and calibration type
                ui.horizontal(|ui| {
                    ui.label(protocol_badge(process.protocol));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("{}", process.calibration_type))
                            .weak()
                            .small(),
                    );
                });
            });

            // Progress indicator on the right
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let progress = process.progress_percent();
                if progress > 0.0 && progress < 100.0 {
                    ui.label(RichText::new(format!("{:.0}%", progress)).weak());
                }
            });
        });
    }

    /// Render calibration type selection.
    fn render_type_selection(&mut self, ui: &mut Ui) -> CalibrationDialogAction {
        ui.label(RichText::new("Select calibration type:").strong());
        ui.add_space(8.0);

        let types = [
            (CalibrationType::ZeroOffset, "Zero Offset", "Standard calibration - resets the baseline torque reading", true),
            (CalibrationType::ManualCalibration, "Manual", "Advanced calibration with known weight (if supported)", false),
            (CalibrationType::AutomaticCalibration, "Automatic", "Let the power meter calibrate automatically", false),
        ];

        for (cal_type, name, description, recommended) in types {
            let is_selected = self.state.selected_type == cal_type;

            let (bg_color, border_color) = if is_selected {
                (
                    Color32::from_rgba_unmultiplied(66, 133, 244, 40),
                    Color32::from_rgb(66, 133, 244),
                )
            } else {
                (
                    ui.visuals().faint_bg_color,
                    ui.visuals().widgets.noninteractive.bg_stroke.color,
                )
            };

            let frame = egui::Frame::new()
                .fill(bg_color)
                .stroke(Stroke::new(if is_selected { 2.0 } else { 1.0 }, border_color))
                .inner_margin(10.0)
                .corner_radius(6.0);

            let response = frame.show(ui, |ui| {
                ui.set_min_width(ui.available_width() - 4.0);

                ui.horizontal(|ui| {
                    // Selection indicator
                    if is_selected {
                        ui.label(
                            RichText::new("●")
                                .color(Color32::from_rgb(66, 133, 244)),
                        );
                    } else {
                        ui.label(RichText::new("○").weak());
                    }

                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(name).strong());
                            if recommended {
                                ui.label(
                                    RichText::new("(Recommended)")
                                        .small()
                                        .color(Color32::from_rgb(102, 187, 106)),
                                );
                            }
                        });
                        ui.label(RichText::new(description).weak().small());
                    });
                });
            });

            let rect = response.response.rect;
            let response = ui.interact(rect, ui.id().with(format!("cal_type_{:?}", cal_type)), Sense::click());

            if response.clicked() {
                self.state.change_calibration_type(cal_type);
            }

            ui.add_space(4.0);
        }

        CalibrationDialogAction::None
    }

    /// Render the instructions step.
    fn render_instructions(&mut self, ui: &mut Ui, process: &CalibrationProcess) -> CalibrationDialogAction {
        let mut action = CalibrationDialogAction::None;

        // Step list with icons
        ui.label(RichText::new("Follow these steps:").strong());
        ui.add_space(8.0);

        for (i, step) in process.instructions.steps.iter().enumerate() {
            ui.horizontal(|ui| {
                let step_num = format!("{}.", i + 1);
                ui.label(RichText::new(step_num).strong().color(Color32::from_rgb(66, 133, 244)));
                ui.add_space(4.0);
                ui.label(step);
            });
            ui.add_space(4.0);
        }

        // Warnings
        if !process.instructions.warnings.is_empty() {
            ui.add_space(8.0);

            let warning_bg = Color32::from_rgba_unmultiplied(251, 188, 4, 30);
            let warning_border = Color32::from_rgb(251, 188, 4);

            egui::Frame::new()
                .fill(warning_bg)
                .stroke(Stroke::new(1.0, warning_border))
                .inner_margin(10.0)
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚠").color(warning_border));
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            for warning in &process.instructions.warnings {
                                ui.label(RichText::new(warning).small());
                            }
                        });
                    });
                });
        }

        // Estimated time
        ui.add_space(12.0);
        ui.label(
            RichText::new(format!(
                "Estimated time: ~{} seconds",
                process.instructions.estimated_time_secs
            ))
            .weak()
            .small(),
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(12.0);

        // Action buttons
        ui.horizontal(|ui| {
            if ui.button("Cancel").clicked() {
                action = CalibrationDialogAction::Cancel;
                self.state.close();
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let button = egui::Button::new(
                    RichText::new("Start Calibration").color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(66, 133, 244));

                if ui.add(button).clicked() {
                    if let Some(p) = &self.state.process {
                        action = CalibrationDialogAction::StartCalibration {
                            device_id: p.device_id.clone(),
                            calibration_type: p.calibration_type,
                        };
                    }
                    self.state.advance_step();
                }
            });
        });

        action
    }

    /// Render the preparing step.
    fn render_preparing(&mut self, ui: &mut Ui, _process: &CalibrationProcess) -> CalibrationDialogAction {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);

            // Preparing message
            ui.label(RichText::new("Preparing...").size(18.0));
            ui.add_space(8.0);
            ui.label(RichText::new("Please ensure the cranks are stationary.").weak());

            ui.add_space(20.0);

            // Spinner
            ui.spinner();
        });

        // Auto-advance after a brief delay (simulated by advancing immediately)
        self.state.advance_step();

        CalibrationDialogAction::None
    }

    /// Render the sending command step.
    fn render_sending_command(&mut self, ui: &mut Ui, process: &CalibrationProcess) -> CalibrationDialogAction {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);

            ui.label(RichText::new("Sending calibration command...").size(18.0));
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("Sending {} command to {}",
                    process.calibration_type,
                    process.device_name
                ))
                .weak(),
            );

            ui.add_space(20.0);

            ui.spinner();
        });

        CalibrationDialogAction::None
    }

    /// Render the waiting for result step.
    fn render_waiting(&mut self, ui: &mut Ui, process: &CalibrationProcess) -> CalibrationDialogAction {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);

            ui.label(RichText::new("Calibrating...").size(18.0));
            ui.add_space(8.0);
            ui.label(RichText::new("Please keep the cranks completely still.").weak());

            ui.add_space(20.0);

            // Progress bar
            let progress = process.progress_percent() / 100.0;
            let bar_width = 300.0;
            let bar_height = 8.0;

            let (rect, _response) = ui.allocate_exact_size(
                Vec2::new(bar_width, bar_height),
                Sense::hover(),
            );

            if ui.is_rect_visible(rect) {
                let painter = ui.painter();

                // Background
                painter.rect_filled(
                    rect,
                    4.0,
                    Color32::from_gray(60),
                );

                // Progress fill
                let fill_width = bar_width * progress;
                if fill_width > 0.0 {
                    let fill_rect = egui::Rect::from_min_size(
                        rect.min,
                        Vec2::new(fill_width, bar_height),
                    );
                    painter.rect_filled(
                        fill_rect,
                        4.0,
                        Color32::from_rgb(66, 133, 244),
                    );
                }
            }

            ui.add_space(12.0);

            let elapsed = process.elapsed_secs();
            ui.label(RichText::new(format!("Elapsed: {:.1}s", elapsed)).weak().small());

            ui.add_space(20.0);

            ui.spinner();
        });

        CalibrationDialogAction::None
    }

    /// Render the completed step.
    fn render_completed(&mut self, ui: &mut Ui, process: &CalibrationProcess) -> CalibrationDialogAction {
        let mut action = CalibrationDialogAction::None;

        let result = match &process.result {
            Some(r) => r.clone(),
            None => return action,
        };

        ui.vertical_centered(|ui| {
            ui.add_space(8.0);

            // Success icon
            ui.label(RichText::new("✓").size(48.0).color(Color32::from_rgb(52, 168, 83)));

            ui.add_space(8.0);

            ui.label(RichText::new("Calibration Successful!").size(20.0).strong());

            ui.add_space(8.0);

            // Offset value if available
            if let Some(offset) = result.offset_value {
                ui.horizontal(|ui| {
                    ui.label("Offset value:");
                    ui.label(RichText::new(format!("{}", offset)).strong());
                });
            }

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Completed in {:.1}s", result.duration_secs)).weak().small());
            });
        });

        ui.add_space(16.0);

        // Notes section
        ui.horizontal(|ui| {
            ui.label("Add notes (optional):");
        });
        ui.add_space(4.0);

        ui.add(
            egui::TextEdit::multiline(&mut self.state.notes_buffer)
                .hint_text("e.g., Cold garage, pre-ride...")
                .desired_width(ui.available_width())
                .desired_rows(2),
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(12.0);

        // Action buttons
        ui.horizontal(|ui| {
            if ui.button("Close without saving").clicked() {
                action = CalibrationDialogAction::Close {
                    record_calibration: false,
                    notes: None,
                };
                self.state.close();
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let button = egui::Button::new(
                    RichText::new("Save & Close").color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(52, 168, 83));

                if ui.add(button).clicked() {
                    action = CalibrationDialogAction::Close {
                        record_calibration: true,
                        notes: self.state.get_notes(),
                    };
                    self.state.close();
                }
            });
        });

        action
    }

    /// Render the failed step.
    fn render_failed(&mut self, ui: &mut Ui, process: &CalibrationProcess) -> CalibrationDialogAction {
        let mut action = CalibrationDialogAction::None;

        let result = match &process.result {
            Some(r) => r.clone(),
            None => return action,
        };

        ui.vertical_centered(|ui| {
            ui.add_space(8.0);

            // Failure icon
            ui.label(RichText::new("✗").size(48.0).color(Color32::from_rgb(234, 67, 53)));

            ui.add_space(8.0);

            ui.label(RichText::new("Calibration Failed").size(20.0).strong());

            ui.add_space(8.0);

            // Error message
            if let Some(error) = &result.error_message {
                let error_bg = Color32::from_rgba_unmultiplied(234, 67, 53, 30);
                let error_border = Color32::from_rgb(234, 67, 53);

                egui::Frame::new()
                    .fill(error_bg)
                    .stroke(Stroke::new(1.0, error_border))
                    .inner_margin(10.0)
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(error).color(error_border));
                    });
            }

            ui.add_space(12.0);

            // Troubleshooting tips
            ui.label(RichText::new("Troubleshooting:").strong());
            ui.add_space(4.0);

            let tips = [
                "Ensure the cranks are completely stationary",
                "Check that the power meter is properly connected",
                "Try moving the bike to a flat surface",
                "Wait a few seconds and try again",
            ];

            for tip in tips {
                ui.horizontal(|ui| {
                    ui.label("•");
                    ui.label(RichText::new(tip).weak().small());
                });
            }
        });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(12.0);

        // Action buttons
        ui.horizontal(|ui| {
            if ui.button("Close").clicked() {
                action = CalibrationDialogAction::Close {
                    record_calibration: false,
                    notes: None,
                };
                self.state.close();
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let button = egui::Button::new(
                    RichText::new("Retry Calibration").color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(66, 133, 244));

                if ui.add(button).clicked() {
                    if let Some(p) = &self.state.process {
                        action = CalibrationDialogAction::Retry {
                            device_id: p.device_id.clone(),
                            calibration_type: p.calibration_type,
                        };
                        // Reset process for retry
                        self.state.change_calibration_type(p.calibration_type);
                    }
                }
            });
        });

        action
    }
}

/// Get a protocol badge for display.
fn protocol_badge(protocol: Protocol) -> RichText {
    let sensor_protocol = protocol.sensor_protocol();
    match sensor_protocol {
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

/// Compact calibration reminder button.
///
/// Shows a button that indicates calibration is due or overdue.
pub struct CalibrationReminderButton {
    /// Device name.
    device_name: String,
    /// Days since last calibration (or None if never calibrated).
    days_since: Option<i64>,
    /// Whether calibration is overdue (2x the reminder period).
    is_overdue: bool,
}

impl CalibrationReminderButton {
    /// Create a new calibration reminder button.
    pub fn new(device_name: String, days_since: Option<i64>, is_overdue: bool) -> Self {
        Self {
            device_name,
            days_since,
            is_overdue,
        }
    }

    /// Create for a device that has never been calibrated.
    pub fn never_calibrated(device_name: String) -> Self {
        Self {
            device_name,
            days_since: None,
            is_overdue: false,
        }
    }

    /// Show the button and return true if clicked.
    pub fn show(&self, ui: &mut Ui) -> bool {
        let (label, color) = if self.days_since.is_none() {
            ("Calibrate".to_string(), Color32::from_rgb(66, 133, 244))
        } else if self.is_overdue {
            let days = self.days_since.unwrap_or(0);
            (format!("Overdue ({} days)", days), Color32::from_rgb(234, 67, 53))
        } else {
            let days = self.days_since.unwrap_or(0);
            (format!("Calibrate ({} days)", days), Color32::from_rgb(251, 188, 4))
        };

        let button = egui::Button::new(
            RichText::new(format!("⚡ {}", label)).color(color),
        )
        .fill(Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 30));

        ui.add(button).clicked()
    }
}

/// Inline calibration status indicator.
///
/// Shows a compact indicator of calibration status for sensor lists.
pub struct CalibrationStatusIndicator {
    /// Days since last calibration.
    days_since: Option<i64>,
    /// The configured reminder period.
    reminder_days: i64,
}

impl CalibrationStatusIndicator {
    /// Create a new calibration status indicator.
    pub fn new(days_since: Option<i64>, reminder_days: i64) -> Self {
        Self {
            days_since,
            reminder_days,
        }
    }

    /// Show the indicator.
    pub fn show(&self, ui: &mut Ui) {
        match self.days_since {
            Some(days) if days >= self.reminder_days * 2 => {
                // Overdue
                ui.label(
                    RichText::new("⚠ Overdue")
                        .small()
                        .color(Color32::from_rgb(234, 67, 53)),
                );
            }
            Some(days) if days >= self.reminder_days => {
                // Due
                ui.label(
                    RichText::new(format!("⚡ Due ({} days)", days))
                        .small()
                        .color(Color32::from_rgb(251, 188, 4)),
                );
            }
            Some(days) if days == 0 => {
                // Calibrated today
                ui.label(
                    RichText::new("✓ Calibrated today")
                        .small()
                        .color(Color32::from_rgb(52, 168, 83)),
                );
            }
            Some(days) => {
                // Recently calibrated
                ui.label(
                    RichText::new(format!("✓ {} days ago", days))
                        .small()
                        .color(Color32::from_rgb(102, 187, 106)),
                );
            }
            None => {
                // Never calibrated
                ui.label(
                    RichText::new("⚡ Never calibrated")
                        .small()
                        .color(Color32::from_rgb(66, 133, 244)),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_state_new() {
        let state = CalibrationDialogState::new();

        assert!(!state.visible);
        assert!(state.process.is_none());
        assert!(state.notes_buffer.is_empty());
        assert_eq!(state.selected_type, CalibrationType::ZeroOffset);
        assert!(!state.show_type_selection);
    }

    #[test]
    fn test_dialog_state_open() {
        let mut state = CalibrationDialogState::new();

        state.open(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );

        assert!(state.visible);
        assert!(state.process.is_some());

        let process = state.process.as_ref().unwrap();
        assert_eq!(process.device_id, "device1");
        assert_eq!(process.device_name, "Stages Power");
        assert_eq!(process.calibration_type, CalibrationType::ZeroOffset);
    }

    #[test]
    fn test_dialog_state_open_with_type() {
        let mut state = CalibrationDialogState::new();

        state.open_with_type(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
            CalibrationType::ManualCalibration,
        );

        assert!(state.visible);
        let process = state.process.as_ref().unwrap();
        assert_eq!(process.calibration_type, CalibrationType::ManualCalibration);
    }

    #[test]
    fn test_dialog_state_open_with_selection() {
        let mut state = CalibrationDialogState::new();

        state.open_with_selection(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );

        assert!(state.visible);
        assert!(state.show_type_selection);
    }

    #[test]
    fn test_dialog_state_close() {
        let mut state = CalibrationDialogState::new();

        state.open(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );

        state.notes_buffer = "Some notes".to_string();

        state.close();

        assert!(!state.visible);
        assert!(state.process.is_none());
        assert!(state.notes_buffer.is_empty());
        assert!(!state.show_type_selection);
    }

    #[test]
    fn test_dialog_state_is_calibrating() {
        let mut state = CalibrationDialogState::new();

        assert!(!state.is_calibrating());

        state.open(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );

        assert!(state.is_calibrating());

        state.complete(Some(100));

        assert!(!state.is_calibrating());
        assert!(state.is_finished());
    }

    #[test]
    fn test_dialog_state_advance_step() {
        let mut state = CalibrationDialogState::new();

        state.open(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );

        let process = state.current_process().unwrap();
        assert_eq!(process.current_step, CalibrationStep::Instructions);

        state.advance_step();

        let process = state.current_process().unwrap();
        assert_eq!(process.current_step, CalibrationStep::Preparing);
    }

    #[test]
    fn test_dialog_state_complete() {
        let mut state = CalibrationDialogState::new();

        state.open(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );

        state.complete(Some(42));

        let process = state.current_process().unwrap();
        assert_eq!(process.current_step, CalibrationStep::Completed);
        assert!(process.result.is_some());
        assert!(process.result.as_ref().unwrap().success);
        assert_eq!(process.result.as_ref().unwrap().offset_value, Some(42));
    }

    #[test]
    fn test_dialog_state_fail() {
        let mut state = CalibrationDialogState::new();

        state.open(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );

        state.fail("Timeout".to_string());

        let process = state.current_process().unwrap();
        assert_eq!(process.current_step, CalibrationStep::Failed);
        assert!(process.result.is_some());
        assert!(!process.result.as_ref().unwrap().success);
    }

    #[test]
    fn test_dialog_state_get_notes() {
        let mut state = CalibrationDialogState::new();

        assert!(state.get_notes().is_none());

        state.notes_buffer = "   ".to_string();
        assert!(state.get_notes().is_none()); // Empty after trim

        state.notes_buffer = "Cold garage".to_string();
        assert_eq!(state.get_notes(), Some("Cold garage".to_string()));
    }

    #[test]
    fn test_dialog_state_change_calibration_type() {
        let mut state = CalibrationDialogState::new();

        state.open(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );

        assert_eq!(state.selected_type, CalibrationType::ZeroOffset);

        state.change_calibration_type(CalibrationType::ManualCalibration);

        assert_eq!(state.selected_type, CalibrationType::ManualCalibration);

        let process = state.current_process().unwrap();
        assert_eq!(process.calibration_type, CalibrationType::ManualCalibration);
        assert_eq!(process.current_step, CalibrationStep::Instructions); // Reset
    }

    #[test]
    fn test_calibration_dialog_action_eq() {
        let action1 = CalibrationDialogAction::StartCalibration {
            device_id: "test".to_string(),
            calibration_type: CalibrationType::ZeroOffset,
        };
        let action2 = CalibrationDialogAction::StartCalibration {
            device_id: "test".to_string(),
            calibration_type: CalibrationType::ZeroOffset,
        };
        let action3 = CalibrationDialogAction::Cancel;

        assert_eq!(action1, action2);
        assert_ne!(action1, action3);
    }

    #[test]
    fn test_calibration_reminder_button() {
        let button = CalibrationReminderButton::new("Stages".to_string(), Some(7), false);
        assert_eq!(button.device_name, "Stages");
        assert_eq!(button.days_since, Some(7));
        assert!(!button.is_overdue);

        let button = CalibrationReminderButton::never_calibrated("Quarq".to_string());
        assert!(button.days_since.is_none());
    }

    #[test]
    fn test_calibration_status_indicator() {
        let indicator = CalibrationStatusIndicator::new(Some(0), 7);
        assert_eq!(indicator.days_since, Some(0));

        let indicator = CalibrationStatusIndicator::new(None, 7);
        assert!(indicator.days_since.is_none());
    }
}
