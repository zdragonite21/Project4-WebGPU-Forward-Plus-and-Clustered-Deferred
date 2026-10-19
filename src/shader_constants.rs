pub const BIND_GROUP_SCENE: u32 = 0;
pub const BIND_GROUP_MODEL: u32 = 1;
pub const BIND_GROUP_TRANSFORMS: u32 = 2;
pub const BIND_GROUP_MATERIAL: u32 = 3;

pub const MOVE_LIGHTS_WORKGROUP_SIZE: u32 = 128;

pub const LIGHT_RADIUS: f32 = 2.0;

pub const CLUSTER_XYZ: (u32, u32, u32) = (16, 9, 20);
pub const CLUSTER_WG_SIZE: (u32, u32, u32) = (8, 8, 8);
pub const MAX_LIGHTS_PER_CLUSTER: u32 = 100;
