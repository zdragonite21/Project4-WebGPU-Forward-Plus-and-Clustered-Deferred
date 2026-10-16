use glam::Mat4;
use wgpu::util::DeviceExt;

use crate::camera::{Camera, CameraRig, Projection};

pub struct RenderResources {
    pub camera: CameraGpu,
}

impl RenderResources {
    pub fn new(device: &wgpu::Device, cam_rig: &CameraRig) -> Self {
        let camera = CameraGpu::new(device, cam_rig);
        Self { camera }
    }

    pub fn upload(&mut self, queue: &wgpu::Queue, cam_rig: &CameraRig) {
        self.camera.upload(queue, cam_rig);
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

pub struct CameraGpu {
    uniform: CameraUniform,
    buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl CameraGpu {
    fn new(device: &wgpu::Device, cam_rig: &CameraRig) -> Self {
        let uniform = CameraUniform::new(&cam_rig.camera, &cam_rig.projection);

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
            uniform,
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    fn upload(&mut self, queue: &wgpu::Queue, cam_rig: &CameraRig) {
        self.uniform
            .update_view_proj(&cam_rig.camera, &cam_rig.projection);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }
}
