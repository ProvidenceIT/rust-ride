//! Cosmetic reward definitions and management.
//!
//! T076: Define cosmetic rewards (jerseys, frames, themes).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::rewards::RewardType;

/// A cosmetic item that can be equipped.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmeticItem {
    /// Unique identifier.
    pub id: String,
    /// Type of cosmetic.
    pub item_type: CosmeticType,
    /// Display name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Whether this is a default item (always available).
    pub is_default: bool,
    /// Whether this item is unlocked.
    pub is_unlocked: bool,
    /// Color value (for color-type items).
    pub color: Option<String>,
    /// Asset path (for visual items).
    pub asset_path: Option<String>,
    /// Preview image path.
    pub preview_path: Option<String>,
}

impl CosmeticItem {
    /// Create a new cosmetic item.
    pub fn new(
        id: impl Into<String>,
        item_type: CosmeticType,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            item_type,
            name: name.into(),
            description: description.into(),
            is_default: false,
            is_unlocked: false,
            color: None,
            asset_path: None,
            preview_path: None,
        }
    }

    /// Mark as default (always available).
    pub fn default_item(mut self) -> Self {
        self.is_default = true;
        self.is_unlocked = true;
        self
    }

    /// Set color.
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set asset path.
    pub fn with_asset(mut self, path: impl Into<String>) -> Self {
        self.asset_path = Some(path.into());
        self
    }

    /// Set preview path.
    pub fn with_preview(mut self, path: impl Into<String>) -> Self {
        self.preview_path = Some(path.into());
        self
    }

    /// Unlock this item.
    pub fn unlock(&mut self) {
        self.is_unlocked = true;
    }
}

/// Type of cosmetic item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CosmeticType {
    /// Jersey/kit color.
    Jersey,
    /// Bike frame.
    BikeFrame,
    /// Wheel style.
    Wheels,
    /// Helmet style.
    Helmet,
    /// UI theme.
    Theme,
    /// UI accent color.
    AccentColor,
    /// Profile badge.
    Badge,
}

impl CosmeticType {
    /// Get display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Jersey => "Jersey",
            Self::BikeFrame => "Bike Frame",
            Self::Wheels => "Wheels",
            Self::Helmet => "Helmet",
            Self::Theme => "Theme",
            Self::AccentColor => "Accent Color",
            Self::Badge => "Badge",
        }
    }

    /// Get all cosmetic types.
    pub fn all() -> &'static [CosmeticType] {
        &[
            Self::Jersey,
            Self::BikeFrame,
            Self::Wheels,
            Self::Helmet,
            Self::Theme,
            Self::AccentColor,
            Self::Badge,
        ]
    }

    /// Convert from RewardType.
    pub fn from_reward_type(reward_type: RewardType) -> Self {
        match reward_type {
            RewardType::JerseyColor => Self::Jersey,
            RewardType::BikeFrame => Self::BikeFrame,
            RewardType::UiTheme => Self::Theme,
            RewardType::AccentColor => Self::AccentColor,
            RewardType::ProfileBadge => Self::Badge,
            RewardType::WheelStyle => Self::Wheels,
            RewardType::HelmetStyle => Self::Helmet,
        }
    }
}

/// Currently equipped cosmetics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EquippedCosmetics {
    /// Currently equipped jersey.
    pub jersey: Option<String>,
    /// Currently equipped bike frame.
    pub bike_frame: Option<String>,
    /// Currently equipped wheels.
    pub wheels: Option<String>,
    /// Currently equipped helmet.
    pub helmet: Option<String>,
    /// Currently equipped theme.
    pub theme: Option<String>,
    /// Currently equipped accent color.
    pub accent_color: Option<String>,
    /// Currently equipped badge.
    pub badge: Option<String>,
}

impl EquippedCosmetics {
    /// Create with default equipment.
    pub fn default_equipment() -> Self {
        Self {
            jersey: Some("jersey_default".to_string()),
            bike_frame: Some("bike_default".to_string()),
            wheels: Some("wheel_default".to_string()),
            helmet: Some("helmet_default".to_string()),
            theme: Some("theme_default".to_string()),
            accent_color: Some("accent_default".to_string()),
            badge: None,
        }
    }

    /// Equip an item.
    pub fn equip(&mut self, item_type: CosmeticType, item_id: impl Into<String>) {
        let id = Some(item_id.into());
        match item_type {
            CosmeticType::Jersey => self.jersey = id,
            CosmeticType::BikeFrame => self.bike_frame = id,
            CosmeticType::Wheels => self.wheels = id,
            CosmeticType::Helmet => self.helmet = id,
            CosmeticType::Theme => self.theme = id,
            CosmeticType::AccentColor => self.accent_color = id,
            CosmeticType::Badge => self.badge = id,
        }
    }

    /// Unequip an item type.
    pub fn unequip(&mut self, item_type: CosmeticType) {
        match item_type {
            CosmeticType::Jersey => self.jersey = None,
            CosmeticType::BikeFrame => self.bike_frame = None,
            CosmeticType::Wheels => self.wheels = None,
            CosmeticType::Helmet => self.helmet = None,
            CosmeticType::Theme => self.theme = None,
            CosmeticType::AccentColor => self.accent_color = None,
            CosmeticType::Badge => self.badge = None,
        }
    }

    /// Get equipped item for a type.
    pub fn get(&self, item_type: CosmeticType) -> Option<&String> {
        match item_type {
            CosmeticType::Jersey => self.jersey.as_ref(),
            CosmeticType::BikeFrame => self.bike_frame.as_ref(),
            CosmeticType::Wheels => self.wheels.as_ref(),
            CosmeticType::Helmet => self.helmet.as_ref(),
            CosmeticType::Theme => self.theme.as_ref(),
            CosmeticType::AccentColor => self.accent_color.as_ref(),
            CosmeticType::Badge => self.badge.as_ref(),
        }
    }
}

/// Inventory of unlocked cosmetics.
#[derive(Debug, Clone, Default)]
pub struct CosmeticInventory {
    /// Set of unlocked item IDs.
    unlocked: HashSet<String>,
    /// Currently equipped items.
    equipped: EquippedCosmetics,
}

impl CosmeticInventory {
    /// Create a new empty inventory.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with default items unlocked.
    pub fn with_defaults() -> Self {
        let mut inv = Self::new();
        // Unlock default items
        for default_id in default_item_ids() {
            inv.unlock(default_id);
        }
        inv.equipped = EquippedCosmetics::default_equipment();
        inv
    }

    /// Unlock an item.
    pub fn unlock(&mut self, item_id: impl Into<String>) {
        self.unlocked.insert(item_id.into());
    }

    /// Check if an item is unlocked.
    pub fn is_unlocked(&self, item_id: &str) -> bool {
        self.unlocked.contains(item_id)
    }

    /// Get all unlocked item IDs.
    pub fn unlocked_ids(&self) -> Vec<&String> {
        self.unlocked.iter().collect()
    }

    /// Get unlocked count.
    pub fn unlocked_count(&self) -> usize {
        self.unlocked.len()
    }

    /// Get equipped cosmetics.
    pub fn equipped(&self) -> &EquippedCosmetics {
        &self.equipped
    }

    /// Equip an item (must be unlocked).
    pub fn equip(&mut self, item_type: CosmeticType, item_id: &str) -> bool {
        if self.is_unlocked(item_id) {
            self.equipped.equip(item_type, item_id);
            true
        } else {
            false
        }
    }
}

/// Get default item IDs.
fn default_item_ids() -> Vec<&'static str> {
    vec![
        "jersey_default",
        "bike_default",
        "wheel_default",
        "helmet_default",
        "theme_default",
        "accent_default",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosmetic_item() {
        let item = CosmeticItem::new("test_jersey", CosmeticType::Jersey, "Test", "A test jersey")
            .with_color("#FF0000");

        assert_eq!(item.id, "test_jersey");
        assert!(!item.is_unlocked);
        assert_eq!(item.color, Some("#FF0000".to_string()));
    }

    #[test]
    fn test_default_item() {
        let item = CosmeticItem::new("default", CosmeticType::Jersey, "Default", "Default jersey")
            .default_item();

        assert!(item.is_default);
        assert!(item.is_unlocked);
    }

    #[test]
    fn test_equipped_cosmetics() {
        let mut equipped = EquippedCosmetics::default();

        equipped.equip(CosmeticType::Jersey, "test_jersey");
        assert_eq!(
            equipped.get(CosmeticType::Jersey),
            Some(&"test_jersey".to_string())
        );

        equipped.unequip(CosmeticType::Jersey);
        assert!(equipped.get(CosmeticType::Jersey).is_none());
    }

    #[test]
    fn test_inventory() {
        let mut inv = CosmeticInventory::with_defaults();

        assert!(inv.unlocked_count() > 0);
        assert!(inv.is_unlocked("jersey_default"));
        assert!(!inv.is_unlocked("premium_jersey"));

        inv.unlock("premium_jersey");
        assert!(inv.is_unlocked("premium_jersey"));
    }

    #[test]
    fn test_equip_locked_item() {
        let mut inv = CosmeticInventory::new();

        // Can't equip locked item
        assert!(!inv.equip(CosmeticType::Jersey, "locked_jersey"));

        // Can equip after unlock
        inv.unlock("locked_jersey");
        assert!(inv.equip(CosmeticType::Jersey, "locked_jersey"));
    }

    #[test]
    fn test_cosmetic_type_from_reward() {
        assert_eq!(
            CosmeticType::from_reward_type(RewardType::JerseyColor),
            CosmeticType::Jersey
        );
        assert_eq!(
            CosmeticType::from_reward_type(RewardType::BikeFrame),
            CosmeticType::BikeFrame
        );
    }
}
