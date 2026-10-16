use crate::{
    camera::{CameraRig, CameraSettings},
    input::{CursorState, Input},
};
use std::time::Duration;

// cpu side state that is updated once per frame
pub struct World {
    pub cam_rig: CameraRig,
}

impl World {
    pub fn new(width: u32, height: u32) -> Self {
        let cam_rig = CameraRig::new(CameraSettings::default(), width, height);

        Self { cam_rig }
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
