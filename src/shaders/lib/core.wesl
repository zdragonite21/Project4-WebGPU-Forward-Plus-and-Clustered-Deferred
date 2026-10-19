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
    near: f32,
    far: f32,
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

fn ndc_to_cluster_idx(ndc: vec2f, slice_z: f32) -> u32 {
    let coords = ndc_to_coords(ndc);
    let cluster_id = vec3f(coords, slice_z);
    let c = min(vec3u(max(floor(cluster_id), vec3f(0))), cluster_xyz - 1);

    return cluster_id_to_idx(c);
}

fn ndc_to_coords(ndc: vec2f) -> vec2f {
    var _ndc: vec2f = ndc;
    _ndc.x = 0.5 * _ndc.x + 0.5;
    _ndc.y = 0.5 * _ndc.y + 0.5;

    return _ndc * vec2f(cluster_xyz.xy);
}

fn cluster_id_to_idx(c: vec3u) -> u32 {
    return c.z * cluster_xyz.x * cluster_xyz.y + c.y * cluster_xyz.x + c.x;
}

fn slice_to_depth(k: f32, n: f32, f: f32, Z: f32) -> f32 {
    return n * pow(f / n, k / Z);
}

fn depth_to_slice(d: f32, n: f32, f: f32, Z: f32) -> f32 {
    return 	log(d / n) / log(f / n) * Z;
}
