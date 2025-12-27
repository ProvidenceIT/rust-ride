//! Layout module for customizable dashboard layouts.
//!
//! Provides drag-and-drop widget arrangement and layout profile management.

pub mod editor;
pub mod profiles;

use egui::Rect;
use serde::{Deserialize, Serialize};

// Re-export types
pub use editor::LayoutEditor;
pub use profiles::{LayoutProfile, LayoutProfileManager, WidgetPlacement};

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
