# Contract: 3D Virtual World Module

**Module**: `src/world/` (new module)
**Feature**: 002-3d-world-features
**Status**: New module

## Purpose

Implement a 3D virtual cycling environment with avatar movement based on real-time power data. This is the core visual experience for immersive indoor training.

## Module Structure

```
src/world/
├── mod.rs           # Module exports
├── renderer.rs      # wgpu rendering pipeline
├── scene.rs         # Scene graph management
├── camera.rs        # Third-person camera
├── avatar.rs        # Cyclist avatar and animations
├── terrain.rs       # Road and landscape
├── physics.rs       # Power-to-speed calculations
├── route.rs         # Route path and elevation
├── hud.rs           # 2D overlay on 3D view
└── worlds/
    ├── mod.rs
    ├── countryside.rs
    ├── mountains.rs
    └── coastal.rs
```

## Public Interface

### World3D (Main Entry Point)

```rust
pub struct World3D {
    renderer: Renderer,
    scene: Scene,
    camera: Camera,
    avatar: Avatar,
    physics: PhysicsEngine,
    current_route: Route,
    hud: Hud,
}

impl World3D {
    /// Create new 3D world with specified world and route
    pub fn new(
        render_state: &egui_wgpu::RenderState,
        world: WorldDefinition,
        route: RouteDefinition,
        avatar_config: AvatarConfig,
    ) -> Result<Self, WorldError>;

    /// Update world state (called each frame)
    /// power_watts: current power reading
    /// delta_time: seconds since last update
    pub fn update(&mut self, power_watts: u16, delta_time: f32);

    /// Render frame to texture
    pub fn render(&mut self) -> &egui::TextureId;

    /// Get current ride stats for HUD
    pub fn get_stats(&self) -> WorldStats;

    /// Get current position on route (0.0 - 1.0)
    pub fn get_route_progress(&self) -> f32;

    /// Cleanup GPU resources
    pub fn destroy(&mut self);
}

pub struct WorldStats {
    pub speed_mps: f32,
    pub distance_meters: f32,
    pub elevation_meters: f32,
    pub gradient_percent: f32,
    pub route_remaining_meters: f32,
}
```

### Renderer

```rust
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    depth_texture: wgpu::Texture,
    output_texture: wgpu::Texture,
}

impl Renderer {
    /// Initialize wgpu pipeline from eframe's render state
    pub fn new(render_state: &egui_wgpu::RenderState, width: u32, height: u32) -> Result<Self, RendererError>;

    /// Render scene to texture
    pub fn render(&mut self, scene: &Scene, camera: &Camera) -> &wgpu::Texture;

    /// Resize render target
    pub fn resize(&mut self, width: u32, height: u32);
}
```

### Scene

```rust
pub struct Scene {
    pub terrain: Terrain,
    pub road: Road,
    pub avatar: Avatar,
    pub scenery: Vec<SceneryObject>,
    pub sky: Sky,
    pub lighting: Lighting,
}

pub struct SceneryObject {
    pub model: ModelHandle,
    pub transform: Transform,
    pub lod_distance: f32,
}

pub struct Transform {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}
```

### Camera

```rust
pub struct Camera {
    pub position: glam::Vec3,
    pub target: glam::Vec3,
    pub up: glam::Vec3,
    pub fov_degrees: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    /// Update camera to follow avatar from behind
    pub fn follow(&mut self, avatar_position: glam::Vec3, avatar_direction: glam::Vec3);

    /// Get view-projection matrix for rendering
    pub fn view_projection(&self, aspect_ratio: f32) -> glam::Mat4;
}
```

### Avatar

```rust
pub struct Avatar {
    pub model: ModelHandle,
    pub bike_model: ModelHandle,
    pub position: glam::Vec3,
    pub rotation: f32,          // Heading in radians
    pub animation_state: AnimationState,
    pub config: AvatarConfig,
}

pub struct AvatarConfig {
    pub jersey_color: [u8; 3],
    pub bike_style: BikeStyle,
}

pub enum BikeStyle {
    RoadBike,
    TimeTrial,
    Gravel,
}

pub enum AnimationState {
    Idle,
    Pedaling { cadence: f32 },
    Coasting,
}

impl Avatar {
    /// Update avatar position along route
    pub fn update(&mut self, speed: f32, route: &Route, delta_time: f32);

    /// Set pedaling animation speed based on cadence
    pub fn set_cadence(&mut self, cadence: u8);
}
```

### Physics Engine

```rust
pub struct PhysicsEngine {
    pub rider_mass_kg: f32,
    pub bike_mass_kg: f32,
    pub cda: f32,               // Drag coefficient × area
    pub crr: f32,               // Rolling resistance coefficient
}

impl PhysicsEngine {
    /// Calculate speed from power, gradient, and physics parameters
    pub fn calculate_speed(&self, power_watts: u16, gradient_percent: f32) -> f32;

    /// Update rider mass (from settings)
    pub fn set_rider_mass(&mut self, mass_kg: f32);
}

// Physics constants
const AIR_DENSITY: f32 = 1.225;     // kg/m³
const GRAVITY: f32 = 9.81;          // m/s²
const DEFAULT_CDA: f32 = 0.32;      // m² (hoods position)
const DEFAULT_CRR: f32 = 0.004;     // Road tires
const BIKE_MASS: f32 = 8.0;         // kg
```

### Route

```rust
pub struct Route {
    pub name: String,
    pub total_distance: f32,
    pub waypoints: Vec<Waypoint>,
    pub elevation_profile: Vec<f32>,
}

pub struct Waypoint {
    pub position: glam::Vec3,
    pub distance: f32,          // Distance from start
    pub gradient: f32,          // Gradient at this point
}

impl Route {
    /// Get position and heading at given distance
    pub fn get_position(&self, distance: f32) -> (glam::Vec3, f32);

    /// Get gradient at given distance
    pub fn get_gradient(&self, distance: f32) -> f32;

    /// Get elevation at given distance
    pub fn get_elevation(&self, distance: f32) -> f32;
}
```

### World Definitions

```rust
pub struct WorldDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub preview_image: String,
    pub terrain_config: TerrainConfig,
    pub scenery_config: SceneryConfig,
    pub sky_config: SkyConfig,
    pub routes: Vec<RouteDefinition>,
}

pub struct RouteDefinition {
    pub id: String,
    pub name: String,
    pub distance_meters: f32,
    pub elevation_gain_meters: f32,
    pub difficulty: RouteDifficulty,
    pub waypoints_file: String,
}

pub enum RouteDifficulty {
    Easy,
    Moderate,
    Challenging,
    Extreme,
}
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("GPU initialization failed: {0}")]
    GpuInitError(String),

    #[error("Shader compilation failed: {0}")]
    ShaderError(String),

    #[error("Asset loading failed: {0}")]
    AssetError(String),

    #[error("World definition not found: {0}")]
    WorldNotFound(String),

    #[error("Route not found: {0}")]
    RouteNotFound(String),

    #[error("Insufficient GPU memory")]
    OutOfMemory,
}
```

## Rendering Pipeline

```
┌────────────────────────────────────────────────────────┐
│                    Frame Update                         │
├────────────────────────────────────────────────────────┤
│  1. Update Physics (power → speed)                     │
│  2. Update Avatar Position (speed → distance)          │
│  3. Update Camera (follow avatar)                      │
│  4. Cull Scenery (frustum + LOD)                       │
│  5. Render Pass:                                        │
│     a. Clear depth + color                             │
│     b. Render sky                                       │
│     c. Render terrain                                   │
│     d. Render road                                      │
│     e. Render scenery                                   │
│     f. Render avatar                                    │
│  6. Render HUD overlay                                  │
└────────────────────────────────────────────────────────┘
```

## Integration with egui

```rust
// In ride screen
impl RideScreen {
    fn render_3d_view(&mut self, ui: &mut egui::Ui, power: u16) {
        // Update world
        let delta = self.last_frame.elapsed().as_secs_f32();
        self.world.update(power, delta);
        self.last_frame = Instant::now();

        // Get rendered texture
        let texture_id = self.world.render();

        // Display in egui
        let available = ui.available_size();
        ui.image(texture_id, available);

        // Overlay HUD
        self.render_hud(ui, self.world.get_stats());
    }
}
```

## Asset Requirements

```
assets/
├── models/
│   ├── cyclist.glb          # ~2MB - Avatar with rig
│   ├── bikes/
│   │   ├── road_bike.glb    # ~500KB each
│   │   ├── tt_bike.glb
│   │   └── gravel_bike.glb
│   └── scenery/
│       ├── tree_pine.glb    # ~100KB each
│       ├── tree_oak.glb
│       ├── building_house.glb
│       └── ...
├── textures/
│   ├── terrain/
│   │   ├── grass.png        # 1024x1024, ~1MB
│   │   ├── asphalt.png
│   │   └── dirt.png
│   └── sky/
│       └── sky_gradient.png
└── worlds/
    ├── countryside.json      # World definition
    ├── mountains.json
    └── coastal.json
```

## Performance Targets

| Metric | Target | Minimum |
|--------|--------|---------|
| Frame rate | 60 FPS | 30 FPS |
| Draw calls | <100 | <200 |
| GPU memory | <500MB | <1GB |
| Load time | <5s | <10s |
| Latency (power→visual) | <100ms | <200ms |

## Implementation Notes

1. **wgpu access**: Use `eframe::egui_wgpu::RenderState` to get device/queue
2. **Render to texture**: Render 3D to wgpu texture, then display in egui `Image`
3. **LOD system**: 3 levels of detail based on distance from camera
4. **Frustum culling**: Only render objects in camera view
5. **Asset loading**: Lazy load world assets, show loading screen
6. **Fallback**: If GPU doesn't support required features, offer 2D mode

## Dependencies

```toml
wgpu = "0.20"              # GPU abstraction (via eframe)
glam = "0.27"              # 3D math
gltf = "1.4"               # Model loading
bytemuck = { version = "1.14", features = ["derive"] }
image = { version = "0.25", features = ["png"] }
```

## Testing Requirements

- Unit tests for physics calculations
- Unit tests for route interpolation
- Integration test for world loading
- Manual performance testing on target hardware
- Visual regression testing (screenshot comparison)
