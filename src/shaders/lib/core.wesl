import package::lib::constants::{cluster_xyz, light_radius, max_lights_per_cluster};

struct Light {
    pos: vec3f,
    color: vec3f,
}

struct LightSet {
    num_lights: u32,
    lights: array<Light>,
}

// TODO-2: you may want to create a ClusterSet struct similar to LightSet
struct Aabb {
    min: vec3f,
    max: vec3f,
}

struct Cluster {
    bounds: Aabb,
}

struct ClusterLights {
    num_lights: u32,
    lights: array<u32, max_lights_per_cluster>,
}

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
}

// CHECKITOUT: this special attenuation function ensures lights don't affect geometry outside the maximum light radius
fn range_attenuation(distance: f32) -> f32 {
    return clamp(1.f - pow(distance / light_radius, 4.f), 0.f, 1.f) / (distance * distance);
}

fn calculate_light_contrib(light: Light, posWorld: vec3f, nor: vec3f) -> vec3f {
    let vecToLight = light.pos - posWorld;
    let distToLight = length(vecToLight);

    let lambert = max(dot(nor, normalize(vecToLight)), 0.f);
    return light.color * lambert * range_attenuation(distToLight);
}

fn ndc_to_cluster_idx(ndc: vec3f) -> u32 {
    let cluster_id = ndc_to_cluster_id(ndc);
    let c = min(vec3u(max(floor(cluster_id), vec3f(0))), cluster_xyz - 1);

    return cluster_id_to_idx(c);
}

fn ndc_to_cluster_id(ndc: vec3f) -> vec3f {
    var ndc = ndc;
    ndc.xy = 0.5 * ndc.xy + 0.5;

    return ndc * vec3f(cluster_xyz);
}

fn cluster_id_to_idx(c: vec3u) -> u32 {
    return c.z * cluster_xyz.x * cluster_xyz.y + c.y * cluster_xyz.x + c.x;
}

// todo: add logrithmic depth slicing in view space
