use std::time::Duration;
use crate::{
    camera::{CameraRig, CameraSettings}, input::{CursorState, Input}, scene::Scene,
};
// updated once per frame
pub struct World {
    pub cam_rig: CameraRig,
    pub scene: Scene,
}

impl World {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let cam_rig = CameraRig::new(&device, CameraSettings::default(), width, height);
        let scene = Scene::new(&device);

        Self { cam_rig, scene }
    }

    pub fn update(&mut self, queue: &wgpu::Queue, input: &Input, cursor: &mut CursorState, dt: Duration) {
        self.cam_rig.update(queue, input, cursor, dt);
    }
}
