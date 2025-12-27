//! Layout Customization Module Contract
//!
//! Public API for customizable UI layouts, widget placement, and profile management.

use uuid::Uuid;

// ============================================================================
// Layout Profile Management
// ============================================================================

/// Manages layout profiles (max 10 per user).
pub trait LayoutProfileManager {
    /// Get all layout profiles.
    fn list_profiles(&self) -> Vec<LayoutProfileSummary>;

    /// Get a specific profile by ID.
    fn get_profile(&self, id: Uuid) -> Option<LayoutProfile>;

    /// Get the default profile.
    fn get_default_profile(&self) -> LayoutProfile;

    /// Get the currently active profile.
    fn get_active_profile(&self) -> LayoutProfile;

    /// Set the active profile.
    fn set_active_profile(&mut self, id: Uuid) -> Result<(), LayoutError>;

    /// Create a new profile.
    fn create_profile(&mut self, name: &str, widgets: Vec<WidgetPlacement>) -> Result<LayoutProfile, LayoutError>;

    /// Update an existing profile.
    fn update_profile(&mut self, id: Uuid, name: Option<&str>, widgets: Option<Vec<WidgetPlacement>>) -> Result<(), LayoutError>;

    /// Delete a profile (cannot delete default).
    fn delete_profile(&mut self, id: Uuid) -> Result<(), LayoutError>;

    /// Duplicate a profile with a new name.
    fn duplicate_profile(&mut self, id: Uuid, new_name: &str) -> Result<LayoutProfile, LayoutError>;

    /// Reset a profile to default layout.
    fn reset_to_default(&mut self, id: Uuid) -> Result<(), LayoutError>;

    /// Get the number of profiles (for limit checking).
    fn profile_count(&self) -> usize;

    /// Check if more profiles can be created.
    fn can_create_profile(&self) -> bool;
}

/// Summary information for profile listing.
#[derive(Clone, Debug)]
pub struct LayoutProfileSummary {
    pub id: Uuid,
    pub name: String,
    pub is_default: bool,
    pub is_active: bool,
    pub widget_count: usize,
}

// ============================================================================
// Layout Editor
// ============================================================================

/// Layout editor for drag-and-drop widget arrangement.
pub trait LayoutEditor {
    /// Enter edit mode.
    fn start_editing(&mut self, profile_id: Uuid);

    /// Exit edit mode without saving.
    fn cancel_editing(&mut self);

    /// Save current edits.
    fn save_edits(&mut self) -> Result<(), LayoutError>;

    /// Check if currently in edit mode.
    fn is_editing(&self) -> bool;

    /// Get the current edit state.
    fn edit_state(&self) -> Option<&EditState>;

    /// Add a widget to the layout.
    fn add_widget(&mut self, metric: MetricType, position: GridPosition) -> Result<(), LayoutError>;

    /// Remove a widget from the layout.
    fn remove_widget(&mut self, widget_id: usize) -> Result<(), LayoutError>;

    /// Move a widget to a new position.
    fn move_widget(&mut self, widget_id: usize, new_position: GridPosition) -> Result<(), LayoutError>;

    /// Resize a widget.
    fn resize_widget(&mut self, widget_id: usize, new_size: GridSize) -> Result<(), LayoutError>;

    /// Change widget size tier (primary/secondary/tertiary).
    fn set_widget_tier(&mut self, widget_id: usize, tier: WidgetSizeTier) -> Result<(), LayoutError>;

    /// Check if a position is valid (not overlapping).
    fn is_position_valid(&self, position: GridPosition, size: GridSize, exclude_widget: Option<usize>) -> bool;
}

/// Current state of layout editing.
#[derive(Clone, Debug)]
pub struct EditState {
    pub profile_id: Uuid,
    pub original_widgets: Vec<WidgetPlacement>,
    pub current_widgets: Vec<WidgetPlacement>,
    pub has_changes: bool,
    pub dragging_widget: Option<usize>,
}

/// Position on the layout grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridPosition {
    pub column: u8,
    pub row: u8,
}

/// Size on the layout grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridSize {
    pub width: u8,
    pub height: u8,
}

// ============================================================================
// Widget Rendering
// ============================================================================

/// Renders widgets according to layout profile.
pub trait LayoutRenderer {
    /// Render the dashboard with current layout.
    fn render_dashboard(&self, ctx: &egui::Context, metrics: &CurrentMetrics);

    /// Render a single widget.
    fn render_widget(&self, ctx: &egui::Context, widget: &WidgetPlacement, value: MetricValue);

    /// Get the pixel rect for a grid position.
    fn grid_to_pixels(&self, position: GridPosition, size: GridSize, available: egui::Rect) -> egui::Rect;
}

/// Current metric values for rendering.
pub struct CurrentMetrics {
    pub power: u16,
    pub power_3s: u16,
    pub heart_rate: Option<u8>,
    pub cadence: Option<u8>,
    pub speed_kmh: f32,
    pub distance_km: f64,
    pub duration_secs: u32,
    pub calories: u32,
    pub normalized_power: Option<u16>,
    pub tss: Option<f32>,
    pub intensity_factor: Option<f32>,
    pub power_zone: u8,
    pub hr_zone: Option<u8>,
}

/// Value for a single metric.
pub enum MetricValue {
    Power(u16),
    HeartRate(u8),
    Cadence(u8),
    Speed(f32, &'static str),  // value, unit label
    Distance(f64, &'static str),
    Duration(u32),
    Calories(u32),
    Zone(u8, &'static str),  // zone number, zone name
    Percentage(f32),
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("Profile not found: {0}")]
    ProfileNotFound(Uuid),

    #[error("Cannot delete default profile")]
    CannotDeleteDefault,

    #[error("Maximum profiles reached (10)")]
    MaxProfilesReached,

    #[error("Profile name already exists: {0}")]
    DuplicateName(String),

    #[error("Invalid widget position: overlaps with existing widget")]
    OverlappingWidgets,

    #[error("Widget position out of bounds")]
    PositionOutOfBounds,

    #[error("Not in edit mode")]
    NotEditing,

    #[error("Database error: {0}")]
    DatabaseError(String),
}
