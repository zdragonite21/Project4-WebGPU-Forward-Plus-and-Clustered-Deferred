use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::{render::world::RenderWorld, shader_constants::{BIND_GROUP_SCENE, MOVE_LIGHTS_WORKGROUP_SIZE}};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct Light {
    pos: [f32; 3],
    _pad0: f32,
    color: [f32; 3],
    _pad1: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightsHeader {
    num_lights: u32,
    _pad: [u32; 3],
}

pub struct GpuLights {
    pub num_lights: u32,
    pub buffer: wgpu::Buffer,
}

fn hue_to_rgb(h: f32) -> Vec3 {
    let f = |n: f32| {
        let k = (n + h * 6.0) % 6.0;
        1.0 - k.min(4.0 - k).min(1.0).max(0.0)
    };
    Vec3::ONE.lerp(Vec3::new(f(5.0), f(3.0), f(1.0)), 0.8)
}

impl GpuLights {
    pub const NUM_LIGHTS: u32 = 5000;
    pub const MAX_NUM_LIGHTS: u32 = 5000;
    pub const LIGHT_INTENSITY: f32 = 0.1;

    pub fn new(device: &wgpu::Device) -> Self {
        let mut lights_array = vec![Light::default(); Self::MAX_NUM_LIGHTS as usize];

        for light_idx in 0..(Self::MAX_NUM_LIGHTS as usize) {
            lights_array[light_idx].color =
                (hue_to_rgb(rand::random()) * Self::LIGHT_INTENSITY).into();
        }

        let light_set = LightsHeader {
            num_lights: Self::NUM_LIGHTS,
            _pad: [0; 3],
        };

        let mut contents = bytemuck::bytes_of(&light_set).to_vec();
        contents.extend_from_slice(bytemuck::cast_slice(&lights_array));

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lights Buffer"),
            contents: &contents,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            buffer,
            num_lights: Self::NUM_LIGHTS,
        }
    }
}


pub struct LightsComputePass {
    pipeline: wgpu::ComputePipeline,
}

impl LightsComputePass {
    pub fn new(device: &wgpu::Device, world: &RenderWorld) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("move lights"),
            source: wgpu::ShaderSource::Wgsl(wesl_core::include_wesl!("move_lights_cs").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("move lights layout"),
            bind_group_layouts: &[Some(&world.scene_bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("move lights"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        Self { pipeline }
    }

    pub fn dispatch(&mut self, encoder: &mut wgpu::CommandEncoder, world: &RenderWorld) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("move lights dispatch"),
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(BIND_GROUP_SCENE, &world.scene_bind_group, &[]);

        let num_workgroups = world.lights.num_lights.div_ceil(MOVE_LIGHTS_WORKGROUP_SIZE);
        pass.dispatch_workgroups(num_workgroups, 1, 1);
    }
}
