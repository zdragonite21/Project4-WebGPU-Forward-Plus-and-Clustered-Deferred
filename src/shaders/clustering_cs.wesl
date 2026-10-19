import package::lib::constants::{
    bind_group_scene,
    bind_group_clustering,
    cluster_wg_size,
    cluster_xyz,
    light_radius,
    max_lights_per_cluster
};
import package::lib::core::{Aabb, CameraUniforms, Cluster, ClusterLights, LightSet, cluster_id_to_idx};

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

fn ndc_to_view(ndc: vec3f) -> vec3f {
    var h = camera.inv_proj * vec4f(ndc, 1);
    return h.xyz / h.w;
}

fn set_bounds(bounds: Aabb, ndc: vec3f) -> Aabb {
    let p = ndc_to_view(ndc);
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

    let zmn = f32(gid.z) / f32(cluster_xyz.z);
    let zmx = f32(gid.z + 1) / f32(cluster_xyz.z);

    let p = ndc_to_view(vec3f(smn.xy, zmn));
    var bounds: Aabb = Aabb(p, p);
    bounds = set_bounds(bounds, vec3f(smx.xy, zmn));
    bounds = set_bounds(bounds, vec3f(smn.x, smx.y, zmn));
    bounds = set_bounds(bounds, vec3f(smx.x, smn.y, zmn));

    bounds = set_bounds(bounds, vec3f(smn.xy, zmx));
    bounds = set_bounds(bounds, vec3f(smx.xy, zmx));
    bounds = set_bounds(bounds, vec3f(smn.x, smx.y, zmx));
    bounds = set_bounds(bounds, vec3f(smx.x, smn.y, zmx));

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
