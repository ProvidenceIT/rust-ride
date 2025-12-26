# Research: 3D World & Content

**Feature Branch**: `005-3d-world-content`
**Date**: 2025-12-25

## Technology Decisions

### 1. GPX/FIT/TCX File Parsing

**Decision**: Use `gpx` crate for GPX, `fitparser` crate for FIT, and extend existing `quick-xml` for TCX

**Rationale**:
- `gpx` crate is the most mature Rust GPX parser with 200+ GitHub stars, serde support, and handles all GPX versions
- `fitparser` is a pure Rust FIT file parser that handles Garmin's binary format correctly
- `quick-xml` is already in the project for ZWO parsing; TCX is XML-based and uses the same approach
- All three crates support async and can be integrated with existing tokio runtime

**Alternatives Considered**:
- `geo` crate: Too heavy for just parsing; focused on GIS operations
- Manual XML parsing: Error-prone and time-consuming
- `gpx-rust`: Less maintained than `gpx`

### 2. Elevation API Service

**Decision**: Use Open-Elevation API (open-elevation.com) as primary, with fallback to OpenTopoData

**Rationale**:
- Open-Elevation is free, self-hostable, and has no API key requirements
- Batch requests up to 200 points per call for efficiency
- OpenTopoData provides SRTM data as backup
- Both support simple REST JSON APIs compatible with existing reqwest usage

**Alternatives Considered**:
- Google Elevation API: Requires API key, costs money at scale
- Mapbox Terrain API: Requires authentication, rate limited
- Local DEM files: Large storage requirements, complex setup

### 3. Procedural Terrain Generation

**Decision**: Use `noise` crate with Perlin/Simplex noise for terrain heightmaps

**Rationale**:
- `noise` crate is the de-facto standard for Rust noise generation
- Supports multiple noise types (Perlin, Simplex, Worley, etc.)
- Deterministic seeding for reproducible worlds
- GPU-friendly output (can generate heightmap textures)

**Alternatives Considered**:
- `simdnoise`: Faster but less feature-rich
- Custom implementation: Unnecessary given mature crate ecosystem
- Pre-generated heightmaps: Limited variety

### 4. Particle System for Weather Effects

**Decision**: Implement GPU-instanced particle system in existing wgpu renderer

**Rationale**:
- wgpu already in use; can add particle compute/render pipeline
- GPU instancing handles 10,000+ particles efficiently for rain/snow
- Integrates with existing camera and scene management
- No additional dependency needed

**Alternatives Considered**:
- `bevy_hanabi`: Requires Bevy ECS, incompatible with egui architecture
- CPU particles: Too slow for dense rain/snow effects
- Sprite-based effects: Less realistic for weather

### 5. Time-of-Day Lighting

**Decision**: Procedural sky shader with HDR skybox + sun position calculation

**Rationale**:
- Procedural sky avoids large texture assets
- Sun position drives directional light and shadows
- HDR output supports bloom and atmospheric effects
- Can smoothly interpolate between any two times

**Alternatives Considered**:
- Pre-baked skybox textures: 6 textures per time = large assets, no smooth transitions
- Physical sky simulation: Overkill for cycling app
- Solid color gradients: Too simplistic

### 6. NPC AI Pathfinding

**Decision**: Simple waypoint following with speed variation based on gradient and FTP

**Rationale**:
- Routes are linear paths; no complex navigation needed
- Speed = f(power, gradient, mass) using same physics as player
- NPC power varies around target (user FTP * difficulty multiplier)
- No collision avoidance needed beyond basic spacing

**Alternatives Considered**:
- A* pathfinding: Overkill for linear routes
- Behavior trees: Unnecessary complexity
- Machine learning: Over-engineered for this use case

### 7. Leaderboard Storage

**Decision**: SQLite for local storage with optional cloud sync via REST API

**Rationale**:
- SQLite already used for rides, workouts, users
- Local-first means offline functionality preserved
- Cloud sync can be added later without breaking local experience
- Indexed queries for fast leaderboard retrieval

**Alternatives Considered**:
- Dedicated leaderboard service: Adds infrastructure complexity
- File-based storage: Poor query performance
- In-memory only: Loses data on restart

### 8. Map Projection for GPS to 3D Coordinates

**Decision**: Web Mercator (EPSG:3857) for regional routes, with local tangent plane for accuracy

**Rationale**:
- Web Mercator is standard for mapping and cycling apps
- Routes are relatively local; distortion is acceptable
- Simple math: lon/lat to meters using scale factor
- Elevation data uses meters, direct integration

**Alternatives Considered**:
- UTM zones: More accurate but zone boundaries complicate routes
- Lambert Conformal: Better for specific regions but complex
- Raw lat/lon: Distance calculations become complicated

### 9. Famous Route Data Source

**Decision**: Bundle GPX data from OpenStreetMap extracts + cycling databases

**Rationale**:
- OpenStreetMap has cycling route data under ODbL license
- VeloViewer and similar sites provide public GPX for famous climbs
- Historical data (records, race info) from public cycling databases
- Bundle as embedded assets (~50KB per route)

**Alternatives Considered**:
- Strava Segments API: Requires authentication, rate limited
- Download on demand: Requires internet for offline users
- User-contributed only: No curated experience at launch

### 10. Achievement System Architecture

**Decision**: Event-driven system with achievement definitions in TOML/JSON config

**Rationale**:
- Achievements are rule-based triggers on ride events
- Config-driven allows easy addition without code changes
- Events: segment_complete, landmark_discovered, ride_finished, etc.
- Progress stored in SQLite achievements table

**Alternatives Considered**:
- Hardcoded achievements: Inflexible
- Database-stored rules: Overkill for static content
- Lua scripting: Adds complexity and security concerns

## Best Practices

### Rust 3D Graphics with wgpu

1. Use compute shaders for particle updates
2. Instance rendering for NPCs and particles
3. Frustum culling for terrain chunks
4. LOD (Level of Detail) for distant objects
5. Double-buffer render targets for smooth weather transitions

### GPS Data Processing

1. Simplify routes using Ramer-Douglas-Peucker algorithm
2. Smooth elevation data with moving average
3. Calculate gradient from elevation delta / distance delta
4. Handle GPS jitter with Kalman filtering
5. Normalize timestamps to elapsed seconds

### Performance Optimization

1. Chunk terrain into tiles loaded on demand
2. Pool NPC instances to avoid allocation
3. Use spatial hashing for proximity queries (drafting)
4. Batch database writes for segment times
5. Background thread for route import/generation

## Integration Patterns

### Route Import Flow

```
User selects file → File parser (GPX/FIT/TCX)
                 → Extract coordinates + elevation
                 → If elevation missing: async fetch from API
                 → Convert to world coordinates (Mercator)
                 → Generate terrain mesh from elevation profile
                 → Create Route struct with waypoints
                 → Save to database + generate preview
```

### Weather State Machine

```
States: Clear, Cloudy, Rain, Fog, Snow
Transitions: Gradual (30-60 second lerp)
Triggers: Automatic (time-based), Manual (user selection)
Effects: Particle density, sky color, visibility, ambient occlusion
```

### NPC Lifecycle

```
Spawn: At route start or random distance ahead
Update: Follow route at calculated speed, respond to gradient
Interact: Trigger drafting detection, pass/overtake events
Despawn: When too far behind player or route complete
```

## Dependencies to Add

```toml
# Cargo.toml additions
gpx = "0.9"
fitparser = "0.5"
noise = "0.8"
```

## Database Schema Extensions

New tables needed (added via migration):
- `routes` - Imported and generated routes
- `route_waypoints` - Route point data
- `segments` - Defined segments with start/end
- `segment_times` - User times on segments
- `landmarks` - Points of interest
- `landmark_discoveries` - User discovery tracking
- `achievements` - Achievement definitions
- `achievement_progress` - User progress per achievement
- `collectibles` - In-world collectible definitions
- `collectible_pickups` - User collection tracking

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Large GPX files (50MB+) | Stream parsing, progress indicator |
| Elevation API rate limits | Local caching, batch requests |
| GPU memory for many NPCs | Instance pooling, LOD |
| Cloud sync conflicts | Last-write-wins with timestamps |
| Procedural impassable terrain | Validation pass to ensure rideable path |
