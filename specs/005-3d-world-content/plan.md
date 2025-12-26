# Implementation Plan: 3D World & Content

**Branch**: `005-3d-world-content` | **Date**: 2025-12-25 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/005-3d-world-content/spec.md`

## Summary

This feature extends the existing 3D virtual world rendering system (from feature 002) to support GPS/GPX route import, dynamic weather and time-of-day, NPC cyclists, segment leaderboards, famous routes, landmarks, difficulty modifiers, drafting mechanics, procedural generation, world creation tools, and achievements. The approach builds on the existing `src/world/` module architecture using wgpu rendering, extending Route/Terrain/Scene structures.

## Technical Context

**Language/Version**: Rust 1.75+ (stable)
**Primary Dependencies**:
- eframe 0.33 + egui 0.33 (GUI)
- wgpu 0.20 (3D rendering)
- glam 0.27 (math)
- gpx (GPX parsing)
- fit-sdk or fitparser (FIT file parsing)
- quick-xml 0.31 (TCX parsing - already used)
- noise (procedural terrain generation)
- reqwest 0.11 (elevation API, already used)
- rusqlite 0.31 (database, already used)
- serde + serde_json (serialization, already used)

**Storage**: SQLite (existing database.rs) + file-based route/world storage
**Testing**: cargo test (unit + integration)
**Target Platform**: Windows/macOS/Linux desktop (existing)
**Project Type**: Single application with library
**Performance Goals**:
- 60 FPS rendering with 50+ NPCs
- <30s route import and terrain generation
- <2s leaderboard queries
**Constraints**:
- Stylized terrain (moderate hardware requirements)
- Local-first leaderboards with optional cloud sync
- Visual-only drafting (no trainer resistance changes)
**Scale/Scope**:
- Routes up to 500km, GPX files up to 50MB
- 20+ famous routes at launch
- 50+ achievements

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Note**: The constitution.md is a template without specific principles defined. Proceeding with standard Rust best practices:

| Gate | Status | Notes |
|------|--------|-------|
| Test-First Development | PASS | Will write tests before implementation |
| Single Responsibility | PASS | Modules separated by concern (import, weather, npc, leaderboard, etc.) |
| Error Handling | PASS | Using thiserror + anyhow (existing pattern) |
| Documentation | PASS | Doc comments on public APIs |
| Integration Testing | PASS | Integration tests for route import, rendering |

## Project Structure

### Documentation (this feature)

```text
specs/005-3d-world-content/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── world/                      # Existing 3D world module (extend)
│   ├── mod.rs                  # World3D controller (extend)
│   ├── avatar.rs               # Cyclist avatar (existing)
│   ├── camera.rs               # Camera controller (existing)
│   ├── hud.rs                  # HUD overlay (extend)
│   ├── physics.rs              # Physics engine (extend for drafting)
│   ├── renderer.rs             # GPU renderer (extend for weather/particles)
│   ├── route.rs                # Route definitions (extend)
│   ├── scene.rs                # Scene configuration (extend for weather/time)
│   ├── terrain.rs              # Terrain rendering (extend for stylized terrain)
│   ├── worlds/                 # Pre-built worlds (existing)
│   │   ├── mod.rs
│   │   ├── coastal.rs
│   │   ├── countryside.rs
│   │   └── mountains.rs
│   ├── import/                 # NEW: Route import subsystem
│   │   ├── mod.rs              # Import orchestrator
│   │   ├── gpx.rs              # GPX parser
│   │   ├── fit.rs              # FIT parser
│   │   ├── tcx.rs              # TCX parser
│   │   └── elevation.rs        # Elevation service client
│   ├── weather/                # NEW: Weather system
│   │   ├── mod.rs              # Weather controller
│   │   ├── particles.rs        # Rain/snow/fog particles
│   │   └── skybox.rs           # Time-of-day sky rendering
│   ├── npc/                    # NEW: NPC system
│   │   ├── mod.rs              # NPC manager
│   │   ├── ai.rs               # AI behavior/pathfinding
│   │   └── spawner.rs          # NPC spawn logic
│   ├── segments/               # NEW: Segment leaderboards
│   │   ├── mod.rs              # Segment manager
│   │   ├── timing.rs           # Segment timing
│   │   └── leaderboard.rs      # Leaderboard storage/queries
│   ├── landmarks/              # NEW: Landmarks/POI
│   │   ├── mod.rs              # Landmark manager
│   │   └── discovery.rs        # Discovery tracking
│   ├── procedural/             # NEW: Procedural generation
│   │   ├── mod.rs              # Generator controller
│   │   ├── noise.rs            # Noise-based terrain
│   │   └── biomes.rs           # Biome definitions
│   ├── creator/                # NEW: World creator tools
│   │   ├── mod.rs              # Editor state
│   │   ├── tools.rs            # Editing tools
│   │   └── serialization.rs    # Save/load custom worlds
│   └── achievements/           # NEW: Achievement system
│       ├── mod.rs              # Achievement manager
│       ├── definitions.rs      # Achievement criteria
│       └── collectibles.rs     # In-world collectibles
├── storage/
│   ├── database.rs             # Extend with routes, segments, leaderboards, achievements tables
│   └── schema.rs               # Add migration for new tables
├── ui/
│   └── screens/
│       ├── route_browser.rs    # NEW: Route selection screen
│       ├── route_import.rs     # NEW: Import wizard
│       ├── leaderboards.rs     # NEW: Leaderboard viewer
│       ├── world_creator.rs    # NEW: World editor screen
│       └── achievements.rs     # NEW: Achievement gallery

tests/
├── integration/
│   ├── route_import_test.rs    # GPX/FIT/TCX import tests
│   ├── weather_test.rs         # Weather system tests
│   ├── npc_test.rs             # NPC behavior tests
│   └── segment_test.rs         # Segment timing tests
└── fixtures/
    ├── routes/                 # Sample GPX/FIT/TCX files
    └── famous_routes/          # Pre-built famous route data
```

**Structure Decision**: Extending the existing single-project structure with new submodules under `src/world/` to keep related functionality organized. This follows the existing pattern of `src/world/worlds/` for pre-built worlds.

## Complexity Tracking

> No constitution violations detected - no complexity justification required.
