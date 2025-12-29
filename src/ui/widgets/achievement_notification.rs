//! Achievement notification widget.
//!
//! T041: Create achievement notification widget.
//!
//! Displays a notification when achievements are earned, with XP animation.

use std::time::Instant;

use egui::{Color32, Pos2, Rect, RichText, Stroke, StrokeKind, Ui, Vec2};

use crate::achievements::{
    AchievementCategory, AchievementNotification, AchievementTier, NotificationQueue,
};

/// Configuration for achievement notification display.
#[derive(Debug, Clone)]
pub struct AchievementNotificationConfig {
    /// Width of the notification popup.
    pub width: f32,
    /// Height of the notification popup.
    pub height: f32,
    /// Time to display before auto-dismissing (seconds).
    pub display_duration_secs: f32,
    /// Offset from top-right corner.
    pub offset: Vec2,
    /// Animation duration for slide-in/out.
    pub animation_duration_secs: f32,
}

impl Default for AchievementNotificationConfig {
    fn default() -> Self {
        Self {
            width: 320.0,
            height: 120.0,
            display_duration_secs: 5.0,
            offset: Vec2::new(-20.0, 80.0),
            animation_duration_secs: 0.3,
        }
    }
}

/// Widget for displaying achievement notifications.
pub struct AchievementNotificationWidget {
    /// Display configuration.
    config: AchievementNotificationConfig,
    /// Animation state.
    animation_progress: f32,
    /// Whether currently animating in or out.
    animating_in: bool,
    /// Animation start time.
    animation_start: Option<Instant>,
}

impl Default for AchievementNotificationWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl AchievementNotificationWidget {
    /// Create a new achievement notification widget.
    pub fn new() -> Self {
        Self {
            config: AchievementNotificationConfig::default(),
            animation_progress: 0.0,
            animating_in: true,
            animation_start: None,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: AchievementNotificationConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Show the notification queue.
    ///
    /// Call this once per frame to display any pending achievements.
    pub fn show(&mut self, ui: &mut Ui, queue: &mut NotificationQueue) {
        // Update the queue
        queue.update();

        // Get current notification if any
        let notification = match queue.current() {
            Some(n) => n.clone(),
            None => {
                // Reset animation state when no notification
                self.animation_progress = 0.0;
                self.animation_start = None;
                return;
            }
        };

        // Update animation
        self.update_animation();

        // Calculate position (top-right with offset)
        let screen_rect = ui.ctx().available_rect();
        let x = screen_rect.max.x + self.config.offset.x - self.config.width;
        let y = screen_rect.min.y + self.config.offset.y;

        // Apply slide animation
        let slide_offset = (1.0 - self.animation_progress) * (self.config.width + 40.0);
        let final_x = x + slide_offset;

        let rect = Rect::from_min_size(
            Pos2::new(final_x, y),
            Vec2::new(self.config.width, self.config.height),
        );

        // Draw background
        let bg_color = tier_background_color(notification.tier);
        let border_color = tier_accent_color(notification.tier);

        ui.painter().rect_filled(rect, 8.0, bg_color);
        ui.painter().rect_stroke(
            rect,
            8.0,
            Stroke::new(2.0, border_color),
            StrokeKind::Middle,
        );

        // Draw content
        let content_rect = rect.shrink(12.0);
        let mut ui_child = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));

        self.draw_content(&mut ui_child, &notification, queue);

        // Request repaint for animation
        if self.animation_progress < 1.0 {
            ui.ctx().request_repaint();
        }
    }

    /// Update animation state.
    fn update_animation(&mut self) {
        if self.animation_start.is_none() {
            self.animation_start = Some(Instant::now());
            self.animating_in = true;
            self.animation_progress = 0.0;
        }

        if let Some(start) = self.animation_start {
            let elapsed = start.elapsed().as_secs_f32();
            let duration = self.config.animation_duration_secs;

            if self.animating_in {
                self.animation_progress = (elapsed / duration).clamp(0.0, 1.0);
                // Apply easing (ease-out)
                self.animation_progress = ease_out_cubic(self.animation_progress);
            }
        }
    }

    /// Draw notification content.
    fn draw_content(
        &self,
        ui: &mut Ui,
        notification: &AchievementNotification,
        queue: &mut NotificationQueue,
    ) {
        ui.vertical(|ui| {
            // Header row: tier badge + category
            ui.horizontal(|ui| {
                // Tier badge
                let tier_color = tier_accent_color(notification.tier);
                let tier_text = tier_label(notification.tier);
                ui.label(
                    RichText::new(tier_text)
                        .color(tier_color)
                        .strong()
                        .size(12.0),
                );

                ui.separator();

                // Category
                let category_text = category_label(notification.category);
                ui.label(RichText::new(category_text).weak().size(11.0));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Dismiss button
                    if ui.small_button("x").clicked() {
                        queue.dismiss_current();
                    }
                });
            });

            ui.add_space(4.0);

            // Title
            ui.label(RichText::new(&notification.title).strong().size(18.0));

            // Description
            ui.label(RichText::new(&notification.description).weak().size(12.0));

            ui.add_space(4.0);

            // XP awarded
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("+{} XP", notification.xp_awarded))
                        .color(Color32::from_rgb(255, 215, 0)) // Gold
                        .strong(),
                );

                // Progress bar showing time remaining
                let progress = queue.display_progress();
                let remaining = 1.0 - progress;
                let bar_width = ui.available_width() - 40.0;

                ui.add_space(8.0);

                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(bar_width, 4.0), egui::Sense::hover());

                // Background
                ui.painter().rect_filled(rect, 2.0, Color32::from_gray(60));

                // Progress
                let filled_rect = Rect::from_min_size(
                    rect.min,
                    Vec2::new(rect.width() * remaining, rect.height()),
                );
                ui.painter()
                    .rect_filled(filled_rect, 2.0, tier_accent_color(notification.tier));
            });

            // Queue indicator
            let pending = queue.pending_count();
            if pending > 1 {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
                    ui.label(
                        RichText::new(format!("+{} more", pending - 1))
                            .weak()
                            .small(),
                    );
                });
            }
        });
    }
}

/// Get background color for tier.
fn tier_background_color(tier: AchievementTier) -> Color32 {
    match tier {
        AchievementTier::Bronze => Color32::from_rgba_unmultiplied(139, 90, 43, 230),
        AchievementTier::Silver => Color32::from_rgba_unmultiplied(120, 120, 130, 230),
        AchievementTier::Gold => Color32::from_rgba_unmultiplied(139, 117, 0, 230),
        AchievementTier::Diamond => Color32::from_rgba_unmultiplied(70, 130, 180, 230),
        AchievementTier::Legendary => Color32::from_rgba_unmultiplied(128, 0, 128, 230),
    }
}

/// Get accent color for tier.
fn tier_accent_color(tier: AchievementTier) -> Color32 {
    match tier {
        AchievementTier::Bronze => Color32::from_rgb(205, 127, 50),
        AchievementTier::Silver => Color32::from_rgb(192, 192, 192),
        AchievementTier::Gold => Color32::from_rgb(255, 215, 0),
        AchievementTier::Diamond => Color32::from_rgb(185, 242, 255),
        AchievementTier::Legendary => Color32::from_rgb(186, 85, 211),
    }
}

/// Get tier label.
fn tier_label(tier: AchievementTier) -> &'static str {
    match tier {
        AchievementTier::Bronze => "BRONZE",
        AchievementTier::Silver => "SILVER",
        AchievementTier::Gold => "GOLD",
        AchievementTier::Diamond => "DIAMOND",
        AchievementTier::Legendary => "LEGENDARY",
    }
}

/// Get category label.
fn category_label(category: AchievementCategory) -> &'static str {
    match category {
        AchievementCategory::Distance => "Distance",
        AchievementCategory::Climbing => "Climbing",
        AchievementCategory::Consistency => "Consistency",
        AchievementCategory::Competition => "Competition",
        AchievementCategory::Exploration => "Exploration",
        AchievementCategory::Training => "Training",
        AchievementCategory::Special => "Special",
        AchievementCategory::Power => "Power",
    }
}

/// Ease out cubic function for smooth animation.
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// Compact inline achievement notification (for embedding in other UI).
pub struct AchievementBadge;

impl AchievementBadge {
    /// Show a compact achievement badge.
    pub fn show(ui: &mut Ui, title: &str, tier: AchievementTier, xp: u32) {
        let bg_color = tier_background_color(tier);
        let accent = tier_accent_color(tier);

        egui::Frame::new()
            .fill(bg_color)
            .stroke(Stroke::new(1.0, accent))
            .corner_radius(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tier_label(tier)).color(accent).small());
                    ui.label(RichText::new(title).strong());
                    ui.label(
                        RichText::new(format!("+{} XP", xp)).color(Color32::from_rgb(255, 215, 0)),
                    );
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_colors() {
        // Ensure all tiers have distinct colors
        let tiers = [
            AchievementTier::Bronze,
            AchievementTier::Silver,
            AchievementTier::Gold,
            AchievementTier::Diamond,
            AchievementTier::Legendary,
        ];

        for tier in &tiers {
            let bg = tier_background_color(*tier);
            let accent = tier_accent_color(*tier);
            // Colors should be semi-transparent for background
            assert!(bg.a() > 200);
            // Accent should be fully opaque
            assert_eq!(accent.a(), 255);
        }
    }

    #[test]
    fn test_ease_out_cubic() {
        assert!((ease_out_cubic(0.0) - 0.0).abs() < 0.001);
        assert!((ease_out_cubic(1.0) - 1.0).abs() < 0.001);
        // Ease out should be > linear at midpoint
        assert!(ease_out_cubic(0.5) > 0.5);
    }

    #[test]
    fn test_config_defaults() {
        let config = AchievementNotificationConfig::default();
        assert!(config.width > 0.0);
        assert!(config.height > 0.0);
        assert!(config.display_duration_secs > 0.0);
    }
}
