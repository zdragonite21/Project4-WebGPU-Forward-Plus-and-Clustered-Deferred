import package::lib::constants::{
    bind_group_material,
    bind_group_scene,
    bind_group_transforms
};
import package::lib::core::{CameraUniforms};

struct VertexInput {
    @location(0) pos: vec3<f32>,
    @location(1) nor: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) frag_pos: vec4f,
    @location(0) pos: vec3f,
    @location(1) nor: vec3f,
    @location(2) uv: vec2f,
}

@group(bind_group_scene) @binding(0)
var<uniform> camera: CameraUniforms;

@group(bind_group_transforms) @binding(0)
var<storage, read> transforms: array<mat4x4f>;

@vertex
fn vs_main(@builtin(instance_index) trans_idx: u32, in: VertexInput) -> VertexOutput {
    let model_pos = transforms[trans_idx] * vec4<f32>(in.pos, 1.0);

    var out: VertexOutput;
    out.frag_pos = camera.proj * camera.view * model_pos;
    out.pos = model_pos.xyz / model_pos.w;
    out.nor = normalize(in.nor);
    out.uv = in.uv;
    return out;
}
