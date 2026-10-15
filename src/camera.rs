use glam::{Mat4, Quat, Vec3, camera};
use instant::Duration;
use std::f32::consts::FRAC_PI_2;
use wgpu::util::DeviceExt;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseScrollDelta},
    keyboard::KeyCode,
};

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;
pub struct CameraRig {
    pub camera: Camera,
    pub projection: Projection,
    pub controller: CameraController,

    uniform: CameraUniform,
    buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

pub struct Camera {
    pub position: Vec3,
    pub yaw_rad: f32,
    pub pitch_rad: f32,
    pub velocity: Vec3,
    pub settings: CameraSettings,
}

pub struct Projection {
    aspect: f32,
    fovy_rad: f32,
    znear: f32,
    zfar: f32,
}

#[derive(Debug, Clone)]
pub struct CameraSettings {
    accel: f32,
    damping: f32,
    mouse_sens: f32,
    scroll_sens: f32,
}

#[derive(Debug, Default)]
pub struct CameraController {
    input: CameraInput,
    captured: bool,
    rot_hor: f32,
    rot_vert: f32,
    scroll: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: Mat4,
}

#[derive(Default, Debug)]
pub struct CameraInput {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 15.0),
            yaw_rad: 0.0,
            pitch_rad: 0.0,
            velocity: Vec3::ZERO,
            settings: CameraSettings::default(),
        }
    }
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            accel: 200.0,
            damping: 25.0,
            mouse_sens: 0.0025,
            scroll_sens: 0.1,
        }
    }
}

impl Camera {
    pub fn new(settings: &CameraSettings) -> Self {
        Self {
            velocity: Vec3::ZERO,
            settings: settings.clone(),
            ..Self::default()
        }
    }

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

impl CameraUniform {
    fn new(camera: &Camera, projection: &Projection) -> Self {
        Self {
            view_proj: projection.proj_matrix() * camera.view_matrix(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_proj = projection.proj_matrix() * camera.view_matrix();
    }
}

impl CameraInput {
    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let pressed = state == ElementState::Pressed;
        match key {
            KeyCode::KeyW => self.forward = pressed,
            KeyCode::KeyS => self.backward = pressed,
            KeyCode::KeyA => self.left = pressed,
            KeyCode::KeyD => self.right = pressed,
            KeyCode::Space => self.up = pressed,
            KeyCode::ShiftLeft => self.down = pressed,
            _ => return false,
        }
        true
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    fn axis(positive: bool, negative: bool) -> f32 {
        positive as i32 as f32 - negative as i32 as f32
    }

    pub fn movement_vector(&self, camera: &Camera) -> Vec3 {
        let dir = camera.forward() * Self::axis(self.forward, self.backward)
            + camera.right() * Self::axis(self.right, self.left)
            + Vec3::Y * Self::axis(self.up, self.down);

        if dir.length_squared() > 0.0 {
            dir.normalize()
        } else {
            Vec3::ZERO
        }
    }
}

impl CameraController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_captured(&mut self, captured: bool) {
        self.captured = captured;

        if !captured {
            self.input.clear();
        }
    }

    pub fn is_captured(&self) -> bool {
        self.captured
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        if self.is_captured() {
            self.input.process_keyboard(key, state)
        } else {
            false
        }
    }

    pub fn handle_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        if self.is_captured() {
            self.rot_hor += mouse_dx as f32;
            self.rot_vert += mouse_dy as f32;
        }
    }

    pub fn handle_mouse_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        let dir;
        if self.is_captured() {
            camera.settings.accel = (camera.settings.accel
                - self.scroll * camera.settings.scroll_sens * dt)
                .clamp(0.0, 300.0);
            dir = self.input.movement_vector(camera);
        } else {
            camera.position += camera.forward() * -self.scroll * camera.settings.scroll_sens * dt;
            dir = Vec3::ZERO;
        }

        camera.velocity += dir * camera.settings.accel * dt;
        camera.velocity *= (-camera.settings.damping * dt).exp();
        camera.position += camera.velocity * dt;

        camera.yaw_rad += -self.rot_hor * camera.settings.mouse_sens;
        camera.pitch_rad += -self.rot_vert * camera.settings.mouse_sens;
        camera.pitch_rad = camera.pitch_rad.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);

        self.reset_frame_input();
    }

    fn reset_frame_input(&mut self) {
        self.rot_hor = 0.0;
        self.rot_vert = 0.0;
        self.scroll = 0.0;
    }
}

impl CameraRig {
    const MAX_FRAME_DT: Duration = Duration::from_millis(120);

    pub fn new(device: &wgpu::Device, settings: &CameraSettings, width: u32, height: u32) -> Self {
        let camera = Camera::new(settings);
        let projection = Projection::new(width, height, 45.0, 0.1, 100.0);
        let controller = CameraController::new();
        let uniform = CameraUniform::new(&camera, &projection);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        Self {
            camera,
            projection,
            controller,
            uniform,
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.projection.resize(width, height);
    }

    pub fn update(&mut self, queue: &wgpu::Queue, dt: Duration) {
        let dt = dt.min(Self::MAX_FRAME_DT);

        self.controller.update_camera(&mut self.camera, dt);

        self.uniform
            .update_view_proj(&self.camera, &self.projection);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }
}
