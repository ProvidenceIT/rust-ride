//! TV Mode for large display viewing.
//!
//! Optimized for 65"+ displays at 3+ meter viewing distance with enlarged fonts
//! and simplified layouts.

use crate::storage::config::MetricType;
use egui::{Context, Vec2};
use serde::{Deserialize, Serialize};

/// TV Mode layout configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
            primary_font_size: 72.0, // Readable from 3m on 65" TV
            secondary_font_size: 48.0,
        }
    }
}

impl TvModeLayout {
    /// Create a power-focused layout.
    pub fn power_focused() -> Self {
        Self {
            primary_metrics: vec![MetricType::Power, MetricType::Power3s],
            secondary_metrics: vec![MetricType::HeartRate, MetricType::Cadence],
            ..Default::default()
        }
    }

    /// Create a heart rate focused layout.
    pub fn hr_focused() -> Self {
        Self {
            primary_metrics: vec![MetricType::HeartRate, MetricType::HrZone],
            secondary_metrics: vec![MetricType::Power, MetricType::Duration],
            ..Default::default()
        }
    }

    /// Create a minimal layout with just power.
    pub fn minimal() -> Self {
        Self {
            primary_metrics: vec![MetricType::Power],
            secondary_metrics: vec![],
            show_progress: true,
            show_zone_indicator: true,
            primary_font_size: 96.0,
            secondary_font_size: 48.0,
        }
    }
}

/// TV Mode renderer.
pub struct TvModeRenderer {
    /// Layout configuration
    layout: TvModeLayout,
    /// Font scale multiplier
    font_scale: f32,
    /// Minimum button size
    min_button_size: Vec2,
    /// Spacing multiplier
    spacing_scale: f32,
    /// Low-priority metrics to hide
    hidden_metrics: Vec<MetricType>,
}

impl Default for TvModeRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TvModeRenderer {
    /// Create a new TV Mode renderer.
    pub fn new() -> Self {
        Self {
            layout: TvModeLayout::default(),
            font_scale: 2.0,
            min_button_size: Vec2::new(100.0, 60.0),
            spacing_scale: 1.5,
            hidden_metrics: vec![
                MetricType::Tss,
                MetricType::IntensityFactor,
                MetricType::Calories,
            ],
        }
    }

    /// Get the font scale multiplier.
    pub fn font_scale(&self) -> f32 {
        self.font_scale
    }

    /// Set the font scale.
    pub fn set_font_scale(&mut self, scale: f32) {
        self.font_scale = scale.clamp(1.5, 3.0);
    }

    /// Get the minimum button size.
    pub fn min_button_size(&self) -> Vec2 {
        self.min_button_size
    }

    /// Get the spacing multiplier.
    pub fn spacing_scale(&self) -> f32 {
        self.spacing_scale
    }

    /// Check if a metric should be hidden in TV Mode.
    pub fn should_hide_metric(&self, metric: MetricType) -> bool {
        self.hidden_metrics.contains(&metric)
    }

    /// Get the layout.
    pub fn layout(&self) -> &TvModeLayout {
        &self.layout
    }

    /// Set the layout.
    pub fn set_layout(&mut self, layout: TvModeLayout) {
        self.layout = layout;
    }

    /// Apply TV Mode styling to the context.
    pub fn apply_style(&self, ctx: &Context) {
        let mut style = (*ctx.style()).clone();

        // Scale fonts
        style.text_styles.iter_mut().for_each(|(_, font_id)| {
            font_id.size *= self.font_scale;
        });

        // Increase spacing
        style.spacing.item_spacing *= self.spacing_scale;
        style.spacing.button_padding *= self.spacing_scale;

        ctx.set_style(style);
    }

    /// Revert TV Mode styling.
    pub fn revert_style(&self, ctx: &Context) {
        let mut style = (*ctx.style()).clone();

        // Restore fonts
        style.text_styles.iter_mut().for_each(|(_, font_id)| {
            font_id.size /= self.font_scale;
        });

        // Restore spacing
        style.spacing.item_spacing /= self.spacing_scale;
        style.spacing.button_padding /= self.spacing_scale;

        ctx.set_style(style);
    }

    /// Get the primary font size.
    pub fn primary_font_size(&self) -> f32 {
        self.layout.primary_font_size
    }

    /// Get the secondary font size.
    pub fn secondary_font_size(&self) -> f32 {
        self.layout.secondary_font_size
    }

    /// Set a metric to be hidden.
    pub fn hide_metric(&mut self, metric: MetricType) {
        if !self.hidden_metrics.contains(&metric) {
            self.hidden_metrics.push(metric);
        }
    }

    /// Set a metric to be shown.
    pub fn show_metric(&mut self, metric: MetricType) {
        self.hidden_metrics.retain(|m| *m != metric);
    }
}

/// Trait for TV Mode rendering.
pub trait TvModeRendererTrait {
    /// Get the font scale multiplier.
    fn font_scale(&self) -> f32;

    /// Get the minimum button size.
    fn min_button_size(&self) -> Vec2;

    /// Get the spacing multiplier.
    fn spacing_scale(&self) -> f32;

    /// Check if a metric should be hidden.
    fn should_hide_metric(&self, metric: MetricType) -> bool;

    /// Get the layout configuration.
    fn tv_layout(&self) -> &TvModeLayout;

    /// Apply TV Mode styling.
    fn apply_tv_style(&self, ctx: &Context);

    /// Revert TV Mode styling.
    fn revert_tv_style(&self, ctx: &Context);
}

impl TvModeRendererTrait for TvModeRenderer {
    fn font_scale(&self) -> f32 {
        self.font_scale
    }

    fn min_button_size(&self) -> Vec2 {
        self.min_button_size
    }

    fn spacing_scale(&self) -> f32 {
        self.spacing_scale
    }

    fn should_hide_metric(&self, metric: MetricType) -> bool {
        TvModeRenderer::should_hide_metric(self, metric)
    }

    fn tv_layout(&self) -> &TvModeLayout {
        &self.layout
    }

    fn apply_tv_style(&self, ctx: &Context) {
        self.apply_style(ctx);
    }

    fn revert_tv_style(&self, ctx: &Context) {
        self.revert_style(ctx);
    }
}
