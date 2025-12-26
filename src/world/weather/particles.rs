//! Particle system for weather effects (rain, snow, fog).

use glam::Vec3;

/// Particle instance for weather effects
#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub size: f32,
    pub alpha: f32,
}

impl Particle {
    pub fn new(position: Vec3, velocity: Vec3, lifetime: f32, size: f32) -> Self {
        Self {
            position,
            velocity,
            lifetime,
            max_lifetime: lifetime,
            size,
            alpha: 1.0,
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        self.position += self.velocity * delta_time;
        self.lifetime -= delta_time;
        // Fade out as lifetime decreases
        self.alpha = (self.lifetime / self.max_lifetime).max(0.0);
    }

    pub fn is_alive(&self) -> bool {
        self.lifetime > 0.0
    }
}

/// Particle system for weather effects
pub struct ParticleSystem {
    particles: Vec<Particle>,
    max_particles: usize,
    spawn_rate: f32,
    spawn_accumulator: f32,
    particle_type: ParticleType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleType {
    Rain,
    Snow,
    Fog,
}

impl ParticleSystem {
    pub fn new(max_particles: usize) -> Self {
        Self {
            particles: Vec::with_capacity(max_particles),
            max_particles,
            spawn_rate: 100.0, // particles per second
            spawn_accumulator: 0.0,
            particle_type: ParticleType::Rain,
        }
    }

    pub fn set_particle_type(&mut self, particle_type: ParticleType) {
        self.particle_type = particle_type;
    }

    pub fn set_density(&mut self, density: f32) {
        self.spawn_rate = density * 500.0; // Scale density to spawn rate
    }

    pub fn update(&mut self, delta_time: f32, camera_pos: Vec3, wind: glam::Vec2) {
        // Update existing particles
        self.particles.retain_mut(|p| {
            p.update(delta_time);
            p.is_alive()
        });

        // Spawn new particles
        self.spawn_accumulator += delta_time * self.spawn_rate;
        while self.spawn_accumulator >= 1.0 && self.particles.len() < self.max_particles {
            self.spawn_accumulator -= 1.0;
            self.spawn_particle(camera_pos, wind);
        }
    }

    fn spawn_particle(&mut self, camera_pos: Vec3, wind: glam::Vec2) {
        // Spawn in a box around the camera
        let spawn_radius = 50.0;
        let spawn_height = 30.0;

        let offset_x = (rand_f32() - 0.5) * spawn_radius * 2.0;
        let offset_z = (rand_f32() - 0.5) * spawn_radius * 2.0;

        let position = Vec3::new(
            camera_pos.x + offset_x,
            camera_pos.y + spawn_height,
            camera_pos.z + offset_z,
        );

        let (velocity, lifetime, size) = match self.particle_type {
            ParticleType::Rain => {
                let vel = Vec3::new(wind.x * 0.5, -15.0, wind.y * 0.5);
                (vel, 2.0, 0.1)
            }
            ParticleType::Snow => {
                let vel = Vec3::new(
                    wind.x * 0.2 + (rand_f32() - 0.5) * 0.5,
                    -2.0,
                    wind.y * 0.2 + (rand_f32() - 0.5) * 0.5,
                );
                (vel, 10.0, 0.15)
            }
            ParticleType::Fog => {
                let vel = Vec3::new(wind.x * 0.1, 0.0, wind.y * 0.1);
                (vel, 15.0, 2.0)
            }
        };

        self.particles
            .push(Particle::new(position, velocity, lifetime, size));
    }

    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    pub fn clear(&mut self) {
        self.particles.clear();
    }
}

/// Simple random number generator (0.0-1.0)
fn rand_f32() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    nanos as f32 / 4_294_967_295.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_lifecycle() {
        let mut particle = Particle::new(Vec3::ZERO, Vec3::new(0.0, -1.0, 0.0), 1.0, 0.1);
        assert!(particle.is_alive());

        particle.update(0.5);
        assert!(particle.is_alive());
        assert!((particle.alpha - 0.5).abs() < 0.01);

        particle.update(0.6);
        assert!(!particle.is_alive());
    }

    #[test]
    fn test_particle_system() {
        let mut system = ParticleSystem::new(100);
        system.set_density(1.0);

        let camera_pos = Vec3::ZERO;
        let wind = glam::Vec2::ZERO;

        system.update(0.1, camera_pos, wind);
        assert!(!system.particles().is_empty());
    }
}
