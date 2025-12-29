# Implementation Plan: Competitive Feature Gaps

**Branch**: `010-competitive-features` | **Date**: 2025-12-28 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/010-competitive-features/spec.md`

## Summary

Implement five competitive features to close gaps with Zwift, TrainerRoad, Wahoo SYSTM, and Rouvy:
1. **Gradient-Responsive Resistance (P1)**: Use existing GPX parser and FTMS simulation commands to auto-adjust trainer resistance based on route elevation
2. **Achievement Badges & XP System (P2)**: Extend existing achievement definitions with XP values and level progression tracking
3. **4D Power Profiling (P3)**: Leverage existing PDC (Power Duration Curve) module for multi-duration power analysis with 90-day rolling window
4. **Multi-Discipline Training Plans (P4)**: Add discipline metadata to existing workout/plan infrastructure
5. **Career Levels & Progression (P5)**: Build on XP system to unlock cosmetic rewards

## Technical Context

**Language/Version**: Rust 1.75+ (stable)
**Primary Dependencies**:
- eframe/egui 0.33 (GUI)
- btleplug 0.11 (BLE/FTMS)
- gpx 0.9 (GPX parsing)
- rusqlite 0.31 (database)
- tokio 1.x (async runtime)
- serde/chrono (serialization/time)

**Storage**: SQLite via rusqlite (existing `Database` struct with migrations)
**Testing**: cargo test (existing test infrastructure with unit/integration tests)
**Target Platform**: Windows, macOS, Linux (desktop)
**Project Type**: Single desktop application with library crate

**Performance Goals**:
- Power profile calculations < 5 seconds
- Achievement notifications < 2 seconds
- Gradient updates at 1Hz minimum
- Route load < 60 seconds

**Constraints**:
- Local-only (no server infrastructure)
- FTMS trainer protocol (existing)
- Offline-capable
- Existing UI patterns (egui)

**Scale/Scope**:
- ~50 achievement definitions (expandable)
- ~50 career levels
- 4+ disciplines for training plans
- 9 power duration buckets (5s to 60min)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

The constitution file contains placeholder template content. Proceeding with standard best practices:

| Gate | Status | Notes |
|------|--------|-------|
| Simplicity | PASS | Features extend existing infrastructure (achievements, PDC, GPX) |
| Test-First | PASS | Will follow existing test patterns in codebase |
| Library-First | PASS | Core logic in lib crate, UI in screens |
| Integration Testing | PASS | Add contract tests for new database schemas |

## Project Structure

### Documentation (this feature)

```text
specs/010-competitive-features/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/
├── achievements/        # NEW: Achievement & XP system
│   ├── mod.rs
│   ├── tracker.rs       # Achievement progress tracking
│   ├── xp.rs           # XP calculation and level progression
│   ├── notifications.rs # Queued notification system
│   └── definitions.rs   # Achievement definitions with XP values
├── gradient/           # NEW: Gradient simulation
│   ├── mod.rs
│   ├── calculator.rs   # Grade calculation from GPX elevation
│   ├── resistance.rs   # Physics model for resistance
│   └── controller.rs   # FTMS command generation
├── power_profile/      # NEW: 4D Power profiling (extends metrics/analytics/pdc.rs)
│   ├── mod.rs
│   ├── profile.rs      # Multi-duration profile with rolling window
│   ├── comparison.rs   # Strength/weakness analysis
│   └── history.rs      # 90-day rolling + lifetime tracking
├── training_plans/     # NEW: Multi-discipline plans (extends workouts/)
│   ├── mod.rs
│   ├── disciplines.rs  # Discipline definitions
│   ├── plan_builder.rs # Plan generation with scheduling
│   └── customization.rs # Plan modification support
├── career/             # NEW: Career levels (depends on achievements)
│   ├── mod.rs
│   ├── levels.rs       # Level progression system
│   └── rewards.rs      # Unlock system for cosmetics
├── storage/
│   ├── achievements_store.rs  # NEW: Achievement persistence
│   └── schema.rs              # EXTEND: Add achievement/XP tables
├── ui/
│   ├── screens/
│   │   ├── achievements.rs    # NEW: Achievement gallery
│   │   ├── power_profile.rs   # NEW: Power curve visualization
│   │   └── training_plans.rs  # EXTEND: Add discipline filter
│   └── widgets/
│       └── achievement_notification.rs  # NEW: Queued notification widget

tests/
├── unit/
│   ├── gradient_tests.rs
│   ├── achievements_tests.rs
│   ├── power_profile_tests.rs
│   └── career_tests.rs
└── integration/
    └── gradient_ride_tests.rs
```

**Structure Decision**: Extend existing single-project structure with new feature modules. Reuse existing infrastructure:
- `world/import/gpx.rs` for GPX parsing
- `world/achievements/` for achievement infrastructure (extend with XP)
- `metrics/analytics/pdc.rs` for power duration curves
- `sensors/ftms.rs` for trainer control commands
- `storage/` for database operations

## Complexity Tracking

No violations to justify - features extend existing patterns without adding new architectural complexity.
