//! Rewards gallery screen.
//!
//! T079: Create rewards gallery screen.
//!
//! Dedicated screen for browsing, previewing, and equipping cosmetic rewards.

use egui::{Align, Color32, Layout, RichText, ScrollArea, Stroke, StrokeKind, Ui, Vec2};

use crate::career::{
    all_rewards, CareerManager, CosmeticInventory, CosmeticType, EquippedCosmetics, Reward,
    RewardType,
};

/// Rewards gallery screen state.
pub struct RewardsScreen {
    /// Selected category tab.
    pub selected_category: Option<RewardType>,
    /// Selected reward for preview.
    pub selected_reward: Option<String>,
    /// Show locked items.
    pub show_locked: bool,
}

impl Default for RewardsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl RewardsScreen {
    /// Create a new rewards screen.
    pub fn new() -> Self {
        Self {
            selected_category: None,
            selected_reward: None,
            show_locked: true,
        }
    }

    /// Show the rewards screen.
    pub fn show(&mut self, ui: &mut Ui, manager: &mut CareerManager) -> Option<RewardsAction> {
        let mut action = None;

        ui.vertical(|ui| {
            self.show_header(ui, manager.inventory(), manager.equipped());
            ui.add_space(8.0);
            self.show_category_tabs(ui);
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                // Main gallery
                let gallery_width = ui.available_width() * 0.65;
                ui.allocate_ui(Vec2::new(gallery_width, ui.available_height()), |ui| {
                    if let Some(a) = self.show_gallery(ui, manager) {
                        action = Some(a);
                    }
                });

                ui.separator();

                // Preview panel
                if let Some(reward_id) = &self.selected_reward.clone() {
                    if let Some(a) = self.show_preview(ui, reward_id, manager) {
                        action = Some(a);
                    }
                } else {
                    self.show_no_selection(ui);
                }
            });
        });

        action
    }

    /// Show header with collection stats.
    fn show_header(
        &self,
        ui: &mut Ui,
        inventory: &CosmeticInventory,
        equipped: &EquippedCosmetics,
    ) {
        let all_rewards = all_rewards();
        let unlocked_count = inventory.unlocked_count();
        let total_count = all_rewards.len();

        ui.horizontal(|ui| {
            ui.heading("Rewards Gallery");

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.checkbox(&mut { self.show_locked }, "Show Locked");

                ui.separator();

                // Collection progress
                let progress = unlocked_count as f32 / total_count as f32;
                ui.label(
                    RichText::new(format!(
                        "{} / {} collected ({:.0}%)",
                        unlocked_count,
                        total_count,
                        progress * 100.0
                    ))
                    .size(14.0),
                );
            });
        });

        // Currently equipped summary
        ui.horizontal(|ui| {
            ui.label(RichText::new("Equipped:").weak().size(12.0));

            if let Some(jersey) = &equipped.jersey {
                self.show_mini_equipped(ui, "\u{1F455}", jersey);
            }
            if let Some(bike) = &equipped.bike_frame {
                self.show_mini_equipped(ui, "\u{1F6B2}", bike);
            }
            if let Some(theme) = &equipped.theme {
                self.show_mini_equipped(ui, "\u{1F3A8}", theme);
            }
            if let Some(badge) = &equipped.badge {
                self.show_mini_equipped(ui, "\u{1F396}", badge);
            }
        });
    }

    /// Show mini equipped badge.
    fn show_mini_equipped(&self, ui: &mut Ui, emoji: &str, id: &str) {
        ui.label(
            RichText::new(format!(
                "{} {}",
                emoji,
                id.split('_').next_back().unwrap_or(id)
            ))
            .size(11.0)
            .color(Color32::from_rgb(150, 200, 150)),
        );
    }

    /// Show category tabs.
    fn show_category_tabs(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // All categories
            let all_selected = self.selected_category.is_none();
            if ui.selectable_label(all_selected, "All").clicked() {
                self.selected_category = None;
            }

            ui.separator();

            // Individual categories
            for reward_type in [
                RewardType::JerseyColor,
                RewardType::BikeFrame,
                RewardType::WheelStyle,
                RewardType::HelmetStyle,
                RewardType::UiTheme,
                RewardType::AccentColor,
                RewardType::ProfileBadge,
            ] {
                let selected = self.selected_category == Some(reward_type);
                let emoji = reward_type_emoji(reward_type);
                let label = format!("{} {}", emoji, reward_type.display_name());

                if ui.selectable_label(selected, label).clicked() {
                    self.selected_category = Some(reward_type);
                }
            }
        });

        ui.separator();
    }

    /// Show reward gallery.
    fn show_gallery(&mut self, ui: &mut Ui, manager: &CareerManager) -> Option<RewardsAction> {
        let all_rewards = all_rewards();
        let inventory = manager.inventory();

        // Filter rewards
        let filtered: Vec<_> = all_rewards
            .iter()
            .filter(|r| {
                // Category filter
                if let Some(cat) = self.selected_category {
                    if r.reward_type != cat {
                        return false;
                    }
                }
                // Locked filter
                if !self.show_locked && !inventory.is_unlocked(&r.id) {
                    return false;
                }
                true
            })
            .collect();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for reward in filtered {
                        let unlocked = inventory.is_unlocked(&reward.id);
                        let is_selected = self.selected_reward.as_ref() == Some(&reward.id);

                        if self.show_reward_tile(ui, reward, unlocked, is_selected) {
                            self.selected_reward = Some(reward.id.clone());
                        }
                    }
                });
            });

        None
    }

    /// Show a single reward tile.
    fn show_reward_tile(
        &self,
        ui: &mut Ui,
        reward: &Reward,
        unlocked: bool,
        selected: bool,
    ) -> bool {
        let tile_size = Vec2::new(100.0, 100.0);
        let (rect, response) = ui.allocate_exact_size(tile_size, egui::Sense::click());

        let bg_color = if unlocked {
            if selected {
                Color32::from_rgb(60, 80, 100)
            } else {
                Color32::from_rgb(40, 50, 60)
            }
        } else {
            Color32::from_gray(25)
        };

        let border_color = if selected {
            Color32::from_rgb(100, 149, 237)
        } else if unlocked {
            reward_type_accent_color(reward.reward_type)
        } else {
            Color32::from_gray(40)
        };

        // Background
        ui.painter().rect_filled(rect, 8.0, bg_color);
        ui.painter().rect_stroke(
            rect,
            8.0,
            Stroke::new(if selected { 2.0 } else { 1.0 }, border_color),
            StrokeKind::Middle,
        );

        // Content
        let center = rect.center();

        // Icon
        let emoji = reward_type_emoji(reward.reward_type);
        ui.painter().text(
            center - Vec2::new(0.0, 15.0),
            egui::Align2::CENTER_CENTER,
            emoji,
            egui::FontId::proportional(28.0),
            if unlocked {
                Color32::WHITE
            } else {
                Color32::from_gray(80)
            },
        );

        // Name (truncated)
        let name = if reward.name.len() > 12 {
            format!("{}...", &reward.name[..9])
        } else {
            reward.name.clone()
        };
        ui.painter().text(
            center + Vec2::new(0.0, 20.0),
            egui::Align2::CENTER_CENTER,
            name,
            egui::FontId::proportional(10.0),
            if unlocked {
                Color32::from_gray(200)
            } else {
                Color32::from_gray(80)
            },
        );

        // Lock icon if locked
        if !unlocked {
            ui.painter().text(
                rect.right_top() - Vec2::new(12.0, -12.0),
                egui::Align2::CENTER_CENTER,
                "\u{1F512}", // Lock
                egui::FontId::proportional(14.0),
                Color32::from_gray(100),
            );
        }

        response.clicked()
    }

    /// Show preview panel for selected reward.
    fn show_preview(
        &mut self,
        ui: &mut Ui,
        reward_id: &str,
        manager: &mut CareerManager,
    ) -> Option<RewardsAction> {
        let all_rewards = all_rewards();
        let reward = match all_rewards.iter().find(|r| r.id == reward_id) {
            Some(r) => r,
            None => {
                self.selected_reward = None;
                return None;
            }
        };

        let inventory = manager.inventory();
        let unlocked = inventory.is_unlocked(reward_id);
        let equipped = manager.equipped();

        // Check if this reward is currently equipped
        let cosmetic_type = CosmeticType::from_reward_type(reward.reward_type);
        let is_equipped = equipped
            .get(cosmetic_type)
            .map(|id| id == reward_id)
            .unwrap_or(false);

        egui::Frame::new()
            .fill(Color32::from_gray(30))
            .corner_radius(8.0)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    // Large icon
                    let emoji = reward_type_emoji(reward.reward_type);
                    ui.label(RichText::new(emoji).size(64.0));

                    ui.add_space(8.0);

                    // Name
                    ui.label(RichText::new(&reward.name).strong().size(20.0));

                    // Type
                    ui.label(
                        RichText::new(reward.reward_type.display_name())
                            .color(reward_type_accent_color(reward.reward_type))
                            .size(12.0),
                    );

                    ui.add_space(8.0);

                    // Description
                    ui.label(RichText::new(&reward.description).weak().size(14.0));

                    ui.add_space(16.0);

                    // Color preview if applicable
                    if let Some(color) = &reward.color {
                        ui.label(RichText::new("Color:").weak().size(12.0));
                        let (color_rect, _) =
                            ui.allocate_exact_size(Vec2::new(60.0, 30.0), egui::Sense::hover());
                        if let Ok(parsed_color) = parse_hex_color(color) {
                            ui.painter().rect_filled(color_rect, 4.0, parsed_color);
                            ui.painter().rect_stroke(
                                color_rect,
                                4.0,
                                Stroke::new(1.0, Color32::WHITE),
                                StrokeKind::Middle,
                            );
                        }
                        ui.add_space(8.0);
                    }

                    // Unlock status
                    if unlocked {
                        ui.label(
                            RichText::new("UNLOCKED")
                                .color(Color32::from_rgb(100, 200, 100))
                                .strong(),
                        );

                        if is_equipped {
                            ui.label(
                                RichText::new("Currently Equipped")
                                    .color(Color32::from_rgb(100, 149, 237))
                                    .size(12.0),
                            );
                        } else {
                            // Equip button
                            ui.add_space(8.0);
                            if ui.button("Equip").clicked() {
                                manager.equip(cosmetic_type, reward_id);
                            }
                        }
                    } else {
                        ui.label(
                            RichText::new(format!("Unlocks at Level {}", reward.unlock_level))
                                .color(Color32::from_gray(150)),
                        );
                    }
                });
            });

        None
    }

    /// Show no selection message.
    fn show_no_selection(&self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 3.0);
            ui.label(
                RichText::new("Select a reward to preview")
                    .weak()
                    .size(16.0),
            );
        });
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

/// Get accent color for reward type.
fn reward_type_accent_color(reward_type: RewardType) -> Color32 {
    match reward_type {
        RewardType::JerseyColor => Color32::from_rgb(180, 120, 200),
        RewardType::BikeFrame => Color32::from_rgb(120, 180, 120),
        RewardType::UiTheme => Color32::from_rgb(150, 150, 200),
        RewardType::AccentColor => Color32::from_rgb(200, 120, 150),
        RewardType::ProfileBadge => Color32::from_rgb(255, 215, 0),
        RewardType::WheelStyle => Color32::from_rgb(150, 170, 190),
        RewardType::HelmetStyle => Color32::from_rgb(170, 150, 180),
    }
}

/// Parse hex color string to Color32.
fn parse_hex_color(hex: &str) -> Result<Color32, ()> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Err(());
    }

    let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| ())?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| ())?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| ())?;

    Ok(Color32::from_rgb(r, g, b))
}

/// Action from rewards screen.
#[derive(Debug, Clone)]
pub enum RewardsAction {
    /// Go back.
    Back,
    /// Equip a reward.
    Equip(CosmeticType, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_creation() {
        let screen = RewardsScreen::new();
        assert!(screen.selected_category.is_none());
        assert!(screen.selected_reward.is_none());
        assert!(screen.show_locked);
    }

    #[test]
    fn test_parse_hex_color() {
        let color = parse_hex_color("#FF0000").unwrap();
        assert_eq!(color.r(), 255);
        assert_eq!(color.g(), 0);
        assert_eq!(color.b(), 0);

        let color = parse_hex_color("00FF00").unwrap();
        assert_eq!(color.r(), 0);
        assert_eq!(color.g(), 255);
        assert_eq!(color.b(), 0);

        assert!(parse_hex_color("invalid").is_err());
    }

    #[test]
    fn test_reward_type_colors() {
        for reward_type in [
            RewardType::JerseyColor,
            RewardType::BikeFrame,
            RewardType::UiTheme,
        ] {
            let color = reward_type_accent_color(reward_type);
            assert!(color.r() > 0 || color.g() > 0 || color.b() > 0);
        }
    }
}
