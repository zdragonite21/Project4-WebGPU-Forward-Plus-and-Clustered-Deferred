use glam::Mat4;
use std::time::Duration;
use crate::{
    assets::{Handle, Material, Mesh}, camera::{CameraRig, CameraSettings}, input::{CursorState, Input},
};

// cpu side state that is updated once per frame
pub struct World {
    pub cam_rig: CameraRig,
    pub instances: Vec<MeshInstance>,
    pub transforms: Vec<Mat4>,
}

impl World {
    pub fn new(width: u32, height: u32) -> Self {
        let cam_rig = CameraRig::new(CameraSettings::default(), width, height);

        Self {
            cam_rig,
            instances: Vec::new(),
            transforms: Vec::new(),
        }
    }

    pub fn update(
        &mut self,
        input: &Input,
        cursor: &mut CursorState,
        dt: Duration,
    ) {
        self.cam_rig.update(input, cursor, dt);
    }
}

pub struct MeshInstance {
    pub mesh: Handle<Mesh>,
    pub material: Handle<Material>,
    pub transform: u32,
}
