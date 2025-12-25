// World rendering shader
// Basic vertex-color shading with directional lighting

struct Uniforms {
    view_proj: mat4x4<f32>,
    sun_direction: vec4<f32>,
    sun_color: vec4<f32>,
    ambient_color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(in.position, 1.0);
    out.world_normal = in.normal;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Normalize the interpolated normal
    let normal = normalize(in.world_normal);

    // Calculate diffuse lighting
    let light_dir = normalize(uniforms.sun_direction.xyz);
    let ndotl = max(dot(normal, light_dir), 0.0);

    // Combine ambient and diffuse
    let ambient = uniforms.ambient_color.rgb;
    let diffuse = uniforms.sun_color.rgb * ndotl;

    // Final color
    let lighting = ambient + diffuse;
    let final_color = in.color * lighting;

    return vec4<f32>(final_color, 1.0);
}
