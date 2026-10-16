use crate::input::{CursorState, Input};
use glam::{Mat4, Quat, Vec3, camera};
use instant::Duration;
use std::f32::consts::FRAC_PI_2;
use wgpu::util::DeviceExt;
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
            scroll_sens: 0.1,
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
            position: Vec3::new(0.0, 0.0, 15.0),
            yaw_rad: 0.0,
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
    znear: f32,
    zfar: f32,
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

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: Mat4,
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
                .clamp(0.0, 300.0);
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

    uniform: CameraUniform,
    buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl CameraRig {
    const MAX_FRAME_DT: Duration = Duration::from_millis(120);

    pub fn new(device: &wgpu::Device, settings: CameraSettings, width: u32, height: u32) -> Self {
        let camera = Camera::default();
        let projection = Projection::new(width, height, 45.0, 0.1, 100.0);
        let controller = CameraController::new(settings);
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

    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        input: &Input,
        cursor: &mut CursorState,
        dt: Duration,
    ) {
        let dt = dt.min(Self::MAX_FRAME_DT);

        self.controller
            .update_camera(&mut self.camera, input, cursor, dt.as_secs_f32());

        self.uniform
            .update_view_proj(&self.camera, &self.projection);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }
}
