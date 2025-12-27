//! Display modes module for TV Mode and Flow Mode.
//!
//! Provides specialized display configurations for different viewing scenarios.

pub mod flow_mode;
pub mod tv_mode;

use serde::{Deserialize, Serialize};

// Re-export types
pub use flow_mode::{FlowModeRenderer, FlowModeSettings};
pub use tv_mode::{TvModeLayout, TvModeRenderer};

/// Display mode options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DisplayMode {
    /// Normal display mode
    #[default]
    Normal,
    /// TV Mode - optimized for large displays at distance
    TvMode,
    /// Flow Mode - minimal distraction with single metric
    FlowMode,
}

impl std::fmt::Display for DisplayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayMode::Normal => write!(f, "Normal"),
            DisplayMode::TvMode => write!(f, "TV Mode"),
            DisplayMode::FlowMode => write!(f, "Flow Mode"),
        }
    }
}

/// Display mode manager for switching between modes.
pub struct DisplayModeManager {
    /// Current display mode
    current_mode: DisplayMode,
    /// Mode before entering Flow Mode (for restoration)
    pre_flow_mode: Option<DisplayMode>,
    /// TV Mode renderer
    tv_renderer: TvModeRenderer,
    /// Flow Mode renderer
    flow_renderer: FlowModeRenderer,
}

impl Default for DisplayModeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayModeManager {
    /// Create a new display mode manager.
    pub fn new() -> Self {
        Self {
            current_mode: DisplayMode::Normal,
            pre_flow_mode: None,
            tv_renderer: TvModeRenderer::new(),
            flow_renderer: FlowModeRenderer::new(),
        }
    }

    /// Get the current display mode.
    pub fn current_mode(&self) -> DisplayMode {
        self.current_mode
    }

    /// Set the display mode.
    pub fn set_mode(&mut self, mode: DisplayMode) {
        if mode == DisplayMode::FlowMode && self.current_mode != DisplayMode::FlowMode {
            self.pre_flow_mode = Some(self.current_mode);
        }
        self.current_mode = mode;
    }

    /// Toggle TV Mode on/off.
    pub fn toggle_tv_mode(&mut self) {
        if self.current_mode == DisplayMode::TvMode {
            self.current_mode = DisplayMode::Normal;
        } else {
            self.current_mode = DisplayMode::TvMode;
        }
    }

    /// Enter Flow Mode.
    pub fn enter_flow_mode(&mut self) {
        if self.current_mode != DisplayMode::FlowMode {
            self.pre_flow_mode = Some(self.current_mode);
            self.current_mode = DisplayMode::FlowMode;
        }
    }

    /// Exit Flow Mode (returns to previous mode).
    pub fn exit_flow_mode(&mut self) {
        if self.current_mode == DisplayMode::FlowMode {
            self.current_mode = self.pre_flow_mode.take().unwrap_or(DisplayMode::Normal);
        }
    }

    /// Check if in Flow Mode.
    pub fn is_flow_mode(&self) -> bool {
        self.current_mode == DisplayMode::FlowMode
    }

    /// Check if in TV Mode.
    pub fn is_tv_mode(&self) -> bool {
        self.current_mode == DisplayMode::TvMode
    }

    /// Get the mode before Flow Mode was entered.
    pub fn pre_flow_mode(&self) -> Option<DisplayMode> {
        self.pre_flow_mode
    }

    /// Get the TV Mode renderer.
    pub fn tv_renderer(&self) -> &TvModeRenderer {
        &self.tv_renderer
    }

    /// Get the TV Mode renderer mutably.
    pub fn tv_renderer_mut(&mut self) -> &mut TvModeRenderer {
        &mut self.tv_renderer
    }

    /// Get the Flow Mode renderer.
    pub fn flow_renderer(&self) -> &FlowModeRenderer {
        &self.flow_renderer
    }

    /// Get the Flow Mode renderer mutably.
    pub fn flow_renderer_mut(&mut self) -> &mut FlowModeRenderer {
        &mut self.flow_renderer
    }
}

/// Trait for display mode managers.
pub trait DisplayModeManagerTrait {
    /// Get the current display mode.
    fn current_mode(&self) -> DisplayMode;

    /// Set the display mode.
    fn set_mode(&mut self, mode: DisplayMode);

    /// Toggle TV Mode on/off.
    fn toggle_tv_mode(&mut self);

    /// Enter Flow Mode.
    fn enter_flow_mode(&mut self);

    /// Exit Flow Mode.
    fn exit_flow_mode(&mut self);

    /// Check if in Flow Mode.
    fn is_flow_mode(&self) -> bool;

    /// Check if in TV Mode.
    fn is_tv_mode(&self) -> bool;
}

impl DisplayModeManagerTrait for DisplayModeManager {
    fn current_mode(&self) -> DisplayMode {
        self.current_mode
    }

    fn set_mode(&mut self, mode: DisplayMode) {
        DisplayModeManager::set_mode(self, mode);
    }

    fn toggle_tv_mode(&mut self) {
        DisplayModeManager::toggle_tv_mode(self);
    }

    fn enter_flow_mode(&mut self) {
        DisplayModeManager::enter_flow_mode(self);
    }

    fn exit_flow_mode(&mut self) {
        DisplayModeManager::exit_flow_mode(self);
    }

    fn is_flow_mode(&self) -> bool {
        DisplayModeManager::is_flow_mode(self)
    }

    fn is_tv_mode(&self) -> bool {
        DisplayModeManager::is_tv_mode(self)
    }
}
