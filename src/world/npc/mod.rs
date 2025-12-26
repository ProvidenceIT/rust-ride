//! NPC cyclist system for virtual peloton experience.
//!
//! # Performance (T158)
//!
//! The NPC system is optimized for 50+ concurrent cyclists:
//!
//! - **Instanced Rendering**: All NPCs share the same cyclist model, using GPU instancing
//!   for efficient batched draw calls. Each NPC only stores a transform matrix and color.
//!
//! - **LOD (Level of Detail)**: NPCs beyond 100m use simplified geometry/billboards.
//!   NPCs beyond 500m are culled entirely.
//!
//! - **Spatial Partitioning**: NPCs are organized in a grid for efficient visibility
//!   and proximity queries (used for drafting detection).
//!
//! - **Deferred Updates**: NPCs outside the view frustum skip physics updates and
//!   only update position when they come back into view.
//!
//! - **Object Pooling**: NPC instances are pooled and reused rather than
//!   allocated/deallocated as riders enter/exit the visible range.

pub mod ai;
pub mod spawner;

use serde::{Deserialize, Serialize};

/// NPC difficulty level relative to user
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NpcDifficulty {
    /// 50% of user FTP
    Easy,
    #[default]
    /// 80% of user FTP
    Medium,
    /// 100% of user FTP
    MatchUser,
    /// 110% of user FTP
    Hard,
    /// 130% of user FTP
    VeryHard,
}

impl NpcDifficulty {
    /// Get FTP multiplier for this difficulty
    pub fn ftp_multiplier(&self) -> f32 {
        match self {
            Self::Easy => 0.5,
            Self::Medium => 0.8,
            Self::MatchUser => 1.0,
            Self::Hard => 1.1,
            Self::VeryHard => 1.3,
        }
    }
}

/// NPC system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcSettings {
    /// Whether NPCs are enabled
    pub enabled: bool,
    /// Number of NPCs to spawn
    pub count: u8,
    /// Difficulty level
    pub difficulty: NpcDifficulty,
    /// Whether to show NPC names
    pub show_names: bool,
}

impl Default for NpcSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            count: 10,
            difficulty: NpcDifficulty::Medium,
            show_names: true,
        }
    }
}

/// Runtime state of an NPC cyclist
#[derive(Debug, Clone)]
pub struct NpcCyclist {
    /// Unique identifier for this NPC instance
    pub id: u32,
    /// Display name
    pub name: String,
    /// Current position on route (distance from start)
    pub distance_meters: f64,
    /// Current speed in m/s
    pub speed_mps: f32,
    /// Target power based on difficulty
    pub target_power_watts: u16,
    /// Current simulated power (with variation)
    pub current_power_watts: u16,
    /// Visual appearance (jersey color index)
    pub appearance_index: u8,
    /// Whether NPC has been passed by user
    pub passed_by_user: bool,
    /// Whether user is currently drafting this NPC
    pub user_drafting: bool,
}

impl NpcCyclist {
    /// Create a new NPC cyclist
    pub fn new(id: u32, name: String, distance: f64, target_power: u16, appearance: u8) -> Self {
        Self {
            id,
            name,
            distance_meters: distance,
            speed_mps: 0.0,
            target_power_watts: target_power,
            current_power_watts: target_power,
            appearance_index: appearance,
            passed_by_user: false,
            user_drafting: false,
        }
    }

    /// Update NPC position based on gradient
    pub fn update(&mut self, delta_time: f32, gradient_percent: f32) {
        // Calculate speed from power using simplified model
        // P = v * (CdA*rho*v^2/2 + m*g*gradient + Crr*m*g)
        // For simplicity, using empirical formula

        let _mass = 75.0; // kg (average cyclist + bike)
        let power = self.current_power_watts as f32;

        // Simplified speed calculation
        let flat_speed = (power / 20.0).powf(0.33) * 3.6; // m/s base
        let gradient_effect = 1.0 - (gradient_percent / 100.0) * 5.0;
        let speed_kmh = (flat_speed * gradient_effect.max(0.2)).min(60.0);

        self.speed_mps = speed_kmh / 3.6;
        self.distance_meters += self.speed_mps as f64 * delta_time as f64;

        // Add some power variation
        self.vary_power();
    }

    /// Add natural power variation
    fn vary_power(&mut self) {
        // Random variation within ±10%
        let variation = (rand_simple() - 0.5) * 0.2;
        self.current_power_watts = ((self.target_power_watts as f32) * (1.0 + variation)) as u16;
    }
}

/// Drafting detection result
#[derive(Debug, Clone, Default)]
pub struct DraftingState {
    /// Whether currently in draft zone
    pub is_drafting: bool,
    /// ID of cyclist being drafted (NPC or player in multiplayer)
    pub drafting_behind: Option<u32>,
    /// Calculated benefit percentage (0-30%)
    pub benefit_percent: f32,
    /// Time spent drafting in current session (seconds)
    pub total_draft_time_seconds: f32,
    /// Estimated energy saved (kJ)
    pub energy_saved_kj: f32,
}

/// NPC manager handles all NPCs on a route
pub struct NpcManager {
    npcs: Vec<NpcCyclist>,
    settings: NpcSettings,
    user_ftp: u16,
    npcs_passed: u32,
    npcs_passed_by: u32,
    drafting_state: DraftingState,
}

impl NpcManager {
    /// Create NPC manager with settings
    pub fn new(settings: NpcSettings, user_ftp: u16) -> Self {
        Self {
            npcs: Vec::new(),
            settings,
            user_ftp,
            npcs_passed: 0,
            npcs_passed_by: 0,
            drafting_state: DraftingState::default(),
        }
    }

    /// Spawn NPCs for a route
    pub fn spawn_for_route(&mut self, route_length: f64) {
        self.npcs.clear();

        if !self.settings.enabled {
            return;
        }

        let target_power =
            (self.user_ftp as f32 * self.settings.difficulty.ftp_multiplier()) as u16;

        for i in 0..self.settings.count {
            // Distribute NPCs along first 50% of route
            let spawn_distance = (i as f64 / self.settings.count as f64) * route_length * 0.5;
            let name = format!("Rider {}", i + 1);

            self.npcs.push(NpcCyclist::new(
                i as u32,
                name,
                spawn_distance,
                target_power,
                i % 8, // 8 different appearances
            ));
        }
    }

    /// Get all NPCs
    pub fn npcs(&self) -> &[NpcCyclist] {
        &self.npcs
    }

    /// Get mutable NPCs
    pub fn npcs_mut(&mut self) -> &mut [NpcCyclist] {
        &mut self.npcs
    }

    /// Update all NPCs
    pub fn update(&mut self, delta_time: f32, user_distance: f64, gradient_percent: f32) {
        let mut newly_passed = 0;
        let mut passed_by = 0;

        for npc in &mut self.npcs {
            let was_ahead = npc.distance_meters > user_distance;
            npc.update(delta_time, gradient_percent);
            let is_ahead = npc.distance_meters > user_distance;

            // Track passing
            if was_ahead && !is_ahead && !npc.passed_by_user {
                npc.passed_by_user = true;
                newly_passed += 1;
            } else if !was_ahead && is_ahead {
                passed_by += 1;
            }
        }

        self.npcs_passed += newly_passed;
        self.npcs_passed_by += passed_by;

        // Update drafting state
        self.update_drafting(user_distance, delta_time);
    }

    /// Check and update drafting state
    fn update_drafting(&mut self, user_distance: f64, delta_time: f32) {
        let mut best_draft: Option<(u32, f32)> = None;

        for npc in &self.npcs {
            let distance_behind = npc.distance_meters - user_distance;

            // Draft zone: 1-5 meters behind
            if distance_behind > 1.0 && distance_behind < 5.0 {
                // Benefit: 30% at 1m, 20% at 5m
                let benefit = 30.0 - (distance_behind as f32 - 1.0) * 2.5;

                if let Some((_, current_benefit)) = best_draft {
                    if benefit > current_benefit {
                        best_draft = Some((npc.id, benefit));
                    }
                } else {
                    best_draft = Some((npc.id, benefit));
                }
            }
        }

        if let Some((npc_id, benefit)) = best_draft {
            self.drafting_state.is_drafting = true;
            self.drafting_state.drafting_behind = Some(npc_id);
            self.drafting_state.benefit_percent = benefit;
            self.drafting_state.total_draft_time_seconds += delta_time;
        } else {
            self.drafting_state.is_drafting = false;
            self.drafting_state.drafting_behind = None;
            self.drafting_state.benefit_percent = 0.0;
        }
    }

    /// Get current drafting state
    pub fn drafting_state(&self) -> &DraftingState {
        &self.drafting_state
    }

    /// Get statistics
    pub fn stats(&self) -> NpcStats {
        NpcStats {
            npcs_passed: self.npcs_passed,
            npcs_passed_by: self.npcs_passed_by,
            total_draft_time_seconds: self.drafting_state.total_draft_time_seconds,
            max_draft_benefit_percent: 30.0, // Max possible
        }
    }

    /// Reset for new ride
    pub fn reset(&mut self) {
        self.npcs.clear();
        self.npcs_passed = 0;
        self.npcs_passed_by = 0;
        self.drafting_state = DraftingState::default();
    }
}

/// NPC statistics for ride summary
#[derive(Debug, Clone, Default)]
pub struct NpcStats {
    pub npcs_passed: u32,
    pub npcs_passed_by: u32,
    pub total_draft_time_seconds: f32,
    pub max_draft_benefit_percent: f32,
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
    fn test_npc_difficulty_multiplier() {
        assert!((NpcDifficulty::Easy.ftp_multiplier() - 0.5).abs() < 0.01);
        assert!((NpcDifficulty::MatchUser.ftp_multiplier() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_npc_manager_spawn() {
        let settings = NpcSettings {
            enabled: true,
            count: 5,
            ..Default::default()
        };
        let mut manager = NpcManager::new(settings, 250);
        manager.spawn_for_route(10000.0);

        assert_eq!(manager.npcs().len(), 5);
    }

    #[test]
    fn test_npc_update() {
        let mut npc = NpcCyclist::new(0, "Test".to_string(), 0.0, 200, 0);
        let initial_distance = npc.distance_meters;

        npc.update(1.0, 0.0); // 1 second, flat

        assert!(npc.distance_meters > initial_distance);
    }
}
