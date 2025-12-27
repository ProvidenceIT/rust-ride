//! Display Modes Module Contract
//!
//! Public API for TV Mode and Flow Mode display configurations.

use egui::Context;

// ============================================================================
// Display Mode Management
// ============================================================================

/// Manages display mode switching.
pub trait DisplayModeManager {
    /// Get the current display mode.
    fn current_mode(&self) -> DisplayMode;

    /// Set the display mode.
    fn set_mode(&mut self, mode: DisplayMode);

    /// Toggle TV Mode on/off.
    fn toggle_tv_mode(&mut self);

    /// Enter Flow Mode.
    fn enter_flow_mode(&mut self);

    /// Exit Flow Mode (returns to previous mode).
    fn exit_flow_mode(&mut self);

    /// Check if in Flow Mode.
    fn is_flow_mode(&self) -> bool;

    /// Check if in TV Mode.
    fn is_tv_mode(&self) -> bool;

    /// Get the mode before Flow Mode was entered (for restoration).
    fn pre_flow_mode(&self) -> Option<DisplayMode>;
}

/// Display mode options.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DisplayMode {
    #[default]
    Normal,
    TvMode,
    FlowMode,
}

// ============================================================================
// TV Mode
// ============================================================================

/// TV Mode rendering configuration.
pub trait TvModeRenderer {
    /// Get the font scale multiplier for TV Mode.
    fn font_scale(&self) -> f32;

    /// Get the minimum button size for TV Mode.
    fn min_button_size(&self) -> egui::Vec2;

    /// Get the spacing multiplier for TV Mode.
    fn spacing_scale(&self) -> f32;

    /// Check if a metric should be hidden in TV Mode (low priority).
    fn should_hide_metric(&self, metric: MetricType) -> bool;

    /// Get the simplified layout for TV Mode.
    fn tv_layout(&self) -> TvModeLayout;

    /// Apply TV Mode styling to context.
    fn apply_tv_style(&self, ctx: &Context);

    /// Revert TV Mode styling.
    fn revert_tv_style(&self, ctx: &Context);
}

/// TV Mode layout configuration.
#[derive(Clone, Debug)]
pub struct TvModeLayout {
    /// Primary metrics to show (large, top)
    pub primary_metrics: Vec<MetricType>,

    /// Secondary metrics to show (medium, middle)
    pub secondary_metrics: Vec<MetricType>,

    /// Whether to show workout progress bar
    pub show_progress: bool,

    /// Whether to show zone indicator
    pub show_zone_indicator: bool,

    /// Font size for primary metrics (points)
    pub primary_font_size: f32,

    /// Font size for secondary metrics (points)
    pub secondary_font_size: f32,
}

impl Default for TvModeLayout {
    fn default() -> Self {
        Self {
            primary_metrics: vec![MetricType::Power, MetricType::HeartRate],
            secondary_metrics: vec![MetricType::Cadence, MetricType::Duration],
            show_progress: true,
            show_zone_indicator: true,
            primary_font_size: 72.0,  // Readable from 3m on 65" TV
            secondary_font_size: 48.0,
        }
    }
}

// ============================================================================
// Flow Mode
// ============================================================================

/// Flow Mode rendering configuration.
pub trait FlowModeRenderer {
    /// Get Flow Mode settings.
    fn settings(&self) -> &FlowModeSettings;

    /// Update Flow Mode settings.
    fn update_settings(&mut self, settings: FlowModeSettings);

    /// Get the primary metric to display.
    fn primary_metric(&self) -> MetricType;

    /// Set the primary metric to display.
    fn set_primary_metric(&mut self, metric: MetricType);

    /// Render Flow Mode UI.
    fn render(&self, ctx: &Context, metrics: &CurrentMetrics, world_visible: bool);

    /// Show interval notification (brief, fades).
    fn show_interval_notification(&mut self, interval_name: &str, duration_remaining: u32);

    /// Check if notification is currently visible.
    fn is_notification_visible(&self) -> bool;
}

/// Flow Mode configuration.
#[derive(Clone, Debug)]
pub struct FlowModeSettings {
    /// The single metric to display prominently
    pub primary_metric: MetricType,

    /// Show 3D world as background
    pub show_world_background: bool,

    /// Show brief interval change notifications
    pub show_interval_notifications: bool,

    /// Notification display duration (seconds)
    pub notification_duration_secs: f32,

    /// Notification fade duration (seconds)
    pub notification_fade_secs: f32,

    /// Opacity of metric overlay (0.0 - 1.0)
    pub overlay_opacity: f32,

    /// Position of the primary metric
    pub metric_position: FlowMetricPosition,
}

impl Default for FlowModeSettings {
    fn default() -> Self {
        Self {
            primary_metric: MetricType::Power,
            show_world_background: true,
            show_interval_notifications: true,
            notification_duration_secs: 3.0,
            notification_fade_secs: 0.5,
            overlay_opacity: 0.9,
            metric_position: FlowMetricPosition::Center,
        }
    }
}

/// Position options for Flow Mode metric display.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FlowMetricPosition {
    #[default]
    Center,
    TopCenter,
    BottomCenter,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

// ============================================================================
// Shared Types
// ============================================================================

/// Metric types (duplicated from config for contract independence).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MetricType {
    Power,
    Power3s,
    HeartRate,
    Cadence,
    Speed,
    Distance,
    Duration,
    Calories,
    NormalizedPower,
    Tss,
    IntensityFactor,
    PowerZone,
    HrZone,
}

/// Current metrics for rendering (duplicated for contract independence).
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

// ============================================================================
// Display Mode Hotkeys
// ============================================================================

/// Hotkey actions for display mode control.
pub enum DisplayModeAction {
    /// Toggle Flow Mode (e.g., F key or Escape to exit)
    ToggleFlowMode,

    /// Toggle TV Mode (e.g., T key)
    ToggleTvMode,

    /// Cycle primary metric in Flow Mode (e.g., M key)
    CycleFlowMetric,

    /// Exit any special mode to Normal
    ExitToNormal,
}

/// Default hotkey bindings:
/// - F: Toggle Flow Mode
/// - T: Toggle TV Mode
/// - M: Cycle Flow Mode metric
/// - Escape: Exit Flow Mode (or close modal)
