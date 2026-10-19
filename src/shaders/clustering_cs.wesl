import package::lib::constants::{
    bind_group_clustering,
    bind_group_scene,
    cluster_wg_size,
    cluster_xyz,
    light_radius,
    max_lights_per_cluster
};
import package::lib::core::{
    Aabb,
    CameraUniforms,
    Cluster,
    ClusterLights,
    LightSet,
    slice_to_depth,
    cluster_id_to_idx
};

// ------------------------------------
// Calculating cluster bounds:
// ------------------------------------
// For each cluster (X, Y, Z):
//     - Calculate the screen-space bounds for this cluster in 2D (XY).
//     - Calculate the depth bounds for this cluster in Z (near and far planes).
//     - Convert these screen and depth bounds into view-space coordinates.
//     - Store the computed bounding box (AABB) for the cluster.

// ------------------------------------
// Assigning lights to clusters:
// ------------------------------------
// For each cluster:
//     - Initialize a counter for the number of lights in this cluster.

//     For each light:
//         - Check if the light intersects with the cluster’s bounding box (AABB).
//         - If it does, add the light to the cluster's light list.
//         - Stop adding lights if the maximum number of lights is reached.

//     - Store the number of lights assigned to this cluster.

@group(bind_group_scene) @binding(0)
var<uniform> camera: CameraUniforms;
@group(bind_group_scene) @binding(1)
var<storage, read_write> light_set: LightSet;

@group(bind_group_clustering) @binding(0)
var<storage, read_write> cluster_lights: array<ClusterLights>;
@group(bind_group_clustering) @binding(1)
var<storage, read_write> clusters: array<Cluster>;

fn ndc_to_view(ndc: vec2f, d: f32) -> vec3f {
    var h = camera.inv_proj * vec4f(ndc, 1, 1);
    var p = h.xyz / h.w;
    p *= d / -p.z;
    return p;
}

fn set_bounds(bounds: Aabb, ndc: vec2f, z: f32) -> Aabb {
    var p = ndc_to_view(ndc, z);
    return Aabb(min(bounds.min, p), max(bounds.max, p));
}



@compute @workgroup_size(cluster_wg_size.x, cluster_wg_size.y, cluster_wg_size.z)
fn cluster_bounds(@builtin(global_invocation_id) gid: vec3u) {
    if any(gid >= cluster_xyz) {
        return;
    }
    let idx = cluster_id_to_idx(gid);

    let smn = vec2f(gid.xy) / vec2f(cluster_xyz.xy) * 2 - 1;
    let smx = vec2f(gid.xy + 1) / vec2f(cluster_xyz.xy) * 2 - 1;

    let d0 = slice_to_depth(f32(gid.z), camera.near, camera.far, f32(cluster_xyz.z));
    let d1 = slice_to_depth(f32(gid.z + 1), camera.near, camera.far, f32(cluster_xyz.z));

    var p = ndc_to_view(vec2f(smn.xy), d0);
    var bounds: Aabb = Aabb(p, p);
    bounds = set_bounds(bounds, vec2f(smx.xy), d0);
    bounds = set_bounds(bounds, vec2f(smn.x, smx.y), d0);
    bounds = set_bounds(bounds, vec2f(smx.x, smn.y), d0);

    bounds = set_bounds(bounds, vec2f(smn.xy), d1);
    bounds = set_bounds(bounds, vec2f(smx.xy), d1);
    bounds = set_bounds(bounds, vec2f(smn.x, smx.y), d1);
    bounds = set_bounds(bounds, vec2f(smx.x, smn.y), d1);

    clusters[idx] = Cluster(bounds);
}

@compute @workgroup_size(cluster_wg_size.x, cluster_wg_size.y, cluster_wg_size.z)
fn assign_lights(@builtin(global_invocation_id) gid: vec3u) {
    if any(gid >= cluster_xyz) {
        return;
    }
    let idx = cluster_id_to_idx(gid);

    let cluster = &cluster_lights[idx];
    var nlights = 0u;
    let bounds = clusters[idx].bounds;

    for(var i = 0u; i < light_set.num_lights; i++) {
        if nlights >= max_lights_per_cluster {
            break;
        }

        let light = light_set.lights[i];
        let c = camera.view * vec4f(light.pos, 1);
        let closest = clamp(c.xyz, bounds.min, bounds.max);
        let dist = closest - c.xyz;

        if dot(dist, dist) <= light_radius * light_radius {
            cluster.lights[nlights] = i;
            nlights++;
        }
    }
    cluster.num_lights = nlights;
}
