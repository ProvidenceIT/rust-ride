//! Rider profile screen.
//!
//! Displays and edits rider profile information, stats, and badges.
//! T044: Add XP/level display to user profile screen.
//! T019: Import confirmation dialog with conflict resolution.
//! T020: Progress indicators, success toasts, and error display for export/import.

use std::path::Path;
use std::time::Instant;

use egui::{Color32, RichText, Ui, Vec2};

use crate::achievements::XpStatus;
use crate::social::export::{
    ConflictResolution, ProfileConflict, ProfileExport, ProfileExportError, ProfileImportResult,
};
use crate::social::types::{Badge, RiderProfile};

/// T020: Status of a profile export/import operation.
#[derive(Debug, Clone)]
pub enum ProfileOperationStatus {
    /// No operation in progress.
    Idle,
    /// Export is in progress.
    Exporting,
    /// Import is in progress.
    Importing,
    /// Export completed successfully.
    ExportSuccess {
        /// Path where the profile was exported.
        path: String,
        /// When the success message was shown (for auto-dismiss).
        shown_at: Instant,
    },
    /// Import completed successfully.
    ImportSuccess {
        /// Summary of what was imported.
        summary: String,
        /// When the success message was shown (for auto-dismiss).
        shown_at: Instant,
    },
    /// Operation failed with an error.
    Error {
        /// Error message to display.
        message: String,
        /// When the error was shown (for auto-dismiss).
        shown_at: Instant,
    },
}

impl Default for ProfileOperationStatus {
    fn default() -> Self {
        Self::Idle
    }
}

/// T020: Duration to show success/error messages before auto-dismiss (5 seconds).
const STATUS_DISPLAY_DURATION_SECS: f32 = 5.0;

/// Rider profile screen actions.
#[derive(Debug, Clone)]
pub enum RiderProfileAction {
    /// Save profile changes.
    SaveProfile(RiderProfile),
    /// Change avatar.
    ChangeAvatar,
    /// Export profile to JSON file.
    ExportProfile,
    /// Import profile from JSON file (opens file picker).
    ImportProfile,
    /// Confirm import with conflict resolution strategy.
    ConfirmImport {
        /// The parsed profile export data.
        export: ProfileExport,
        /// The chosen conflict resolution strategy.
        resolution: ConflictResolution,
    },
    /// Cancel import operation.
    CancelImport,
    /// Navigate back.
    Back,
}

/// Rider profile view mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RiderProfileView {
    /// View mode.
    #[default]
    View,
    /// Edit mode.
    Edit,
}

/// T019: State for the import conflict resolution dialog.
#[derive(Debug, Clone)]
pub struct ImportConflictDialogState {
    /// The parsed profile export data to import.
    pub export: ProfileExport,
    /// Conflicts detected with the existing profile.
    pub conflicts: Vec<ProfileConflict>,
    /// Currently selected resolution strategy.
    pub selected_resolution: ConflictResolution,
}

/// Rider profile screen state.
pub struct RiderProfileScreen {
    /// Current view mode.
    view: RiderProfileView,
    /// Edited display name.
    edit_name: String,
    /// Edited bio.
    edit_bio: String,
    /// Edited sharing preference.
    edit_sharing_enabled: bool,
    /// T044: Current XP status
    xp_status: Option<XpStatus>,
    /// T019: Import conflict dialog state.
    import_conflict_dialog: Option<ImportConflictDialogState>,
    /// T020: Current export/import operation status.
    operation_status: ProfileOperationStatus,
}

impl Default for RiderProfileScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl RiderProfileScreen {
    /// Create a new rider profile screen.
    pub fn new() -> Self {
        Self {
            view: RiderProfileView::View,
            edit_name: String::new(),
            edit_bio: String::new(),
            edit_sharing_enabled: true,
            xp_status: None,
            import_conflict_dialog: None,
            operation_status: ProfileOperationStatus::Idle,
        }
    }

    /// T044: Set the current XP status for display.
    pub fn set_xp_status(&mut self, status: XpStatus) {
        self.xp_status = Some(status);
    }

    /// T019: Show the import conflict resolution dialog.
    ///
    /// Call this when conflicts are detected during profile import.
    /// The dialog will display the conflicts and allow the user to choose
    /// how to resolve them: Replace existing, Merge data, or Cancel.
    pub fn show_import_conflicts(
        &mut self,
        export: ProfileExport,
        conflicts: Vec<ProfileConflict>,
    ) {
        self.import_conflict_dialog = Some(ImportConflictDialogState {
            export,
            conflicts,
            selected_resolution: ConflictResolution::Merge, // Default to merge
        });
    }

    /// T019: Check if the import conflict dialog is currently open.
    pub fn has_import_dialog_open(&self) -> bool {
        self.import_conflict_dialog.is_some()
    }

    /// T019: Close the import conflict dialog.
    pub fn close_import_dialog(&mut self) {
        self.import_conflict_dialog = None;
    }

    /// T020: Mark that an export operation is starting.
    ///
    /// Shows a progress indicator while the export is in progress.
    pub fn start_export(&mut self) {
        self.operation_status = ProfileOperationStatus::Exporting;
    }

    /// T020: Mark that an import operation is starting.
    ///
    /// Shows a progress indicator while the import is in progress.
    pub fn start_import(&mut self) {
        self.operation_status = ProfileOperationStatus::Importing;
    }

    /// T020: Show export success message.
    ///
    /// Displays a success toast showing where the profile was exported.
    pub fn show_export_success(&mut self, path: &Path) {
        self.operation_status = ProfileOperationStatus::ExportSuccess {
            path: path.display().to_string(),
            shown_at: Instant::now(),
        };
    }

    /// T020: Show import success message with summary.
    ///
    /// Displays a success toast with details about what was imported
    /// (e.g., "Imported 5 FTP records").
    pub fn show_import_success(&mut self, result: &ProfileImportResult) {
        let summary = format_import_summary(result);
        self.operation_status = ProfileOperationStatus::ImportSuccess {
            summary,
            shown_at: Instant::now(),
        };
    }

    /// T020: Show an error message.
    ///
    /// Displays error details to help the user understand what went wrong.
    pub fn show_error(&mut self, error: &ProfileExportError) {
        self.operation_status = ProfileOperationStatus::Error {
            message: format_export_error(error),
            shown_at: Instant::now(),
        };
    }

    /// T020: Show a custom error message.
    pub fn show_error_message(&mut self, message: String) {
        self.operation_status = ProfileOperationStatus::Error {
            message,
            shown_at: Instant::now(),
        };
    }

    /// T020: Clear any operation status.
    pub fn clear_status(&mut self) {
        self.operation_status = ProfileOperationStatus::Idle;
    }

    /// T020: Check if an operation is in progress.
    pub fn is_operation_in_progress(&self) -> bool {
        matches!(
            self.operation_status,
            ProfileOperationStatus::Exporting | ProfileOperationStatus::Importing
        )
    }

    /// Start editing with current profile values.
    fn start_editing(&mut self, profile: &RiderProfile) {
        self.edit_name = profile.display_name.clone();
        self.edit_bio = profile.bio.clone().unwrap_or_default();
        self.edit_sharing_enabled = profile.sharing_enabled;
        self.view = RiderProfileView::Edit;
    }

    /// Render the rider profile screen.
    #[allow(unused_assignments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        profile: &RiderProfile,
        badges: &[Badge],
    ) -> Option<RiderProfileAction> {
        let mut action = None;

        ui.heading("My Profile");
        ui.add_space(10.0);

        match self.view {
            RiderProfileView::View => {
                action = self.show_view_mode(ui, profile, badges);
            }
            RiderProfileView::Edit => {
                action = self.show_edit_mode(ui, profile);
            }
        }

        ui.add_space(20.0);

        // Export/Import section (only in view mode)
        if self.view == RiderProfileView::View {
            let is_busy = self.is_operation_in_progress();

            ui.horizontal(|ui| {
                // Disable buttons during operations
                ui.add_enabled_ui(!is_busy, |ui| {
                    if ui.button("Export Profile").clicked() {
                        action = Some(RiderProfileAction::ExportProfile);
                    }
                    if ui.button("Import Profile").clicked() {
                        action = Some(RiderProfileAction::ImportProfile);
                    }
                });
            });

            // T020: Render operation status (progress/success/error)
            ui.add_space(8.0);
            self.render_operation_status(ui);

            ui.add_space(10.0);
        }

        if ui.button("Back").clicked() {
            if self.view == RiderProfileView::Edit {
                self.view = RiderProfileView::View;
            } else {
                action = Some(RiderProfileAction::Back);
            }
        }

        // T019: Render import conflict dialog if open
        if let Some(dialog_action) = self.render_import_conflict_dialog(ui) {
            action = Some(dialog_action);
        }

        action
    }

    /// Show view mode.
    fn show_view_mode(
        &mut self,
        ui: &mut Ui,
        profile: &RiderProfile,
        badges: &[Badge],
    ) -> Option<RiderProfileAction> {
        let mut action = None;

        ui.horizontal(|ui| {
            // Avatar
            let avatar_size = 80.0;
            let (rect, response) =
                ui.allocate_exact_size(Vec2::splat(avatar_size), egui::Sense::click());
            ui.painter().circle_filled(
                rect.center(),
                avatar_size / 2.0,
                Color32::from_rgb(80, 100, 120),
            );
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                profile
                    .display_name
                    .chars()
                    .next()
                    .unwrap_or('?')
                    .to_string(),
                egui::FontId::proportional(36.0),
                Color32::WHITE,
            );

            if response.clicked() {
                action = Some(RiderProfileAction::ChangeAvatar);
            }

            ui.add_space(20.0);

            ui.vertical(|ui| {
                ui.label(RichText::new(&profile.display_name).size(24.0).strong());

                if let Some(ref bio) = profile.bio {
                    ui.label(bio);
                }

                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    if profile.sharing_enabled {
                        ui.label(RichText::new("Sharing: On").color(Color32::GREEN));
                    } else {
                        ui.label(RichText::new("Sharing: Off").color(Color32::GRAY));
                    }
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                if ui.button("Edit").clicked() {
                    self.start_editing(profile);
                }
            });
        });

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(15.0);

        // Stats section
        ui.label(RichText::new("Statistics").strong().size(18.0));
        ui.add_space(10.0);

        egui::Grid::new("profile_stats")
            .num_columns(4)
            .spacing([40.0, 10.0])
            .show(ui, |ui| {
                // Row 1
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("{:.0}", profile.total_distance_km))
                            .size(24.0)
                            .strong(),
                    );
                    ui.label("km ridden");
                });
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("{:.1}", profile.total_time_hours))
                            .size(24.0)
                            .strong(),
                    );
                    ui.label("hours");
                });
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("{}", profile.total_rides))
                            .size(24.0)
                            .strong(),
                    );
                    ui.label("rides");
                });
                ui.vertical(|ui| {
                    if let Some(ftp) = profile.ftp {
                        ui.label(RichText::new(format!("{}", ftp)).size(24.0).strong());
                    } else {
                        ui.label(RichText::new("--").size(24.0));
                    }
                    ui.label("FTP (W)");
                });
                ui.end_row();
            });

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(15.0);

        // T044: Career Level & XP section
        self.show_xp_section(ui);

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(15.0);

        // Badges section
        ui.label(RichText::new("Badges").strong().size(18.0));
        ui.add_space(10.0);

        if badges.is_empty() {
            ui.label(RichText::new("No badges earned yet").italics());
            ui.label("Complete challenges and reach milestones to earn badges!");
        } else {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for badge in badges {
                        self.show_badge(ui, badge);
                    }
                });
            });
        }

        action
    }

    /// Show edit mode.
    fn show_edit_mode(
        &mut self,
        ui: &mut Ui,
        profile: &RiderProfile,
    ) -> Option<RiderProfileAction> {
        let mut action = None;

        ui.label(RichText::new("Edit Profile").strong().size(18.0));
        ui.add_space(15.0);

        egui::Grid::new("profile_edit_form")
            .num_columns(2)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                ui.label("Display Name:");
                ui.text_edit_singleline(&mut self.edit_name);
                ui.end_row();

                ui.label("Bio:");
                ui.text_edit_multiline(&mut self.edit_bio);
                ui.end_row();

                ui.label("Share Activities:");
                ui.checkbox(
                    &mut self.edit_sharing_enabled,
                    "Allow others to see my rides",
                );
                ui.end_row();
            });

        ui.add_space(20.0);

        ui.horizontal(|ui| {
            if ui.button("Save").clicked() && !self.edit_name.is_empty() {
                let mut updated = profile.clone();
                updated.display_name = self.edit_name.clone();
                updated.bio = if self.edit_bio.is_empty() {
                    None
                } else {
                    Some(self.edit_bio.clone())
                };
                updated.sharing_enabled = self.edit_sharing_enabled;

                action = Some(RiderProfileAction::SaveProfile(updated));
                self.view = RiderProfileView::View;
            }
            if ui.button("Cancel").clicked() {
                self.view = RiderProfileView::View;
            }
        });

        action
    }

    /// T044: Show the XP and career level section.
    fn show_xp_section(&self, ui: &mut Ui) {
        ui.label(RichText::new("Career Level").strong().size(18.0));
        ui.add_space(10.0);

        if let Some(ref status) = self.xp_status {
            // Level display with circular progress indicator
            ui.horizontal(|ui| {
                // Level circle
                let level_size = 80.0;
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::splat(level_size), egui::Sense::hover());

                // Draw outer circle (progress ring background)
                ui.painter().circle_stroke(
                    rect.center(),
                    level_size / 2.0 - 4.0,
                    egui::Stroke::new(6.0, Color32::from_rgb(60, 60, 70)),
                );

                // Draw progress arc
                if !status.is_max_level() {
                    let progress = status.level_progress;
                    let start_angle = -std::f32::consts::FRAC_PI_2; // Start from top
                    let end_angle = start_angle + progress * std::f32::consts::TAU;

                    // Draw progress arc using small line segments
                    let center = rect.center();
                    let radius = level_size / 2.0 - 4.0;
                    let segments = 32;
                    let angle_step = (end_angle - start_angle) / segments as f32;

                    for i in 0..segments {
                        let a1 = start_angle + i as f32 * angle_step;
                        let a2 = start_angle + (i + 1) as f32 * angle_step;
                        let p1 = center + Vec2::new(a1.cos(), a1.sin()) * radius;
                        let p2 = center + Vec2::new(a2.cos(), a2.sin()) * radius;
                        ui.painter().line_segment(
                            [p1, p2],
                            egui::Stroke::new(6.0, Color32::from_rgb(76, 175, 80)),
                        );
                    }
                }

                // Inner circle
                ui.painter().circle_filled(
                    rect.center(),
                    level_size / 2.0 - 10.0,
                    Color32::from_rgb(40, 45, 55),
                );

                // Level number
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", status.level),
                    egui::FontId::proportional(32.0),
                    Color32::WHITE,
                );

                ui.add_space(20.0);

                // XP details
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("Level {}", status.level))
                            .size(24.0)
                            .strong(),
                    );

                    ui.add_space(5.0);

                    // XP progress bar
                    let xp_progress = status.level_progress;
                    let progress_bar = egui::ProgressBar::new(xp_progress)
                        .fill(Color32::from_rgb(76, 175, 80))
                        .desired_width(200.0);
                    ui.add(progress_bar);

                    ui.add_space(5.0);

                    // XP text
                    if status.is_max_level() {
                        ui.label(
                            RichText::new(format!("{} XP (Max Level)", status.total_xp))
                                .color(Color32::GOLD),
                        );
                    } else {
                        let xp_remaining = status.xp_for_next.saturating_sub(status.xp_into_level);
                        ui.label(format!(
                            "{} / {} XP to next level",
                            status.xp_into_level, status.xp_for_next
                        ));
                        ui.label(
                            RichText::new(format!("{} XP remaining", xp_remaining))
                                .weak()
                                .small(),
                        );
                    }

                    ui.add_space(5.0);

                    // Total XP
                    ui.label(
                        RichText::new(format!("Total: {} XP", status.total_xp))
                            .weak()
                            .small(),
                    );
                });
            });
        } else {
            // No XP data yet
            ui.label(RichText::new("Level 1").size(24.0).strong());
            ui.add_space(5.0);
            ui.label("Complete rides and earn achievements to gain XP!");
        }
    }

    /// Show a badge.
    fn show_badge(&self, ui: &mut Ui, badge: &Badge) {
        let size = 60.0;

        egui::Frame::new()
            .fill(Color32::from_rgb(50, 50, 60))
            .inner_margin(8.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Badge icon
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());

                    let color = if badge.earned {
                        Color32::GOLD
                    } else {
                        Color32::from_rgb(60, 60, 60)
                    };

                    ui.painter()
                        .circle_filled(rect.center(), size / 2.0 - 5.0, color);

                    // Badge icon text
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &badge.icon,
                        egui::FontId::proportional(24.0),
                        if badge.earned {
                            Color32::BLACK
                        } else {
                            Color32::GRAY
                        },
                    );

                    // Badge name
                    let name = if badge.name.len() > 10 {
                        format!("{}...", &badge.name[..10])
                    } else {
                        badge.name.clone()
                    };
                    ui.label(RichText::new(&name).small());

                    // Progress if not earned
                    if !badge.earned {
                        let progress = badge.progress / badge.target;
                        ui.add(egui::ProgressBar::new(progress as f32).desired_width(size));
                    }
                });
            });
    }

    /// T019: Render the import conflict resolution dialog.
    ///
    /// Displays detected conflicts and offers options: Replace existing, Merge data, Cancel.
    /// Returns the chosen action when user confirms or cancels.
    fn render_import_conflict_dialog(&mut self, ui: &mut Ui) -> Option<RiderProfileAction> {
        let mut action = None;

        // Clone dialog state to avoid borrow conflicts during rendering
        if let Some(state) = self.import_conflict_dialog.clone() {
            egui::Window::new("Import Profile Conflicts")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.set_min_width(400.0);

                    // Header description
                    ui.label("The imported profile conflicts with your existing data.");
                    ui.label(
                        RichText::new("Choose how to resolve these conflicts:")
                            .weak()
                            .size(13.0),
                    );
                    ui.add_space(12.0);

                    // Display conflicts section
                    ui.label(RichText::new("What will be changed:").strong());
                    ui.add_space(6.0);

                    egui::Frame::new()
                        .fill(Color32::from_rgb(40, 40, 50))
                        .inner_margin(10.0)
                        .corner_radius(4.0)
                        .show(ui, |ui| {
                            self.render_conflict_details(ui, &state.conflicts, &state.export);
                        });

                    ui.add_space(16.0);

                    // Resolution options
                    ui.label(RichText::new("Resolution Strategy:").strong());
                    ui.add_space(8.0);

                    // Get mutable reference to dialog state for radio buttons
                    if let Some(ref mut dialog) = self.import_conflict_dialog {
                        ui.vertical(|ui| {
                            ui.radio_value(
                                &mut dialog.selected_resolution,
                                ConflictResolution::Merge,
                                "Merge data — Combine FTP history, keep existing profile",
                            );
                            ui.add_space(4.0);
                            ui.radio_value(
                                &mut dialog.selected_resolution,
                                ConflictResolution::Replace,
                                "Replace existing — Overwrite with imported data",
                            );
                        });
                    }

                    ui.add_space(16.0);

                    // Action buttons
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            action = Some(RiderProfileAction::CancelImport);
                            self.import_conflict_dialog = None;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let import_btn = ui.add(
                                egui::Button::new("Import")
                                    .fill(Color32::from_rgb(66, 133, 244)),
                            );
                            if import_btn.clicked() {
                                if let Some(dialog) = self.import_conflict_dialog.take() {
                                    action = Some(RiderProfileAction::ConfirmImport {
                                        export: dialog.export,
                                        resolution: dialog.selected_resolution,
                                    });
                                }
                            }
                        });
                    });
                });
        }

        action
    }

    /// T019: Render details about detected conflicts.
    fn render_conflict_details(
        &self,
        ui: &mut Ui,
        conflicts: &[ProfileConflict],
        export: &ProfileExport,
    ) {
        if conflicts.is_empty() {
            ui.label(
                RichText::new("No conflicts detected. Profile can be imported directly.")
                    .color(Color32::from_rgb(76, 175, 80)),
            );
            return;
        }

        for conflict in conflicts {
            match conflict {
                ProfileConflict::ExistingProfile {
                    existing_name,
                    rider_id: _,
                } => {
                    ui.horizontal(|ui| {
                        ui.label("•");
                        ui.label(format!(
                            "Profile exists: \"{}\" → \"{}\"",
                            existing_name, export.profile.display_name
                        ));
                    });
                }
                ProfileConflict::DisplayNameMismatch {
                    imported_name,
                    existing_name,
                } => {
                    ui.horizontal(|ui| {
                        ui.label("•");
                        ui.label(format!(
                            "Name change: \"{}\" → \"{}\"",
                            existing_name, imported_name
                        ));
                    });
                }
                ProfileConflict::FtpMismatch {
                    imported_ftp,
                    existing_ftp,
                } => {
                    ui.horizontal(|ui| {
                        ui.label("•");
                        let existing = existing_ftp
                            .map(|f| format!("{}W", f))
                            .unwrap_or_else(|| "None".to_string());
                        let imported = imported_ftp
                            .map(|f| format!("{}W", f))
                            .unwrap_or_else(|| "None".to_string());
                        ui.label(format!("FTP change: {} → {}", existing, imported));
                    });
                }
                ProfileConflict::AvatarMismatch {
                    import_has_avatar,
                    existing_has_avatar,
                } => {
                    ui.horizontal(|ui| {
                        ui.label("•");
                        let msg = match (existing_has_avatar, import_has_avatar) {
                            (true, true) => "Avatar will be replaced",
                            (true, false) => "Avatar will be removed",
                            (false, true) => "Avatar will be added",
                            (false, false) => "No avatar changes",
                        };
                        ui.label(msg);
                    });
                }
            }
        }

        // Show FTP history info if any
        if !export.ftp_history.is_empty() {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("•");
                ui.label(format!(
                    "{} FTP history entries will be imported",
                    export.ftp_history.len()
                ));
            });
        }
    }

    /// T020: Render the operation status (progress, success, or error).
    ///
    /// Shows progress indicator during operations, success toast with summary,
    /// or error details on failure. Messages auto-dismiss after 5 seconds.
    fn render_operation_status(&mut self, ui: &mut Ui) {
        // Check for auto-dismiss based on time
        let should_dismiss = match &self.operation_status {
            ProfileOperationStatus::ExportSuccess { shown_at, .. }
            | ProfileOperationStatus::ImportSuccess { shown_at, .. }
            | ProfileOperationStatus::Error { shown_at, .. } => {
                shown_at.elapsed().as_secs_f32() > STATUS_DISPLAY_DURATION_SECS
            }
            _ => false,
        };

        if should_dismiss {
            self.operation_status = ProfileOperationStatus::Idle;
            return;
        }

        match &self.operation_status {
            ProfileOperationStatus::Idle => {}

            ProfileOperationStatus::Exporting => {
                self.render_progress_indicator(ui, "Exporting profile...");
            }

            ProfileOperationStatus::Importing => {
                self.render_progress_indicator(ui, "Importing profile...");
            }

            ProfileOperationStatus::ExportSuccess { path, shown_at } => {
                let remaining = STATUS_DISPLAY_DURATION_SECS - shown_at.elapsed().as_secs_f32();
                self.render_success_toast(ui, "Profile exported successfully!", Some(path), remaining);
            }

            ProfileOperationStatus::ImportSuccess { summary, shown_at } => {
                let remaining = STATUS_DISPLAY_DURATION_SECS - shown_at.elapsed().as_secs_f32();
                self.render_success_toast(ui, summary, None, remaining);
            }

            ProfileOperationStatus::Error { message, shown_at } => {
                let remaining = STATUS_DISPLAY_DURATION_SECS - shown_at.elapsed().as_secs_f32();
                self.render_error_toast(ui, message, remaining);
            }
        }

        // Request repaint for progress animations and auto-dismiss
        if !matches!(self.operation_status, ProfileOperationStatus::Idle) {
            ui.ctx().request_repaint();
        }
    }

    /// T020: Render a progress indicator with spinner.
    fn render_progress_indicator(&self, ui: &mut Ui, message: &str) {
        egui::Frame::new()
            .fill(Color32::from_rgba_unmultiplied(40, 60, 80, 230))
            .inner_margin(12.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.add_space(8.0);
                    ui.label(RichText::new(message).color(Color32::WHITE));
                });
            });
    }

    /// T020: Render a success toast with optional path and dismiss progress.
    fn render_success_toast(&self, ui: &mut Ui, message: &str, path: Option<&str>, remaining_secs: f32) {
        egui::Frame::new()
            .fill(Color32::from_rgba_unmultiplied(46, 125, 50, 230)) // Green
            .inner_margin(12.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("✓").color(Color32::WHITE).strong().size(16.0));
                        ui.add_space(8.0);
                        ui.label(RichText::new(message).color(Color32::WHITE).strong());
                    });

                    if let Some(path) = path {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(path)
                                .color(Color32::from_rgb(200, 230, 200))
                                .small()
                                .monospace(),
                        );
                    }

                    // Progress bar showing time until dismiss
                    ui.add_space(6.0);
                    let progress = remaining_secs / STATUS_DISPLAY_DURATION_SECS;
                    let bar_rect = ui.available_rect_before_wrap();
                    let bar_height = 3.0;
                    let bar_rect = egui::Rect::from_min_size(
                        egui::Pos2::new(bar_rect.min.x, bar_rect.min.y),
                        Vec2::new(bar_rect.width(), bar_height),
                    );
                    ui.painter().rect_filled(bar_rect, 1.5, Color32::from_rgb(100, 160, 100));
                    let filled_rect = egui::Rect::from_min_size(
                        bar_rect.min,
                        Vec2::new(bar_rect.width() * progress, bar_height),
                    );
                    ui.painter().rect_filled(filled_rect, 1.5, Color32::WHITE);
                    ui.allocate_space(Vec2::new(0.0, bar_height));
                });
            });
    }

    /// T020: Render an error toast with dismiss progress.
    fn render_error_toast(&self, ui: &mut Ui, message: &str, remaining_secs: f32) {
        egui::Frame::new()
            .fill(Color32::from_rgba_unmultiplied(198, 40, 40, 230)) // Red
            .inner_margin(12.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("✕").color(Color32::WHITE).strong().size(16.0));
                        ui.add_space(8.0);
                        ui.label(RichText::new("Error").color(Color32::WHITE).strong());
                    });

                    ui.add_space(4.0);
                    ui.label(RichText::new(message).color(Color32::from_rgb(255, 200, 200)));

                    // Progress bar showing time until dismiss
                    ui.add_space(6.0);
                    let progress = remaining_secs / STATUS_DISPLAY_DURATION_SECS;
                    let bar_rect = ui.available_rect_before_wrap();
                    let bar_height = 3.0;
                    let bar_rect = egui::Rect::from_min_size(
                        egui::Pos2::new(bar_rect.min.x, bar_rect.min.y),
                        Vec2::new(bar_rect.width(), bar_height),
                    );
                    ui.painter().rect_filled(bar_rect, 1.5, Color32::from_rgb(160, 80, 80));
                    let filled_rect = egui::Rect::from_min_size(
                        bar_rect.min,
                        Vec2::new(bar_rect.width() * progress, bar_height),
                    );
                    ui.painter().rect_filled(filled_rect, 1.5, Color32::WHITE);
                    ui.allocate_space(Vec2::new(0.0, bar_height));
                });
            });
    }
}

/// T020: Format a ProfileImportResult into a human-readable summary.
fn format_import_summary(result: &ProfileImportResult) -> String {
    let mut parts = Vec::new();

    if result.ftp_entries_imported > 0 {
        parts.push(format!(
            "Imported {} FTP record{}",
            result.ftp_entries_imported,
            if result.ftp_entries_imported == 1 { "" } else { "s" }
        ));
    }

    if result.ftp_entries_skipped > 0 {
        parts.push(format!(
            "skipped {} duplicate{}",
            result.ftp_entries_skipped,
            if result.ftp_entries_skipped == 1 { "" } else { "s" }
        ));
    }

    if result.profile_updated {
        parts.push("updated profile".to_string());
    }

    if result.avatar_updated {
        parts.push("updated avatar".to_string());
    }

    if parts.is_empty() {
        "Import complete (no changes)".to_string()
    } else {
        format!("Import complete: {}", parts.join(", "))
    }
}

/// T020: Format a ProfileExportError into a human-readable error message.
fn format_export_error(error: &ProfileExportError) -> String {
    match error {
        ProfileExportError::DatabaseError(msg) => {
            format!("Database error: {}", msg)
        }
        ProfileExportError::SerializationFailed(msg) => {
            format!("Failed to serialize profile: {}", msg)
        }
        ProfileExportError::ParseError(msg) => {
            format!("Failed to parse file: {}", msg)
        }
        ProfileExportError::ProfileNotFound => {
            "No profile found to export".to_string()
        }
        ProfileExportError::InvalidVersion { expected, found } => {
            format!("Incompatible version: expected {}, found {}", expected, found)
        }
        ProfileExportError::IoError(msg) => {
            format!("File error: {}", msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_operation_status_default() {
        let status = ProfileOperationStatus::default();
        assert!(matches!(status, ProfileOperationStatus::Idle));
    }

    #[test]
    fn test_rider_profile_screen_new() {
        let screen = RiderProfileScreen::new();
        assert!(matches!(screen.operation_status, ProfileOperationStatus::Idle));
        assert!(!screen.is_operation_in_progress());
    }

    #[test]
    fn test_start_export() {
        let mut screen = RiderProfileScreen::new();
        screen.start_export();
        assert!(matches!(screen.operation_status, ProfileOperationStatus::Exporting));
        assert!(screen.is_operation_in_progress());
    }

    #[test]
    fn test_start_import() {
        let mut screen = RiderProfileScreen::new();
        screen.start_import();
        assert!(matches!(screen.operation_status, ProfileOperationStatus::Importing));
        assert!(screen.is_operation_in_progress());
    }

    #[test]
    fn test_show_export_success() {
        let mut screen = RiderProfileScreen::new();
        screen.show_export_success(std::path::Path::new("/tmp/profile.json"));

        match &screen.operation_status {
            ProfileOperationStatus::ExportSuccess { path, .. } => {
                assert!(path.contains("profile.json"));
            }
            _ => panic!("Expected ExportSuccess status"),
        }
        assert!(!screen.is_operation_in_progress());
    }

    #[test]
    fn test_show_import_success() {
        let mut screen = RiderProfileScreen::new();
        let result = ProfileImportResult::success(5, 2, true, true);
        screen.show_import_success(&result);

        match &screen.operation_status {
            ProfileOperationStatus::ImportSuccess { summary, .. } => {
                assert!(summary.contains("5 FTP records"));
                assert!(summary.contains("skipped 2 duplicates"));
                assert!(summary.contains("updated profile"));
                assert!(summary.contains("updated avatar"));
            }
            _ => panic!("Expected ImportSuccess status"),
        }
    }

    #[test]
    fn test_show_error() {
        let mut screen = RiderProfileScreen::new();
        let error = ProfileExportError::ProfileNotFound;
        screen.show_error(&error);

        match &screen.operation_status {
            ProfileOperationStatus::Error { message, .. } => {
                assert!(message.contains("No profile found"));
            }
            _ => panic!("Expected Error status"),
        }
    }

    #[test]
    fn test_clear_status() {
        let mut screen = RiderProfileScreen::new();
        screen.start_export();
        assert!(screen.is_operation_in_progress());

        screen.clear_status();
        assert!(matches!(screen.operation_status, ProfileOperationStatus::Idle));
        assert!(!screen.is_operation_in_progress());
    }

    #[test]
    fn test_format_import_summary_with_all_changes() {
        let result = ProfileImportResult::success(3, 1, true, true);
        let summary = format_import_summary(&result);

        assert!(summary.contains("Import complete"));
        assert!(summary.contains("3 FTP records"));
        assert!(summary.contains("skipped 1 duplicate"));
        assert!(summary.contains("updated profile"));
        assert!(summary.contains("updated avatar"));
    }

    #[test]
    fn test_format_import_summary_ftp_only() {
        let result = ProfileImportResult::success(5, 0, false, false);
        let summary = format_import_summary(&result);

        assert!(summary.contains("5 FTP records"));
        assert!(!summary.contains("updated profile"));
        assert!(!summary.contains("updated avatar"));
    }

    #[test]
    fn test_format_import_summary_no_changes() {
        let result = ProfileImportResult::success(0, 0, false, false);
        let summary = format_import_summary(&result);

        assert_eq!(summary, "Import complete (no changes)");
    }

    #[test]
    fn test_format_import_summary_singular() {
        let result = ProfileImportResult::success(1, 1, false, false);
        let summary = format_import_summary(&result);

        assert!(summary.contains("1 FTP record"));
        assert!(summary.contains("skipped 1 duplicate"));
        // Should NOT have plural 's'
        assert!(!summary.contains("records"));
        assert!(!summary.contains("duplicates"));
    }

    #[test]
    fn test_format_export_error_database() {
        let error = ProfileExportError::DatabaseError("connection failed".to_string());
        let message = format_export_error(&error);
        assert!(message.contains("Database error"));
        assert!(message.contains("connection failed"));
    }

    #[test]
    fn test_format_export_error_invalid_version() {
        let error = ProfileExportError::InvalidVersion {
            expected: "1.0".to_string(),
            found: "2.0".to_string(),
        };
        let message = format_export_error(&error);
        assert!(message.contains("Incompatible version"));
        assert!(message.contains("1.0"));
        assert!(message.contains("2.0"));
    }

    #[test]
    fn test_format_export_error_io() {
        let error = ProfileExportError::IoError("permission denied".to_string());
        let message = format_export_error(&error);
        assert!(message.contains("File error"));
        assert!(message.contains("permission denied"));
    }
}
