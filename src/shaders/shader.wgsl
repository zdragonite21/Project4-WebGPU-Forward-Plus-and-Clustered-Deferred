struct CameraUniform {
    view_proj: mat4x4<f32>,
}

struct VertexInput {
    @location(0) pos: vec3<f32>,
    @location(1) nor: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<storage, read> transforms: array<mat4x4<f32>>;

@group(2) @binding(0)
var t_texture: texture_2d<f32>;
@group(2) @binding(1)
var t_sampler: sampler;

@vertex
fn vs_main(@builtin(instance_index) idx: u32, vert_in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * transforms[idx] * vec4<f32>(vert_in.pos, 1.0);
    out.uv = vert_in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_texture, t_sampler, in.uv);
    return color;
}
