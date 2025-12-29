# Research: Competitive Feature Gaps

**Feature**: 010-competitive-features
**Date**: 2025-12-28

## Research Summary

All technical unknowns have been resolved through codebase analysis. The existing infrastructure supports all five competitive features with minimal new code.

---

## 1. Gradient-Responsive Resistance

### Decision: Use FTMS Simulation Mode Commands

**Rationale**: The existing `sensors/ftms.rs` module already implements `build_set_simulation_grade()` and `build_set_simulation()` functions that construct the correct FTMS Control Point commands for grade simulation. The `FtmsControlOpcode::SetIndoorBikeSimulation (0x11)` command accepts grade in 0.01% resolution.

**Alternatives Considered**:
- `SetTargetInclination (0x03)`: Simpler but less widely supported by trainers
- `SetTargetResistanceLevel (0x04)`: Doesn't simulate grade physics, just raw resistance
- `SetTargetPower (0x05)`: ERG mode, not suitable for gradient simulation

**Implementation Notes**:
- Use `build_set_simulation_with_crr()` for grade + rolling resistance
- Default CRR: 0.004 (road), CW: 0.51 (default)
- Grade range: -100% to +100% in 0.01% resolution
- User-configurable gradient cap with smooth transitions (per clarification)

### Decision: Reuse Existing GPX Parser

**Rationale**: `world/import/gpx.rs` already parses GPX files and extracts `GpsPoint` with latitude, longitude, and elevation. The `parse_gpx()` function returns `Vec<GpsPoint>` with optional elevation data.

**Implementation Notes**:
- Calculate gradient: `(elevation_change / distance) * 100`
- Use Haversine formula for point-to-point distance
- Smooth gradient transitions to avoid jerky resistance changes
- Handle missing elevation: fallback to 0% grade

---

## 2. Achievement Badges & XP System

### Decision: Extend Existing Achievement Infrastructure

**Rationale**: `world/achievements/definitions.rs` already defines 40+ achievements with categories (Distance, Climbing, Consistency, Competition, Exploration, Training, Special) and tiers (Bronze, Silver, Gold, Diamond, Legendary). The `Achievement` struct can be extended with XP values.

**Alternatives Considered**:
- New achievement system from scratch: Unnecessary duplication
- Third-party gamification library: Over-engineering, no Rust options

**Implementation Notes**:
- Add `xp_value: u32` field to `Achievement` struct
- XP tiers: Bronze=100, Silver=250, Gold=500, Diamond=1000, Legendary=2500
- Secret achievements: 1.5x XP bonus
- Store earned achievements with timestamps in SQLite

### Decision: Queue Notifications During Rides

**Rationale**: Per clarification, notifications should appear at natural break points (interval rest, pause, ride end) to avoid distracting riders during efforts.

**Implementation Notes**:
- `NotificationQueue` collects pending achievements during ride
- Check `WorkoutState` for rest intervals or paused state
- Display queued notifications on ride end screen
- Each notification includes badge icon, name, XP earned

---

## 3. 4D Power Profiling

### Decision: Extend Existing PDC Module

**Rationale**: `metrics/analytics/pdc.rs` already implements:
- `PowerDurationCurve` with interpolation
- `MmpCalculator` with standard durations (1s to 5h)
- `PdcBatchProcessor` for processing ride history
- Gap interpolation for sensor dropouts

**Alternatives Considered**:
- Use Critical Power model only: Loses short-duration insights
- External analytics service: Violates offline-capable constraint

**Implementation Notes**:
- Key durations for 4D profile: 5s, 15s, 30s, 1min, 3min, 5min, 10min, 20min, 60min
- Rolling 90-day window for "current" fitness (per clarification)
- Separate lifetime bests storage with `achieved_at` timestamp
- W/kg calculation requires user weight from `UserProfile`

### Decision: Use Reference Curves for Comparison

**Rationale**: Compare user's power curve shape against established rider phenotypes to identify strengths (sprinter, all-rounder, time trialist, climber).

**Implementation Notes**:
- Normalize power values to W/kg for comparison
- Calculate ratio: `(actual / expected)` at each duration
- Identify peaks (strengths) and valleys (weaknesses)
- Reference: Coggan power profile chart data

---

## 4. Multi-Discipline Training Plans

### Decision: Add Discipline Metadata to Existing Plan Structure

**Rationale**: `workouts/` module already handles workout parsing (.zwo, .mrc), workout library, and structured execution. Training plans need discipline tagging, not a new system.

**Alternatives Considered**:
- Separate plan engine: Would duplicate workout scheduling logic
- Dynamic plan generation: Complex ML, out of scope (see spec 004)

**Implementation Notes**:
- Disciplines: Road, Gravel, Triathlon, MTB, General Fitness
- Characteristics per discipline:
  - Road: VO2max intervals, threshold work
  - Gravel: Muscular endurance, tempo
  - Triathlon: Brick suggestions, aerobic base
  - MTB: Short bursts, recovery skills
- Pre-built plan templates as bundled workout files
- User schedule: available days/week, max hours/day

---

## 5. Career Levels & Progression

### Decision: Exponential XP Curve with 50 Levels

**Rationale**: Standard RPG progression keeps early levels achievable while making higher levels aspirational. 50 levels provides ~2 years of progression for regular users.

**Alternatives Considered**:
- Linear XP: Too easy at high levels
- 100+ levels (like Rouvy): Dilutes milestone feeling
- Prestige system: Over-complication for MVP

**Implementation Notes**:
- Formula: `XP_needed(level) = 1000 * (1.15 ^ (level - 1))`
- Level 10: ~4,000 XP (achievable in ~2 weeks)
- Level 25: ~32,000 XP (achievable in ~3 months)
- Level 50: ~540,000 XP (achievable in ~2 years)

### Decision: Cosmetic Unlocks at Milestone Levels

**Rationale**: Non-functional rewards maintain fairness while providing motivation. Integrates with existing avatar/theme systems.

**Implementation Notes**:
- Level 5: UI accent color options
- Level 10: Additional avatar jersey colors
- Level 15: New bike frame styles
- Level 20: Gradient background themes
- Level 25+: Premium cosmetic bundles
- Store unlock state in user_rewards table

---

## Technical Dependencies

| Feature | Depends On | Status |
|---------|------------|--------|
| Gradient Resistance | FTMS control, GPX parser | Existing |
| Achievements/XP | Achievement definitions, Database | Existing (extend) |
| Power Profile | PDC module, Ride samples | Existing (extend) |
| Training Plans | Workout library, Calendar | Existing (extend) |
| Career Levels | XP system, Avatar config | New (builds on P2) |

---

## Database Schema Changes

New tables required:
1. `user_achievements` - Earned achievements with timestamps
2. `user_xp` - Current XP and level
3. `power_profile_history` - PDC snapshots with dates
4. `training_plan_assignments` - Active plan with progress
5. `user_rewards` - Unlocked cosmetics

All changes will be added as database migration in `schema.rs`.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Trainer compatibility issues with simulation mode | Medium | Medium | Fallback to SetTargetInclination, document supported trainers |
| XP balance issues | Low | Low | Tunable values, post-launch adjustments |
| Power profile calculation performance | Low | Medium | Existing MmpCalculator is optimized with prefix sums |
| Plan content creation effort | Medium | Medium | Start with 4 disciplines × 2 plans each (8 total) |
