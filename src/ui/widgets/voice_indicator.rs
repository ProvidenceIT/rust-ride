//! Voice control indicator widget.
//!
//! T129: Shows voice control status and provides visual feedback when unavailable.
//! T130: Connect VoiceEngine state changes to visual feedback.
//!
//! ## Architecture
//!
//! The widget provides visual feedback for voice control state:
//!
//! ```text
//! VoiceEngine ─────────────────────────────────────────────────────────┐
//!     │                                                                │
//!     ▼ (events via VoiceIndicatorBridge)                              │
//! ┌─────────────────────────────────────────────────────────────────┐  │
//! │ VoiceIndicatorBridge                                             │  │
//! │  - Subscribes to VoiceEngineEvent                                │  │
//! │  - Maps VoiceEngineState -> VoiceIndicatorState                  │  │
//! │  - Tracks partial/recognized text with timeout                   │  │
//! └───────────────────────────┬─────────────────────────────────────┘  │
//!                             │                                        │
//!                             ▼                                        │
//! ┌─────────────────────────────────────────────────────────────────┐  │
//! │ VoiceIndicator Widget                                            │  │
//! │  - Shows state (icon + animation)                                │  │
//! │  - Shows partial/recognized text briefly                         │  │
//! │  - Shows confirmation popup with command                         │  │
//! └─────────────────────────────────────────────────────────────────┘  │
//! └────────────────────────────────────────────────────────────────────┘
//! ```

#[cfg(feature = "voice-control")]
use crate::accessibility::voice_control::{
    CommandAudioCue, VoiceCommand, VoiceControlState, VoskVoiceControl,
};

#[cfg(feature = "voice-control")]
use crate::voice::engine::{VoiceEngineEvent, VoiceEngineState};

#[cfg(feature = "voice-control")]
use std::sync::{Arc, RwLock};

#[cfg(feature = "voice-control")]
use std::time::{Duration, Instant};

#[cfg(feature = "voice-control")]
use tokio::sync::broadcast;

use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

/// Voice control indicator widget.
pub struct VoiceIndicator {
    /// Current voice control state
    state: VoiceIndicatorState,
    /// Reason for unavailability (if any)
    unavailable_reason: Option<String>,
    /// Whether to show detailed tooltip
    show_tooltip: bool,
    /// Confirmation message to display
    confirmation_message: Option<String>,
    /// Audio cue type for confirmation
    confirmation_cue: Option<ConfirmationCue>,
    /// Partial text being recognized (shown during listening)
    partial_text: Option<String>,
    /// Recognized text to display briefly
    recognized_text: Option<String>,
    /// Whether wake word is active (for visual indicator)
    wake_word_active: bool,
}

/// Voice indicator state (mirrors VoiceControlState for non-feature builds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VoiceIndicatorState {
    /// Voice control is not initialized
    #[default]
    Uninitialized,
    /// Voice control is initializing
    Initializing,
    /// Voice control is ready
    Ready,
    /// Voice control is listening
    Listening,
    /// Voice control is unavailable
    Unavailable,
    /// Voice control encountered an error
    Error,
}

#[cfg(feature = "voice-control")]
impl From<VoiceControlState> for VoiceIndicatorState {
    fn from(state: VoiceControlState) -> Self {
        match state {
            VoiceControlState::Uninitialized => VoiceIndicatorState::Uninitialized,
            VoiceControlState::Initializing => VoiceIndicatorState::Initializing,
            VoiceControlState::Ready => VoiceIndicatorState::Ready,
            VoiceControlState::Listening => VoiceIndicatorState::Listening,
            VoiceControlState::Unavailable => VoiceIndicatorState::Unavailable,
            VoiceControlState::Error => VoiceIndicatorState::Error,
        }
    }
}

/// Convert VoiceEngineState to VoiceIndicatorState.
///
/// This mapping allows the UI to reflect the engine's internal state.
#[cfg(feature = "voice-control")]
impl From<VoiceEngineState> for VoiceIndicatorState {
    fn from(state: VoiceEngineState) -> Self {
        match state {
            VoiceEngineState::Uninitialized => VoiceIndicatorState::Uninitialized,
            VoiceEngineState::Ready => VoiceIndicatorState::Ready,
            VoiceEngineState::Listening => VoiceIndicatorState::Listening,
            VoiceEngineState::Paused => VoiceIndicatorState::Ready, // Paused shows as Ready
            VoiceEngineState::Error => VoiceIndicatorState::Error,
            VoiceEngineState::ShuttingDown => VoiceIndicatorState::Uninitialized,
        }
    }
}

/// Confirmation cue type for visual feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationCue {
    /// Positive confirmation (start, resume)
    Positive,
    /// Neutral confirmation (pause, end)
    Neutral,
    /// Action taken (skip)
    Action,
    /// Adjustment made (increase, decrease)
    Adjustment,
    /// Information provided (status)
    Info,
    /// Error/unrecognized
    Error,
}

#[cfg(feature = "voice-control")]
impl From<CommandAudioCue> for ConfirmationCue {
    fn from(cue: CommandAudioCue) -> Self {
        match cue {
            CommandAudioCue::Positive => ConfirmationCue::Positive,
            CommandAudioCue::Neutral => ConfirmationCue::Neutral,
            CommandAudioCue::Action => ConfirmationCue::Action,
            CommandAudioCue::Adjustment => ConfirmationCue::Adjustment,
            CommandAudioCue::Info => ConfirmationCue::Info,
            CommandAudioCue::Error => ConfirmationCue::Error,
        }
    }
}

impl Default for VoiceIndicator {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceIndicator {
    /// Create a new voice indicator.
    pub fn new() -> Self {
        Self {
            state: VoiceIndicatorState::Uninitialized,
            unavailable_reason: None,
            show_tooltip: true,
            confirmation_message: None,
            confirmation_cue: None,
            partial_text: None,
            recognized_text: None,
            wake_word_active: false,
        }
    }

    /// Set the voice control state.
    pub fn with_state(mut self, state: VoiceIndicatorState) -> Self {
        self.state = state;
        self
    }

    /// Set the unavailability reason.
    pub fn with_unavailable_reason(mut self, reason: impl Into<String>) -> Self {
        self.unavailable_reason = Some(reason.into());
        self
    }

    /// Enable or disable tooltip.
    pub fn with_tooltip(mut self, show: bool) -> Self {
        self.show_tooltip = show;
        self
    }

    /// Set a confirmation message to display.
    pub fn with_confirmation(mut self, message: impl Into<String>, cue: ConfirmationCue) -> Self {
        self.confirmation_message = Some(message.into());
        self.confirmation_cue = Some(cue);
        self
    }

    /// Set partial text being recognized.
    ///
    /// This text is shown during active listening to provide feedback
    /// about what speech is being recognized in real-time.
    pub fn with_partial_text(mut self, text: impl Into<String>) -> Self {
        self.partial_text = Some(text.into());
        self
    }

    /// Set recognized text to display briefly.
    ///
    /// This shows the final recognized text before it becomes a command.
    pub fn with_recognized_text(mut self, text: impl Into<String>) -> Self {
        self.recognized_text = Some(text.into());
        self
    }

    /// Set whether wake word is active.
    ///
    /// When active, shows a distinct visual indicator that the system
    /// is actively listening for commands (vs just listening for wake word).
    pub fn with_wake_word_active(mut self, active: bool) -> Self {
        self.wake_word_active = active;
        self
    }

    /// Clear partial text.
    pub fn clear_partial_text(mut self) -> Self {
        self.partial_text = None;
        self
    }

    /// Clear recognized text.
    pub fn clear_recognized_text(mut self) -> Self {
        self.recognized_text = None;
        self
    }

    /// Get the current partial text, if any.
    pub fn partial_text(&self) -> Option<&str> {
        self.partial_text.as_deref()
    }

    /// Get the current recognized text, if any.
    pub fn recognized_text(&self) -> Option<&str> {
        self.recognized_text.as_deref()
    }

    /// Check if wake word is active.
    pub fn is_wake_word_active(&self) -> bool {
        self.wake_word_active
    }

    /// Show the voice indicator.
    pub fn show(&self, ui: &mut Ui) -> VoiceIndicatorResponse {
        let size = Vec2::new(32.0, 32.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());

        if ui.is_rect_visible(rect) {
            self.draw_indicator(ui, rect);
        }

        // Show tooltip on hover
        let response = if self.show_tooltip {
            response.on_hover_text(self.get_tooltip_text())
        } else {
            response
        };

        // Show partial text during listening (takes priority over confirmation)
        if let Some(ref partial) = self.partial_text {
            if self.state == VoiceIndicatorState::Listening {
                self.draw_partial_text_popup(ui, rect, partial);
            }
        }
        // Show recognized text briefly (takes priority over partial)
        else if let Some(ref recognized) = self.recognized_text {
            self.draw_recognized_text_popup(ui, rect, recognized);
        }
        // Show confirmation popup if present
        else if let Some(ref message) = self.confirmation_message {
            self.draw_confirmation_popup(ui, rect, message);
        }

        VoiceIndicatorResponse {
            response,
            is_available: matches!(
                self.state,
                VoiceIndicatorState::Ready | VoiceIndicatorState::Listening
            ),
        }
    }

    /// Draw the indicator icon.
    fn draw_indicator(&self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter();
        let center = rect.center();
        let radius = rect.width().min(rect.height()) / 2.0 - 2.0;

        // Background circle
        let (bg_color, icon_color) = self.get_colors();
        painter.circle_filled(center, radius, bg_color);

        // Draw microphone icon
        self.draw_microphone_icon(painter, center, radius * 0.5, icon_color);

        // Draw state-specific overlays
        match self.state {
            VoiceIndicatorState::Unavailable | VoiceIndicatorState::Error => {
                // Draw X overlay
                let offset = radius * 0.4;
                painter.line_segment(
                    [
                        Pos2::new(center.x - offset, center.y - offset),
                        Pos2::new(center.x + offset, center.y + offset),
                    ],
                    Stroke::new(2.0, Color32::from_rgb(200, 50, 50)),
                );
                painter.line_segment(
                    [
                        Pos2::new(center.x + offset, center.y - offset),
                        Pos2::new(center.x - offset, center.y + offset),
                    ],
                    Stroke::new(2.0, Color32::from_rgb(200, 50, 50)),
                );
            }
            VoiceIndicatorState::Listening => {
                // Draw pulsing animation circles
                let time = ui.ctx().input(|i| i.time);
                let pulse = ((time * 3.0).sin() * 0.5 + 0.5) as f32;

                // Enhanced animation when wake word is active (actively listening for commands)
                if self.wake_word_active {
                    // Brighter, faster pulse when actively listening
                    let fast_pulse = ((time * 5.0).sin() * 0.5 + 0.5) as f32;
                    let active_color =
                        Color32::from_rgba_unmultiplied(50, 255, 50, (fast_pulse * 150.0) as u8);

                    // Draw two concentric pulsing circles for active state
                    painter.circle_stroke(
                        center,
                        radius + 3.0 + fast_pulse * 4.0,
                        Stroke::new(2.0, active_color),
                    );
                    painter.circle_stroke(
                        center,
                        radius + 6.0 + fast_pulse * 3.0,
                        Stroke::new(1.0, Color32::from_rgba_unmultiplied(50, 200, 50, (fast_pulse * 80.0) as u8)),
                    );
                } else {
                    // Normal pulse when waiting for wake word
                    let pulse_color =
                        Color32::from_rgba_unmultiplied(100, 200, 100, (pulse * 100.0) as u8);
                    painter.circle_stroke(
                        center,
                        radius + 3.0 + pulse * 3.0,
                        Stroke::new(1.5, pulse_color),
                    );
                }
                ui.ctx().request_repaint(); // Continue animation
            }
            VoiceIndicatorState::Initializing => {
                // Draw loading indicator
                let time = ui.ctx().input(|i| i.time);
                let angle = (time * 2.0) as f32;
                let arc_start = Pos2::new(
                    center.x + (radius + 3.0) * angle.cos(),
                    center.y + (radius + 3.0) * angle.sin(),
                );
                let arc_end = Pos2::new(
                    center.x + (radius + 3.0) * (angle + 1.0).cos(),
                    center.y + (radius + 3.0) * (angle + 1.0).sin(),
                );
                painter.line_segment([arc_start, arc_end], Stroke::new(2.0, Color32::YELLOW));
                ui.ctx().request_repaint();
            }
            _ => {}
        }
    }

    /// Draw microphone icon.
    fn draw_microphone_icon(
        &self,
        painter: &egui::Painter,
        center: Pos2,
        size: f32,
        color: Color32,
    ) {
        // Simplified microphone shape
        let mic_width = size * 0.4;
        let mic_height = size * 0.7;

        // Microphone body (rounded rectangle)
        let mic_rect = Rect::from_center_size(
            Pos2::new(center.x, center.y - size * 0.15),
            Vec2::new(mic_width, mic_height),
        );
        painter.rect_filled(mic_rect, mic_width / 2.0, color);

        // Stand
        let stand_top = center.y + mic_height * 0.2;
        let stand_bottom = center.y + size * 0.5;
        painter.line_segment(
            [
                Pos2::new(center.x, stand_top),
                Pos2::new(center.x, stand_bottom),
            ],
            Stroke::new(1.5, color),
        );

        // Base
        painter.line_segment(
            [
                Pos2::new(center.x - size * 0.3, stand_bottom),
                Pos2::new(center.x + size * 0.3, stand_bottom),
            ],
            Stroke::new(1.5, color),
        );
    }

    /// Get colors based on state.
    fn get_colors(&self) -> (Color32, Color32) {
        match self.state {
            VoiceIndicatorState::Ready => (
                Color32::from_rgb(40, 80, 40),
                Color32::from_rgb(150, 220, 150),
            ),
            VoiceIndicatorState::Listening => (
                Color32::from_rgb(40, 100, 40),
                Color32::from_rgb(100, 255, 100),
            ),
            VoiceIndicatorState::Unavailable => (
                Color32::from_rgb(80, 40, 40),
                Color32::from_rgb(150, 100, 100),
            ),
            VoiceIndicatorState::Error => (
                Color32::from_rgb(100, 30, 30),
                Color32::from_rgb(200, 100, 100),
            ),
            VoiceIndicatorState::Initializing => (
                Color32::from_rgb(80, 80, 40),
                Color32::from_rgb(200, 200, 100),
            ),
            VoiceIndicatorState::Uninitialized => (
                Color32::from_rgb(60, 60, 60),
                Color32::from_rgb(120, 120, 120),
            ),
        }
    }

    /// Get tooltip text.
    fn get_tooltip_text(&self) -> String {
        let status_text = match self.state {
            VoiceIndicatorState::Ready => "Ready",
            VoiceIndicatorState::Listening => "Listening...",
            VoiceIndicatorState::Unavailable => "Unavailable",
            VoiceIndicatorState::Error => "Error",
            VoiceIndicatorState::Initializing => "Initializing...",
            VoiceIndicatorState::Uninitialized => "Not Initialized",
        };

        let mut tooltip = format!("Voice Control: {}", status_text);

        if let Some(ref reason) = self.unavailable_reason {
            tooltip.push_str(&format!("\n{}", reason));
        }

        if matches!(
            self.state,
            VoiceIndicatorState::Ready | VoiceIndicatorState::Listening
        ) {
            tooltip.push_str(
                "\nCommands: Start, Pause, Resume, End, Skip, Increase, Decrease, Status",
            );
        }

        tooltip
    }

    /// Draw confirmation popup.
    fn draw_confirmation_popup(&self, ui: &mut Ui, indicator_rect: Rect, message: &str) {
        let popup_size = Vec2::new(150.0, 40.0);
        let popup_rect = Rect::from_min_size(
            Pos2::new(
                indicator_rect.right() + 8.0,
                indicator_rect.center().y - popup_size.y / 2.0,
            ),
            popup_size,
        );

        let painter = ui.painter();

        // Background
        let bg_color =
            self.confirmation_cue
                .map_or(Color32::from_rgb(50, 50, 50), |cue| match cue {
                    ConfirmationCue::Positive => Color32::from_rgb(30, 70, 30),
                    ConfirmationCue::Neutral => Color32::from_rgb(50, 50, 60),
                    ConfirmationCue::Action => Color32::from_rgb(60, 50, 30),
                    ConfirmationCue::Adjustment => Color32::from_rgb(50, 50, 70),
                    ConfirmationCue::Info => Color32::from_rgb(40, 50, 70),
                    ConfirmationCue::Error => Color32::from_rgb(70, 30, 30),
                });

        painter.rect_filled(popup_rect, 4.0, bg_color);
        painter.rect_stroke(
            popup_rect,
            4.0,
            Stroke::new(1.0, Color32::from_gray(100)),
            StrokeKind::Middle,
        );

        // Text
        let text_color = Color32::WHITE;
        painter.text(
            popup_rect.center(),
            egui::Align2::CENTER_CENTER,
            message,
            egui::FontId::proportional(14.0),
            text_color,
        );
    }

    /// Draw partial text popup during recognition.
    ///
    /// This shows the text being recognized in real-time, giving users
    /// feedback that their speech is being captured.
    fn draw_partial_text_popup(&self, ui: &mut Ui, indicator_rect: Rect, text: &str) {
        // Truncate long text and add ellipsis
        let display_text = if text.len() > 30 {
            format!("{}...", &text[..27])
        } else {
            text.to_string()
        };

        let popup_size = Vec2::new(180.0, 36.0);
        let popup_rect = Rect::from_min_size(
            Pos2::new(
                indicator_rect.right() + 8.0,
                indicator_rect.center().y - popup_size.y / 2.0,
            ),
            popup_size,
        );

        let painter = ui.painter();

        // Semi-transparent background for in-progress recognition
        let bg_color = Color32::from_rgba_unmultiplied(40, 60, 40, 220);
        painter.rect_filled(popup_rect, 4.0, bg_color);
        painter.rect_stroke(
            popup_rect,
            4.0,
            Stroke::new(1.0, Color32::from_rgb(80, 120, 80)),
            StrokeKind::Middle,
        );

        // Pulsing text effect
        let time = ui.ctx().input(|i| i.time);
        let alpha = (((time * 2.0).sin() * 0.3 + 0.7) * 255.0) as u8;
        let text_color = Color32::from_rgba_unmultiplied(200, 255, 200, alpha);

        // Draw with italic style for partial text
        painter.text(
            popup_rect.center(),
            egui::Align2::CENTER_CENTER,
            &display_text,
            egui::FontId::proportional(12.0),
            text_color,
        );

        ui.ctx().request_repaint(); // Continue pulsing animation
    }

    /// Draw recognized text popup briefly.
    ///
    /// This shows the final recognized text before it's converted to a command,
    /// giving users confirmation of what was heard.
    fn draw_recognized_text_popup(&self, ui: &mut Ui, indicator_rect: Rect, text: &str) {
        // Truncate long text and add ellipsis
        let display_text = if text.len() > 35 {
            format!("{}...", &text[..32])
        } else {
            text.to_string()
        };

        let popup_size = Vec2::new(200.0, 40.0);
        let popup_rect = Rect::from_min_size(
            Pos2::new(
                indicator_rect.right() + 8.0,
                indicator_rect.center().y - popup_size.y / 2.0,
            ),
            popup_size,
        );

        let painter = ui.painter();

        // Solid background for recognized text (more prominent)
        let bg_color = Color32::from_rgb(35, 65, 35);
        painter.rect_filled(popup_rect, 4.0, bg_color);
        painter.rect_stroke(
            popup_rect,
            4.0,
            Stroke::new(1.5, Color32::from_rgb(100, 180, 100)),
            StrokeKind::Middle,
        );

        // Bright text for recognized text
        let text_color = Color32::from_rgb(220, 255, 220);
        painter.text(
            popup_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("\"{}\"", display_text),
            egui::FontId::proportional(13.0),
            text_color,
        );
    }
}

/// Response from showing a voice indicator.
pub struct VoiceIndicatorResponse {
    /// The egui response
    pub response: egui::Response,
    /// Whether voice control is available
    pub is_available: bool,
}

/// Compact voice indicator for use in status bars.
pub struct CompactVoiceIndicator {
    state: VoiceIndicatorState,
}

impl CompactVoiceIndicator {
    /// Create a new compact indicator.
    pub fn new(state: VoiceIndicatorState) -> Self {
        Self { state }
    }

    /// Show the compact indicator.
    pub fn show(&self, ui: &mut Ui) -> egui::Response {
        let size = Vec2::new(16.0, 16.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let center = rect.center();
            let radius = 6.0;

            let color = match self.state {
                VoiceIndicatorState::Ready | VoiceIndicatorState::Listening => {
                    Color32::from_rgb(100, 200, 100)
                }
                VoiceIndicatorState::Unavailable | VoiceIndicatorState::Error => {
                    Color32::from_rgb(200, 100, 100)
                }
                VoiceIndicatorState::Initializing => Color32::from_rgb(200, 200, 100),
                VoiceIndicatorState::Uninitialized => Color32::from_gray(100),
            };

            painter.circle_filled(center, radius, color);

            // Animate listening state
            if matches!(self.state, VoiceIndicatorState::Listening) {
                let time = ui.ctx().input(|i| i.time);
                let pulse = ((time * 3.0).sin() * 0.5 + 0.5) as f32;
                let pulse_color =
                    Color32::from_rgba_unmultiplied(100, 200, 100, (pulse * 80.0) as u8);
                painter.circle_stroke(
                    center,
                    radius + 2.0 + pulse * 2.0,
                    Stroke::new(1.0, pulse_color),
                );
                ui.ctx().request_repaint();
            }
        }

        response
    }
}

// ============================================================================
// VoiceIndicatorBridge - Connects VoiceEngine to VoiceIndicator
// ============================================================================

/// Default duration to show recognized text (milliseconds).
#[cfg(feature = "voice-control")]
const DEFAULT_RECOGNIZED_TEXT_DURATION_MS: u64 = 2000;

/// Default duration to show confirmation popup (milliseconds).
#[cfg(feature = "voice-control")]
const DEFAULT_CONFIRMATION_DURATION_MS: u64 = 2500;

/// Bridge state for connecting VoiceEngine events to VoiceIndicator.
///
/// This struct maintains the state needed to update the VoiceIndicator
/// based on events from the VoiceEngine. It handles:
///
/// - State mapping from VoiceEngineState to VoiceIndicatorState
/// - Partial text display during recognition
/// - Recognized text display with timeout
/// - Command confirmation with timeout
/// - Wake word activation state
#[cfg(feature = "voice-control")]
#[derive(Debug, Clone)]
pub struct VoiceIndicatorBridgeState {
    /// Current indicator state
    pub state: VoiceIndicatorState,
    /// Partial text being recognized
    pub partial_text: Option<String>,
    /// Recognized text (cleared after timeout)
    pub recognized_text: Option<String>,
    /// When recognized text was set (for timeout)
    pub recognized_text_time: Option<Instant>,
    /// Confirmation message
    pub confirmation_message: Option<String>,
    /// Confirmation cue type
    pub confirmation_cue: Option<ConfirmationCue>,
    /// When confirmation was set (for timeout)
    pub confirmation_time: Option<Instant>,
    /// Whether wake word is active
    pub wake_word_active: bool,
    /// Last error message
    pub last_error: Option<String>,
    /// Duration to show recognized text
    pub recognized_text_duration: Duration,
    /// Duration to show confirmation
    pub confirmation_duration: Duration,
}

#[cfg(feature = "voice-control")]
impl Default for VoiceIndicatorBridgeState {
    fn default() -> Self {
        Self {
            state: VoiceIndicatorState::Uninitialized,
            partial_text: None,
            recognized_text: None,
            recognized_text_time: None,
            confirmation_message: None,
            confirmation_cue: None,
            confirmation_time: None,
            wake_word_active: false,
            last_error: None,
            recognized_text_duration: Duration::from_millis(DEFAULT_RECOGNIZED_TEXT_DURATION_MS),
            confirmation_duration: Duration::from_millis(DEFAULT_CONFIRMATION_DURATION_MS),
        }
    }
}

#[cfg(feature = "voice-control")]
impl VoiceIndicatorBridgeState {
    /// Create a new bridge state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check and clear expired timeouts.
    ///
    /// Call this regularly (e.g., each frame) to clear timed-out text.
    pub fn update_timeouts(&mut self) {
        // Clear recognized text after timeout
        if let Some(time) = self.recognized_text_time {
            if time.elapsed() >= self.recognized_text_duration {
                self.recognized_text = None;
                self.recognized_text_time = None;
            }
        }

        // Clear confirmation after timeout
        if let Some(time) = self.confirmation_time {
            if time.elapsed() >= self.confirmation_duration {
                self.confirmation_message = None;
                self.confirmation_cue = None;
                self.confirmation_time = None;
            }
        }
    }

    /// Build a VoiceIndicator from the current state.
    pub fn build_indicator(&self) -> VoiceIndicator {
        let mut indicator = VoiceIndicator::new()
            .with_state(self.state)
            .with_wake_word_active(self.wake_word_active);

        if let Some(ref text) = self.partial_text {
            indicator = indicator.with_partial_text(text);
        }

        if let Some(ref text) = self.recognized_text {
            indicator = indicator.with_recognized_text(text);
        }

        if let (Some(ref msg), Some(cue)) = (&self.confirmation_message, self.confirmation_cue) {
            indicator = indicator.with_confirmation(msg, cue);
        }

        if let Some(ref reason) = self.last_error {
            indicator = indicator.with_unavailable_reason(reason);
        }

        indicator
    }

    /// Set the state.
    pub fn set_state(&mut self, state: VoiceIndicatorState) {
        self.state = state;
    }

    /// Set partial text.
    pub fn set_partial_text(&mut self, text: Option<String>) {
        self.partial_text = text;
    }

    /// Set recognized text with auto-timeout.
    pub fn set_recognized_text(&mut self, text: String) {
        self.recognized_text = Some(text);
        self.recognized_text_time = Some(Instant::now());
        // Clear partial text when we have a final result
        self.partial_text = None;
    }

    /// Set confirmation message with auto-timeout.
    pub fn set_confirmation(&mut self, message: String, cue: ConfirmationCue) {
        self.confirmation_message = Some(message);
        self.confirmation_cue = Some(cue);
        self.confirmation_time = Some(Instant::now());
        // Clear recognized text when showing confirmation
        self.recognized_text = None;
        self.recognized_text_time = None;
    }

    /// Set wake word active state.
    pub fn set_wake_word_active(&mut self, active: bool) {
        self.wake_word_active = active;
    }

    /// Set error state.
    pub fn set_error(&mut self, message: Option<String>) {
        self.last_error = message;
    }

    /// Clear all transient state (partial text, recognized text, confirmation).
    pub fn clear_transient(&mut self) {
        self.partial_text = None;
        self.recognized_text = None;
        self.recognized_text_time = None;
        self.confirmation_message = None;
        self.confirmation_cue = None;
        self.confirmation_time = None;
    }
}

/// Bridge that connects VoiceEngine events to VoiceIndicator updates.
///
/// This bridge subscribes to VoiceEngine events and maintains state
/// that can be used to build a VoiceIndicator for rendering.
///
/// # Example
///
/// ```rust,ignore
/// use rustride::ui::widgets::voice_indicator::VoiceIndicatorBridge;
/// use rustride::voice::VoiceEngine;
///
/// // Create bridge from engine
/// let engine = VoiceEngine::new(config)?;
/// let bridge = VoiceIndicatorBridge::from_engine(&engine);
///
/// // In your UI update loop:
/// bridge.poll_events(); // Process new events
/// bridge.update_timeouts(); // Clear expired text
///
/// // Build indicator for rendering
/// let indicator = bridge.build_indicator();
/// indicator.show(ui);
/// ```
#[cfg(feature = "voice-control")]
pub struct VoiceIndicatorBridge {
    /// Shared state
    state: Arc<RwLock<VoiceIndicatorBridgeState>>,
    /// Event receiver
    event_rx: Option<broadcast::Receiver<VoiceEngineEvent>>,
}

#[cfg(feature = "voice-control")]
impl VoiceIndicatorBridge {
    /// Create a new bridge with default state.
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(VoiceIndicatorBridgeState::new())),
            event_rx: None,
        }
    }

    /// Create a bridge and subscribe to a VoiceEngine.
    pub fn from_engine(engine: &crate::voice::VoiceEngine) -> Self {
        let mut bridge = Self::new();
        bridge.subscribe(engine);
        bridge
    }

    /// Subscribe to VoiceEngine events.
    pub fn subscribe(&mut self, engine: &crate::voice::VoiceEngine) {
        self.event_rx = Some(engine.subscribe());

        // Set initial state from engine
        let engine_state = engine.state();
        let mut state = self.state.write().unwrap();
        state.set_state(engine_state.into());
    }

    /// Get a clone of the shared state.
    pub fn state(&self) -> Arc<RwLock<VoiceIndicatorBridgeState>> {
        Arc::clone(&self.state)
    }

    /// Poll for new events and update state.
    ///
    /// Call this regularly (e.g., each frame) to process engine events.
    /// Returns the number of events processed.
    pub fn poll_events(&mut self) -> usize {
        let mut count = 0;

        if let Some(ref mut rx) = self.event_rx {
            loop {
                match rx.try_recv() {
                    Ok(event) => {
                        self.handle_event(event);
                        count += 1;
                    }
                    Err(broadcast::error::TryRecvError::Empty) => break,
                    Err(broadcast::error::TryRecvError::Lagged(n)) => {
                        tracing::warn!("VoiceIndicatorBridge lagged {} events", n);
                    }
                    Err(broadcast::error::TryRecvError::Closed) => {
                        self.event_rx = None;
                        break;
                    }
                }
            }
        }

        count
    }

    /// Handle a single VoiceEngine event.
    fn handle_event(&self, event: VoiceEngineEvent) {
        let mut state = self.state.write().unwrap();

        match event {
            VoiceEngineEvent::StateChanged { to, .. } => {
                state.set_state(to.into());
            }
            VoiceEngineEvent::PartialResult { text } => {
                if !text.is_empty() {
                    state.set_partial_text(Some(text));
                }
            }
            VoiceEngineEvent::FinalResult { text, .. } => {
                if !text.is_empty() {
                    state.set_recognized_text(text);
                }
            }
            VoiceEngineEvent::CommandRecognized { command, text, .. } => {
                // Show recognized text briefly
                state.set_recognized_text(text);

                // Then show confirmation after a short delay
                // (In practice, you might want to delay this slightly)
                let confirmation = VoskVoiceControl::command_confirmation(&command);
                let cue = VoskVoiceControl::command_audio_cue(&command).into();
                state.set_confirmation(confirmation.to_string(), cue);
            }
            VoiceEngineEvent::WakeWordDetected { .. } => {
                state.set_wake_word_active(true);
            }
            VoiceEngineEvent::WakeWordTimeout => {
                state.set_wake_word_active(false);
            }
            VoiceEngineEvent::WakeWordExtended { .. } => {
                // Keep wake word active
                state.set_wake_word_active(true);
            }
            VoiceEngineEvent::Error { message } => {
                state.set_state(VoiceIndicatorState::Error);
                state.set_error(Some(message));
            }
            VoiceEngineEvent::Initialized => {
                state.set_state(VoiceIndicatorState::Ready);
                state.set_error(None);
            }
            VoiceEngineEvent::Started => {
                state.set_state(VoiceIndicatorState::Listening);
            }
            VoiceEngineEvent::Stopped => {
                state.set_state(VoiceIndicatorState::Ready);
                state.clear_transient();
            }
            VoiceEngineEvent::CommandCooldown { .. } => {
                // Optionally show a visual indication that command was blocked
            }
            VoiceEngineEvent::ActivationModeChanged { .. } => {
                // Mode change doesn't affect indicator state
            }
            VoiceEngineEvent::AudioLevel { .. } => {
                // Audio level could be used for visualization
            }
        }
    }

    /// Update timeouts to clear expired text.
    ///
    /// Call this regularly (e.g., each frame) to ensure text clears.
    pub fn update_timeouts(&self) {
        self.state.write().unwrap().update_timeouts();
    }

    /// Build a VoiceIndicator from the current state.
    pub fn build_indicator(&self) -> VoiceIndicator {
        self.state.read().unwrap().build_indicator()
    }

    /// Get the current indicator state.
    pub fn indicator_state(&self) -> VoiceIndicatorState {
        self.state.read().unwrap().state
    }

    /// Check if wake word is active.
    pub fn is_wake_word_active(&self) -> bool {
        self.state.read().unwrap().wake_word_active
    }

    /// Get partial text being recognized.
    pub fn partial_text(&self) -> Option<String> {
        self.state.read().unwrap().partial_text.clone()
    }

    /// Manually set the indicator state.
    ///
    /// Useful for testing or when not connected to a VoiceEngine.
    pub fn set_state(&self, state: VoiceIndicatorState) {
        self.state.write().unwrap().set_state(state);
    }

    /// Manually set partial text.
    pub fn set_partial_text(&self, text: Option<String>) {
        self.state.write().unwrap().set_partial_text(text);
    }

    /// Manually set recognized text.
    pub fn set_recognized_text(&self, text: String) {
        self.state.write().unwrap().set_recognized_text(text);
    }

    /// Manually set confirmation.
    pub fn set_confirmation(&self, message: String, cue: ConfirmationCue) {
        self.state.write().unwrap().set_confirmation(message, cue);
    }

    /// Clear all transient state.
    pub fn clear_transient(&self) {
        self.state.write().unwrap().clear_transient();
    }
}

#[cfg(feature = "voice-control")]
impl Default for VoiceIndicatorBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_indicator_new_fields() {
        let indicator = VoiceIndicator::new();
        assert!(indicator.partial_text().is_none());
        assert!(indicator.recognized_text().is_none());
        assert!(!indicator.is_wake_word_active());
    }

    #[test]
    fn test_voice_indicator_with_partial_text() {
        let indicator = VoiceIndicator::new()
            .with_partial_text("hello");
        assert_eq!(indicator.partial_text(), Some("hello"));
    }

    #[test]
    fn test_voice_indicator_with_recognized_text() {
        let indicator = VoiceIndicator::new()
            .with_recognized_text("pause");
        assert_eq!(indicator.recognized_text(), Some("pause"));
    }

    #[test]
    fn test_voice_indicator_with_wake_word_active() {
        let indicator = VoiceIndicator::new()
            .with_wake_word_active(true);
        assert!(indicator.is_wake_word_active());
    }

    #[test]
    fn test_voice_indicator_clear_text() {
        let indicator = VoiceIndicator::new()
            .with_partial_text("hello")
            .clear_partial_text();
        assert!(indicator.partial_text().is_none());

        let indicator = VoiceIndicator::new()
            .with_recognized_text("pause")
            .clear_recognized_text();
        assert!(indicator.recognized_text().is_none());
    }

    #[cfg(feature = "voice-control")]
    mod bridge_tests {
        use super::*;

        #[test]
        fn test_bridge_state_default() {
            let state = VoiceIndicatorBridgeState::default();
            assert_eq!(state.state, VoiceIndicatorState::Uninitialized);
            assert!(state.partial_text.is_none());
            assert!(state.recognized_text.is_none());
            assert!(!state.wake_word_active);
        }

        #[test]
        fn test_bridge_state_set_recognized_text() {
            let mut state = VoiceIndicatorBridgeState::new();
            state.set_partial_text(Some("hel".to_string()));
            state.set_recognized_text("hello".to_string());

            // Recognized text should clear partial text
            assert!(state.partial_text.is_none());
            assert_eq!(state.recognized_text, Some("hello".to_string()));
            assert!(state.recognized_text_time.is_some());
        }

        #[test]
        fn test_bridge_state_set_confirmation() {
            let mut state = VoiceIndicatorBridgeState::new();
            state.set_recognized_text("pause".to_string());
            state.set_confirmation("Pausing".to_string(), ConfirmationCue::Neutral);

            // Confirmation should clear recognized text
            assert!(state.recognized_text.is_none());
            assert_eq!(state.confirmation_message, Some("Pausing".to_string()));
            assert_eq!(state.confirmation_cue, Some(ConfirmationCue::Neutral));
        }

        #[test]
        fn test_bridge_state_clear_transient() {
            let mut state = VoiceIndicatorBridgeState::new();
            state.set_partial_text(Some("test".to_string()));
            state.set_recognized_text("hello".to_string());
            state.set_confirmation("Done".to_string(), ConfirmationCue::Positive);

            state.clear_transient();

            assert!(state.partial_text.is_none());
            assert!(state.recognized_text.is_none());
            assert!(state.confirmation_message.is_none());
        }

        #[test]
        fn test_bridge_state_build_indicator() {
            let mut state = VoiceIndicatorBridgeState::new();
            state.set_state(VoiceIndicatorState::Listening);
            state.set_wake_word_active(true);
            state.set_partial_text(Some("hello".to_string()));

            let indicator = state.build_indicator();
            assert_eq!(indicator.state, VoiceIndicatorState::Listening);
            assert!(indicator.is_wake_word_active());
            assert_eq!(indicator.partial_text(), Some("hello"));
        }

        #[test]
        fn test_bridge_new() {
            let bridge = VoiceIndicatorBridge::new();
            assert_eq!(bridge.indicator_state(), VoiceIndicatorState::Uninitialized);
            assert!(!bridge.is_wake_word_active());
        }

        #[test]
        fn test_bridge_manual_set() {
            let bridge = VoiceIndicatorBridge::new();

            bridge.set_state(VoiceIndicatorState::Listening);
            assert_eq!(bridge.indicator_state(), VoiceIndicatorState::Listening);

            bridge.set_partial_text(Some("test".to_string()));
            assert_eq!(bridge.partial_text(), Some("test".to_string()));

            bridge.set_recognized_text("hello".to_string());
            // Partial text should be cleared
            assert!(bridge.partial_text().is_none());
        }

        #[test]
        fn test_voice_engine_state_to_indicator_state() {
            assert_eq!(
                VoiceIndicatorState::from(VoiceEngineState::Uninitialized),
                VoiceIndicatorState::Uninitialized
            );
            assert_eq!(
                VoiceIndicatorState::from(VoiceEngineState::Ready),
                VoiceIndicatorState::Ready
            );
            assert_eq!(
                VoiceIndicatorState::from(VoiceEngineState::Listening),
                VoiceIndicatorState::Listening
            );
            assert_eq!(
                VoiceIndicatorState::from(VoiceEngineState::Paused),
                VoiceIndicatorState::Ready
            );
            assert_eq!(
                VoiceIndicatorState::from(VoiceEngineState::Error),
                VoiceIndicatorState::Error
            );
            assert_eq!(
                VoiceIndicatorState::from(VoiceEngineState::ShuttingDown),
                VoiceIndicatorState::Uninitialized
            );
        }
    }
}
