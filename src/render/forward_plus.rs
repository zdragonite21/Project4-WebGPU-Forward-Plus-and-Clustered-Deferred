use glam::UVec2;

use crate::{
    assets::{Handle, material::GpuMaterial, mesh::Vertex, texture::GpuTexture},
    render::{assets::RenderAssets, clustering::ClusteringComputePass, world::RenderWorld},
    shader_constants::{
        BIND_GROUP_CLUSTERING, BIND_GROUP_MATERIAL, BIND_GROUP_SCENE, BIND_GROUP_TRANSFORMS,
    },
    world::World,
};

use super::Renderer;

pub struct ForwardPlusRenderer {
    pipeline: wgpu::RenderPipeline,
    depth_texture: GpuTexture,
    cluster_pass: ClusteringComputePass,
}

impl ForwardPlusRenderer {
    pub fn new(
        device: &wgpu::Device,
        render_format: wgpu::TextureFormat,
        render_world: &RenderWorld,
        render_assets: &RenderAssets,
        width: u32,
        height: u32,
    ) -> Self {
        let cluster_pass = ClusteringComputePass::new(&device, &render_world);

        let shader_vs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("naive_vs"),
            source: wgpu::ShaderSource::Wgsl(wesl_core::include_wesl!("naive_vs").into()),
        });

        let shader_fs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("forward_plus_fs"),
            source: wgpu::ShaderSource::Wgsl(wesl_core::include_wesl!("forward_plus_fs").into()),
        });

        let depth_texture =
            GpuTexture::create_depth_texture(device, width, height, "depth_texture");

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&render_world.scene_bind_group_layout),
                    Some(&render_world.camera.bind_group_layout),
                    Some(&render_world.transforms.bind_group_layout),
                    Some(&render_assets.material_layout),
                    Some(&cluster_pass.resources.bind_group_layout),
                ],
                immediate_size: 0,
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_vs,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::desc())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_fs,
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
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: GpuTexture::DEPTH_FORMAT,
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
            cluster_pass,
        }
    }

    fn shade(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        render_world: &RenderWorld,
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
        render_pass.set_bind_group(BIND_GROUP_SCENE, &render_world.scene_bind_group, &[]);
        render_pass.set_bind_group(BIND_GROUP_SCENE, &render_world.camera.bind_group, &[]);
        render_pass.set_bind_group(
            BIND_GROUP_CLUSTERING,
            &self.cluster_pass.resources.bind_group,
            &[],
        );
        render_pass.set_bind_group(
            BIND_GROUP_TRANSFORMS,
            &render_world.transforms.bind_group,
            &[],
        );

        render_pass.set_vertex_buffer(0, render_assets.vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            render_assets.index_buffer.slice(..),
            RenderAssets::INDEX_FORMAT,
        );

        let mut last_mat: Option<Handle<GpuMaterial>> = None;
        for instance in &world.instances {
            let mesh = render_assets.meshes.get(instance.mesh);

            if Some(instance.material) != last_mat {
                let mat = render_assets.materials.get(instance.material);
                render_pass.set_bind_group(BIND_GROUP_MATERIAL, &mat.bind_group, &[]);
                last_mat = Some(instance.material);
            }

            // instance index is our transform idx
            render_pass.draw_indexed(
                mesh.first_index..mesh.first_index + mesh.index_count,
                mesh.base_vertex,
                instance.transform..instance.transform + 1,
            );
        }

        Ok(())
    }
}

impl Renderer for ForwardPlusRenderer {
    fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        render_world: &RenderWorld,
        world: &World,
        render_assets: &RenderAssets,
    ) -> anyhow::Result<()> {
        self.cluster_pass.dispatch(encoder, render_world);
        self.shade(encoder, view, render_world, world, render_assets)
    }

    fn resize(&mut self, device: &wgpu::Device, size: UVec2) {
        self.depth_texture =
            GpuTexture::create_depth_texture(device, size.x, size.y, "depth_texture");
    }
}
