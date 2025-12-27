//! Accessibility Module Contract
//!
//! Public API for the accessibility subsystem including focus management,
//! screen reader integration, colorblind palettes, and voice control.

use egui::{Color32, Context, Key, Response};

// ============================================================================
// Focus Management
// ============================================================================

/// Manages keyboard focus and navigation order.
pub trait FocusManager {
    /// Register a widget in the focus order.
    fn register(&mut self, id: egui::Id, order: u32);

    /// Move focus to the next widget in order.
    fn focus_next(&mut self, ctx: &Context);

    /// Move focus to the previous widget in order.
    fn focus_prev(&mut self, ctx: &Context);

    /// Trap focus within a modal/overlay (for accessibility).
    fn trap_focus(&mut self, container_id: egui::Id);

    /// Release focus trap.
    fn release_focus_trap(&mut self);

    /// Get the currently focused widget ID.
    fn current_focus(&self) -> Option<egui::Id>;
}

/// Keyboard shortcut registration and handling.
pub trait KeyboardShortcuts {
    /// Register a global keyboard shortcut.
    fn register_shortcut(&mut self, key: Key, modifiers: Modifiers, action: ShortcutAction);

    /// Handle keyboard input, returning true if a shortcut was triggered.
    fn handle_input(&mut self, ctx: &Context) -> Option<ShortcutAction>;

    /// Show the shortcut overlay (triggered by ? or F1).
    fn show_shortcut_overlay(&self, ctx: &Context);

    /// Get all registered shortcuts for display.
    fn list_shortcuts(&self) -> Vec<ShortcutInfo>;
}

pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

#[derive(Clone, Debug)]
pub enum ShortcutAction {
    ShowHelp,
    ToggleFlowMode,
    TogglePause,
    SkipInterval,
    ToggleFullscreen,
    FocusSearch,
    Custom(String),
}

pub struct ShortcutInfo {
    pub key: Key,
    pub modifiers: Modifiers,
    pub action: ShortcutAction,
    pub description: String,
}

// ============================================================================
// Screen Reader Support
// ============================================================================

/// Screen reader announcement and accessibility tree management.
pub trait ScreenReaderSupport {
    /// Announce a message to the screen reader (live region).
    fn announce(&self, message: &str, priority: AnnouncementPriority);

    /// Announce current metrics on demand (hotkey triggered).
    fn announce_metrics(&self, power: u16, heart_rate: Option<u8>, cadence: Option<u8>);

    /// Set accessible label for a widget.
    fn set_accessible_label(response: &Response, label: &str);

    /// Set accessible description for a widget.
    fn set_accessible_description(response: &Response, description: &str);

    /// Mark a region as a live region for dynamic updates.
    fn mark_live_region(&self, id: egui::Id, politeness: LiveRegionPoliteness);
}

#[derive(Clone, Copy, Debug)]
pub enum AnnouncementPriority {
    /// Polite: Wait for current speech to finish
    Polite,
    /// Assertive: Interrupt current speech
    Assertive,
}

#[derive(Clone, Copy, Debug)]
pub enum LiveRegionPoliteness {
    Off,
    Polite,
    Assertive,
}

// ============================================================================
// Colorblind Palettes
// ============================================================================

/// Colorblind-safe color palette provider.
pub trait ColorPaletteProvider {
    /// Get the zone color for a given zone number (1-7).
    fn zone_color(&self, zone: u8) -> Color32;

    /// Get all zone colors as an array.
    fn all_zone_colors(&self) -> [Color32; 7];

    /// Get a distinguishable color for charts (by index).
    fn chart_color(&self, index: usize) -> Color32;

    /// Check if the current palette requires pattern fills.
    fn requires_patterns(&self) -> bool;

    /// Get the pattern type for a zone (for charts).
    fn zone_pattern(&self, zone: u8) -> PatternType;
}

#[derive(Clone, Copy, Debug)]
pub enum PatternType {
    Solid,
    Horizontal,
    Vertical,
    Diagonal,
    Dots,
    CrossHatch,
}

/// Factory for creating color palettes.
pub fn create_palette(mode: ColorMode) -> Box<dyn ColorPaletteProvider>;

// ============================================================================
// High Contrast Theme
// ============================================================================

/// High contrast theme provider meeting WCAG AAA (7:1 contrast).
pub trait HighContrastTheme {
    /// Get the background color.
    fn background(&self) -> Color32;

    /// Get the foreground (text) color.
    fn foreground(&self) -> Color32;

    /// Get the accent color.
    fn accent(&self) -> Color32;

    /// Get the error color.
    fn error(&self) -> Color32;

    /// Check if a color pair meets AAA contrast (7:1).
    fn meets_aaa_contrast(fg: Color32, bg: Color32) -> bool;

    /// Calculate contrast ratio between two colors.
    fn contrast_ratio(fg: Color32, bg: Color32) -> f32;
}

// ============================================================================
// Voice Control
// ============================================================================

/// Voice command recognition and handling.
pub trait VoiceControl {
    /// Initialize voice recognition (may download model on first run).
    fn initialize(&mut self) -> Result<(), VoiceControlError>;

    /// Check if voice control is available and ready.
    fn is_available(&self) -> bool;

    /// Start listening for commands.
    fn start_listening(&mut self) -> Result<(), VoiceControlError>;

    /// Stop listening for commands.
    fn stop_listening(&mut self);

    /// Poll for recognized commands (non-blocking).
    fn poll_command(&mut self) -> Option<VoiceCommand>;

    /// Get the current listening state.
    fn state(&self) -> VoiceControlState;

    /// Get the last error message (for display).
    fn last_error(&self) -> Option<&str>;
}

#[derive(Clone, Debug)]
pub enum VoiceCommand {
    StartRide,
    PauseRide,
    ResumeRide,
    EndRide,
    SkipInterval,
    Unknown(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceControlState {
    Uninitialized,
    Initializing,
    Ready,
    Listening,
    Processing,
    Unavailable,
    Error,
}

#[derive(Debug, thiserror::Error)]
pub enum VoiceControlError {
    #[error("Model download failed: {0}")]
    ModelDownloadFailed(String),

    #[error("Audio device not available")]
    AudioDeviceNotAvailable,

    #[error("Recognition engine failed: {0}")]
    RecognitionFailed(String),

    #[error("Voice control not supported on this platform")]
    NotSupported,
}
