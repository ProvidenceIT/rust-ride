//! Voice control indicator widget.
//!
//! T129: Shows voice control status and provides visual feedback when unavailable.

#[cfg(feature = "voice-control")]
use crate::accessibility::voice_control::{CommandAudioCue, VoiceControlState};

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

        // Show confirmation popup if present
        if let Some(ref message) = self.confirmation_message {
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
                let pulse_color =
                    Color32::from_rgba_unmultiplied(100, 200, 100, (pulse * 100.0) as u8);
                painter.circle_stroke(
                    center,
                    radius + 3.0 + pulse * 3.0,
                    Stroke::new(1.5, pulse_color),
                );
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
