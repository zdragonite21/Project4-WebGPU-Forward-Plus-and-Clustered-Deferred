import package::lib::constants::{bind_group_scene, bind_group_model, bind_group_material};
import package::lib::core::{CameraUniforms, LightSet, calculate_light_contrib};

struct VertexInput {
    @location(0) pos: vec3<f32>,
    @location(1) nor: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct FragInput {
    @builtin(position) frag_pos: vec4f,
    @location(0) pos: vec3f,
    @location(1) nor: vec3f,
    @location(2) uv: vec2f
}

@group(bind_group_scene) @binding(0)
var<uniform> camera: CameraUniforms;

@group(bind_group_model) @binding(0)
var<storage, read> transforms: array<mat4x4<f32>>;

@group(bind_group_material) @binding(0)
var diffuse_tex: texture_2d<f32>;
@group(bind_group_material) @binding(1)
var diffuse_tex_sampler: sampler;

@group(bind_group_scene) @binding(0) var<storage, read_write> light_set: LightSet;

@vertex
fn vs_main(@builtin(instance_index) trans_idx: u32, in: VertexInput) -> FragInput {
    let model_pos = transforms[trans_idx] * vec4<f32>(in.pos, 1.0);

    var out: FragInput;
    out.frag_pos = camera.view_proj * model_pos;
    out.pos = model_pos.xyz / model_pos.w;
    out.nor = in.nor;
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: FragInput) -> @location(0) vec4<f32> {
    let color = textureSample(diffuse_tex, diffuse_tex_sampler, in.uv);
    if color.a < 0.5 {
        discard;
    }

    var contrib = vec3f(0);
    for (var idx = 0u; idx < light_set.numLights; idx++) {
        let light = light_set.lights[idx];
        contrib += calculate_light_contrib(light, in.pos, in.nor);
    }

    let out_col = color.rgb * contrib;
    return vec4(out_col, 1);
}
