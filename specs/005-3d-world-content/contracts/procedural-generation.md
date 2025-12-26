# Contract: Procedural Generation Module

**Module**: `src/world/procedural/`
**Date**: 2025-12-25

## Purpose

Generate infinite, reproducible terrain and routes from seed values with selectable biomes and rideable paths guaranteed.

## Public API

### Types

```rust
/// Procedural world generator
pub struct WorldGenerator {
    config: WorldSeed,
    noise: NoiseGenerator,
}

/// Generated world output
pub struct GeneratedWorld {
    pub seed: WorldSeed,
    pub route: Route,
    pub terrain: TerrainData,
    pub biome_regions: Vec<BiomeRegion>,
}

/// Terrain heightmap and metadata
pub struct TerrainData {
    /// Heightmap values (elevation in meters)
    pub heightmap: Vec<f32>,
    /// Width of heightmap in cells
    pub width: u32,
    /// Height of heightmap in cells
    pub height: u32,
    /// Meters per cell
    pub cell_size: f32,
    /// World-space offset
    pub origin: Vec2,
}

/// Region with specific biome
pub struct BiomeRegion {
    pub biome: Biome,
    pub bounds: Rect,
    pub blend_radius: f32,
}
```

### Functions

```rust
impl WorldGenerator {
    /// Create generator with seed configuration
    pub fn new(seed: WorldSeed) -> Self;

    /// Generate complete world
    ///
    /// Creates terrain, route, and biome data.
    /// Guarantees route is rideable (no impassable terrain).
    pub fn generate(&self) -> Result<GeneratedWorld, GenerationError>;

    /// Generate just the route (for preview)
    pub fn generate_route_only(&self) -> Result<Route, GenerationError>;

    /// Generate terrain chunk at position
    ///
    /// For streaming/chunked loading.
    pub fn generate_chunk(&self, chunk_x: i32, chunk_z: i32) -> TerrainChunk;

    /// Get elevation at world position
    pub fn elevation_at(&self, x: f32, z: f32) -> f32;

    /// Get gradient at world position
    pub fn gradient_at(&self, x: f32, z: f32) -> Vec2;
}

/// Generation errors
pub enum GenerationError {
    InvalidSeed(String),
    GenerationFailed(String),
    RouteUnrideable(String),
}
```

### Noise Generator

```rust
impl NoiseGenerator {
    /// Create noise generator from seed
    pub fn new(seed: u64) -> Self;

    /// Get 2D noise value at position
    pub fn noise2d(&self, x: f64, y: f64) -> f64;

    /// Get 2D noise with octaves (fractal)
    pub fn fbm2d(&self, x: f64, y: f64, octaves: u32) -> f64;

    /// Get ridged noise (for mountains)
    pub fn ridged2d(&self, x: f64, y: f64, octaves: u32) -> f64;
}
```

### Biome Configuration

```rust
impl Biome {
    /// Get biome-specific noise parameters
    pub fn noise_params(&self) -> BiomeNoiseParams;

    /// Get biome color palette
    pub fn colors(&self) -> BiomeColors;

    /// Get terrain feature frequency
    pub fn feature_density(&self) -> f32;
}

pub struct BiomeNoiseParams {
    pub base_frequency: f64,
    pub amplitude: f64,
    pub octaves: u32,
    pub lacunarity: f64,
    pub persistence: f64,
    pub height_offset: f32,
}

pub struct BiomeColors {
    pub ground: Vec3,
    pub ground_variation: Vec3,
    pub foliage: Option<Vec3>,
    pub rock: Vec3,
}
```

## Generation Algorithm

```
1. Initialize RNG from seed
2. Generate base terrain heightmap using FBM noise
3. Apply biome-specific modifications
4. Generate route path using constrained spline
5. Carve road into terrain (flatten road surface)
6. Validate route is rideable (gradients within limits)
7. If not rideable, adjust terrain and retry (max 3 attempts)
8. Generate biome regions along route
9. Return complete world
```

## Biome Characteristics

| Biome | Elevation Range | Max Gradient | Features |
|-------|-----------------|--------------|----------|
| Countryside | 0-200m | 8% | Rolling hills, farms |
| Alpine | 500-3000m | 20% | Mountains, switchbacks |
| Desert | 0-500m | 5% | Flat, dunes |
| Coastal | 0-100m | 10% | Cliffs, beaches |
| Forest | 100-800m | 12% | Dense trees, shade |

## Route Generation

```
1. Start at origin
2. Generate control points using biased random walk
   - Bias toward forward progress
   - Constrain gradient changes
3. Fit spline through control points
4. Sample spline at regular intervals (10m)
5. Calculate gradients from elevation differences
6. Validate all gradients within biome limits
7. If invalid, adjust control points and retry
```

## Rideability Validation

```
For each segment of route:
  gradient = (elevation_change / distance) * 100
  if gradient > max_gradient_percent:
    mark as invalid

If any invalid:
  Option 1: Adjust terrain to reduce gradient
  Option 2: Add switchbacks to extend distance
  Option 3: Regenerate with different random path

Must pass 100% of segments to be rideable.
```

## Performance Requirements

- Full world generation: <30s for 50km route
- Chunk generation: <100ms per 256x256 chunk
- Elevation query: <0.01ms
- Memory: <100MB for full world data

## Dependencies

- `noise` crate for Perlin/Simplex noise
- `splines` crate for route splines (or custom implementation)
