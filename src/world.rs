use glam::Mat4;
use std::time::Duration;
use crate::{
    assets::{Handle, material::GpuMaterial, mesh::GpuSubMesh}, camera::{CameraRig, CameraSettings}, frame_clock::FrameClock, input::{CursorState, Input},
};

// cpu side state that is updated once per frame
pub struct World {
    pub cam_rig: CameraRig,
    pub instances: Vec<MeshInstance>,
    pub global_transforms: Vec<Mat4>,
    pub time: f32,
}

impl World {
    pub fn new(width: u32, height: u32) -> Self {
        let cam_rig = CameraRig::new(CameraSettings::default(), width, height);

        Self {
            cam_rig,
            instances: Vec::new(),
            global_transforms: Vec::new(),
            time: 0.0,
        }
    }

    pub fn update(
        &mut self,
        input: &Input,
        cursor: &mut CursorState,
        clock: &FrameClock,
    ) {
        self.cam_rig.update(input, cursor, clock.dt);
        self.time = clock.elapsed.as_secs_f32();
    }
}

pub struct MeshInstance {
    pub mesh: Handle<GpuSubMesh>,
    pub material: Handle<GpuMaterial>,
    pub transform: u32,
}
