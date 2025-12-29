//! Level up notification widget.
//!
//! T077: Create level up notification widget.
//!
//! Displays a celebratory notification when the user levels up,
//! showing new level, rewards unlocked, and progress to next milestone.

use std::time::Instant;

use egui::{Color32, Pos2, Rect, RichText, Stroke, StrokeKind, Ui, Vec2};

use crate::career::{is_milestone_level, level_title, CareerEventQueue, LevelUpEvent, RewardType};

/// Configuration for level up notification display.
#[derive(Debug, Clone)]
pub struct LevelUpNotificationConfig {
    /// Width of the notification popup.
    pub width: f32,
    /// Height of the notification popup.
    pub height: f32,
    /// Time to display before auto-dismissing (seconds).
    pub display_duration_secs: f32,
    /// Animation duration for slide-in/out.
    pub animation_duration_secs: f32,
}

impl Default for LevelUpNotificationConfig {
    fn default() -> Self {
        Self {
            width: 380.0,
            height: 200.0,
            display_duration_secs: 8.0,
            animation_duration_secs: 0.4,
        }
    }
}

/// Widget for displaying level up notifications.
pub struct LevelUpNotificationWidget {
    /// Display configuration.
    config: LevelUpNotificationConfig,
    /// Animation progress (0.0 to 1.0).
    animation_progress: f32,
    /// Whether currently animating in.
    animating_in: bool,
    /// Animation start time.
    animation_start: Option<Instant>,
    /// Display start time (for auto-dismiss).
    display_start: Option<Instant>,
    /// Current event being displayed.
    current_event: Option<LevelUpEvent>,
}

impl Default for LevelUpNotificationWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl LevelUpNotificationWidget {
    /// Create a new level up notification widget.
    pub fn new() -> Self {
        Self {
            config: LevelUpNotificationConfig::default(),
            animation_progress: 0.0,
            animating_in: true,
            animation_start: None,
            display_start: None,
            current_event: None,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: LevelUpNotificationConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Show the notification if there's a level up event.
    ///
    /// Call this once per frame to display any pending level up events.
    pub fn show(&mut self, ui: &mut Ui, queue: &mut CareerEventQueue) {
        // Check if we need a new event
        if self.current_event.is_none() {
            if let Some(event) = queue.pop_level_up() {
                self.current_event = Some(event);
                self.animation_start = Some(Instant::now());
                self.display_start = Some(Instant::now());
                self.animating_in = true;
                self.animation_progress = 0.0;
            }
        }

        // No event to display
        let event = match &self.current_event {
            Some(e) => e.clone(),
            None => return,
        };

        // Check auto-dismiss
        if let Some(start) = self.display_start {
            if start.elapsed().as_secs_f32() > self.config.display_duration_secs {
                self.current_event = None;
                return;
            }
        }

        // Update animation
        self.update_animation();

        // Calculate center position
        let screen_rect = ui.ctx().available_rect();
        let center = screen_rect.center();

        // Apply scale animation
        let scale = self.animation_progress;
        let scaled_width = self.config.width * scale;
        let scaled_height = self.config.height * scale;
        let scaled_x = center.x - scaled_width / 2.0;
        let scaled_y = center.y - scaled_height / 2.0;

        let rect = Rect::from_min_size(
            Pos2::new(scaled_x, scaled_y),
            Vec2::new(scaled_width, scaled_height),
        );

        // Draw semi-transparent backdrop
        if self.animation_progress > 0.5 {
            let backdrop_alpha = ((self.animation_progress - 0.5) * 2.0 * 128.0) as u8;
            ui.painter().rect_filled(
                screen_rect,
                0.0,
                Color32::from_rgba_unmultiplied(0, 0, 0, backdrop_alpha),
            );
        }

        // Determine colors based on milestone status
        let is_milestone = is_milestone_level(event.new_level);
        let (bg_color, border_color, title_color) = if is_milestone {
            (
                Color32::from_rgba_unmultiplied(139, 117, 0, 240), // Gold background
                Color32::from_rgb(255, 215, 0),                    // Gold border
                Color32::from_rgb(255, 255, 200),                  // Light gold text
            )
        } else {
            (
                Color32::from_rgba_unmultiplied(30, 60, 100, 240), // Blue background
                Color32::from_rgb(100, 149, 237),                  // Cornflower blue
                Color32::WHITE,
            )
        };

        // Draw notification panel
        ui.painter().rect_filled(rect, 12.0, bg_color);
        ui.painter().rect_stroke(
            rect,
            12.0,
            Stroke::new(3.0, border_color),
            StrokeKind::Middle,
        );

        // Draw content if animation is past threshold
        if self.animation_progress > 0.3 {
            let content_rect = rect.shrink(16.0);
            let mut ui_child = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));

            self.draw_content(
                &mut ui_child,
                &event,
                is_milestone,
                title_color,
                border_color,
            );
        }

        // Request repaint for animation
        ui.ctx().request_repaint();
    }

    /// Update animation state.
    fn update_animation(&mut self) {
        if let Some(start) = self.animation_start {
            let elapsed = start.elapsed().as_secs_f32();
            let duration = self.config.animation_duration_secs;

            if self.animating_in {
                self.animation_progress = (elapsed / duration).clamp(0.0, 1.0);
                // Apply easing (ease-out-back for bounce effect)
                self.animation_progress = ease_out_back(self.animation_progress);
            }
        }
    }

    /// Draw notification content.
    fn draw_content(
        &self,
        ui: &mut Ui,
        event: &LevelUpEvent,
        is_milestone: bool,
        title_color: Color32,
        accent_color: Color32,
    ) {
        ui.vertical_centered(|ui| {
            // Header
            let header = if is_milestone {
                "MILESTONE REACHED!"
            } else {
                "LEVEL UP!"
            };
            ui.label(
                RichText::new(header)
                    .color(accent_color)
                    .strong()
                    .size(14.0),
            );

            ui.add_space(8.0);

            // Level number
            ui.label(
                RichText::new(format!("Level {}", event.new_level))
                    .color(title_color)
                    .strong()
                    .size(36.0),
            );

            // Level title
            let title = level_title(event.new_level);
            ui.label(RichText::new(title).color(accent_color).size(18.0));

            ui.add_space(8.0);

            // XP info
            ui.label(
                RichText::new(format!("+{} XP", event.xp_gained))
                    .color(Color32::from_rgb(255, 215, 0))
                    .strong(),
            );

            // Celebration message
            let msg = event.celebration_message();
            ui.add_space(4.0);
            ui.label(RichText::new(msg).italics().color(Color32::from_gray(200)));

            // Unlocked rewards
            if !event.unlocked_rewards.is_empty() {
                ui.add_space(8.0);
                ui.label(RichText::new("Rewards Unlocked:").strong().size(12.0));

                for reward in &event.unlocked_rewards {
                    ui.horizontal(|ui| {
                        let type_emoji = reward_type_emoji(reward.reward_type);
                        ui.label(RichText::new(type_emoji).size(14.0));
                        ui.label(RichText::new(&reward.name).color(accent_color));
                    });
                }
            }
        });
    }

    /// Dismiss the current notification.
    pub fn dismiss(&mut self) {
        self.current_event = None;
    }

    /// Check if a notification is currently showing.
    pub fn is_showing(&self) -> bool {
        self.current_event.is_some()
    }
}

/// Get emoji for reward type.
fn reward_type_emoji(reward_type: RewardType) -> &'static str {
    match reward_type {
        RewardType::JerseyColor => "\u{1F455}",  // T-shirt
        RewardType::BikeFrame => "\u{1F6B2}",    // Bicycle
        RewardType::UiTheme => "\u{1F3A8}",      // Palette
        RewardType::AccentColor => "\u{1F308}",  // Rainbow
        RewardType::ProfileBadge => "\u{1F396}", // Medal
        RewardType::WheelStyle => "\u{26AA}",    // Circle
        RewardType::HelmetStyle => "\u{26D1}",   // Helmet
    }
}

/// Ease out back function for bounce animation.
fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

/// Compact level progress bar widget.
pub struct LevelProgressBar;

impl LevelProgressBar {
    /// Show a compact level progress bar.
    pub fn show(ui: &mut Ui, current_level: u32, progress: f32, xp_to_next: u64) {
        let available_width = ui.available_width().min(200.0);

        ui.horizontal(|ui| {
            // Level indicator
            ui.label(
                RichText::new(format!("Lv.{}", current_level))
                    .strong()
                    .color(Color32::from_rgb(100, 149, 237)),
            );

            // Progress bar
            let (rect, _) = ui
                .allocate_exact_size(Vec2::new(available_width - 80.0, 8.0), egui::Sense::hover());

            // Background
            ui.painter().rect_filled(rect, 4.0, Color32::from_gray(60));

            // Fill
            let filled_width = rect.width() * progress;
            let filled_rect = Rect::from_min_size(rect.min, Vec2::new(filled_width, rect.height()));
            ui.painter()
                .rect_filled(filled_rect, 4.0, Color32::from_rgb(100, 149, 237));

            // XP to next
            ui.label(RichText::new(format!("{} XP", xp_to_next)).small().weak());
        });
    }
}

/// Mini level badge for use in headers/profiles.
pub struct LevelBadge;

impl LevelBadge {
    /// Show a mini level badge.
    pub fn show(ui: &mut Ui, level: u32, is_milestone: bool) {
        let (bg_color, text_color) = if is_milestone {
            (
                Color32::from_rgb(139, 117, 0),
                Color32::from_rgb(255, 215, 0),
            )
        } else {
            (
                Color32::from_rgb(30, 60, 100),
                Color32::from_rgb(100, 149, 237),
            )
        };

        egui::Frame::new()
            .fill(bg_color)
            .corner_radius(12.0)
            .inner_margin(egui::vec2(8.0, 2.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(format!("Lv.{}", level))
                        .color(text_color)
                        .strong()
                        .size(12.0),
                );
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ease_out_back() {
        assert!((ease_out_back(0.0) - 0.0).abs() < 0.001);
        assert!((ease_out_back(1.0) - 1.0).abs() < 0.001);
        // Ease out back should overshoot slightly
        assert!(ease_out_back(0.9) > 0.9);
    }

    #[test]
    fn test_reward_type_emoji() {
        assert_eq!(reward_type_emoji(RewardType::JerseyColor), "\u{1F455}");
        assert_eq!(reward_type_emoji(RewardType::BikeFrame), "\u{1F6B2}");
        assert_eq!(reward_type_emoji(RewardType::ProfileBadge), "\u{1F396}");
    }

    #[test]
    fn test_config_defaults() {
        let config = LevelUpNotificationConfig::default();
        assert!(config.width > 0.0);
        assert!(config.height > 0.0);
        assert!(config.display_duration_secs > 0.0);
    }

    #[test]
    fn test_widget_creation() {
        let widget = LevelUpNotificationWidget::new();
        assert!(!widget.is_showing());
    }
}
