# Contract: Weather System Module

**Module**: `src/world/weather/`
**Date**: 2025-12-25

## Purpose

Manage weather state, particle effects (rain/snow/fog), and time-of-day sky rendering for the 3D virtual world.

## Public API

### Types

```rust
/// Weather controller manages weather state and transitions
pub struct WeatherController {
    state: WeatherState,
    particle_system: ParticleSystem,
    skybox: Skybox,
}

/// User preferences for weather
pub struct WeatherPreferences {
    /// Auto-change weather during ride
    pub auto_weather: bool,
    /// Auto-change time during ride
    pub auto_time: bool,
    /// Default weather type
    pub default_weather: WeatherType,
    /// Default time of day
    pub default_time: TimeOfDay,
    /// Weather change interval (minutes, if auto)
    pub change_interval_minutes: u32,
}
```

### Functions

```rust
impl WeatherController {
    /// Create new weather controller
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self;

    /// Get current weather state
    pub fn state(&self) -> &WeatherState;

    /// Set weather type (starts transition)
    pub fn set_weather(&mut self, weather: WeatherType);

    /// Set time of day (starts transition)
    pub fn set_time_of_day(&mut self, time: TimeOfDay);

    /// Set exact time (0.0-24.0)
    pub fn set_time_hours(&mut self, hours: f32);

    /// Enable/disable realistic time progression
    pub fn set_realistic_time(&mut self, enabled: bool);

    /// Update weather state and particles
    ///
    /// # Arguments
    /// * `delta_time` - Time since last update in seconds
    /// * `camera_position` - Camera position for particle spawning
    pub fn update(&mut self, delta_time: f32, camera_position: Vec3);

    /// Render particles to the scene
    ///
    /// Must be called during render pass.
    pub fn render_particles(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        camera: &Camera,
    );

    /// Get sky colors for current time/weather
    pub fn sky_colors(&self) -> SkyColors;

    /// Get ambient light color and intensity
    pub fn ambient_light(&self) -> (Vec3, f32);

    /// Get sun direction and color
    pub fn sun_light(&self) -> (Vec3, Vec3);
}

pub struct SkyColors {
    pub zenith: Vec3,
    pub horizon: Vec3,
    pub sun: Vec3,
    pub fog: Vec3,
}
```

### Particle System

```rust
impl ParticleSystem {
    /// Create particle system for weather effects
    pub fn new(device: &wgpu::Device, max_particles: u32) -> Self;

    /// Update particle positions and spawn new ones
    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        delta_time: f32,
        weather: WeatherType,
        camera_pos: Vec3,
        wind: Vec2,
    );

    /// Render particles using instancing
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        camera: &Camera,
    );

    /// Set particle density (0.0-1.0)
    pub fn set_density(&mut self, density: f32);
}
```

### Skybox

```rust
impl Skybox {
    /// Create procedural skybox
    pub fn new(device: &wgpu::Device) -> Self;

    /// Update sky colors based on time and weather
    pub fn update(&mut self, time_hours: f32, weather: WeatherType);

    /// Render skybox
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        camera: &Camera,
    );
}
```

## Weather Transition Logic

```
Transition Duration: 30-60 seconds
Interpolation: Smooth lerp between states

Weather Effects:
- Clear: No particles, full visibility
- Cloudy: No particles, slightly reduced visibility, overcast sky
- Rain: Rain particles falling, reduced visibility, darker sky
- HeavyRain: Dense rain, significantly reduced visibility
- Fog: Volumetric fog effect, very reduced visibility
- Snow: Snow particles, white horizon, cold lighting
```

## Time-of-Day Logic

```
Time Ranges:
- Dawn: 5:00-7:00 (orange/pink sky, rising sun)
- Day: 7:00-17:00 (blue sky, overhead sun)
- Dusk: 17:00-19:00 (orange/purple sky, setting sun)
- Night: 19:00-5:00 (dark blue/black sky, moon/stars)

Sun Position:
- Calculated from time_hours
- East at dawn, overhead at noon, west at dusk
- Below horizon at night
```

## Performance Requirements

- Particle update: <1ms for 10,000 particles
- Render: <2ms for particle rendering
- Skybox render: <0.5ms
- Weather transition: Smooth at 60 FPS
- Memory: <50MB for particle buffers

## GPU Resources

- Particle vertex/index buffers (instanced)
- Particle texture atlas (rain drop, snowflake, fog)
- Sky gradient texture (generated)
- Compute shader for particle physics (optional)
