use glam::UVec2;

use crate::{
    render::{assets::RenderAssets, world::RenderWorld}, shader_constants::{BIND_GROUP_SCENE, MOVE_LIGHTS_WORKGROUP_SIZE}, world::World,
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
