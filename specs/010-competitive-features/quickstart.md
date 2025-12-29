# Quickstart: Competitive Feature Gaps

**Feature**: 010-competitive-features
**Date**: 2025-12-28

## Overview

This feature adds five competitive capabilities to RustRide:

| Priority | Feature | Description |
|----------|---------|-------------|
| P1 | Gradient-Responsive Resistance | Auto-adjust trainer resistance based on GPX route elevation |
| P2 | Achievement Badges & XP | Gamification with 50+ achievements and level progression |
| P3 | 4D Power Profiling | Multi-duration power analysis with 90-day rolling window |
| P4 | Multi-Discipline Plans | Training plans for road, gravel, triathlon, MTB |
| P5 | Career Levels | 50-level progression with cosmetic unlocks |

## Prerequisites

- Rust 1.75+ installed
- RustRide codebase cloned
- Existing dependencies (`cargo build` succeeds)

## Development Setup

```bash
# Ensure you're on the feature branch
git checkout 010-competitive-features

# Build the project
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run
```

## Feature Dependencies

```
P5: Career Levels
      │
      ▼
P2: Achievement Badges & XP ──┐
      │                       │
      ▼                       ▼
P3: 4D Power Profiling    P4: Training Plans
      │
      ▼
P1: Gradient-Responsive Resistance (independent)
```

- **P1** can be developed independently
- **P2** must be completed before P5
- **P3** and **P4** can be developed in parallel after P2
- **P5** depends on P2's XP system

## Quick Implementation Guide

### P1: Gradient-Responsive Resistance

**Location**: `src/gradient/`

```rust
// 1. Create gradient calculator
use crate::world::import::gpx::parse_gpx;
use crate::sensors::ftms::build_set_simulation_with_crr;

// 2. Calculate gradient between points
fn calculate_gradient(p1: &GpsPoint, p2: &GpsPoint) -> f32 {
    let distance = haversine_distance(p1, p2);
    let elevation_change = p2.elevation.unwrap_or(0.0) - p1.elevation.unwrap_or(0.0);
    (elevation_change / distance) * 100.0
}

// 3. Send FTMS command
let cmd = build_set_simulation_with_crr(gradient_percent, 0.004);
// Write cmd to FTMS Control Point characteristic
```

**Test**: Load GPX file, verify trainer resistance changes with elevation.

### P2: Achievement Badges & XP

**Location**: `src/achievements/`

```rust
// 1. Extend existing Achievement struct
use crate::world::achievements::{Achievement, AchievementTier};

impl Achievement {
    pub fn xp_value(&self) -> u32 {
        self.xp_override.unwrap_or_else(|| self.tier.base_xp())
    }
}

// 2. Create tracker
pub struct AchievementTracker {
    earned: HashSet<String>,
    pending_notifications: VecDeque<AchievementNotification>,
    total_xp: u64,
    current_level: u32,
}

// 3. Check achievements on ride end
fn on_ride_complete(&mut self, ride: &Ride) {
    for achievement in self.check_ride_achievements(ride) {
        self.queue_notification(achievement);
        self.add_xp(achievement.xp_value());
    }
}
```

**Test**: Complete rides, verify badges earned and XP accumulated.

### P3: 4D Power Profiling

**Location**: `src/power_profile/`

```rust
// 1. Use existing PDC module
use crate::metrics::analytics::pdc::{MmpCalculator, PowerDurationCurve};

// 2. Add rolling window filter
fn get_current_profile(&self, user_id: i64) -> PowerDurationCurve {
    let ninety_days_ago = Utc::now() - Duration::days(90);
    self.pdc.filter_date_range(ninety_days_ago, Utc::now())
}

// 3. Analyze strengths/weaknesses
fn analyze(&self, pdc: &PowerDurationCurve, weight_kg: f32) -> ProfileAnalysis {
    let reference = get_reference_curve(weight_kg);
    // Compare user curve to reference at each duration
}
```

**Test**: Process ride history, verify 90-day vs lifetime separation.

### P4: Multi-Discipline Training Plans

**Location**: `src/training_plans/`

```rust
// 1. Define disciplines
pub enum Discipline {
    Road, Gravel, Triathlon, MTB, GeneralFitness,
}

// 2. Create plan structure
pub struct TrainingPlan {
    id: Uuid,
    name: String,
    discipline: Discipline,
    duration_weeks: u8,
    weeks: Vec<PlanWeek>,
}

// 3. Integrate with existing workout library
fn get_workout(&self, id: Uuid) -> Option<&Workout> {
    self.workout_library.get(&id)
}
```

**Test**: Assign plan, verify workouts appear on correct days.

### P5: Career Levels

**Location**: `src/career/`

```rust
// 1. Define XP curve
fn xp_for_level(level: u32) -> u64 {
    (1000.0 * 1.15_f64.powi(level as i32 - 1)) as u64
}

// 2. Define rewards
const REWARDS: &[(u32, RewardType, &str)] = &[
    (5, RewardType::AccentColor, "blue_accent"),
    (10, RewardType::JerseyColor, "red_jersey"),
    // ...
];

// 3. Check level up on XP change
fn add_xp(&mut self, xp: u32) -> Option<LevelUpEvent> {
    self.total_xp += xp as u64;
    let new_level = level_from_xp(self.total_xp);
    if new_level > self.current_level {
        // Return level up event with unlocked rewards
    }
    None
}
```

**Test**: Add XP, verify level ups trigger at correct thresholds.

## Database Migration

Add to `src/storage/schema.rs`:

```rust
pub const MIGRATION_V8_TO_V9: &str = r#"
    -- Achievement system
    CREATE TABLE user_achievements (
        user_id INTEGER NOT NULL,
        achievement_key TEXT NOT NULL,
        earned_at TEXT NOT NULL,
        ride_id TEXT,
        progress_value REAL,
        PRIMARY KEY (user_id, achievement_key)
    );

    CREATE TABLE user_xp (
        user_id INTEGER PRIMARY KEY,
        total_xp INTEGER NOT NULL DEFAULT 0,
        current_level INTEGER NOT NULL DEFAULT 1,
        updated_at TEXT NOT NULL
    );

    -- ... additional tables from data-model.md

    INSERT INTO schema_version (version) VALUES (9);
"#;
```

## Testing Strategy

```bash
# Unit tests for each module
cargo test gradient::
cargo test achievements::
cargo test power_profile::
cargo test training_plans::
cargo test career::

# Integration tests
cargo test --test gradient_ride_tests
cargo test --test achievement_flow_tests

# Run specific test
cargo test test_xp_curve -- --nocapture
```

## Key Files to Modify

| File | Change |
|------|--------|
| `src/lib.rs` | Add new module exports |
| `src/storage/mod.rs` | Add new store modules |
| `src/storage/schema.rs` | Add migration |
| `src/ui/screens/mod.rs` | Add new screens |
| `src/world/achievements/definitions.rs` | Add XP values |
| `src/app.rs` | Integrate new features into app state |

## Debugging Tips

```rust
// Enable tracing for specific modules
RUST_LOG=rustride::gradient=debug cargo run

// Check FTMS commands being sent
RUST_LOG=rustride::sensors::ftms=trace cargo run

// View achievement checks
RUST_LOG=rustride::achievements=debug cargo run
```

## Common Issues

1. **GPX with no elevation**: Falls back to 0% gradient
2. **Trainer not responding to simulation**: Check FTMS feature support
3. **XP not persisting**: Verify database migration ran
4. **Achievement not triggering**: Check cumulative totals in database

## Next Steps

After implementation:
1. Run `/speckit.tasks` to generate detailed task breakdown
2. Create unit tests for each contract
3. Implement in priority order (P1 → P2 → P3/P4 → P5)
4. Add UI screens last (after core logic works)
