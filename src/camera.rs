use crate::input::{CursorState, Input};
use glam::{Mat4, Quat, UVec2, Vec3, camera};
use instant::Duration;
use std::f32::consts::FRAC_PI_2;
use winit::{event::MouseButton, keyboard::KeyCode};

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Debug, Clone)]
pub struct CameraSettings {
    accel: f32,
    damping: f32,
    mouse_sens: f32,
    scroll_sens: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            accel: 200.0,
            damping: 25.0,
            mouse_sens: 0.002,
            scroll_sens: 0.5,
        }
    }
}

pub struct Camera {
    pub position: Vec3,
    pub yaw_rad: f32,
    pub pitch_rad: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(-7.0, 2.0, 0.0),
            yaw_rad: -FRAC_PI_2,
            pitch_rad: 0.0,
        }
    }
}

impl Camera {
    pub fn orient(&self) -> Quat {
        let yaw = Quat::from_axis_angle(Vec3::Y, self.yaw_rad);
        let pitch = Quat::from_axis_angle(Vec3::X, self.pitch_rad);

        yaw * pitch
    }

    pub fn forward(&self) -> Vec3 {
        self.orient() * Vec3::NEG_Z
    }

    pub fn right(&self) -> Vec3 {
        self.orient() * Vec3::X
    }

    pub fn view_matrix(&self) -> Mat4 {
        camera::rh::view::look_to_mat4(self.position, self.forward(), Vec3::Y)
    }
}

pub struct Projection {
    aspect: f32,
    fovy_rad: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Projection {
    pub fn new(width: u32, height: u32, fovy_deg: f32, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy_rad: fovy_deg.to_radians(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn proj_matrix(&self) -> Mat4 {
        camera::rh::proj::directx::perspective(self.fovy_rad, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Debug, Default)]
pub struct CameraController {
    settings: CameraSettings,
    velocity: Vec3,
}

impl CameraController {
    pub fn new(settings: CameraSettings) -> Self {
        Self {
            velocity: Vec3::ZERO,
            settings,
        }
    }

    fn axis(positive: bool, negative: bool) -> f32 {
        positive as i32 as f32 - negative as i32 as f32
    }

    fn movement_vector(&self, camera: &mut Camera, input: &Input) -> Vec3 {
        let forward = input.key_held(KeyCode::KeyW);
        let backward = input.key_held(KeyCode::KeyS);
        let left = input.key_held(KeyCode::KeyA);
        let right = input.key_held(KeyCode::KeyD);
        let up = input.key_held(KeyCode::Space);
        let down = input.key_held(KeyCode::ShiftLeft);

        let dir = camera.forward() * Self::axis(forward, backward)
            + camera.right() * Self::axis(right, left)
            + Vec3::Y * Self::axis(up, down);

        if dir.length_squared() > 0.0 {
            dir.normalize()
        } else {
            Vec3::ZERO
        }
    }

    pub fn update_camera(
        &mut self,
        camera: &mut Camera,
        input: &Input,
        cursor: &mut CursorState,
        dt: f32,
    ) {
        cursor.want_grab = input.mouse_held(MouseButton::Left);
        if cursor.grabbed {
            self.settings.accel = (self.settings.accel
                - input.mouse_scroll() * self.settings.scroll_sens)
                .clamp(0.0, 500.0);
            let dir = self.movement_vector(camera, input);
            self.velocity += dir * self.settings.accel * dt;
            camera.yaw_rad += -input.mouse_delta().x * self.settings.mouse_sens;
            camera.pitch_rad += -input.mouse_delta().y * self.settings.mouse_sens;
            camera.pitch_rad = camera.pitch_rad.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);
        }

        self.velocity *= (-self.settings.damping * dt).exp();
        camera.position += self.velocity * dt;
    }
}

pub struct CameraRig {
    pub camera: Camera,
    pub projection: Projection,
    pub controller: CameraController,
}

impl CameraRig {
    const MAX_FRAME_DT: Duration = Duration::from_millis(120);

    pub fn new(settings: CameraSettings, width: u32, height: u32) -> Self {
        let camera = Camera::default();
        let projection = Projection::new(width, height, 45.0, 0.1, 100.0);
        let controller = CameraController::new(settings);

        Self {
            camera,
            projection,
            controller,
        }
    }

    pub fn resize(&mut self, size: UVec2) {
        self.projection.resize(size.x, size.y);
    }

    pub fn update(&mut self, input: &Input, cursor: &mut CursorState, dt: Duration) {
        let dt = dt.min(Self::MAX_FRAME_DT);

        self.controller
            .update_camera(&mut self.camera, input, cursor, dt.as_secs_f32());
    }
}
