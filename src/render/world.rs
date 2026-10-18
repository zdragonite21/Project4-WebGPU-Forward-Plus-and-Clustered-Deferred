use glam::Mat4;
use wgpu::util::DeviceExt;

use crate::{
    camera::{Camera, CameraRig, Projection},
    world::World,
};

pub struct RenderWorld {
    pub camera: GpuCamera,
    pub transforms: GpuTransforms,
}

impl RenderWorld {
    pub fn new(device: &wgpu::Device, world: &World) -> Self {
        let camera = GpuCamera::new(device, &world.cam_rig);
        let transforms = GpuTransforms::new(device, &world.global_transforms);
        Self { camera, transforms }
    }

    pub fn upload(&mut self, queue: &wgpu::Queue, world: &World) {
        self.camera.upload(queue, &world.cam_rig);
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
}

pub struct GpuCamera {
    buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl GpuCamera {
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
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    fn upload(&mut self, queue: &wgpu::Queue, cam_rig: &CameraRig) {
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(&[CameraUniform::new(&cam_rig.camera, &cam_rig.projection)]),
        );
    }
}

pub struct GpuTransforms {
    _buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl GpuTransforms {
    fn new(device: &wgpu::Device, transforms: &Vec<Mat4>) -> Self {
        let _buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transforms Buffer"),
            contents: bytemuck::cast_slice(&transforms),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("transforms_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: _buffer.as_entire_binding(),
            }],
            label: Some("transforms_bind_group"),
        });

        Self {
            _buffer,
            bind_group_layout,
            bind_group,
        }
    }
}
