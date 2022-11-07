// Vertex shader

struct Camera {
    view_pos: vec4<f32>,
    view_front: vec4<f32>,
    view_proj: mat4x4<f32>,
}

struct Node {
    model_mat: mat4x4<f32>,
    normal_mat: mat4x4<f32>,
}

struct Material {
    base_color_factor: vec4<f32>,
}

@group(1) @binding(0)
var<uniform> camera: Camera;

@group(2) @binding(0)
var<uniform> node_uniform: Node;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) ws_position: vec3<f32>,
    @location(2) ws_normal: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.ws_position = (node_uniform.model_mat * vec4<f32>(model.position, 1.0)).xyz;
    // TODO: normal matrix
    out.ws_normal = normalize((node_uniform.normal_mat * vec4<f32>(model.normal, 0.0)).xyz);
    out.clip_position = camera.view_proj * vec4(out.ws_position, 1.0);
    out.tex_coords = model.tex_coords;
    return out;
}

// Fragment shader
@group(0) @binding(0)
var<uniform> material: Material;
@group(0) @binding(1)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(2)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let brightness = 0.5 + 0.5 * dot(in.ws_normal.xyz, -camera.view_front.xyz);
    let sampled = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let color = brightness * sampled.rgb * material.base_color_factor.rgb;
    let alpha = sampled.a * material.base_color_factor.a;
    return vec4<f32>(color, alpha);
}
