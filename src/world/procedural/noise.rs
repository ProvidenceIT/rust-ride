//! Noise generation utilities for procedural content.

use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

/// Noise generator wrapper for terrain and vegetation
#[allow(dead_code)]
pub struct NoiseGenerator {
    /// Base perlin noise
    perlin: Perlin,
    /// Fractal Brownian Motion for terrain
    fbm: Fbm<Perlin>,
    /// Seed value
    seed: u32,
}

impl NoiseGenerator {
    /// Create noise generator with seed
    pub fn new(seed: u32) -> Self {
        let perlin = Perlin::new(seed);
        let fbm = Fbm::<Perlin>::new(seed)
            .set_octaves(4)
            .set_persistence(0.5)
            .set_lacunarity(2.0);

        Self { perlin, fbm, seed }
    }

    /// Sample basic perlin noise at position
    pub fn perlin(&self, x: f64, y: f64) -> f64 {
        self.perlin.get([x, y])
    }

    /// Sample 3D perlin noise
    pub fn perlin_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        self.perlin.get([x, y, z])
    }

    /// Sample FBM terrain noise
    pub fn terrain(&self, x: f64, z: f64, frequency: f64) -> f64 {
        self.fbm.get([x * frequency, z * frequency])
    }

    /// Sample ridged noise (for mountains)
    pub fn ridged(&self, x: f64, z: f64, frequency: f64) -> f64 {
        let n = self.fbm.get([x * frequency, z * frequency]);
        1.0 - n.abs() * 2.0
    }

    /// Sample billow noise (for clouds/soft hills)
    pub fn billow(&self, x: f64, z: f64, frequency: f64) -> f64 {
        self.fbm.get([x * frequency, z * frequency]).abs()
    }

    /// Domain warped noise for more natural terrain
    pub fn warped(&self, x: f64, z: f64, frequency: f64, warp_strength: f64) -> f64 {
        let warp_x = self.perlin.get([x * frequency * 2.0, z * frequency * 2.0]) * warp_strength;
        let warp_z = self
            .perlin
            .get([x * frequency * 2.0 + 100.0, z * frequency * 2.0 + 100.0])
            * warp_strength;

        self.fbm
            .get([(x + warp_x) * frequency, (z + warp_z) * frequency])
    }
}

/// Height map generation for terrain
#[derive(Debug, Clone)]
pub struct HeightMap {
    /// Width in samples
    pub width: usize,
    /// Height in samples
    pub height: usize,
    /// Height values (row-major)
    pub data: Vec<f32>,
    /// Minimum height
    pub min_height: f32,
    /// Maximum height
    pub max_height: f32,
}

impl HeightMap {
    /// Create empty height map
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0.0; width * height],
            min_height: 0.0,
            max_height: 0.0,
        }
    }

    /// Generate height map from noise
    pub fn from_noise(
        noise: &NoiseGenerator,
        width: usize,
        height: usize,
        scale: f64,
        amplitude: f32,
    ) -> Self {
        let mut map = Self::new(width, height);
        let mut min = f32::MAX;
        let mut max = f32::MIN;

        for y in 0..height {
            for x in 0..width {
                let nx = x as f64 / width as f64 * scale;
                let ny = y as f64 / height as f64 * scale;

                let h = noise.terrain(nx, ny, 1.0) as f32 * amplitude;
                map.set(x, y, h);

                min = min.min(h);
                max = max.max(h);
            }
        }

        map.min_height = min;
        map.max_height = max;
        map
    }

    /// Get height at position
    pub fn get(&self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            self.data[y * self.width + x]
        } else {
            0.0
        }
    }

    /// Set height at position
    pub fn set(&mut self, x: usize, y: usize, value: f32) {
        if x < self.width && y < self.height {
            self.data[y * self.width + x] = value;
        }
    }

    /// Sample with bilinear interpolation
    pub fn sample(&self, x: f32, y: f32) -> f32 {
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);

        let fx = x.fract();
        let fy = y.fract();

        let v00 = self.get(x0, y0);
        let v10 = self.get(x1, y0);
        let v01 = self.get(x0, y1);
        let v11 = self.get(x1, y1);

        let v0 = v00 * (1.0 - fx) + v10 * fx;
        let v1 = v01 * (1.0 - fx) + v11 * fx;

        v0 * (1.0 - fy) + v1 * fy
    }

    /// Normalize heights to 0..1 range
    pub fn normalize(&mut self) {
        let range = self.max_height - self.min_height;
        if range > 0.0 {
            for h in &mut self.data {
                *h = (*h - self.min_height) / range;
            }
            self.min_height = 0.0;
            self.max_height = 1.0;
        }
    }
}

/// Mask for feature placement (vegetation, rocks, etc.)
pub struct PlacementMask {
    /// Width in samples
    pub width: usize,
    /// Height in samples
    pub height: usize,
    /// Mask values 0..1
    pub data: Vec<f32>,
}

impl PlacementMask {
    /// Create mask from noise threshold
    pub fn from_noise(
        noise: &NoiseGenerator,
        width: usize,
        height: usize,
        frequency: f64,
        threshold: f32,
    ) -> Self {
        let mut data = Vec::with_capacity(width * height);

        for y in 0..height {
            for x in 0..width {
                let nx = x as f64 / width as f64;
                let ny = y as f64 / height as f64;

                let n = noise.perlin(nx * frequency, ny * frequency) as f32;
                let value = if n > threshold {
                    (n - threshold) / (1.0 - threshold)
                } else {
                    0.0
                };

                data.push(value);
            }
        }

        Self {
            width,
            height,
            data,
        }
    }

    /// Check if position should have feature
    pub fn should_place(&self, x: usize, y: usize) -> bool {
        if x < self.width && y < self.height {
            self.data[y * self.width + x] > 0.0
        } else {
            false
        }
    }

    /// Get placement density at position
    pub fn density(&self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            self.data[y * self.width + x]
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_generator() {
        let gen = NoiseGenerator::new(42);

        let n1 = gen.perlin(0.0, 0.0);
        let n2 = gen.perlin(0.5, 0.5);

        // Values should be in -1..1 range
        assert!(n1 >= -1.0 && n1 <= 1.0);
        assert!(n2 >= -1.0 && n2 <= 1.0);

        // Same seed, same input should give same output
        let gen2 = NoiseGenerator::new(42);
        assert!((gen.perlin(1.0, 1.0) - gen2.perlin(1.0, 1.0)).abs() < 0.001);
    }

    #[test]
    fn test_height_map() {
        let noise = NoiseGenerator::new(42);
        let map = HeightMap::from_noise(&noise, 16, 16, 4.0, 100.0);

        assert_eq!(map.width, 16);
        assert_eq!(map.height, 16);
        assert_eq!(map.data.len(), 256);
    }

    #[test]
    fn test_height_map_sampling() {
        let mut map = HeightMap::new(2, 2);
        map.set(0, 0, 0.0);
        map.set(1, 0, 1.0);
        map.set(0, 1, 0.0);
        map.set(1, 1, 1.0);

        // Midpoint should be interpolated
        let mid = map.sample(0.5, 0.5);
        assert!((mid - 0.5).abs() < 0.01);
    }
}
