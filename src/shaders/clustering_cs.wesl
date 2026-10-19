import package::lib::constants::{
    bind_group_scene,
    cluster_xyz,
    light_radius,
    max_lights_per_cluster,
    move_lights_workgroup_size
};
import package::lib::core::{
    Aabb,
    CameraUniforms,
    Cluster,
    ClusterSet,
    LightSet,
    cluster_id_to_idx,
    cluster_idx_to_view,
    ndc_to_cluster_id,
    world_to_ndc
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

@group(bind_group_model) @binding(0)
var<uniform> camera: CameraUniforms;

@group(bind_group_scene) @binding(0)
var<storage, read_write> light_set: LightSet;
@group(bind_group_scene) @binding(1)
var<storage, read_write> cluster_set: array<ClusterSet>;
@group(bind_group_scene) @binding(2)
var<storage, read_write> clusters: array<Cluster>;

// todo: 3d workgroup
@compute @workgroup_size(move_lights_workgroup_size)
fn cluster_bounds(
    @builtin(global_invocation_id) id: vec3u,
    @builtin(global_invocation_index) idx: u32,
) {
    if idx >= cluster_xyz.x * cluster_xyz.y * cluster_xyz.z {
        return;
    }

    var bounds: Aabb;
    bounds.min = cluster_idx_to_view(id, camera.inv_proj);
    bounds.max = cluster_idx_to_view(id + 1, camera.inv_proj);

    // could use shared memory here

    clusters[idx] = Cluster(bounds);
}

@compute @workgroup_size(move_lights_workgroup_size)
fn assign_lights(@builtin(global_invocation_index) idx: u32) {
    if idx >= light_set.num_lights {
        return;
    }

    let light = light_set.lights[idx];
    let view_proj = camera.proj * camera.view;

    var b: Aabb;
    b.min = light.pos - light_radius;
    b.max = light.pos + light_radius;

    b.min = world_to_ndc(b.min, view_proj);
    b.max = world_to_ndc(b.max, view_proj);

    b.min = ndc_to_cluster_id(b.min);
    b.max = ndc_to_cluster_id(b.max);

    let cmin = vec3u(max(floor(b.min), vec3f(0)));
    let cmax = min(vec3u(ceil(b.max)), cluster_xyz);

    for(var z = cmin.z; z < cmax.z; z++) {
        for(var y = cmin.y; y < cmax.y; y++) {
            for(var x = cmin.x; x < cmax.x; x++) {
                // can do a finer sphere overlap check 
                let cidx = cluster_id_to_idx(vec3u(x, y, z));
                let nlights = cluster_set[cidx].num_lights;

                if nlights >= max_lights_per_cluster {
                    continue;
                }

                cluster_set[cidx].lights[nlights] = idx;
                cluster_set[cidx].num_lights++;
            }
        }
    }
}
