# Implementation Plan: 3D Virtual World & Complete Feature Implementation

**Branch**: `002-3d-world-features` | **Date**: 2025-12-24 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-3d-world-features/spec.md`

## Summary

This feature completes all "coming soon" functionality in RustRide and adds a Zwift-like 3D virtual world for immersive indoor cycling. The implementation spans 8 user stories:

1. **P1 - Sensor Control**: Complete BLE discovery, connection, and disconnection
2. **P1 - Ride Persistence**: Save rides, crash recovery, auto-save
3. **P2 - Ride History**: Browse, view details, export, delete rides
4. **P2 - Workout Library**: File browser import, ERG mode execution
5. **P2 - Settings**: FTP, zones, units, themes with persistence
6. **P3 - 3D Virtual World**: Render 3D environment with avatar movement based on power
7. **P3 - World Selection**: Multiple worlds with routes and elevation
8. **P4 - Avatar Customization**: Jersey, bike style personalization

Technical approach: Extend existing Rust/egui architecture with a 3D rendering layer using wgpu (GPU-accelerated graphics), integrate with existing BLE sensor and data persistence systems.

## Technical Context

**Language/Version**: Rust stable (1.75+)
**Primary Dependencies**:
- Existing: egui/eframe (GUI), btleplug (BLE), rusqlite (database), tokio (async)
- New: wgpu (3D rendering), glam (3D math), gltf (model loading), rfd (native file dialogs)

**Storage**: SQLite via rusqlite (rides, workouts, settings, avatar config)
**Testing**: cargo test (unit), integration tests for sensor/storage flows
**Target Platform**: Windows, macOS, Linux desktop (OpenGL 3.3+ / Vulkan / Metal capable)
**Project Type**: Single desktop application with GUI
**Performance Goals**:
- 3D rendering: 30+ FPS minimum, 60 FPS target
- Sensor latency: <100ms from pedal to screen
- ERG mode: <2 seconds power target application

**Constraints**:
- Offline-capable (no network required)
- <500MB disk for 3D assets
- <1GB RAM during 3D rendering
- Cross-platform graphics (wgpu abstracts Vulkan/Metal/DX12/OpenGL)

**Scale/Scope**:
- 3 virtual worlds minimum
- 5-10 routes per world
- Avatar with 10+ jersey color options, 3+ bike styles

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

The project constitution is a template and not yet ratified. Applying standard Rust best practices:

| Principle | Status | Notes |
|-----------|--------|-------|
| Modular design | PASS | New 3D module isolated from existing code |
| Test coverage | PASS | Unit tests for new modules, integration tests for sensor/storage |
| Error handling | PASS | Using thiserror/anyhow pattern consistently |
| Cross-platform | PASS | wgpu abstracts graphics APIs |
| Performance | NEEDS VERIFICATION | Must validate 30 FPS on minimum spec hardware |

**Gate Status**: PASS - No blocking violations

## Project Structure

### Documentation (this feature)

```text
specs/002-3d-world-features/
├── plan.md              # This file
├── research.md          # Phase 0 output - 3D tech research
├── data-model.md        # Phase 1 output - Entity definitions
├── quickstart.md        # Phase 1 output - Developer setup
├── contracts/           # Phase 1 output - Module interfaces
│   ├── sensor-control.md
│   ├── ride-persistence.md
│   ├── workout-library.md
│   ├── settings.md
│   └── world-3d.md
└── tasks.md             # Phase 2 output (via /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── main.rs              # Application entry point
├── app.rs               # egui application state (extend for 3D mode)
├── lib.rs               # Library exports
├── sensors/             # BLE sensor management (complete TODOs)
│   ├── mod.rs
│   ├── manager.rs       # Add actual BLE scan/connect/disconnect
│   ├── ftms.rs
│   └── types.rs
├── workouts/            # Workout parsing and execution
│   ├── mod.rs
│   ├── engine.rs        # ERG mode power targeting
│   ├── parser_zwo.rs
│   └── parser_mrc.rs
├── recording/           # Ride recording and export
│   ├── mod.rs
│   ├── recorder.rs      # Complete save, recovery TODOs
│   ├── exporter_tcx.rs  # Add max speed calculation
│   └── exporter_csv.rs
├── metrics/             # Training metric calculations
├── storage/             # SQLite and config persistence
│   ├── mod.rs
│   ├── database.rs      # Add autosave table operations
│   └── config.rs
├── ui/                  # egui screens and widgets
│   ├── mod.rs
│   ├── screens/
│   │   ├── mod.rs
│   │   ├── home.rs          # Add sensor status display
│   │   ├── sensor_setup.rs  # Connect actual BLE operations
│   │   ├── ride.rs          # Integrate 3D view, save ride
│   │   ├── ride_summary.rs
│   │   ├── ride_history.rs
│   │   ├── ride_detail.rs   # Complete power distribution
│   │   ├── workout_library.rs # Add file picker
│   │   ├── settings.rs
│   │   └── world_select.rs  # NEW: World/route selection
│   ├── widgets/
│   └── theme.rs             # Implement light theme
└── world/               # NEW: 3D virtual world module
    ├── mod.rs
    ├── renderer.rs      # wgpu rendering pipeline
    ├── scene.rs         # Scene graph management
    ├── camera.rs        # Third-person camera following avatar
    ├── avatar.rs        # Avatar model and animation
    ├── terrain.rs       # Road and landscape rendering
    ├── physics.rs       # Speed/position calculation from power
    ├── route.rs         # Route path and elevation
    ├── worlds/          # World definitions
    │   ├── mod.rs
    │   ├── countryside.rs
    │   ├── mountains.rs
    │   └── coastal.rs
    └── assets/          # 3D models, textures (referenced, not stored in src)

assets/                  # NEW: 3D asset files
├── models/
│   ├── cyclist.gltf
│   ├── bikes/
│   └── environment/
├── textures/
│   ├── terrain/
│   ├── sky/
│   └── objects/
└── worlds/
    ├── countryside/
    ├── mountains/
    └── coastal/

tests/
├── unit/
├── integration/
│   ├── sensor_integration.rs  # NEW: BLE integration tests
│   ├── storage_integration.rs # NEW: Ride persistence tests
│   └── world_integration.rs   # NEW: 3D world loading tests
└── fixtures/
```

**Structure Decision**: Single project architecture maintained. The new `world/` module encapsulates all 3D functionality. Assets stored in `assets/` directory at repository root, bundled with release builds.

## Complexity Tracking

No constitution violations requiring justification. The 3D world module adds complexity but is well-isolated and necessary for the core feature request.

## Phase 0 Research Topics

1. **3D Rendering in Rust/egui**: wgpu integration with egui, render pipeline setup
2. **Asset Pipeline**: GLTF loading, texture formats, asset bundling for distribution
3. **Physics Model**: Power-to-speed conversion, virtual gradient effects
4. **Native File Dialogs**: rfd crate for cross-platform file picker
5. **BLE Integration**: Completing btleplug workflow for scan/connect/disconnect

## Phase 1 Deliverables

1. `research.md` - Technology decisions and rationale
2. `data-model.md` - Entity definitions for worlds, routes, avatars
3. `contracts/` - Module interface definitions
4. `quickstart.md` - Developer setup for 3D development
