# Contract: NPC System Module

**Module**: `src/world/npc/`
**Date**: 2025-12-25

## Purpose

Manage AI-controlled virtual cyclists that populate routes, providing visual company and pacing references for solo riders.

## Public API

### Types

```rust
/// NPC manager controls all NPCs on the route
pub struct NpcManager {
    npcs: Vec<NpcCyclist>,
    settings: NpcSettings,
    spawner: NpcSpawner,
}

/// NPC appearance for rendering
pub struct NpcAppearance {
    pub jersey_color: [u8; 3],
    pub jersey_secondary: Option<[u8; 3]>,
    pub bike_color: [u8; 3],
    pub name: String,
}
```

### Functions

```rust
impl NpcManager {
    /// Create NPC manager for a route
    pub fn new(
        route: &Route,
        settings: NpcSettings,
        user_ftp: u16,
    ) -> Self;

    /// Get current NPC settings
    pub fn settings(&self) -> &NpcSettings;

    /// Update settings (respawns NPCs if count/difficulty changes)
    pub fn set_settings(&mut self, settings: NpcSettings, user_ftp: u16);

    /// Get all active NPCs
    pub fn npcs(&self) -> &[NpcCyclist];

    /// Get NPC count
    pub fn count(&self) -> usize;

    /// Update all NPCs
    ///
    /// # Arguments
    /// * `delta_time` - Time since last update in seconds
    /// * `route` - Current route for gradient data
    /// * `user_distance` - User's position on route (meters from start)
    pub fn update(
        &mut self,
        delta_time: f32,
        route: &Route,
        user_distance: f64,
    );

    /// Check if user is in draft zone of any NPC
    ///
    /// Returns drafting state with benefit calculation.
    pub fn check_drafting(&self, user_distance: f64) -> DraftingState;

    /// Get positions for rendering
    pub fn render_data(&self) -> Vec<NpcRenderData>;

    /// Reset all NPCs to initial positions
    pub fn reset(&mut self);

    /// Get statistics for ride summary
    pub fn stats(&self) -> NpcStats;
}

pub struct NpcRenderData {
    pub position: Vec3,
    pub rotation: f32,
    pub appearance: NpcAppearance,
    pub animation_phase: f32,
}

pub struct NpcStats {
    pub npcs_passed: u32,
    pub npcs_passed_by: u32,
    pub total_draft_time_seconds: f32,
    pub max_draft_benefit_percent: f32,
}
```

### NPC AI Behavior

```rust
impl NpcCyclist {
    /// Calculate speed based on power and gradient
    ///
    /// Uses same physics model as player for consistency.
    pub fn calculate_speed(&self, gradient: f32) -> f32;

    /// Update position along route
    pub fn update(
        &mut self,
        delta_time: f32,
        gradient: f32,
        user_distance: f64,
    );

    /// Get 3D position from route distance
    pub fn position(&self, route: &Route) -> Vec3;

    /// Get heading direction
    pub fn direction(&self, route: &Route) -> f32;
}
```

### NPC Spawner

```rust
impl NpcSpawner {
    /// Create spawner for route
    pub fn new(route: &Route, count: u8, user_ftp: u16, difficulty: NpcDifficulty) -> Self;

    /// Generate initial NPC positions
    pub fn spawn_initial(&self) -> Vec<NpcCyclist>;

    /// Generate NPC appearance
    pub fn random_appearance(&self, index: u8) -> NpcAppearance;

    /// Calculate target power for difficulty
    pub fn target_power(&self, difficulty: NpcDifficulty) -> u16;
}
```

## Drafting Detection

```
Draft Zone:
- Distance behind: 1-5 meters
- Lateral tolerance: ±1 meter
- Benefit: 20-30% (linear interpolation based on distance)

Draft Calculation:
benefit = 30% at 1m behind
benefit = 20% at 5m behind
benefit = 0% at >5m or >1m lateral offset
```

## NPC Speed Calculation

```
Base Power = User FTP × Difficulty Multiplier
  - Easy: 0.5
  - Medium: 0.8
  - MatchUser: 1.0
  - Hard: 1.1
  - VeryHard: 1.3

Power Variation: ±10% random fluctuation every 5-10 seconds
Speed = f(power, gradient, mass=75kg) using physics engine
```

## Spawn Distribution

```
Initial Spawn:
- Spread evenly over first 50% of route
- Mix of ahead and behind user start position
- Minimum spacing: 20 meters

Respawn (when too far behind):
- Teleport to random position ahead of user
- Only if >500m behind user
- Preserve appearance
```

## Performance Requirements

- Update 50 NPCs: <1ms
- Drafting detection: <0.1ms
- Render data generation: <0.5ms
- Memory per NPC: ~200 bytes

## Rendering Integration

NPCs are rendered as instanced cyclist models using the existing avatar rendering system with different jersey colors and bike styles.
