//! Level definitions and progression.
//!
//! T074: Define all 50 levels with reward unlocks.

use super::rewards::{Reward, RewardType};

/// Get all level definitions with their rewards.
pub fn all_level_rewards() -> Vec<LevelDefinition> {
    vec![
        // Early Levels (1-10): Basic customization
        level(1, "Beginner", vec![]),
        level(2, "Novice", vec![
            reward("jersey_white", RewardType::JerseyColor, "White Jersey", "A clean white racing jersey", "#FFFFFF"),
        ]),
        level(3, "Novice", vec![
            reward("accent_blue", RewardType::AccentColor, "Blue Accent", "Blue UI accent color", "#0066CC"),
        ]),
        level(4, "Novice", vec![
            reward("jersey_black", RewardType::JerseyColor, "Black Jersey", "A sleek black jersey", "#1A1A1A"),
        ]),
        level(5, "Enthusiast", vec![
            reward("badge_starter", RewardType::ProfileBadge, "Starter Badge", "For beginning your journey", None).milestone(),
        ]),
        level(6, "Enthusiast", vec![
            reward("jersey_red", RewardType::JerseyColor, "Red Jersey", "Bold red racing colors", "#CC0000"),
        ]),
        level(7, "Enthusiast", vec![
            reward("accent_green", RewardType::AccentColor, "Green Accent", "Green UI accent color", "#00AA00"),
        ]),
        level(8, "Enthusiast", vec![
            reward("jersey_blue", RewardType::JerseyColor, "Blue Jersey", "Ocean blue jersey", "#0044AA"),
        ]),
        level(9, "Enthusiast", vec![
            reward("wheel_aero", RewardType::WheelStyle, "Aero Wheels", "Sleek aerodynamic wheels", None),
        ]),
        level(10, "Regular", vec![
            reward("badge_dedicated", RewardType::ProfileBadge, "Dedicated Badge", "For reaching level 10", None).milestone(),
            reward("theme_dark_blue", RewardType::UiTheme, "Dark Blue Theme", "A calming dark blue theme", None),
        ]),

        // Mid Levels (11-25): More variety
        level(11, "Regular", vec![
            reward("jersey_green", RewardType::JerseyColor, "Green Jersey", "Forest green jersey", "#228B22"),
        ]),
        level(12, "Regular", vec![
            reward("bike_carbon", RewardType::BikeFrame, "Carbon Frame", "Lightweight carbon frame", None),
        ]),
        level(13, "Regular", vec![
            reward("accent_purple", RewardType::AccentColor, "Purple Accent", "Royal purple accent", "#8800AA"),
        ]),
        level(14, "Regular", vec![
            reward("jersey_yellow", RewardType::JerseyColor, "Yellow Jersey", "Racing yellow jersey", "#FFD700"),
        ]),
        level(15, "Dedicated", vec![
            reward("helmet_aero", RewardType::HelmetStyle, "Aero Helmet", "Time trial style helmet", None),
        ]),
        level(16, "Dedicated", vec![
            reward("jersey_orange", RewardType::JerseyColor, "Orange Jersey", "Vibrant orange jersey", "#FF6600"),
        ]),
        level(17, "Dedicated", vec![
            reward("wheel_deep", RewardType::WheelStyle, "Deep Section Wheels", "Deep carbon wheels", None),
        ]),
        level(18, "Dedicated", vec![
            reward("jersey_pink", RewardType::JerseyColor, "Pink Jersey", "Giro pink jersey", "#FF69B4"),
        ]),
        level(19, "Dedicated", vec![
            reward("accent_gold", RewardType::AccentColor, "Gold Accent", "Premium gold accent", "#FFD700"),
        ]),
        level(20, "Committed", vec![
            reward("badge_committed", RewardType::ProfileBadge, "Committed Badge", "For reaching level 20", None).milestone(),
            reward("theme_sunset", RewardType::UiTheme, "Sunset Theme", "Warm sunset colors", None),
            reward("bike_titanium", RewardType::BikeFrame, "Titanium Frame", "Premium titanium frame", None),
        ]),
        level(21, "Committed", vec![
            reward("jersey_polka", RewardType::JerseyColor, "Polka Dot Jersey", "King of the Mountains", "#FFFFFF"),
        ]),
        level(22, "Committed", vec![
            reward("helmet_classic", RewardType::HelmetStyle, "Classic Helmet", "Traditional road helmet", None),
        ]),
        level(23, "Committed", vec![
            reward("jersey_rainbow", RewardType::JerseyColor, "Rainbow Jersey", "World champion stripes", None),
        ]),
        level(24, "Committed", vec![
            reward("wheel_disc", RewardType::WheelStyle, "Disc Wheel", "Full disc rear wheel", None),
        ]),
        level(25, "Veteran", vec![
            reward("badge_veteran", RewardType::ProfileBadge, "Veteran Badge", "Quarter century achievement", None).milestone(),
            reward("theme_pro", RewardType::UiTheme, "Pro Team Theme", "Professional team colors", None),
        ]),

        // Upper Levels (26-40): Premium unlocks
        level(26, "Veteran", vec![
            reward("jersey_cyan", RewardType::JerseyColor, "Cyan Jersey", "Electric cyan jersey", "#00CED1"),
        ]),
        level(27, "Veteran", vec![
            reward("bike_aero", RewardType::BikeFrame, "Aero Frame", "Aerodynamic racing frame", None),
        ]),
        level(28, "Veteran", vec![
            reward("accent_crimson", RewardType::AccentColor, "Crimson Accent", "Deep crimson accent", "#DC143C"),
        ]),
        level(29, "Veteran", vec![
            reward("jersey_gradient", RewardType::JerseyColor, "Gradient Jersey", "Modern gradient design", None),
        ]),
        level(30, "Expert", vec![
            reward("badge_expert", RewardType::ProfileBadge, "Expert Badge", "Expert level achieved", None).milestone(),
            reward("theme_midnight", RewardType::UiTheme, "Midnight Theme", "Deep midnight blue", None),
            reward("helmet_tt", RewardType::HelmetStyle, "TT Helmet", "Full time trial helmet", None),
        ]),
        level(31, "Expert", vec![
            reward("jersey_silver", RewardType::JerseyColor, "Silver Jersey", "Metallic silver jersey", "#C0C0C0"),
        ]),
        level(32, "Expert", vec![
            reward("wheel_ceramic", RewardType::WheelStyle, "Ceramic Bearing Wheels", "Ultra-smooth ceramic wheels", None),
        ]),
        level(33, "Expert", vec![
            reward("jersey_neon", RewardType::JerseyColor, "Neon Jersey", "High-visibility neon", "#39FF14"),
        ]),
        level(34, "Expert", vec![
            reward("accent_teal", RewardType::AccentColor, "Teal Accent", "Cool teal accent", "#008080"),
        ]),
        level(35, "Elite", vec![
            reward("bike_superbike", RewardType::BikeFrame, "Superbike Frame", "Top-tier racing machine", None),
        ]),
        level(36, "Elite", vec![
            reward("jersey_stealth", RewardType::JerseyColor, "Stealth Jersey", "Matte black stealth", "#0A0A0A"),
        ]),
        level(37, "Elite", vec![
            reward("helmet_pro", RewardType::HelmetStyle, "Pro Helmet", "Professional racing helmet", None),
        ]),
        level(38, "Elite", vec![
            reward("accent_rose", RewardType::AccentColor, "Rose Gold Accent", "Premium rose gold", "#B76E79"),
        ]),
        level(39, "Elite", vec![
            reward("jersey_geometric", RewardType::JerseyColor, "Geometric Jersey", "Bold geometric patterns", None),
        ]),
        level(40, "Champion", vec![
            reward("badge_champion", RewardType::ProfileBadge, "Champion Badge", "Champion level achieved", None).milestone(),
            reward("theme_champion", RewardType::UiTheme, "Champion Theme", "Golden champion colors", None),
            reward("bike_champion", RewardType::BikeFrame, "Champion Frame", "Gold-detailed champion bike", None),
        ]),

        // Top Levels (41-50): Legendary unlocks
        level(41, "Champion", vec![
            reward("jersey_platinum", RewardType::JerseyColor, "Platinum Jersey", "Platinum metallic finish", "#E5E4E2"),
        ]),
        level(42, "Champion", vec![
            reward("wheel_platinum", RewardType::WheelStyle, "Platinum Wheels", "Premium platinum wheels", None),
        ]),
        level(43, "Champion", vec![
            reward("jersey_galaxy", RewardType::JerseyColor, "Galaxy Jersey", "Space-themed design", None),
        ]),
        level(44, "Champion", vec![
            reward("accent_diamond", RewardType::AccentColor, "Diamond Accent", "Sparkling diamond accent", "#B9F2FF"),
        ]),
        level(45, "Master", vec![
            reward("badge_master", RewardType::ProfileBadge, "Master Badge", "Master level achieved", None).milestone(),
            reward("helmet_master", RewardType::HelmetStyle, "Master Helmet", "Exclusive master helmet", None),
        ]),
        level(46, "Master", vec![
            reward("jersey_fire", RewardType::JerseyColor, "Fire Jersey", "Flame pattern jersey", None),
        ]),
        level(47, "Master", vec![
            reward("bike_master", RewardType::BikeFrame, "Master Frame", "Exclusive master frame", None),
        ]),
        level(48, "Master", vec![
            reward("jersey_ice", RewardType::JerseyColor, "Ice Jersey", "Cool ice pattern", None),
        ]),
        level(49, "Master", vec![
            reward("wheel_gold", RewardType::WheelStyle, "Gold Wheels", "Legendary gold wheels", None),
        ]),
        level(50, "Legend", vec![
            reward("badge_legend", RewardType::ProfileBadge, "Legend Badge", "The ultimate achievement", None).milestone(),
            reward("theme_legend", RewardType::UiTheme, "Legend Theme", "Exclusive legend theme", None),
            reward("bike_legend", RewardType::BikeFrame, "Legend Frame", "The ultimate bike frame", None),
            reward("jersey_legend", RewardType::JerseyColor, "Legend Jersey", "The legendary jersey", "#FFD700"),
            reward("helmet_legend", RewardType::HelmetStyle, "Legend Helmet", "The ultimate helmet", None),
        ]),
    ]
}

/// Get all rewards as a flat list.
pub fn all_rewards() -> Vec<Reward> {
    all_level_rewards()
        .into_iter()
        .flat_map(|l| {
            let level = l.level;
            l.rewards.into_iter().map(move |mut r| {
                r.unlock_level = level;
                r
            })
        })
        .collect()
}

/// Get rewards for a specific level.
pub fn rewards_for_level(level: u32) -> Vec<Reward> {
    all_rewards()
        .into_iter()
        .filter(|r| r.unlock_level == level)
        .collect()
}

/// Get all rewards up to and including a level.
pub fn rewards_up_to_level(level: u32) -> Vec<Reward> {
    all_rewards()
        .into_iter()
        .filter(|r| r.unlock_level <= level)
        .collect()
}

/// Get next milestone level.
pub fn next_milestone(current_level: u32) -> Option<u32> {
    const MILESTONES: [u32; 6] = [10, 20, 25, 30, 40, 50];
    MILESTONES.iter().find(|&&m| m > current_level).copied()
}

/// Definition for a single level.
#[derive(Debug, Clone)]
pub struct LevelDefinition {
    /// Level number.
    pub level: u32,
    /// Level title.
    pub title: String,
    /// Rewards unlocked at this level.
    pub rewards: Vec<Reward>,
}

/// Helper to create a level definition.
fn level(level: u32, title: &str, rewards: Vec<Reward>) -> LevelDefinition {
    LevelDefinition {
        level,
        title: title.to_string(),
        rewards,
    }
}

/// Helper to create a reward.
fn reward(id: &str, reward_type: RewardType, name: &str, description: &str, color: impl Into<Option<&'static str>>) -> Reward {
    let r = Reward::new(id, reward_type, name, description, 0);
    if let Some(c) = color.into() {
        r.with_color(c)
    } else {
        r
    }
}

#[allow(dead_code)]
trait RewardExt {
    fn milestone(self) -> Self;
}

impl RewardExt for Reward {
    fn milestone(self) -> Self {
        Reward { is_milestone: true, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_levels_defined() {
        let levels = all_level_rewards();
        assert_eq!(levels.len(), 50);

        // Check all levels from 1 to 50 are present
        for i in 1..=50 {
            assert!(
                levels.iter().any(|l| l.level == i),
                "Missing level {}",
                i
            );
        }
    }

    #[test]
    fn test_all_rewards() {
        let rewards = all_rewards();
        // Should have many rewards
        assert!(rewards.len() > 50, "Expected at least 50 rewards");

        // Check no duplicate IDs
        let ids: std::collections::HashSet<_> = rewards.iter().map(|r| &r.id).collect();
        assert_eq!(ids.len(), rewards.len(), "Duplicate reward IDs found");
    }

    #[test]
    fn test_milestone_levels() {
        let levels = all_level_rewards();

        for level in &[10, 20, 25, 30, 40, 50] {
            let level_def = levels.iter().find(|l| l.level == *level);
            assert!(level_def.is_some(), "Missing milestone level {}", level);

            let level_def = level_def.unwrap();
            let has_milestone_reward = level_def.rewards.iter().any(|r| r.is_milestone);
            assert!(
                has_milestone_reward,
                "Level {} should have a milestone reward",
                level
            );
        }
    }

    #[test]
    fn test_next_milestone() {
        assert_eq!(next_milestone(1), Some(10));
        assert_eq!(next_milestone(10), Some(20));
        assert_eq!(next_milestone(25), Some(30));
        assert_eq!(next_milestone(50), None);
    }

    #[test]
    fn test_rewards_for_level() {
        let rewards = rewards_for_level(10);
        assert!(!rewards.is_empty());
        assert!(rewards.iter().any(|r| r.is_milestone));
    }

    #[test]
    fn test_rewards_up_to_level() {
        let rewards = rewards_up_to_level(5);
        assert!(!rewards.is_empty());

        let rewards_10 = rewards_up_to_level(10);
        assert!(rewards_10.len() > rewards.len());
    }
}
