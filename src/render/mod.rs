use glam::UVec2;

use crate::{
    render::{assets::RenderAssets, lights::GpuLights, world::{GpuGlobals, RenderWorld}}, world::World,
};

pub mod assets;
pub mod lights;
pub mod naive;
pub mod world;

pub trait Renderer {
    fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        render_world: &RenderWorld,
        world: &World,
        render_assets: &RenderAssets,
    ) -> anyhow::Result<()>;

    fn resize(&mut self, device: &wgpu::Device, size: UVec2);
}

pub struct LightsComputePass {
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl LightsComputePass {
    pub const WORKGROUP_SIZE: u32 = 128;

    fn new(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        layout: &wgpu::PipelineLayout,
        lights: &GpuLights,
        globals: &GpuGlobals,
    ) -> Self {
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("move lights"),
            layout: Some(layout),
            module: shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("move lights compute bind group layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lights.buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.buffer.as_entire_binding(),
            }],
            label: Some("move lights compute bind group"),
        });

        Self {
            pipeline,
            bind_group,
            bind_group_layout,
        }
    }

    fn dispatch(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        globals: &wgpu::BindGroup,
        lights: &GpuLights,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("move lights dispatch"),
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, self.bind_group, &[]);

        let num_workgroups = lights.num_lights.div_Ceil(Self::WORKGROUP_SIZE);
        pass.dispatch_workgroups(num_workgroups, 1, 1);
    }
}
