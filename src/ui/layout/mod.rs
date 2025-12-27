//! Layout module for customizable dashboard layouts.
//!
//! Provides drag-and-drop widget arrangement and layout profile management.
//!
//! T071: Create layout profile save/load UI
//! T072: Implement profile naming dialog
//! T073: Implement profile deletion with confirmation
//! T074: Implement LayoutRenderer for dashboard

pub mod editor;
pub mod profiles;

use egui::{Align, Color32, Layout, Rect, RichText, Ui, Vec2};
use serde::{Deserialize, Serialize};

// Re-export types
pub use editor::LayoutEditor;
pub use profiles::{LayoutProfile, LayoutProfileManager, ProfileError, WidgetPlacement};

/// Widget types that can be placed in a layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WidgetType {
    /// Power display
    Power,
    /// 3-second power average
    Power3s,
    /// Heart rate display
    HeartRate,
    /// Cadence display
    Cadence,
    /// Speed display
    Speed,
    /// Distance display
    Distance,
    /// Duration/elapsed time
    Duration,
    /// Calories burned
    Calories,
    /// Normalized power
    NormalizedPower,
    /// TSS (Training Stress Score)
    Tss,
    /// Intensity Factor
    IntensityFactor,
    /// Power zone indicator
    PowerZone,
    /// HR zone indicator
    HrZone,
    /// Workout progress
    WorkoutProgress,
    /// Power graph
    PowerGraph,
    /// Zone bar
    ZoneBar,
}

impl WidgetType {
    /// Get the display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            WidgetType::Power => "Power",
            WidgetType::Power3s => "3s Power",
            WidgetType::HeartRate => "Heart Rate",
            WidgetType::Cadence => "Cadence",
            WidgetType::Speed => "Speed",
            WidgetType::Distance => "Distance",
            WidgetType::Duration => "Duration",
            WidgetType::Calories => "Calories",
            WidgetType::NormalizedPower => "NP",
            WidgetType::Tss => "TSS",
            WidgetType::IntensityFactor => "IF",
            WidgetType::PowerZone => "Power Zone",
            WidgetType::HrZone => "HR Zone",
            WidgetType::WorkoutProgress => "Workout Progress",
            WidgetType::PowerGraph => "Power Graph",
            WidgetType::ZoneBar => "Zone Bar",
        }
    }

    /// Get the default size for this widget type.
    pub fn default_size(&self) -> (f32, f32) {
        match self {
            WidgetType::PowerGraph => (300.0, 150.0),
            WidgetType::WorkoutProgress => (300.0, 80.0),
            WidgetType::ZoneBar => (200.0, 40.0),
            _ => (100.0, 80.0), // Standard metric widget
        }
    }

    /// Get the minimum size for this widget type.
    pub fn min_size(&self) -> (f32, f32) {
        match self {
            WidgetType::PowerGraph => (200.0, 100.0),
            WidgetType::WorkoutProgress => (200.0, 60.0),
            WidgetType::ZoneBar => (150.0, 30.0),
            _ => (80.0, 60.0),
        }
    }

    /// Check if this widget can be resized.
    pub fn is_resizable(&self) -> bool {
        matches!(
            self,
            WidgetType::PowerGraph | WidgetType::WorkoutProgress | WidgetType::ZoneBar
        )
    }
}

impl std::fmt::Display for WidgetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Trait for rendering layouts.
pub trait LayoutRenderer {
    /// Render the current layout.
    fn render(&self, ui: &mut egui::Ui, layout: &LayoutProfile);

    /// Render a single widget.
    fn render_widget(&self, ui: &mut egui::Ui, widget: &WidgetPlacement);

    /// Get the available area for layout.
    fn available_area(&self) -> Rect;
}

/// T071: Layout profile UI for save/load operations.
pub struct LayoutProfileUi {
    /// Profile manager
    manager: LayoutProfileManager,
    /// Whether the profile selector is open
    selector_open: bool,
    /// Profile naming dialog state
    naming_dialog: Option<NamingDialogState>,
    /// Delete confirmation dialog state
    delete_dialog: Option<DeleteDialogState>,
    /// Error message to display
    error_message: Option<String>,
}

/// T072: State for the profile naming dialog.
#[derive(Debug, Clone)]
struct NamingDialogState {
    /// Profile ID being renamed (None for new profile)
    profile_id: Option<uuid::Uuid>,
    /// Current name input
    name: String,
    /// Whether this is a new profile
    is_new: bool,
    /// Whether to duplicate from another profile
    duplicate_from: Option<uuid::Uuid>,
}

/// T073: State for the delete confirmation dialog.
#[derive(Debug, Clone)]
struct DeleteDialogState {
    /// Profile ID to delete
    profile_id: uuid::Uuid,
    /// Profile name for display
    profile_name: String,
}

impl Default for LayoutProfileUi {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutProfileUi {
    /// Create a new layout profile UI.
    pub fn new() -> Self {
        Self {
            manager: LayoutProfileManager::new(),
            selector_open: false,
            naming_dialog: None,
            delete_dialog: None,
            error_message: None,
        }
    }

    /// Create with an existing profile manager.
    pub fn with_manager(manager: LayoutProfileManager) -> Self {
        Self {
            manager,
            selector_open: false,
            naming_dialog: None,
            delete_dialog: None,
            error_message: None,
        }
    }

    /// Get the profile manager.
    pub fn manager(&self) -> &LayoutProfileManager {
        &self.manager
    }

    /// Get the profile manager mutably.
    pub fn manager_mut(&mut self) -> &mut LayoutProfileManager {
        &mut self.manager
    }

    /// Get the active profile.
    pub fn active_profile(&self) -> &LayoutProfile {
        self.manager.active_profile()
    }

    /// T071: Show the compact profile selector (for toolbar).
    pub fn show_selector(&mut self, ui: &mut Ui) {
        let active = self.manager.active_profile();
        let active_name = active.name.clone();
        let active_id = active.id;

        // Collect profiles first to avoid borrow conflicts
        let profiles: Vec<_> = self
            .manager
            .profiles()
            .iter()
            .map(|p| (p.id, p.name.clone()))
            .collect();

        let mut selected_id = None;
        egui::ComboBox::from_id_salt("layout_profile_selector")
            .selected_text(&active_name)
            .width(150.0)
            .show_ui(ui, |ui| {
                for (id, name) in &profiles {
                    let is_selected = *id == active_id;
                    if ui.selectable_label(is_selected, name).clicked() {
                        selected_id = Some(*id);
                    }
                }
            });

        if let Some(id) = selected_id {
            self.manager.set_active(id);
        }
    }

    /// T071: Show the full profile management UI.
    pub fn show_management_ui(&mut self, ui: &mut Ui) {
        ui.heading("Layout Profiles");
        ui.add_space(10.0);

        // Profile list
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                let active_id = self.manager.active_profile().id;
                let profiles: Vec<_> = self.manager.profiles().to_vec();

                for profile in profiles {
                    ui.horizontal(|ui| {
                        // Radio button for selection
                        let is_active = profile.id == active_id;
                        if ui.radio(is_active, "").clicked() && !is_active {
                            self.manager.set_active(profile.id);
                        }

                        // Profile name
                        let name_text = if profile.is_default {
                            format!("{} (Default)", profile.name)
                        } else {
                            profile.name.clone()
                        };
                        ui.label(&name_text);

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            // Delete button (disabled for default)
                            if !profile.is_default {
                                if ui
                                    .add_enabled(true, egui::Button::new("🗑").small())
                                    .on_hover_text("Delete profile")
                                    .clicked()
                                {
                                    self.delete_dialog = Some(DeleteDialogState {
                                        profile_id: profile.id,
                                        profile_name: profile.name.clone(),
                                    });
                                }
                            }

                            // Rename button
                            if ui
                                .add(egui::Button::new("✏").small())
                                .on_hover_text("Rename profile")
                                .clicked()
                            {
                                self.naming_dialog = Some(NamingDialogState {
                                    profile_id: Some(profile.id),
                                    name: profile.name.clone(),
                                    is_new: false,
                                    duplicate_from: None,
                                });
                            }

                            // Duplicate button
                            if ui
                                .add(egui::Button::new("📋").small())
                                .on_hover_text("Duplicate profile")
                                .clicked()
                            {
                                self.naming_dialog = Some(NamingDialogState {
                                    profile_id: None,
                                    name: format!("{} (Copy)", profile.name),
                                    is_new: true,
                                    duplicate_from: Some(profile.id),
                                });
                            }
                        });
                    });
                    ui.add_space(4.0);
                }
            });

        ui.add_space(10.0);

        // Create new profile button
        ui.horizontal(|ui| {
            let can_create = self.manager.can_create_profile();
            if ui
                .add_enabled(can_create, egui::Button::new("+ New Profile"))
                .clicked()
            {
                self.naming_dialog = Some(NamingDialogState {
                    profile_id: None,
                    name: "New Layout".to_string(),
                    is_new: true,
                    duplicate_from: None,
                });
            }

            if !can_create {
                ui.label(
                    RichText::new(format!("(Max {} profiles)", profiles::MAX_PROFILES))
                        .small()
                        .color(Color32::GRAY),
                );
            }
        });

        // Show error message if any
        if let Some(error) = &self.error_message {
            ui.add_space(10.0);
            ui.colored_label(Color32::from_rgb(234, 67, 53), error);
        }
    }

    /// T072: Show the profile naming dialog.
    pub fn show_naming_dialog(&mut self, ctx: &egui::Context) -> bool {
        let mut changed = false;

        if let Some(state) = self.naming_dialog.clone() {
            let title = if state.is_new {
                if state.duplicate_from.is_some() {
                    "Duplicate Profile"
                } else {
                    "Create New Profile"
                }
            } else {
                "Rename Profile"
            };

            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_min_width(300.0);

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        let response = ui.text_edit_singleline(
                            &mut self.naming_dialog.as_mut().unwrap().name,
                        );

                        // Auto-focus the text field
                        if response.changed() {
                            self.error_message = None;
                        }
                    });

                    ui.add_space(15.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.naming_dialog = None;
                        }

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let name = &self.naming_dialog.as_ref().unwrap().name;
                            let can_save = !name.trim().is_empty();

                            if ui.add_enabled(can_save, egui::Button::new("Save")).clicked() {
                                let state = self.naming_dialog.take().unwrap();
                                let result = if state.is_new {
                                    if let Some(dup_id) = state.duplicate_from {
                                        self.manager.duplicate_profile(dup_id, state.name)
                                    } else {
                                        self.manager.create_profile(state.name)
                                    }
                                } else if let Some(id) = state.profile_id {
                                    self.manager.rename_profile(id, state.name).map(|_| id)
                                } else {
                                    Ok(uuid::Uuid::new_v4()) // Should not happen
                                };

                                match result {
                                    Ok(_) => {
                                        changed = true;
                                        self.error_message = None;
                                    }
                                    Err(e) => {
                                        self.error_message = Some(e.to_string());
                                    }
                                }
                            }
                        });
                    });
                });
        }

        changed
    }

    /// T073: Show the delete confirmation dialog.
    pub fn show_delete_dialog(&mut self, ctx: &egui::Context) -> bool {
        let mut changed = false;

        if let Some(state) = self.delete_dialog.clone() {
            egui::Window::new("Delete Profile?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_min_width(300.0);

                    ui.label(format!(
                        "Are you sure you want to delete \"{}\"?",
                        state.profile_name
                    ));
                    ui.label(
                        RichText::new("This action cannot be undone.")
                            .small()
                            .color(Color32::GRAY),
                    );

                    ui.add_space(15.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.delete_dialog = None;
                        }

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new("Delete")
                                        .fill(Color32::from_rgb(234, 67, 53)),
                                )
                                .clicked()
                            {
                                let state = self.delete_dialog.take().unwrap();
                                if let Err(e) = self.manager.delete_profile(state.profile_id) {
                                    self.error_message = Some(e.to_string());
                                } else {
                                    changed = true;
                                    self.error_message = None;
                                }
                            }
                        });
                    });
                });
        }

        changed
    }

    /// Check if any dialog is currently open.
    pub fn has_dialog_open(&self) -> bool {
        self.naming_dialog.is_some() || self.delete_dialog.is_some()
    }
}

/// T074: Default layout renderer implementation.
pub struct DefaultLayoutRenderer {
    /// Container area
    container: Rect,
}

impl DefaultLayoutRenderer {
    /// Create a new renderer with the given container area.
    pub fn new(container: Rect) -> Self {
        Self { container }
    }

    /// Render a metric widget placeholder.
    fn render_metric_placeholder(&self, ui: &mut Ui, widget: &WidgetPlacement) {
        let rect = widget.rect(self.container.width(), self.container.height());
        let rect = rect.translate(self.container.min.to_vec2());

        // Draw widget background
        let painter = ui.painter();
        painter.rect_filled(rect, 4.0, Color32::from_rgb(38, 38, 48));
        painter.rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)),
            egui::StrokeKind::Inside,
        );

        // Draw widget label
        let label = widget.widget_type.display_name();
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::default(),
            Color32::WHITE,
        );
    }
}

impl LayoutRenderer for DefaultLayoutRenderer {
    fn render(&self, ui: &mut Ui, layout: &LayoutProfile) {
        for widget in &layout.widgets {
            if widget.visible {
                self.render_widget(ui, widget);
            }
        }
    }

    fn render_widget(&self, ui: &mut Ui, widget: &WidgetPlacement) {
        self.render_metric_placeholder(ui, widget);
    }

    fn available_area(&self) -> Rect {
        self.container
    }
}

/// T075: Layout profile selector widget for ride screen.
pub struct LayoutProfileSelector {
    /// Expanded state
    expanded: bool,
}

impl Default for LayoutProfileSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutProfileSelector {
    /// Create a new selector.
    pub fn new() -> Self {
        Self { expanded: false }
    }

    /// Show the compact selector.
    ///
    /// Returns the selected profile ID if changed.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        manager: &LayoutProfileManager,
    ) -> Option<uuid::Uuid> {
        let mut selected = None;
        let active = manager.active_profile();
        let active_id = active.id;

        ui.horizontal(|ui| {
            ui.label("Layout:");

            egui::ComboBox::from_id_salt("ride_layout_selector")
                .selected_text(&active.name)
                .width(120.0)
                .show_ui(ui, |ui| {
                    for profile in manager.profiles() {
                        let is_active = profile.id == active_id;
                        if ui.selectable_label(is_active, &profile.name).clicked() {
                            selected = Some(profile.id);
                        }
                    }
                });
        });

        selected
    }

    /// Show inline buttons for quick profile switching.
    pub fn show_quick_switch(
        &mut self,
        ui: &mut Ui,
        manager: &LayoutProfileManager,
    ) -> Option<uuid::Uuid> {
        let mut selected = None;
        let active_id = manager.active_profile().id;

        ui.horizontal(|ui| {
            for profile in manager.profiles() {
                let is_active = profile.id == active_id;
                if ui.selectable_label(is_active, &profile.name).clicked() && !is_active {
                    selected = Some(profile.id);
                }
            }
        });

        selected
    }
}
