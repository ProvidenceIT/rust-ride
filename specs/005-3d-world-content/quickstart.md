# Quickstart: 3D World & Content

**Feature Branch**: `005-3d-world-content`
**Date**: 2025-12-25

## Prerequisites

- Rust 1.75+ installed
- Git repository cloned and on branch `005-3d-world-content`
- SQLite development libraries (included via `rusqlite` bundled feature)

## Setup

### 1. Add New Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
# GPS file parsing
gpx = "0.9"
fitparser = "0.5"

# Procedural generation
noise = "0.8"
```

### 2. Run Database Migrations

Migrations run automatically on app startup. The new tables are:
- `routes`, `route_waypoints`
- `segments`, `segment_times`
- `landmarks`, `landmark_discoveries`
- `achievements`, `achievement_progress`
- `collectibles`, `collectible_pickups`

### 3. Build and Test

```bash
# Build the project
cargo build

# Run all tests
cargo test

# Run specific module tests
cargo test world::import
cargo test world::weather
cargo test world::npc
cargo test world::segments
```

## Development Tasks

### Task 1: Route Import (P1)

**Files to create/modify**:
- `src/world/import/mod.rs` - Import orchestrator
- `src/world/import/gpx.rs` - GPX parser
- `src/world/import/fit.rs` - FIT parser
- `src/world/import/tcx.rs` - TCX parser
- `src/world/import/elevation.rs` - Elevation API client

**Test with**:
```bash
cargo test world::import

# Manual test with sample GPX
cargo run -- --import-route test_fixtures/sample.gpx
```

### Task 2: Weather System (P2)

**Files to create/modify**:
- `src/world/weather/mod.rs` - Weather controller
- `src/world/weather/particles.rs` - GPU particle system
- `src/world/weather/skybox.rs` - Procedural sky

**Test with**:
```bash
cargo test world::weather

# Visual test - run app and toggle weather
cargo run
# Press W to cycle weather, T to change time
```

### Task 3: NPC System (P3)

**Files to create/modify**:
- `src/world/npc/mod.rs` - NPC manager
- `src/world/npc/ai.rs` - NPC behavior
- `src/world/npc/spawner.rs` - Spawn logic

**Test with**:
```bash
cargo test world::npc

# Visual test - start ride with NPCs enabled
cargo run
# Enable NPCs in settings before starting ride
```

### Task 4: Segment Leaderboards (P4)

**Files to create/modify**:
- `src/world/segments/mod.rs` - Segment manager
- `src/world/segments/timing.rs` - Timing logic
- `src/world/segments/leaderboard.rs` - Leaderboard queries
- `src/storage/database.rs` - Add segment CRUD

**Test with**:
```bash
cargo test world::segments
cargo test storage::database::segment

# Integration test - complete a segment
cargo run
# Ride through a route with defined segments
```

## Module Architecture

```
src/world/
├── mod.rs              # Re-export all submodules
├── import/
│   ├── mod.rs          # pub use gpx::*, fit::*, tcx::*, elevation::*
│   ├── gpx.rs          # GPX parsing
│   ├── fit.rs          # FIT parsing
│   ├── tcx.rs          # TCX parsing
│   └── elevation.rs    # Elevation API
├── weather/
│   ├── mod.rs          # WeatherController
│   ├── particles.rs    # ParticleSystem
│   └── skybox.rs       # Skybox
├── npc/
│   ├── mod.rs          # NpcManager
│   ├── ai.rs           # NPC AI behavior
│   └── spawner.rs      # NpcSpawner
├── segments/
│   ├── mod.rs          # SegmentManager
│   ├── timing.rs       # ActiveSegmentState
│   └── leaderboard.rs  # LeaderboardEntry queries
├── landmarks/
│   ├── mod.rs          # LandmarkManager
│   └── discovery.rs    # Discovery tracking
├── procedural/
│   ├── mod.rs          # WorldGenerator
│   ├── noise.rs        # NoiseGenerator
│   └── biomes.rs       # Biome definitions
├── creator/
│   ├── mod.rs          # WorldCreator
│   ├── tools.rs        # Editor tools
│   └── serialization.rs # Save/load
└── achievements/
    ├── mod.rs          # AchievementManager
    ├── definitions.rs  # Achievement criteria
    └── collectibles.rs # CollectibleManager
```

## Key Patterns

### Error Handling

Use `thiserror` for module-specific errors:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Parse error: {0}")]
    ParseError(String),
}
```

### Async Operations

Use `tokio` for async operations (file I/O, network):

```rust
pub async fn import_route(path: &Path) -> Result<ImportResult, ImportError> {
    let content = tokio::fs::read(path).await?;
    // ...
}
```

### Database Access

Follow existing pattern in `database.rs`:

```rust
impl Database {
    pub fn insert_route(&self, route: &ImportedRoute) -> Result<(), DatabaseError> {
        self.conn.execute(
            "INSERT INTO routes (...) VALUES (...)",
            params![...],
        )?;
        Ok(())
    }
}
```

### GPU Rendering

Follow existing wgpu patterns in `renderer.rs`:

```rust
impl ParticleSystem {
    pub fn new(device: &wgpu::Device, max_particles: u32) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            // ...
        });
        // ...
    }
}
```

## Test Data

### Sample GPX File

Create `tests/fixtures/routes/sample.gpx`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1">
  <trk>
    <name>Test Route</name>
    <trkseg>
      <trkpt lat="45.5" lon="-122.5">
        <ele>100</ele>
        <time>2024-01-01T00:00:00Z</time>
      </trkpt>
      <!-- More points... -->
    </trkseg>
  </trk>
</gpx>
```

### Test Database

Tests use in-memory SQLite:

```rust
#[test]
fn test_route_import() {
    let db = Database::open_in_memory().unwrap();
    // ...
}
```

## Debugging

### Enable Logging

```bash
RUST_LOG=debug cargo run
RUST_LOG=rustride::world::import=trace cargo run
```

### GPU Debugging

Enable wgpu validation:

```rust
let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
    backends: wgpu::Backends::all(),
    dx12_shader_compiler: Default::default(),
    flags: wgpu::InstanceFlags::validation(),
    gles_minor_version: Default::default(),
});
```

## Common Issues

### GPX Parse Fails
- Check file encoding is UTF-8
- Validate XML structure
- Check for required elements (trk, trkseg, trkpt)

### Elevation API Errors
- Check internet connectivity
- API may rate-limit; implement retry with backoff
- Fallback to zero elevation if all else fails

### NPC Rendering Slow
- Reduce NPC count in settings
- Check instance buffer size
- Profile with `tracy` or GPU profiler

### Segment Timing Drift
- Use monotonic clock (Instant) not wall clock
- Don't pause timing during frame drops
