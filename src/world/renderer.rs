//! GPU renderer using wgpu
//!
//! This module provides the 3D rendering pipeline for the virtual world.
//! It integrates with eframe's wgpu backend to share the GPU context.
//!
//! T142: Effort-based vignette effect
//! T143: Effort-based color grading

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use super::camera::Camera;
use super::scene::{Lighting, Scene, Sky};
use super::terrain::{Road, Terrain};
use super::weather::skybox::{ambient_light, sun_position, SkyColors};
use super::weather::WeatherState;
use super::WorldError;

/// Vertex format for 3D rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Uniform buffer for camera and lighting data
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Uniforms {
    pub view_proj: [[f32; 4]; 4],
    pub sun_direction: [f32; 4],
    pub sun_color: [f32; 4],
    pub ambient_color: [f32; 4],
}

impl Uniforms {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            sun_direction: [0.5, 1.0, 0.3, 0.0],
            sun_color: [1.0, 0.95, 0.9, 1.0],
            ambient_color: [0.3, 0.3, 0.35, 1.0],
        }
    }

    pub fn update(&mut self, camera: &Camera, lighting: &Lighting, aspect_ratio: f32) {
        self.view_proj = camera.view_projection(aspect_ratio).to_cols_array_2d();
        self.sun_direction = [
            lighting.sun_direction.x,
            lighting.sun_direction.y,
            lighting.sun_direction.z,
            0.0,
        ];
        self.sun_color = [
            lighting.sun_color.x,
            lighting.sun_color.y,
            lighting.sun_color.z,
            1.0,
        ];
        self.ambient_color = [
            lighting.ambient_color.x,
            lighting.ambient_color.y,
            lighting.ambient_color.z,
            1.0,
        ];
    }
}

impl Default for Uniforms {
    fn default() -> Self {
        Self::new()
    }
}

/// Mesh data for rendering
pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

// ========== T142-T143: Immersion Effects ==========

/// Post-processing immersion effects based on effort level
#[derive(Debug, Clone, Default)]
pub struct ImmersionEffects {
    /// Current effort level (0.0-1.0, where 1.0 = FTP)
    pub effort_level: f32,
    /// Vignette intensity (0.0-1.0)
    pub vignette_intensity: f32,
    /// Vignette radius (0.0-1.0)
    pub vignette_radius: f32,
    /// Color grading: red shift for high effort
    pub color_shift_red: f32,
    /// Color grading: desaturation for extreme effort
    pub desaturation: f32,
    /// Pulse effect phase (0..2π)
    pub pulse_phase: f32,
    /// Whether effects are enabled
    pub enabled: bool,
}

impl ImmersionEffects {
    /// Create immersion effects (enabled by default)
    pub fn new() -> Self {
        Self {
            effort_level: 0.0,
            vignette_intensity: 0.0,
            vignette_radius: 0.8,
            color_shift_red: 0.0,
            desaturation: 0.0,
            pulse_phase: 0.0,
            enabled: true,
        }
    }

    /// Update effects based on current power and FTP
    pub fn update(&mut self, power_watts: u16, ftp: u16, delta_time: f32) {
        if !self.enabled || ftp == 0 {
            self.reset_effects();
            return;
        }

        // Calculate effort level (normalized to FTP)
        self.effort_level = (power_watts as f32 / ftp as f32).clamp(0.0, 2.0);

        // T142: Vignette effect - increases with effort
        // Starts at 75% FTP, maxes out at 120% FTP
        if self.effort_level > 0.75 {
            let vignette_factor = ((self.effort_level - 0.75) / 0.45).clamp(0.0, 1.0);
            self.vignette_intensity = vignette_factor * 0.5; // Max 50% vignette
            self.vignette_radius = 0.8 - vignette_factor * 0.3; // Shrink from 0.8 to 0.5
        } else {
            self.vignette_intensity = 0.0;
            self.vignette_radius = 0.8;
        }

        // T143: Color grading - red shift at high effort
        // Starts at 85% FTP
        if self.effort_level > 0.85 {
            let color_factor = ((self.effort_level - 0.85) / 0.35).clamp(0.0, 1.0);
            self.color_shift_red = color_factor * 0.15; // Max 15% red shift
            self.desaturation = color_factor * 0.2; // Max 20% desaturation
        } else {
            self.color_shift_red = 0.0;
            self.desaturation = 0.0;
        }

        // Pulse effect at very high effort (>FTP)
        if self.effort_level > 1.0 {
            self.pulse_phase += delta_time * 2.0 * std::f32::consts::PI;
            if self.pulse_phase > std::f32::consts::TAU {
                self.pulse_phase -= std::f32::consts::TAU;
            }
        } else {
            self.pulse_phase = 0.0;
        }
    }

    /// Reset all effects to zero
    fn reset_effects(&mut self) {
        self.effort_level = 0.0;
        self.vignette_intensity = 0.0;
        self.vignette_radius = 0.8;
        self.color_shift_red = 0.0;
        self.desaturation = 0.0;
        self.pulse_phase = 0.0;
    }

    /// Get intensity label for HUD
    pub fn intensity_label(&self) -> &'static str {
        if self.effort_level >= 1.2 {
            "Maximum Effort!"
        } else if self.effort_level >= 1.0 {
            "Threshold"
        } else if self.effort_level >= 0.85 {
            "Hard"
        } else if self.effort_level >= 0.75 {
            "Tempo"
        } else if self.effort_level >= 0.55 {
            "Endurance"
        } else {
            "Easy"
        }
    }

    /// Get RGB color adjustment for post-processing
    pub fn color_adjustment(&self) -> [f32; 3] {
        let pulse = if self.effort_level > 1.0 {
            (self.pulse_phase.sin() + 1.0) * 0.5 * 0.05 // 0-5% pulse
        } else {
            0.0
        };

        [
            1.0 + self.color_shift_red + pulse, // R: boost
            1.0 - self.desaturation * 0.3,      // G: reduce
            1.0 - self.desaturation * 0.3,      // B: reduce
        ]
    }

    /// Calculate vignette factor for a pixel at given UV (0-1)
    pub fn vignette_factor(&self, u: f32, v: f32) -> f32 {
        if self.vignette_intensity == 0.0 {
            return 1.0;
        }

        // Distance from center (0.5, 0.5)
        let dx = u - 0.5;
        let dy = v - 0.5;
        let dist = (dx * dx + dy * dy).sqrt() * 2.0; // Normalize to 0-1 at corners

        // Smooth vignette falloff
        let vignette = if dist < self.vignette_radius {
            1.0
        } else {
            let falloff = (dist - self.vignette_radius) / (1.0 - self.vignette_radius);
            1.0 - falloff.powf(2.0) * self.vignette_intensity
        };

        vignette.clamp(0.0, 1.0)
    }
}

/// GPU renderer for the 3D world
pub struct Renderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    render_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    output_texture: wgpu::Texture,
    output_view: wgpu::TextureView,
    width: u32,
    height: u32,
    uniforms: Uniforms,
    // Pre-built meshes
    terrain_mesh: Option<Mesh>,
    road_mesh: Option<Mesh>,
    sky_mesh: Option<Mesh>,
    avatar_mesh: Option<Mesh>,
    /// T093: Landmark visual markers
    landmark_markers: Vec<Mesh>,
    initialized: bool,
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("initialized", &self.initialized)
            .finish()
    }
}

impl Renderer {
    /// Create a new renderer with wgpu pipeline initialization
    ///
    /// # Arguments
    /// * `device` - wgpu device from eframe's render state
    /// * `queue` - wgpu queue from eframe's render state
    /// * `width` - Render target width
    /// * `height` - Render target height
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        width: u32,
        height: u32,
    ) -> Result<Self, WorldError> {
        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("World Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/world.wgsl").into()),
        });

        // Create uniform buffer
        let uniforms = Uniforms::new();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Create bind group
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("World Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // Create depth texture
        let (depth_texture, depth_view) = Self::create_depth_texture(&device, width, height);

        // Create output texture
        let (output_texture, output_view) = Self::create_output_texture(&device, width, height);

        Ok(Self {
            device,
            queue,
            render_pipeline,
            uniform_buffer,
            uniform_bind_group,
            depth_texture,
            depth_view,
            output_texture,
            output_view,
            width,
            height,
            uniforms,
            terrain_mesh: None,
            road_mesh: None,
            sky_mesh: None,
            avatar_mesh: None,
            landmark_markers: Vec::new(),
            initialized: true,
        })
    }

    /// Create a new renderer (placeholder for non-wgpu contexts)
    #[allow(unused_variables)]
    pub fn new_placeholder(_width: u32, _height: u32) -> Result<Self, WorldError> {
        Err(WorldError::GpuInitError(
            "Renderer requires wgpu device and queue".to_string(),
        ))
    }

    fn create_depth_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let size = wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        (texture, view)
    }

    fn create_output_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let size = wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Output Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        (texture, view)
    }

    /// Resize the render target
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }

        self.width = width;
        self.height = height;

        // Recreate textures
        let (depth_texture, depth_view) = Self::create_depth_texture(&self.device, width, height);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;

        let (output_texture, output_view) =
            Self::create_output_texture(&self.device, width, height);
        self.output_texture = output_texture;
        self.output_view = output_view;

        // Rebuild meshes if needed
        self.rebuild_meshes();
    }

    /// Get current dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Check if renderer is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get reference to the output texture
    pub fn output_texture(&self) -> &wgpu::Texture {
        &self.output_texture
    }

    /// Build terrain mesh
    fn build_terrain_mesh(&self, terrain: &Terrain) -> Mesh {
        let half_size = terrain.size / 2.0;
        let color = [
            terrain.base_color.x,
            terrain.base_color.y,
            terrain.base_color.z,
        ];

        // Simple ground plane
        let vertices = vec![
            Vertex {
                position: [-half_size, 0.0, -half_size],
                normal: [0.0, 1.0, 0.0],
                color,
            },
            Vertex {
                position: [half_size, 0.0, -half_size],
                normal: [0.0, 1.0, 0.0],
                color,
            },
            Vertex {
                position: [half_size, 0.0, half_size],
                normal: [0.0, 1.0, 0.0],
                color,
            },
            Vertex {
                position: [-half_size, 0.0, half_size],
                normal: [0.0, 1.0, 0.0],
                color,
            },
        ];

        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Terrain Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Terrain Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        Mesh {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
        }
    }

    /// Build road mesh along a path
    fn build_road_mesh(&self, road: &Road, waypoints: &[Vec3]) -> Mesh {
        let color = [road.color.x, road.color.y, road.color.z];
        let half_width = road.width / 2.0;

        if waypoints.len() < 2 {
            // Create a default straight road if no waypoints
            let default_waypoints = vec![Vec3::new(0.0, 0.01, 0.0), Vec3::new(1000.0, 0.01, 0.0)];
            return self.build_road_from_waypoints(&default_waypoints, half_width, &color);
        }

        self.build_road_from_waypoints(waypoints, half_width, &color)
    }

    fn build_road_from_waypoints(
        &self,
        waypoints: &[Vec3],
        half_width: f32,
        color: &[f32; 3],
    ) -> Mesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for i in 0..waypoints.len() - 1 {
            let p0 = waypoints[i];
            let p1 = waypoints[i + 1];

            // Direction and perpendicular
            let dir = (p1 - p0).normalize();
            let perp = Vec3::new(-dir.z, 0.0, dir.x);

            // Road height slightly above terrain
            let y_offset = 0.01;

            // Four corners of this road segment
            let v0 = p0 + perp * half_width + Vec3::Y * y_offset;
            let v1 = p0 - perp * half_width + Vec3::Y * y_offset;
            let v2 = p1 + perp * half_width + Vec3::Y * y_offset;
            let v3 = p1 - perp * half_width + Vec3::Y * y_offset;

            let base_idx = vertices.len() as u32;

            vertices.push(Vertex {
                position: [v0.x, v0.y, v0.z],
                normal: [0.0, 1.0, 0.0],
                color: *color,
            });
            vertices.push(Vertex {
                position: [v1.x, v1.y, v1.z],
                normal: [0.0, 1.0, 0.0],
                color: *color,
            });
            vertices.push(Vertex {
                position: [v2.x, v2.y, v2.z],
                normal: [0.0, 1.0, 0.0],
                color: *color,
            });
            vertices.push(Vertex {
                position: [v3.x, v3.y, v3.z],
                normal: [0.0, 1.0, 0.0],
                color: *color,
            });

            // Two triangles
            indices.push(base_idx);
            indices.push(base_idx + 1);
            indices.push(base_idx + 2);
            indices.push(base_idx + 1);
            indices.push(base_idx + 3);
            indices.push(base_idx + 2);
        }

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Road Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Road Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        Mesh {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
        }
    }

    /// Build sky dome mesh
    fn build_sky_mesh(&self, sky: &Sky) -> Mesh {
        // Simple sky dome using a hemisphere
        let segments = 32;
        let rings = 16;
        let radius = 900.0;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Generate vertices
        for ring in 0..=rings {
            let phi = std::f32::consts::PI * 0.5 * (ring as f32 / rings as f32);
            let y = radius * phi.sin();
            let ring_radius = radius * phi.cos();

            for seg in 0..=segments {
                let theta = 2.0 * std::f32::consts::PI * (seg as f32 / segments as f32);
                let x = ring_radius * theta.cos();
                let z = ring_radius * theta.sin();

                // Interpolate color from horizon to top
                let t = ring as f32 / rings as f32;
                let color = sky.horizon_color.lerp(sky.top_color, t);

                vertices.push(Vertex {
                    position: [x, y, z],
                    normal: [0.0, -1.0, 0.0], // Point inward
                    color: [color.x, color.y, color.z],
                });
            }
        }

        // Generate indices
        for ring in 0..rings {
            for seg in 0..segments {
                let current = ring * (segments + 1) + seg;
                let next = current + segments + 1;

                indices.push(current as u32);
                indices.push((current + 1) as u32);
                indices.push(next as u32);

                indices.push((current + 1) as u32);
                indices.push((next + 1) as u32);
                indices.push(next as u32);
            }
        }

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sky Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sky Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        Mesh {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
        }
    }

    /// Build avatar mesh (simple cyclist representation)
    fn build_avatar_mesh(&self, color: [f32; 3]) -> Mesh {
        // Simple box representation of cyclist
        let vertices = vec![
            // Body (cube)
            // Front face
            Vertex {
                position: [-0.2, 0.8, 0.15],
                normal: [0.0, 0.0, 1.0],
                color,
            },
            Vertex {
                position: [0.2, 0.8, 0.15],
                normal: [0.0, 0.0, 1.0],
                color,
            },
            Vertex {
                position: [0.2, 1.4, 0.15],
                normal: [0.0, 0.0, 1.0],
                color,
            },
            Vertex {
                position: [-0.2, 1.4, 0.15],
                normal: [0.0, 0.0, 1.0],
                color,
            },
            // Back face
            Vertex {
                position: [-0.2, 0.8, -0.15],
                normal: [0.0, 0.0, -1.0],
                color,
            },
            Vertex {
                position: [-0.2, 1.4, -0.15],
                normal: [0.0, 0.0, -1.0],
                color,
            },
            Vertex {
                position: [0.2, 1.4, -0.15],
                normal: [0.0, 0.0, -1.0],
                color,
            },
            Vertex {
                position: [0.2, 0.8, -0.15],
                normal: [0.0, 0.0, -1.0],
                color,
            },
            // Top face
            Vertex {
                position: [-0.2, 1.4, -0.15],
                normal: [0.0, 1.0, 0.0],
                color,
            },
            Vertex {
                position: [-0.2, 1.4, 0.15],
                normal: [0.0, 1.0, 0.0],
                color,
            },
            Vertex {
                position: [0.2, 1.4, 0.15],
                normal: [0.0, 1.0, 0.0],
                color,
            },
            Vertex {
                position: [0.2, 1.4, -0.15],
                normal: [0.0, 1.0, 0.0],
                color,
            },
            // Bottom face
            Vertex {
                position: [-0.2, 0.8, -0.15],
                normal: [0.0, -1.0, 0.0],
                color,
            },
            Vertex {
                position: [0.2, 0.8, -0.15],
                normal: [0.0, -1.0, 0.0],
                color,
            },
            Vertex {
                position: [0.2, 0.8, 0.15],
                normal: [0.0, -1.0, 0.0],
                color,
            },
            Vertex {
                position: [-0.2, 0.8, 0.15],
                normal: [0.0, -1.0, 0.0],
                color,
            },
            // Right face
            Vertex {
                position: [0.2, 0.8, -0.15],
                normal: [1.0, 0.0, 0.0],
                color,
            },
            Vertex {
                position: [0.2, 1.4, -0.15],
                normal: [1.0, 0.0, 0.0],
                color,
            },
            Vertex {
                position: [0.2, 1.4, 0.15],
                normal: [1.0, 0.0, 0.0],
                color,
            },
            Vertex {
                position: [0.2, 0.8, 0.15],
                normal: [1.0, 0.0, 0.0],
                color,
            },
            // Left face
            Vertex {
                position: [-0.2, 0.8, -0.15],
                normal: [-1.0, 0.0, 0.0],
                color,
            },
            Vertex {
                position: [-0.2, 0.8, 0.15],
                normal: [-1.0, 0.0, 0.0],
                color,
            },
            Vertex {
                position: [-0.2, 1.4, 0.15],
                normal: [-1.0, 0.0, 0.0],
                color,
            },
            Vertex {
                position: [-0.2, 1.4, -0.15],
                normal: [-1.0, 0.0, 0.0],
                color,
            },
        ];

        let indices: Vec<u32> = vec![
            0, 1, 2, 0, 2, 3, // Front
            4, 5, 6, 4, 6, 7, // Back
            8, 9, 10, 8, 10, 11, // Top
            12, 13, 14, 12, 14, 15, // Bottom
            16, 17, 18, 16, 18, 19, // Right
            20, 21, 22, 20, 22, 23, // Left
        ];

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Avatar Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Avatar Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        Mesh {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
        }
    }

    /// Rebuild meshes (called after resize or scene change)
    fn rebuild_meshes(&mut self) {
        // Meshes will be rebuilt lazily during render
        self.terrain_mesh = None;
        self.road_mesh = None;
        self.sky_mesh = None;
        self.avatar_mesh = None;
        self.landmark_markers.clear();
    }

    /// T093: Create a landmark marker mesh at the given position
    ///
    /// Creates a simple vertical pin/marker for landmarks visible in the 3D world.
    fn create_landmark_marker(&self, position: Vec3, color: [f32; 3]) -> Mesh {
        // Simple vertical pin marker
        let height = 3.0_f32;
        let radius = 0.5_f32;

        // Create a simple octagonal prism as the marker
        let segments = 8;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Create vertices for the base and top
        for i in 0..segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;

            // Base vertex
            vertices.push(Vertex {
                position: [position.x + x, position.y, position.z + z],
                normal: [x, 0.0, z],
                color,
            });

            // Top vertex (pointed)
            let top_radius = radius * 0.3;
            let top_x = angle.cos() * top_radius;
            let top_z = angle.sin() * top_radius;
            vertices.push(Vertex {
                position: [position.x + top_x, position.y + height, position.z + top_z],
                normal: [x, 0.5, z],
                color,
            });
        }

        // Create indices for the sides
        for i in 0..segments {
            let base = (i * 2) as u32;
            let next_base = ((i + 1) % segments * 2) as u32;

            // Two triangles per side
            indices.push(base);
            indices.push(base + 1);
            indices.push(next_base);

            indices.push(next_base);
            indices.push(base + 1);
            indices.push(next_base + 1);
        }

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Landmark Marker Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Landmark Marker Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        Mesh {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
        }
    }

    /// T093: Update landmark markers for visible landmarks
    ///
    /// Call this with the list of visible landmark positions and their colors.
    pub fn update_landmark_markers(&mut self, landmarks: &[(Vec3, [f32; 3])]) {
        self.landmark_markers.clear();
        for (position, color) in landmarks {
            let marker = self.create_landmark_marker(*position, *color);
            self.landmark_markers.push(marker);
        }
    }

    /// Update scene from weather state (T052: Integrate weather controller with renderer)
    ///
    /// This method updates the scene's lighting, sky colors, and fog settings
    /// based on the current weather state.
    pub fn apply_weather_to_scene(&self, scene: &mut Scene, weather: &WeatherState) {
        // Get sky colors for current conditions
        let sky_colors = SkyColors::for_conditions(weather.time_hours, weather.weather);

        // Update sky colors
        scene.sky.top_color = sky_colors.zenith;
        scene.sky.horizon_color = sky_colors.horizon;

        // Update sun direction and lighting
        let sun_dir = sun_position(weather.time_hours);
        scene.lighting.sun_direction = sun_dir;
        scene.lighting.sun_color = sky_colors.sun;

        // Update ambient lighting based on time and weather
        let (ambient_color, ambient_intensity) = ambient_light(weather.time_hours, weather.weather);
        scene.lighting.ambient_color = ambient_color * ambient_intensity;
    }

    /// Get fog color and density for current weather
    pub fn get_fog_parameters(&self, weather: &WeatherState) -> (Vec3, f32) {
        let sky_colors = SkyColors::for_conditions(weather.time_hours, weather.weather);
        let fog_color = sky_colors.fog;

        // Calculate fog density based on visibility
        // Lower visibility = higher fog density
        let max_visibility = 10000.0;
        let fog_density = 1.0 - (weather.visibility_meters / max_visibility).clamp(0.0, 1.0);

        (fog_color, fog_density)
    }

    /// Get clear color for render pass based on weather
    pub fn get_clear_color(&self, weather: &WeatherState) -> wgpu::Color {
        let sky_colors = SkyColors::for_conditions(weather.time_hours, weather.weather);

        wgpu::Color {
            r: sky_colors.horizon.x as f64,
            g: sky_colors.horizon.y as f64,
            b: sky_colors.horizon.z as f64,
            a: 1.0,
        }
    }

    /// Render the scene to the output texture
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &Scene,
        camera: &Camera,
        terrain: &Terrain,
        road: &Road,
        route_waypoints: &[Vec3],
        _avatar_position: Vec3,
        _avatar_rotation: f32,
        avatar_color: [f32; 3],
    ) {
        // Update uniforms
        let aspect_ratio = self.width as f32 / self.height.max(1) as f32;
        self.uniforms.update(camera, &scene.lighting, aspect_ratio);
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );

        // Build meshes if needed
        if self.terrain_mesh.is_none() {
            self.terrain_mesh = Some(self.build_terrain_mesh(terrain));
        }
        if self.road_mesh.is_none() {
            self.road_mesh = Some(self.build_road_mesh(road, route_waypoints));
        }
        if self.sky_mesh.is_none() {
            self.sky_mesh = Some(self.build_sky_mesh(&scene.sky));
        }
        if self.avatar_mesh.is_none() {
            self.avatar_mesh = Some(self.build_avatar_mesh(avatar_color));
        }

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("World Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.5,
                            g: 0.7,
                            b: 0.9,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            // Draw sky (rendered first, no depth write would be ideal but keeping simple)
            if let Some(ref mesh) = self.sky_mesh {
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }

            // Draw terrain
            if let Some(ref mesh) = self.terrain_mesh {
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }

            // Draw road
            if let Some(ref mesh) = self.road_mesh {
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }

            // Draw avatar
            if let Some(ref mesh) = self.avatar_mesh {
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }

            // T093: Draw landmark markers
            for marker in &self.landmark_markers {
                render_pass.set_vertex_buffer(0, marker.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(marker.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..marker.num_indices, 0, 0..1);
            }
        }

        // Submit command buffer
        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
