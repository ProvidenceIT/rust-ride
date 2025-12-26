//! Collectible items scattered throughout the world.

use chrono::{DateTime, Utc};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Collectible type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CollectibleType {
    /// Standard point pickup
    Star,
    /// Bonus point pickup
    GoldStar,
    /// XP boost item
    XpBoost,
    /// Power-up item
    PowerUp,
    /// Hidden/secret item
    Hidden,
    /// Seasonal/event item
    Seasonal,
    /// Route badge
    Badge,
}

impl CollectibleType {
    /// Get base point value
    pub fn points(&self) -> u32 {
        match self {
            Self::Star => 10,
            Self::GoldStar => 50,
            Self::XpBoost => 100,
            Self::PowerUp => 25,
            Self::Hidden => 200,
            Self::Seasonal => 150,
            Self::Badge => 500,
        }
    }

    /// Get rarity (0 = common, 4 = legendary)
    pub fn rarity(&self) -> u8 {
        match self {
            Self::Star => 0,
            Self::GoldStar => 1,
            Self::XpBoost => 2,
            Self::PowerUp => 1,
            Self::Hidden => 3,
            Self::Seasonal => 2,
            Self::Badge => 4,
        }
    }
}

/// A collectible in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collectible {
    /// Unique identifier
    pub id: Uuid,
    /// Type of collectible
    pub collectible_type: CollectibleType,
    /// Route this collectible is on
    pub route_id: Option<Uuid>,
    /// Distance from route start (meters)
    pub distance_meters: f64,
    /// World position
    pub position: Vec3,
    /// Whether this respawns
    pub respawns: bool,
    /// Respawn time in seconds (if applicable)
    pub respawn_time: Option<u32>,
}

impl Collectible {
    /// Create new collectible
    pub fn new(collectible_type: CollectibleType, distance_meters: f64, position: Vec3) -> Self {
        Self {
            id: Uuid::new_v4(),
            collectible_type,
            route_id: None,
            distance_meters,
            position,
            respawns: matches!(
                collectible_type,
                CollectibleType::Star | CollectibleType::GoldStar | CollectibleType::PowerUp
            ),
            respawn_time: match collectible_type {
                CollectibleType::Star => Some(60),
                CollectibleType::GoldStar => Some(300),
                CollectibleType::PowerUp => Some(120),
                _ => None,
            },
        }
    }

    /// Associate with a route
    pub fn on_route(mut self, route_id: Uuid) -> Self {
        self.route_id = Some(route_id);
        self
    }
}

/// Record of a collected item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectedItem {
    /// Unique collection ID
    pub id: Uuid,
    /// User who collected
    pub user_id: Uuid,
    /// Collectible ID
    pub collectible_id: Uuid,
    /// Ride during which collected
    pub ride_id: Uuid,
    /// Points earned
    pub points: u32,
    /// When collected
    pub collected_at: DateTime<Utc>,
}

impl CollectedItem {
    /// Create new collection record
    pub fn new(user_id: Uuid, collectible: &Collectible, ride_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            collectible_id: collectible.id,
            ride_id,
            points: collectible.collectible_type.points(),
            collected_at: Utc::now(),
        }
    }
}

/// Collectible manager for a ride
pub struct CollectibleManager {
    /// All collectibles on current route
    collectibles: Vec<Collectible>,
    /// Collected items this ride (collectible_id -> collected_at)
    collected: std::collections::HashMap<Uuid, DateTime<Utc>>,
    /// Total points collected this ride
    points: u32,
    /// Collection radius (meters)
    collection_radius: f64,
}

impl CollectibleManager {
    /// Create collectible manager
    pub fn new() -> Self {
        Self {
            collectibles: Vec::new(),
            collected: std::collections::HashMap::new(),
            points: 0,
            collection_radius: 5.0,
        }
    }

    /// Load collectibles for a route
    pub fn load_route(&mut self, collectibles: Vec<Collectible>) {
        self.collectibles = collectibles;
        self.collected.clear();
        self.points = 0;
    }

    /// Get collectibles near a position
    pub fn nearby(&self, distance: f64, radius: f64) -> Vec<&Collectible> {
        self.collectibles
            .iter()
            .filter(|c| {
                let dist = (c.distance_meters - distance).abs();
                dist <= radius && !self.is_collected(c.id)
            })
            .collect()
    }

    /// Check if collectible has been collected
    pub fn is_collected(&self, id: Uuid) -> bool {
        if let Some(collected_at) = self.collected.get(&id) {
            // Check if respawned
            if let Some(collectible) = self.collectibles.iter().find(|c| c.id == id) {
                if collectible.respawns {
                    if let Some(respawn_time) = collectible.respawn_time {
                        let elapsed = Utc::now()
                            .signed_duration_since(*collected_at)
                            .num_seconds() as u32;
                        return elapsed < respawn_time;
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// Try to collect items at position
    pub fn try_collect(
        &mut self,
        distance: f64,
        user_id: Uuid,
        ride_id: Uuid,
    ) -> Vec<CollectedItem> {
        let mut collected = Vec::new();

        for collectible in &self.collectibles {
            let dist = (collectible.distance_meters - distance).abs();

            if dist <= self.collection_radius && !self.is_collected(collectible.id) {
                let item = CollectedItem::new(user_id, collectible, ride_id);
                self.points += item.points;
                self.collected.insert(collectible.id, Utc::now());
                collected.push(item);
            }
        }

        collected
    }

    /// Get total points collected
    pub fn points(&self) -> u32 {
        self.points
    }

    /// Get collection statistics
    pub fn stats(&self) -> CollectionStats {
        let total = self.collectibles.len();
        let collected = self.collected.len();

        let by_type: std::collections::HashMap<CollectibleType, u32> = self
            .collectibles
            .iter()
            .filter(|c| self.collected.contains_key(&c.id))
            .fold(std::collections::HashMap::new(), |mut acc, c| {
                *acc.entry(c.collectible_type).or_insert(0) += 1;
                acc
            });

        CollectionStats {
            total,
            collected,
            points: self.points,
            by_type,
        }
    }

    /// Reset for new ride
    pub fn reset(&mut self) {
        self.collected.clear();
        self.points = 0;
    }
}

impl Default for CollectibleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Collection statistics
#[derive(Debug, Clone, Default)]
pub struct CollectionStats {
    /// Total collectibles on route
    pub total: usize,
    /// Collected this ride
    pub collected: usize,
    /// Points earned
    pub points: u32,
    /// Breakdown by type
    pub by_type: std::collections::HashMap<CollectibleType, u32>,
}

impl CollectionStats {
    /// Get collection percentage
    pub fn percentage(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.collected as f32 / self.total as f32) * 100.0
        }
    }
}

/// Generate collectibles for a route
pub fn generate_collectibles(route_id: Uuid, route_length: f64, density: f32) -> Vec<Collectible> {
    let mut collectibles = Vec::new();

    // Base spacing in meters
    let base_spacing = 500.0 / density as f64;
    let count = (route_length / base_spacing) as usize;

    for i in 0..count {
        let distance = (i as f64 + 0.5) * base_spacing;

        // Determine type based on position and random factor
        let collectible_type = if i % 20 == 0 {
            CollectibleType::GoldStar
        } else if i % 10 == 0 {
            CollectibleType::XpBoost
        } else if i % 7 == 0 {
            CollectibleType::PowerUp
        } else {
            CollectibleType::Star
        };

        // Simple world position (would be calculated from route geometry)
        let position = Vec3::new(distance as f32, 1.5, 0.0);

        collectibles
            .push(Collectible::new(collectible_type, distance, position).on_route(route_id));
    }

    collectibles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collectible_types() {
        assert!(CollectibleType::GoldStar.points() > CollectibleType::Star.points());
        assert!(CollectibleType::Hidden.rarity() > CollectibleType::Star.rarity());
    }

    #[test]
    fn test_collectible_manager() {
        let mut manager = CollectibleManager::new();

        let route_id = Uuid::new_v4();
        let collectibles = generate_collectibles(route_id, 10000.0, 1.0);

        manager.load_route(collectibles);

        // Collect at first position
        let user_id = Uuid::new_v4();
        let ride_id = Uuid::new_v4();
        let collected = manager.try_collect(250.0, user_id, ride_id);

        assert!(!collected.is_empty());
        assert!(manager.points() > 0);
    }

    #[test]
    fn test_generate_collectibles() {
        let route_id = Uuid::new_v4();
        let collectibles = generate_collectibles(route_id, 5000.0, 1.0);

        // Should have approximately 10 collectibles (5000 / 500)
        assert!(collectibles.len() >= 8 && collectibles.len() <= 12);

        // All should have the route ID
        assert!(collectibles.iter().all(|c| c.route_id == Some(route_id)));
    }
}
