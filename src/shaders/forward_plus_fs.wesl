import package::lib::constants::{
    bind_group_material,
    bind_group_scene,
    bind_group_transforms,
    bind_group_clustering
};
import package::lib::core::{CameraUniforms, LightSet, ClusterLights, calculate_light_contrib, ndc_to_cluster_idx};

struct FragInput {
    @builtin(position) frag_pos: vec4f,
    @location(0) pos: vec3f,
    @location(1) nor: vec3f,
    @location(2) uv: vec2f,
}

@group(bind_group_scene) @binding(0)
var<uniform> camera: CameraUniforms;
@group(bind_group_scene) @binding(1)
var<storage, read_write> light_set: LightSet;

@group(bind_group_clustering) @binding(0)
var<storage, read_write> cluster_lights: array<ClusterLights>;

@group(bind_group_material) @binding(0)
var diffuse_tex: texture_2d<f32>;
@group(bind_group_material) @binding(1)
var diffuse_tex_sampler: sampler;

@fragment
fn fs_main(in: FragInput) -> @location(0) vec4<f32> {
    let color = textureSample(diffuse_tex, diffuse_tex_sampler, in.uv);
    if color.a < 0.5 {
        discard;
    }

    let clip = camera.proj * camera.view * vec4(in.pos, 1);
    let ndc = clip.xyz / clip.w;

    let cluster_idx = ndc_to_cluster_idx(ndc);
    let cluster = &cluster_lights[cluster_idx];

    var contrib = vec3f(0);
    for(var idx = 0u; idx < cluster.num_lights; idx++) {
        let light = light_set.lights[cluster.lights[idx]];
        contrib += calculate_light_contrib(light, in.pos, in.nor);
    }

    let out_col = color.rgb * contrib;
    return vec4(out_col, 1);
}
