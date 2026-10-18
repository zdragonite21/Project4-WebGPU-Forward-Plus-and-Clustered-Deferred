use crate::{
    assets::{
        self, Handle, INDEX_FORMAT, Material, RenderAssets, Texture, gltf_loader::GltfVertex,
    },
    render::resources::RenderResources,
    world::World,
};

use super::Renderer;

pub struct NaiveRenderer {
    pipeline: wgpu::RenderPipeline,
    depth_texture: Texture,
}

impl NaiveRenderer {
    pub fn new(
        device: &wgpu::Device,
        render_format: wgpu::TextureFormat,
        render_resources: &RenderResources,
        render_assets: &RenderAssets,
        width: u32, height: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/shader.wgsl"));

        let depth_texture = Texture::create_depth_texture(device, width, height, "depth_texture");

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&render_resources.camera.bind_group_layout),
                    Some(&render_resources.transforms.bind_group_layout),
                    Some(&render_assets.material_layout),
                ],
                immediate_size: 0,
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(GltfVertex::desc())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: render_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: assets::Texture::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            depth_texture,
        }
    }
}

impl Renderer for NaiveRenderer {
    fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        render_resources: &RenderResources,
        world: &World,
        render_assets: &RenderAssets,
    ) -> anyhow::Result<()> {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &render_resources.camera.bind_group, &[]);
        render_pass.set_bind_group(1, &render_resources.transforms.bind_group, &[]);

        render_pass.set_vertex_buffer(0, render_assets.vertex_buffer.slice(..));
        render_pass.set_index_buffer(render_assets.index_buffer.slice(..), INDEX_FORMAT);

        let mut last_mat: Option<Handle<Material>> = None;
        for instance in &world.instances {
            let mesh = render_assets.meshes.get(instance.mesh);

            if Some(instance.material) != last_mat {
                let mat = render_assets.materials.get(instance.material);
                render_pass.set_bind_group(2, &mat.bind_group, &[]);
                last_mat = Some(instance.material);
            }

            render_pass.draw_indexed(
                mesh.first_index..mesh.first_index + mesh.index_count,
                mesh.base_vertex,
                instance.transform..instance.transform + 1,
            );
        }

        Ok(())
    }
}
