//! NPC spawner for creating and positioning virtual cyclists.

use super::{NpcCyclist, NpcDifficulty};

/// NPC appearance configuration
#[derive(Debug, Clone)]
pub struct NpcAppearance {
    /// Jersey primary color (RGB)
    pub jersey_color: [u8; 3],
    /// Jersey secondary color (RGB)
    pub jersey_secondary: Option<[u8; 3]>,
    /// Bike color (RGB)
    pub bike_color: [u8; 3],
    /// Display name
    pub name: String,
}

impl NpcAppearance {
    /// Create appearance from index
    pub fn from_index(index: u8) -> Self {
        let appearances = [
            (
                "Team Blue",
                [0, 100, 200],
                Some([255, 255, 255]),
                [30, 30, 30],
            ),
            (
                "Team Red",
                [200, 50, 50],
                Some([255, 255, 255]),
                [30, 30, 30],
            ),
            (
                "Team Green",
                [50, 150, 50],
                Some([255, 255, 0]),
                [50, 50, 50],
            ),
            ("Team Yellow", [255, 200, 0], Some([0, 0, 0]), [40, 40, 40]),
            ("Team Orange", [255, 130, 0], Some([0, 0, 0]), [30, 30, 30]),
            (
                "Team Purple",
                [100, 50, 150],
                Some([255, 255, 255]),
                [60, 60, 60],
            ),
            ("Team Pink", [255, 100, 150], Some([0, 0, 0]), [30, 30, 30]),
            ("Team Black", [30, 30, 30], Some([255, 0, 0]), [60, 60, 60]),
        ];

        let (name, primary, secondary, bike) = &appearances[index as usize % appearances.len()];

        Self {
            jersey_color: *primary,
            jersey_secondary: *secondary,
            bike_color: *bike,
            name: name.to_string(),
        }
    }
}

/// NPC spawner configuration
#[allow(dead_code)]
pub struct NpcSpawner {
    /// User's FTP for difficulty calculation
    user_ftp: u16,
    /// Difficulty setting
    difficulty: NpcDifficulty,
    /// Route length for spawn distribution
    route_length: f64,
    /// Minimum spacing between NPCs (meters)
    min_spacing: f64,
}

impl NpcSpawner {
    /// Create spawner for a route
    pub fn new(user_ftp: u16, difficulty: NpcDifficulty, route_length: f64) -> Self {
        Self {
            user_ftp,
            difficulty,
            route_length,
            min_spacing: 20.0,
        }
    }

    /// Spawn a specified number of NPCs
    pub fn spawn(&self, count: u8) -> Vec<NpcCyclist> {
        let target_power = self.target_power();
        let mut npcs = Vec::with_capacity(count as usize);

        // Distribute NPCs across first 50% of route with random variation
        let spawn_range = self.route_length * 0.5;
        let base_spacing = spawn_range / count as f64;

        for i in 0..count {
            let base_distance = base_spacing * (i as f64 + 0.5);
            let variation = (rand_simple() as f64 - 0.5) * base_spacing * 0.3;
            let distance = (base_distance + variation).max(0.0);

            let appearance = NpcAppearance::from_index(i);
            let power_variation = 1.0 + (rand_simple() - 0.5) * 0.2;
            let npc_power = (target_power as f32 * power_variation) as u16;

            npcs.push(NpcCyclist::new(
                i as u32,
                appearance.name,
                distance,
                npc_power,
                i % 8,
            ));
        }

        npcs
    }

    /// Calculate target power for this difficulty
    pub fn target_power(&self) -> u16 {
        (self.user_ftp as f32 * self.difficulty.ftp_multiplier()) as u16
    }

    /// Generate random appearance
    pub fn random_appearance(&self) -> NpcAppearance {
        let index = (rand_simple() * 8.0) as u8;
        NpcAppearance::from_index(index)
    }
}

/// Spawn point determination strategies
pub enum SpawnStrategy {
    /// Spread evenly across route
    EvenDistribution,
    /// Cluster around user start position
    NearUser,
    /// Random positions
    Random,
    /// Peloton formation
    Peloton,
}

/// Generate spawn positions using a strategy
pub fn generate_spawn_positions(
    count: u8,
    route_length: f64,
    user_start: f64,
    strategy: SpawnStrategy,
) -> Vec<f64> {
    match strategy {
        SpawnStrategy::EvenDistribution => {
            let spacing = route_length * 0.5 / count as f64;
            (0..count).map(|i| spacing * (i as f64 + 0.5)).collect()
        }
        SpawnStrategy::NearUser => {
            let spread = 200.0; // 200m spread
            (0..count)
                .map(|i| {
                    let offset = (i as f64 - count as f64 / 2.0) * (spread / count as f64);
                    (user_start + offset).max(0.0).min(route_length)
                })
                .collect()
        }
        SpawnStrategy::Random => (0..count)
            .map(|_| rand_simple() as f64 * route_length * 0.5)
            .collect(),
        SpawnStrategy::Peloton => {
            // Create a pack formation
            let pack_center = user_start + 50.0;
            let _rows = (count as f64 / 3.0).ceil() as u8;
            let mut positions = Vec::with_capacity(count as usize);

            for i in 0..count {
                let row = i / 3;
                let col = i % 3;
                let _x_offset = (col as f64 - 1.0) * 1.5; // 1.5m lateral
                let y_offset = row as f64 * 3.0; // 3m longitudinal
                positions.push(pack_center + y_offset);
            }

            positions
        }
    }
}

/// Simple random number (0.0-1.0)
fn rand_simple() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    nanos as f32 / 4_294_967_295.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_appearance_from_index() {
        let appearance = NpcAppearance::from_index(0);
        assert!(!appearance.name.is_empty());
    }

    #[test]
    fn test_spawner_target_power() {
        let spawner = NpcSpawner::new(250, NpcDifficulty::Medium, 10000.0);
        let power = spawner.target_power();
        assert_eq!(power, 200); // 250 * 0.8
    }

    #[test]
    fn test_spawn_count() {
        let spawner = NpcSpawner::new(250, NpcDifficulty::Medium, 10000.0);
        let npcs = spawner.spawn(5);
        assert_eq!(npcs.len(), 5);
    }

    #[test]
    fn test_spawn_positions_even() {
        let positions = generate_spawn_positions(4, 10000.0, 0.0, SpawnStrategy::EvenDistribution);
        assert_eq!(positions.len(), 4);
        // Should be roughly evenly spaced
        assert!(positions[0] < positions[1]);
        assert!(positions[1] < positions[2]);
    }
}
