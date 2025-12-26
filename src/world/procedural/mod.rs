//! Procedural world generation system.
//!
//! T114: WorldSeed and Biome structs
//! T117: WorldGenerator with seed-based terrain
//! T118: Route generation with rideable path
//! T119: Rideability validation

pub mod biomes;
pub mod noise;

use biomes::BiomeType;
use glam::Vec3;
use noise::NoiseGenerator;
use serde::{Deserialize, Serialize};

/// Terrain generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainParams {
    /// Base terrain height offset
    pub base_height: f32,
    /// Height multiplier for terrain features
    pub height_scale: f32,
    /// Noise frequency (higher = more detailed terrain)
    pub frequency: f64,
    /// Number of noise octaves
    pub octaves: u8,
    /// Persistence for octave blending
    pub persistence: f64,
    /// Lacunarity (frequency multiplier per octave)
    pub lacunarity: f64,
    /// World seed
    pub seed: u32,
}

impl Default for TerrainParams {
    fn default() -> Self {
        Self {
            base_height: 0.0,
            height_scale: 100.0,
            frequency: 0.001,
            octaves: 4,
            persistence: 0.5,
            lacunarity: 2.0,
            seed: 42,
        }
    }
}

/// Road surface type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceType {
    /// Smooth asphalt
    Asphalt,
    /// Rougher chip seal
    ChipSeal,
    /// Gravel/unpaved
    Gravel,
    /// Cobblestones
    Cobbles,
    /// Dirt path
    Dirt,
}

impl SurfaceType {
    /// Rolling resistance factor (1.0 = normal)
    pub fn rolling_resistance(&self) -> f32 {
        match self {
            Self::Asphalt => 1.0,
            Self::ChipSeal => 1.05,
            Self::Gravel => 1.3,
            Self::Cobbles => 1.4,
            Self::Dirt => 1.25,
        }
    }

    /// Vibration intensity for haptic feedback
    pub fn vibration_intensity(&self) -> f32 {
        match self {
            Self::Asphalt => 0.0,
            Self::ChipSeal => 0.1,
            Self::Gravel => 0.4,
            Self::Cobbles => 0.7,
            Self::Dirt => 0.3,
        }
    }
}

/// Generated terrain chunk
#[derive(Debug, Clone)]
pub struct TerrainChunk {
    /// Chunk position (world coordinates)
    pub position: Vec3,
    /// Chunk size in meters
    pub size: f32,
    /// Height samples (resolution x resolution grid)
    pub heights: Vec<f32>,
    /// Resolution (samples per side)
    pub resolution: u32,
    /// Surface type for this chunk
    pub surface: SurfaceType,
    /// LOD level (0 = highest detail)
    pub lod: u8,
}

impl TerrainChunk {
    /// Create empty chunk
    pub fn new(position: Vec3, size: f32, resolution: u32) -> Self {
        let sample_count = (resolution * resolution) as usize;
        Self {
            position,
            size,
            heights: vec![0.0; sample_count],
            resolution,
            surface: SurfaceType::Asphalt,
            lod: 0,
        }
    }

    /// Get height at local coordinates (0..1, 0..1)
    pub fn sample_height(&self, x: f32, z: f32) -> f32 {
        let x_idx = (x * (self.resolution - 1) as f32) as u32;
        let z_idx = (z * (self.resolution - 1) as f32) as u32;
        let idx = (z_idx * self.resolution + x_idx) as usize;

        if idx < self.heights.len() {
            self.heights[idx]
        } else {
            0.0
        }
    }

    /// Set height at grid position
    pub fn set_height(&mut self, x: u32, z: u32, height: f32) {
        let idx = (z * self.resolution + x) as usize;
        if idx < self.heights.len() {
            self.heights[idx] = height;
        }
    }
}

/// Procedural terrain generator
pub struct TerrainGenerator {
    /// Generation parameters
    params: TerrainParams,
    /// Chunk size in world units
    chunk_size: f32,
    /// Chunk resolution (samples per side)
    chunk_resolution: u32,
}

impl TerrainGenerator {
    /// Create generator with parameters
    pub fn new(params: TerrainParams) -> Self {
        Self {
            params,
            chunk_size: 256.0,
            chunk_resolution: 65, // 65x65 for smooth LOD transitions
        }
    }

    /// Generate chunk at world position
    pub fn generate_chunk(&self, chunk_x: i32, chunk_z: i32) -> TerrainChunk {
        let position = Vec3::new(
            chunk_x as f32 * self.chunk_size,
            0.0,
            chunk_z as f32 * self.chunk_size,
        );

        let mut chunk = TerrainChunk::new(position, self.chunk_size, self.chunk_resolution);

        // Generate heights using noise
        for z in 0..self.chunk_resolution {
            for x in 0..self.chunk_resolution {
                let world_x =
                    position.x + (x as f32 / (self.chunk_resolution - 1) as f32) * self.chunk_size;
                let world_z =
                    position.z + (z as f32 / (self.chunk_resolution - 1) as f32) * self.chunk_size;

                let height = self.sample_height(world_x as f64, world_z as f64);
                chunk.set_height(x, z, height);
            }
        }

        chunk
    }

    /// Sample height at world position
    pub fn sample_height(&self, x: f64, z: f64) -> f32 {
        let mut total = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = self.params.frequency;
        let mut max_value = 0.0;

        for _ in 0..self.params.octaves {
            // Simple noise approximation (would use noise crate in full impl)
            let nx = x * frequency + self.params.seed as f64;
            let nz = z * frequency + self.params.seed as f64;

            // Simplified perlin-like noise
            let value = ((nx.sin() + nz.cos()) / 2.0) as f32;
            total += value * amplitude;

            max_value += amplitude;
            amplitude *= self.params.persistence as f32;
            frequency *= self.params.lacunarity;
        }

        // Normalize and scale
        let normalized = total / max_value;
        self.params.base_height + normalized * self.params.height_scale
    }

    /// Get chunk coordinates for a world position
    pub fn world_to_chunk(&self, x: f32, z: f32) -> (i32, i32) {
        (
            (x / self.chunk_size).floor() as i32,
            (z / self.chunk_size).floor() as i32,
        )
    }
}

/// Chunk manager handles loading/unloading chunks around viewer
pub struct ChunkManager {
    /// Active chunks
    chunks: std::collections::HashMap<(i32, i32), TerrainChunk>,
    /// Generator
    generator: TerrainGenerator,
    /// View distance (chunks)
    view_distance: u32,
    /// Center position for loading
    center: (i32, i32),
}

impl ChunkManager {
    /// Create chunk manager
    pub fn new(params: TerrainParams, view_distance: u32) -> Self {
        Self {
            chunks: std::collections::HashMap::new(),
            generator: TerrainGenerator::new(params),
            view_distance,
            center: (0, 0),
        }
    }

    /// Update center and load/unload chunks as needed
    pub fn update_center(&mut self, world_x: f32, world_z: f32) {
        let new_center = self.generator.world_to_chunk(world_x, world_z);

        if new_center != self.center {
            self.center = new_center;
            self.load_chunks();
        }
    }

    /// Load chunks around center
    fn load_chunks(&mut self) {
        let dist = self.view_distance as i32;
        let mut needed: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();

        // Determine needed chunks
        for dz in -dist..=dist {
            for dx in -dist..=dist {
                needed.insert((self.center.0 + dx, self.center.1 + dz));
            }
        }

        // Remove chunks outside view distance
        self.chunks.retain(|coord, _| needed.contains(coord));

        // Generate new chunks
        for coord in needed {
            if !self.chunks.contains_key(&coord) {
                let chunk = self.generator.generate_chunk(coord.0, coord.1);
                self.chunks.insert(coord, chunk);
            }
        }
    }

    /// Get chunk at coordinates
    pub fn get_chunk(&self, x: i32, z: i32) -> Option<&TerrainChunk> {
        self.chunks.get(&(x, z))
    }

    /// Get height at world position
    pub fn sample_height(&self, x: f32, z: f32) -> f32 {
        let (cx, cz) = self.generator.world_to_chunk(x, z);

        if let Some(chunk) = self.get_chunk(cx, cz) {
            // Convert world to chunk-local coordinates
            let local_x = (x - chunk.position.x) / chunk.size;
            let local_z = (z - chunk.position.z) / chunk.size;
            chunk.sample_height(local_x, local_z)
        } else {
            0.0
        }
    }

    /// Get all active chunks
    pub fn chunks(&self) -> impl Iterator<Item = &TerrainChunk> {
        self.chunks.values()
    }
}

// ========== T114: WorldSeed ==========

/// World seed for reproducible procedural generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSeed {
    /// Numeric seed value
    pub value: u32,
    /// Optional human-readable name
    pub name: Option<String>,
    /// Biome preference (None = natural distribution)
    pub biome_preference: Option<BiomeType>,
    /// Difficulty level (affects gradient steepness)
    pub difficulty: ProceduralDifficulty,
    /// Target route length in meters
    pub target_length: f64,
}

impl Default for WorldSeed {
    fn default() -> Self {
        Self {
            value: 42,
            name: None,
            biome_preference: None,
            difficulty: ProceduralDifficulty::Medium,
            target_length: 20_000.0,
        }
    }
}

impl WorldSeed {
    /// Create seed from numeric value
    pub fn from_value(value: u32) -> Self {
        Self {
            value,
            ..Default::default()
        }
    }

    /// Create seed from string (hashed)
    pub fn from_string(s: &str) -> Self {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        s.hash(&mut hasher);
        Self {
            value: hasher.finish() as u32,
            name: Some(s.to_string()),
            ..Default::default()
        }
    }

    /// Set biome preference
    pub fn with_biome(mut self, biome: BiomeType) -> Self {
        self.biome_preference = Some(biome);
        self
    }

    /// Set difficulty
    pub fn with_difficulty(mut self, difficulty: ProceduralDifficulty) -> Self {
        self.difficulty = difficulty;
        self
    }

    /// Set target length
    pub fn with_length(mut self, length: f64) -> Self {
        self.target_length = length;
        self
    }
}

/// Procedural world difficulty levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProceduralDifficulty {
    /// Mostly flat, easy gradients (max 3%)
    Easy,
    /// Gentle hills, moderate gradients (max 6%)
    #[default]
    Medium,
    /// Rolling terrain, challenging gradients (max 10%)
    Hard,
    /// Mountain terrain, steep gradients (max 15%)
    Extreme,
}

impl ProceduralDifficulty {
    /// Maximum gradient for this difficulty
    pub fn max_gradient(&self) -> f32 {
        match self {
            Self::Easy => 3.0,
            Self::Medium => 6.0,
            Self::Hard => 10.0,
            Self::Extreme => 15.0,
        }
    }

    /// Height scale multiplier
    pub fn height_scale(&self) -> f32 {
        match self {
            Self::Easy => 0.3,
            Self::Medium => 0.6,
            Self::Hard => 1.0,
            Self::Extreme => 1.5,
        }
    }

    /// Display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Medium => "Medium",
            Self::Hard => "Hard",
            Self::Extreme => "Extreme",
        }
    }
}

// ========== T117-T119: World Generator with Route Generation ==========

/// Generated procedural world
#[derive(Debug, Clone)]
pub struct ProceduralWorld {
    /// World seed
    pub seed: WorldSeed,
    /// Generated route waypoints
    pub waypoints: Vec<ProceduralWaypoint>,
    /// Total route length in meters
    pub total_length: f64,
    /// Maximum elevation in meters
    pub max_elevation: f32,
    /// Total elevation gain in meters
    pub elevation_gain: f32,
    /// Primary biome
    pub biome: BiomeType,
}

/// Waypoint in procedural route
#[derive(Debug, Clone)]
pub struct ProceduralWaypoint {
    /// Position in world coordinates
    pub position: Vec3,
    /// Distance from start in meters
    pub distance: f64,
    /// Gradient at this point (percentage)
    pub gradient: f32,
    /// Surface type
    pub surface: SurfaceType,
    /// Biome at this location
    pub biome: BiomeType,
}

/// World generator creates rideable procedural worlds
pub struct WorldGenerator {
    noise: NoiseGenerator,
    seed: WorldSeed,
}

impl WorldGenerator {
    /// Create generator with seed
    pub fn new(seed: WorldSeed) -> Self {
        Self {
            noise: NoiseGenerator::new(seed.value),
            seed,
        }
    }

    /// Generate a complete procedural world with rideable route
    pub fn generate(&self) -> ProceduralWorld {
        let waypoints = self.generate_route();
        let total_length = waypoints.last().map(|w| w.distance).unwrap_or(0.0);

        let max_elevation = waypoints
            .iter()
            .map(|w| w.position.y)
            .fold(0.0f32, |a, b| a.max(b));

        let elevation_gain = self.calculate_elevation_gain(&waypoints);

        let biome = self
            .seed
            .biome_preference
            .unwrap_or_else(|| self.determine_primary_biome(&waypoints));

        ProceduralWorld {
            seed: self.seed.clone(),
            waypoints,
            total_length,
            max_elevation,
            elevation_gain,
            biome,
        }
    }

    /// T118: Generate rideable route
    fn generate_route(&self) -> Vec<ProceduralWaypoint> {
        let mut waypoints = Vec::new();
        let target_length = self.seed.target_length;
        let max_gradient = self.seed.difficulty.max_gradient();
        let height_scale = self.seed.difficulty.height_scale();

        // Generate path using noise-based direction changes
        let mut current_pos = Vec3::new(0.0, 100.0, 0.0); // Start at ground level
        let mut current_distance = 0.0;
        let mut heading = 0.0f32; // radians

        // Step size for waypoint generation (10 meters)
        let step_size = 10.0f32;

        while current_distance < target_length {
            // Add current waypoint
            let gradient =
                self.sample_gradient(current_pos.x, current_pos.z, height_scale, max_gradient);
            let biome = self.sample_biome(current_pos.x, current_pos.z);

            waypoints.push(ProceduralWaypoint {
                position: current_pos,
                distance: current_distance,
                gradient,
                surface: self.surface_for_biome(biome),
                biome,
            });

            // Vary heading using noise for natural path
            let heading_noise = self
                .noise
                .perlin(current_pos.x as f64 * 0.01, current_pos.z as f64 * 0.01)
                as f32;
            heading += heading_noise * 0.2; // Gentle turns

            // Move forward
            let dx = heading.cos() * step_size;
            let dz = heading.sin() * step_size;

            // Calculate new elevation with gradient limit enforcement
            let raw_height =
                self.sample_terrain_height(current_pos.x + dx, current_pos.z + dz, height_scale);

            // T119: Enforce rideability - limit gradient
            let max_height_change = step_size * max_gradient / 100.0;
            let clamped_height = (raw_height - current_pos.y)
                .clamp(-max_height_change, max_height_change)
                + current_pos.y;

            current_pos = Vec3::new(current_pos.x + dx, clamped_height, current_pos.z + dz);
            current_distance += step_size as f64;
        }

        // Add final waypoint
        let gradient =
            self.sample_gradient(current_pos.x, current_pos.z, height_scale, max_gradient);
        let biome = self.sample_biome(current_pos.x, current_pos.z);
        waypoints.push(ProceduralWaypoint {
            position: current_pos,
            distance: current_distance,
            gradient,
            surface: self.surface_for_biome(biome),
            biome,
        });

        waypoints
    }

    /// Sample terrain height at position
    fn sample_terrain_height(&self, x: f32, z: f32, height_scale: f32) -> f32 {
        let base_height = 100.0; // Base elevation
        let noise_value = self.noise.terrain(x as f64 * 0.002, z as f64 * 0.002, 1.0);
        base_height + (noise_value as f32) * 50.0 * height_scale
    }

    /// Sample gradient at position
    fn sample_gradient(&self, x: f32, z: f32, height_scale: f32, max_gradient: f32) -> f32 {
        let h1 = self.sample_terrain_height(x, z, height_scale);
        let h2 = self.sample_terrain_height(x + 1.0, z, height_scale);
        let gradient = (h2 - h1) * 100.0; // Convert to percentage
        gradient.clamp(-max_gradient, max_gradient)
    }

    /// Sample biome at position
    fn sample_biome(&self, x: f32, z: f32) -> BiomeType {
        if let Some(pref) = self.seed.biome_preference {
            return pref;
        }

        // Use noise to determine biome
        let moisture = (self.noise.perlin(x as f64 * 0.001, z as f64 * 0.001) * 0.5 + 0.5) as f32;
        let temperature = (self
            .noise
            .perlin(x as f64 * 0.001 + 100.0, z as f64 * 0.001 + 100.0)
            * 0.5
            + 0.5) as f32;
        let elevation =
            (self.noise.terrain(x as f64 * 0.001, z as f64 * 0.001, 1.0) * 0.5 + 0.5) as f32;

        let params = biomes::BiomeParams {
            moisture,
            temperature,
            elevation,
        };
        params.to_biome()
    }

    /// Get surface type for biome
    fn surface_for_biome(&self, biome: BiomeType) -> SurfaceType {
        match biome {
            BiomeType::Urban => SurfaceType::Asphalt,
            BiomeType::Farmland => SurfaceType::ChipSeal,
            BiomeType::Mountain | BiomeType::Alpine => SurfaceType::Gravel,
            BiomeType::Forest => SurfaceType::Dirt,
            _ => SurfaceType::Asphalt,
        }
    }

    /// Calculate total elevation gain
    fn calculate_elevation_gain(&self, waypoints: &[ProceduralWaypoint]) -> f32 {
        let mut gain = 0.0f32;
        for i in 1..waypoints.len() {
            let diff = waypoints[i].position.y - waypoints[i - 1].position.y;
            if diff > 0.0 {
                gain += diff;
            }
        }
        gain
    }

    /// Determine primary biome from waypoints
    fn determine_primary_biome(&self, waypoints: &[ProceduralWaypoint]) -> BiomeType {
        let mut counts = std::collections::HashMap::new();
        for wp in waypoints {
            *counts.entry(wp.biome).or_insert(0) += 1;
        }
        counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(biome, _)| biome)
            .unwrap_or(BiomeType::Meadow)
    }

    /// T119: Validate that route is rideable
    pub fn validate_rideability(&self, world: &ProceduralWorld) -> RideabilityResult {
        let max_gradient = self.seed.difficulty.max_gradient();
        let mut issues = Vec::new();

        for (i, wp) in world.waypoints.iter().enumerate() {
            if wp.gradient.abs() > max_gradient {
                issues.push(format!(
                    "Gradient {:.1}% at distance {:.0}m exceeds max {:.1}%",
                    wp.gradient, wp.distance, max_gradient
                ));
            }

            // Check for sudden elevation changes (>10m in one step)
            if i > 0 {
                let prev = &world.waypoints[i - 1];
                let height_diff = (wp.position.y - prev.position.y).abs();
                if height_diff > 10.0 {
                    issues.push(format!(
                        "Sudden elevation change of {:.1}m at distance {:.0}m",
                        height_diff, wp.distance
                    ));
                }
            }
        }

        RideabilityResult {
            is_rideable: issues.is_empty(),
            issues,
        }
    }
}

/// Result of rideability validation
#[derive(Debug, Clone)]
pub struct RideabilityResult {
    /// Whether the route is rideable
    pub is_rideable: bool,
    /// List of issues found
    pub issues: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_chunk() {
        let mut chunk = TerrainChunk::new(Vec3::ZERO, 256.0, 5);
        chunk.set_height(2, 2, 100.0);

        let height = chunk.sample_height(0.5, 0.5);
        assert!((height - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_terrain_generator() {
        let generator = TerrainGenerator::new(TerrainParams::default());
        let chunk = generator.generate_chunk(0, 0);

        assert_eq!(chunk.resolution, 65);
        assert_eq!(chunk.heights.len(), 65 * 65);
    }

    #[test]
    fn test_chunk_manager() {
        let mut manager = ChunkManager::new(TerrainParams::default(), 2);
        // Move to a position that triggers chunk loading
        manager.update_center(300.0, 300.0);

        // Should have loaded chunks around center
        assert!(!manager.chunks.is_empty());
    }

    // ========== T114-T119: World Generation Tests ==========

    #[test]
    fn test_world_seed_default() {
        let seed = WorldSeed::default();
        assert_eq!(seed.value, 42);
        assert_eq!(seed.difficulty, ProceduralDifficulty::Medium);
        assert_eq!(seed.target_length, 20_000.0);
    }

    #[test]
    fn test_world_seed_from_string() {
        let seed1 = WorldSeed::from_string("hello");
        let seed2 = WorldSeed::from_string("hello");
        let seed3 = WorldSeed::from_string("world");

        // Same string should give same seed
        assert_eq!(seed1.value, seed2.value);
        // Different strings should give different seeds
        assert_ne!(seed1.value, seed3.value);
    }

    #[test]
    fn test_world_seed_builder() {
        let seed = WorldSeed::from_value(123)
            .with_biome(biomes::BiomeType::Mountain)
            .with_difficulty(ProceduralDifficulty::Hard)
            .with_length(50_000.0);

        assert_eq!(seed.value, 123);
        assert_eq!(seed.biome_preference, Some(biomes::BiomeType::Mountain));
        assert_eq!(seed.difficulty, ProceduralDifficulty::Hard);
        assert_eq!(seed.target_length, 50_000.0);
    }

    #[test]
    fn test_procedural_difficulty() {
        assert_eq!(ProceduralDifficulty::Easy.max_gradient(), 3.0);
        assert_eq!(ProceduralDifficulty::Medium.max_gradient(), 6.0);
        assert_eq!(ProceduralDifficulty::Hard.max_gradient(), 10.0);
        assert_eq!(ProceduralDifficulty::Extreme.max_gradient(), 15.0);
    }

    #[test]
    fn test_world_generator_generate() {
        let seed = WorldSeed::from_value(42).with_length(1000.0);
        let generator = WorldGenerator::new(seed);
        let world = generator.generate();

        // Should have waypoints
        assert!(!world.waypoints.is_empty());
        // Total length should be approximately target
        assert!(world.total_length >= 1000.0);
    }

    #[test]
    fn test_world_generator_reproducible() {
        let seed1 = WorldSeed::from_value(42).with_length(500.0);
        let seed2 = WorldSeed::from_value(42).with_length(500.0);

        let world1 = WorldGenerator::new(seed1).generate();
        let world2 = WorldGenerator::new(seed2).generate();

        // Same seed should produce same world
        assert_eq!(world1.waypoints.len(), world2.waypoints.len());
        assert!((world1.max_elevation - world2.max_elevation).abs() < 0.01);
    }

    #[test]
    fn test_world_generator_rideability() {
        let seed = WorldSeed::from_value(42)
            .with_difficulty(ProceduralDifficulty::Medium)
            .with_length(500.0);
        let generator = WorldGenerator::new(seed);
        let world = generator.generate();

        // Validate rideability
        let result = generator.validate_rideability(&world);
        assert!(
            result.is_rideable,
            "World should be rideable: {:?}",
            result.issues
        );
    }

    #[test]
    fn test_world_generator_respects_difficulty() {
        // Easy difficulty should have lower gradients
        let easy_seed = WorldSeed::from_value(42)
            .with_difficulty(ProceduralDifficulty::Easy)
            .with_length(500.0);
        let easy_world = WorldGenerator::new(easy_seed).generate();

        for wp in &easy_world.waypoints {
            assert!(
                wp.gradient.abs() <= 3.0,
                "Easy route gradient {} exceeds 3%",
                wp.gradient
            );
        }
    }

    #[test]
    fn test_world_generator_biome_preference() {
        let seed = WorldSeed::from_value(42)
            .with_biome(biomes::BiomeType::Alpine)
            .with_length(500.0);
        let world = WorldGenerator::new(seed).generate();

        // All waypoints should have Alpine biome
        for wp in &world.waypoints {
            assert_eq!(wp.biome, biomes::BiomeType::Alpine);
        }
    }

    #[test]
    fn test_surface_type_properties() {
        assert_eq!(SurfaceType::Asphalt.rolling_resistance(), 1.0);
        assert!(SurfaceType::Gravel.rolling_resistance() > 1.0);
        assert!(
            SurfaceType::Cobbles.vibration_intensity() > SurfaceType::Asphalt.vibration_intensity()
        );
    }
}
