//! Biome system for environmental variety.

use super::noise::NoiseGenerator;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Biome types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BiomeType {
    /// Grassy meadows with gentle hills
    Meadow,
    /// Dense forest
    Forest,
    /// Rocky mountain terrain
    Mountain,
    /// Dry desert/savanna
    Desert,
    /// Tropical/Mediterranean
    Coastal,
    /// Snow-covered terrain
    Alpine,
    /// Agricultural fields
    Farmland,
    /// Urban/suburban area
    Urban,
}

impl BiomeType {
    /// Get terrain color for this biome
    pub fn base_color(&self) -> Vec3 {
        match self {
            Self::Meadow => Vec3::new(0.4, 0.7, 0.3),
            Self::Forest => Vec3::new(0.2, 0.5, 0.2),
            Self::Mountain => Vec3::new(0.5, 0.5, 0.45),
            Self::Desert => Vec3::new(0.8, 0.7, 0.5),
            Self::Coastal => Vec3::new(0.5, 0.7, 0.6),
            Self::Alpine => Vec3::new(0.9, 0.95, 1.0),
            Self::Farmland => Vec3::new(0.6, 0.7, 0.3),
            Self::Urban => Vec3::new(0.5, 0.5, 0.5),
        }
    }

    /// Get tree density (0..1)
    pub fn tree_density(&self) -> f32 {
        match self {
            Self::Meadow => 0.1,
            Self::Forest => 0.8,
            Self::Mountain => 0.3,
            Self::Desert => 0.02,
            Self::Coastal => 0.4,
            Self::Alpine => 0.15,
            Self::Farmland => 0.05,
            Self::Urban => 0.1,
        }
    }

    /// Get grass density (0..1)
    pub fn grass_density(&self) -> f32 {
        match self {
            Self::Meadow => 0.9,
            Self::Forest => 0.4,
            Self::Mountain => 0.3,
            Self::Desert => 0.1,
            Self::Coastal => 0.6,
            Self::Alpine => 0.2,
            Self::Farmland => 0.5,
            Self::Urban => 0.3,
        }
    }

    /// Get rock density (0..1)
    pub fn rock_density(&self) -> f32 {
        match self {
            Self::Meadow => 0.05,
            Self::Forest => 0.1,
            Self::Mountain => 0.6,
            Self::Desert => 0.3,
            Self::Coastal => 0.15,
            Self::Alpine => 0.4,
            Self::Farmland => 0.02,
            Self::Urban => 0.1,
        }
    }

    /// Get height variation multiplier
    pub fn height_variation(&self) -> f32 {
        match self {
            Self::Meadow => 0.3,
            Self::Forest => 0.4,
            Self::Mountain => 1.0,
            Self::Desert => 0.2,
            Self::Coastal => 0.2,
            Self::Alpine => 0.8,
            Self::Farmland => 0.1,
            Self::Urban => 0.05,
        }
    }
}

/// Biome parameters for generation
#[derive(Debug, Clone)]
pub struct BiomeParams {
    /// Moisture level (0=dry, 1=wet)
    pub moisture: f32,
    /// Temperature (0=cold, 1=hot)
    pub temperature: f32,
    /// Elevation (0=low, 1=high)
    pub elevation: f32,
}

impl BiomeParams {
    /// Determine biome from parameters
    pub fn to_biome(&self) -> BiomeType {
        // Simple biome selection based on parameters
        if self.elevation > 0.8 {
            return BiomeType::Alpine;
        }

        if self.elevation > 0.6 {
            return BiomeType::Mountain;
        }

        if self.temperature > 0.7 && self.moisture < 0.3 {
            return BiomeType::Desert;
        }

        if self.moisture > 0.6 && self.temperature > 0.5 {
            return BiomeType::Coastal;
        }

        if self.moisture > 0.5 {
            return BiomeType::Forest;
        }

        if self.elevation < 0.3 && self.moisture > 0.3 {
            return BiomeType::Farmland;
        }

        BiomeType::Meadow
    }
}

/// Biome map for a region
pub struct BiomeMap {
    /// Width in samples
    width: usize,
    /// Height in samples
    height: usize,
    /// Biome type at each position
    biomes: Vec<BiomeType>,
    /// Blend values for smooth transitions (0..1 for primary biome)
    blends: Vec<f32>,
}

impl BiomeMap {
    /// Generate biome map from noise
    pub fn generate(noise: &NoiseGenerator, width: usize, height: usize, scale: f64) -> Self {
        let mut biomes = Vec::with_capacity(width * height);
        let mut blends = Vec::with_capacity(width * height);

        for y in 0..height {
            for x in 0..width {
                let nx = x as f64 / width as f64 * scale;
                let ny = y as f64 / height as f64 * scale;

                // Generate parameters from different noise frequencies
                let moisture = (noise.perlin(nx * 0.5, ny * 0.5) * 0.5 + 0.5) as f32;
                let temperature =
                    (noise.perlin(nx * 0.3 + 100.0, ny * 0.3 + 100.0) * 0.5 + 0.5) as f32;
                let elevation = (noise.terrain(nx, ny, 1.0) * 0.5 + 0.5) as f32;

                let params = BiomeParams {
                    moisture,
                    temperature,
                    elevation,
                };

                biomes.push(params.to_biome());

                // Calculate blend factor (how strongly this biome applies)
                // Higher values near biome "center", lower near transitions
                let blend = 1.0
                    - (moisture - 0.5).abs() * 2.0 * 0.3
                    - (temperature - 0.5).abs() * 2.0 * 0.3;
                blends.push(blend.clamp(0.3, 1.0));
            }
        }

        Self {
            width,
            height,
            biomes,
            blends,
        }
    }

    /// Get biome at position
    pub fn get(&self, x: usize, y: usize) -> BiomeType {
        if x < self.width && y < self.height {
            self.biomes[y * self.width + x]
        } else {
            BiomeType::Meadow
        }
    }

    /// Get blend factor at position
    pub fn blend(&self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            self.blends[y * self.width + x]
        } else {
            1.0
        }
    }

    /// Get blended color at position
    pub fn color_at(&self, x: usize, y: usize) -> Vec3 {
        let biome = self.get(x, y);
        let blend = self.blend(x, y);

        // Blend with neutral color based on blend factor
        let base = biome.base_color();
        let neutral = Vec3::new(0.5, 0.5, 0.5);

        base.lerp(neutral, 1.0 - blend)
    }
}

/// Vegetation placement based on biome
pub struct VegetationPlacer {
    noise: NoiseGenerator,
}

impl VegetationPlacer {
    /// Create vegetation placer
    pub fn new(seed: u32) -> Self {
        Self {
            noise: NoiseGenerator::new(seed),
        }
    }

    /// Get tree positions for a chunk
    pub fn tree_positions(
        &self,
        biome: BiomeType,
        chunk_x: f32,
        chunk_z: f32,
        chunk_size: f32,
        resolution: u32,
    ) -> Vec<(f32, f32)> {
        let density = biome.tree_density();
        let base_count = (resolution as f32 * resolution as f32 * density * 0.1) as u32;

        let mut positions = Vec::new();

        for i in 0..base_count {
            // Use noise to jitter positions
            let base_x = (i % resolution) as f32 / resolution as f32 * chunk_size;
            let base_z = (i / resolution) as f32 / resolution as f32 * chunk_size;

            let jitter_x = self.noise.perlin(
                (chunk_x + base_x) as f64 * 10.0,
                (chunk_z + base_z) as f64 * 10.0,
            ) as f32
                * chunk_size
                / resolution as f32;

            let jitter_z = self.noise.perlin(
                (chunk_x + base_x) as f64 * 10.0 + 1000.0,
                (chunk_z + base_z) as f64 * 10.0 + 1000.0,
            ) as f32
                * chunk_size
                / resolution as f32;

            // Check if this position should have a tree
            let placement_noise = self.noise.perlin(
                (chunk_x + base_x) as f64 * 5.0,
                (chunk_z + base_z) as f64 * 5.0,
            ) as f32;

            if placement_noise > 1.0 - density * 2.0 {
                positions.push((chunk_x + base_x + jitter_x, chunk_z + base_z + jitter_z));
            }
        }

        positions
    }

    /// Get rock positions for a chunk
    pub fn rock_positions(
        &self,
        biome: BiomeType,
        chunk_x: f32,
        chunk_z: f32,
        chunk_size: f32,
    ) -> Vec<(f32, f32, f32)> {
        let density = biome.rock_density();
        let count = (16.0 * density) as u32;

        let mut positions = Vec::new();

        for i in 0..count {
            let nx = (chunk_x + i as f32 * 17.3) as f64 * 0.1;
            let nz = (chunk_z + i as f32 * 23.7) as f64 * 0.1;

            let x = chunk_x + (self.noise.perlin(nx, nz) as f32 * 0.5 + 0.5) * chunk_size;
            let z =
                chunk_z + (self.noise.perlin(nx + 50.0, nz + 50.0) as f32 * 0.5 + 0.5) * chunk_size;
            let scale = 0.5 + self.noise.perlin(nx + 100.0, nz + 100.0) as f32 * 0.5;

            positions.push((x, z, scale.max(0.2)));
        }

        positions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biome_params() {
        // High elevation should give Alpine
        let alpine = BiomeParams {
            moisture: 0.5,
            temperature: 0.5,
            elevation: 0.9,
        };
        assert_eq!(alpine.to_biome(), BiomeType::Alpine);

        // Hot and dry should give Desert
        let desert = BiomeParams {
            moisture: 0.1,
            temperature: 0.9,
            elevation: 0.3,
        };
        assert_eq!(desert.to_biome(), BiomeType::Desert);
    }

    #[test]
    fn test_biome_map() {
        let noise = NoiseGenerator::new(42);
        let map = BiomeMap::generate(&noise, 16, 16, 4.0);

        // Should have generated valid biomes
        for y in 0..16 {
            for x in 0..16 {
                let _biome = map.get(x, y); // Should not panic
                let blend = map.blend(x, y);
                assert!(blend >= 0.0 && blend <= 1.0);
            }
        }
    }

    #[test]
    fn test_vegetation_placer() {
        let placer = VegetationPlacer::new(42);
        let positions = placer.tree_positions(BiomeType::Forest, 0.0, 0.0, 256.0, 8);

        // Forest should have trees
        assert!(!positions.is_empty());
    }
}
