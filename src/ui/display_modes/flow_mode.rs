//! Flow Mode for minimal distraction display.
//!
//! Shows a single large metric with optional 3D world background,
//! ideal for focused training sessions.

use crate::storage::config::MetricType;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Position options for Flow Mode metric display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
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

impl std::fmt::Display for FlowMetricPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlowMetricPosition::Center => write!(f, "Center"),
            FlowMetricPosition::TopCenter => write!(f, "Top Center"),
            FlowMetricPosition::BottomCenter => write!(f, "Bottom Center"),
            FlowMetricPosition::TopLeft => write!(f, "Top Left"),
            FlowMetricPosition::TopRight => write!(f, "Top Right"),
            FlowMetricPosition::BottomLeft => write!(f, "Bottom Left"),
            FlowMetricPosition::BottomRight => write!(f, "Bottom Right"),
        }
    }
}

/// Flow Mode configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Font size for the primary metric
    pub font_size: f32,
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
            font_size: 120.0,
        }
    }
}

/// An interval notification to display.
#[derive(Debug, Clone)]
struct IntervalNotification {
    /// Interval name
    text: String,
    /// Duration remaining (reserved for future use)
    #[allow(dead_code)]
    duration_remaining: u32,
    /// When the notification was shown
    shown_at: Instant,
}

/// Flow Mode renderer.
pub struct FlowModeRenderer {
    /// Settings
    settings: FlowModeSettings,
    /// Current notification
    current_notification: Option<IntervalNotification>,
    /// Metrics that can be cycled through
    available_metrics: Vec<MetricType>,
    /// Current metric index in the cycle
    current_metric_index: usize,
}

impl Default for FlowModeRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl FlowModeRenderer {
    /// Create a new Flow Mode renderer.
    pub fn new() -> Self {
        let available_metrics = vec![
            MetricType::Power,
            MetricType::HeartRate,
            MetricType::Cadence,
            MetricType::Duration,
            MetricType::Speed,
            MetricType::Power3s,
        ];

        Self {
            settings: FlowModeSettings::default(),
            current_notification: None,
            available_metrics,
            current_metric_index: 0,
        }
    }

    /// Get the settings.
    pub fn settings(&self) -> &FlowModeSettings {
        &self.settings
    }

    /// Update settings.
    pub fn update_settings(&mut self, settings: FlowModeSettings) {
        self.settings = settings;
    }

    /// Get the primary metric to display.
    pub fn primary_metric(&self) -> MetricType {
        self.settings.primary_metric
    }

    /// Set the primary metric to display.
    pub fn set_primary_metric(&mut self, metric: MetricType) {
        self.settings.primary_metric = metric;

        // Update the cycle index
        if let Some(idx) = self.available_metrics.iter().position(|m| *m == metric) {
            self.current_metric_index = idx;
        }
    }

    /// Cycle to the next metric.
    pub fn cycle_metric(&mut self) {
        self.current_metric_index = (self.current_metric_index + 1) % self.available_metrics.len();
        self.settings.primary_metric = self.available_metrics[self.current_metric_index];
    }

    /// Cycle to the previous metric.
    pub fn cycle_metric_back(&mut self) {
        if self.current_metric_index == 0 {
            self.current_metric_index = self.available_metrics.len() - 1;
        } else {
            self.current_metric_index -= 1;
        }
        self.settings.primary_metric = self.available_metrics[self.current_metric_index];
    }

    /// Show an interval notification.
    pub fn show_interval_notification(&mut self, interval_name: &str, duration_remaining: u32) {
        if self.settings.show_interval_notifications {
            self.current_notification = Some(IntervalNotification {
                text: interval_name.to_string(),
                duration_remaining,
                shown_at: Instant::now(),
            });
        }
    }

    /// Check if a notification is currently visible.
    pub fn is_notification_visible(&self) -> bool {
        if let Some(notif) = &self.current_notification {
            let elapsed = notif.shown_at.elapsed().as_secs_f32();
            let total_duration =
                self.settings.notification_duration_secs + self.settings.notification_fade_secs;
            elapsed < total_duration
        } else {
            false
        }
    }

    /// Get the current notification opacity (for fade effect).
    pub fn notification_opacity(&self) -> f32 {
        if let Some(notif) = &self.current_notification {
            let elapsed = notif.shown_at.elapsed().as_secs_f32();

            if elapsed < self.settings.notification_duration_secs {
                1.0
            } else {
                let fade_progress = (elapsed - self.settings.notification_duration_secs)
                    / self.settings.notification_fade_secs;
                (1.0 - fade_progress).max(0.0)
            }
        } else {
            0.0
        }
    }

    /// Get the current notification text.
    pub fn notification_text(&self) -> Option<&str> {
        self.current_notification.as_ref().map(|n| n.text.as_str())
    }

    /// Get the font size for the primary metric.
    pub fn font_size(&self) -> f32 {
        self.settings.font_size
    }

    /// Get the overlay opacity.
    pub fn overlay_opacity(&self) -> f32 {
        self.settings.overlay_opacity
    }

    /// Check if world background should be shown.
    pub fn show_world_background(&self) -> bool {
        self.settings.show_world_background
    }

    /// Get the metric position.
    pub fn metric_position(&self) -> FlowMetricPosition {
        self.settings.metric_position
    }

    /// Set the metric position.
    pub fn set_metric_position(&mut self, position: FlowMetricPosition) {
        self.settings.metric_position = position;
    }

    /// Clear the current notification.
    pub fn clear_notification(&mut self) {
        self.current_notification = None;
    }

    /// Get the available metrics for cycling.
    pub fn available_metrics(&self) -> &[MetricType] {
        &self.available_metrics
    }
}

/// Trait for Flow Mode rendering.
pub trait FlowModeRendererTrait {
    /// Get Flow Mode settings.
    fn settings(&self) -> &FlowModeSettings;

    /// Update Flow Mode settings.
    fn update_settings(&mut self, settings: FlowModeSettings);

    /// Get the primary metric to display.
    fn primary_metric(&self) -> MetricType;

    /// Set the primary metric to display.
    fn set_primary_metric(&mut self, metric: MetricType);

    /// Show interval notification.
    fn show_interval_notification(&mut self, interval_name: &str, duration_remaining: u32);

    /// Check if notification is currently visible.
    fn is_notification_visible(&self) -> bool;
}

impl FlowModeRendererTrait for FlowModeRenderer {
    fn settings(&self) -> &FlowModeSettings {
        FlowModeRenderer::settings(self)
    }

    fn update_settings(&mut self, settings: FlowModeSettings) {
        FlowModeRenderer::update_settings(self, settings);
    }

    fn primary_metric(&self) -> MetricType {
        FlowModeRenderer::primary_metric(self)
    }

    fn set_primary_metric(&mut self, metric: MetricType) {
        FlowModeRenderer::set_primary_metric(self, metric);
    }

    fn show_interval_notification(&mut self, interval_name: &str, duration_remaining: u32) {
        FlowModeRenderer::show_interval_notification(self, interval_name, duration_remaining);
    }

    fn is_notification_visible(&self) -> bool {
        FlowModeRenderer::is_notification_visible(self)
    }
}
