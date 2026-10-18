use std::ops::Range;

use glam::Vec4;
use wgpu::util::DeviceExt;

use crate::{
    assets::{
        Handle,
        gltf_loader::{self, Filter, GltfImage, GltfSampler, GltfScene, Wrap},
        material::GpuMaterial,
        mesh::{GpuSubMesh, Vertex},
        texture::GpuTexture,
    },
    gpu::Gpu,
    render::assets::RenderAssets,
    world::{MeshInstance, World},
};

const PINK: [u8; 4] = [255, 0, 255, 255];

pub fn import_gltf(scene: GltfScene, gpu: &Gpu, world: &mut World, assets: &mut RenderAssets) {
    let device = &gpu.device;

    let material_layout = create_material_layout(device);

    let image_textures: Vec<Handle<GpuTexture>> = scene
        .images
        .iter()
        .map(|image| assets.textures.add(upload_texture(gpu, image)))
        .collect();

    let pink_texture = assets.textures.add(upload_texture(
        gpu,
        &GltfImage {
            width: 1,
            height: 1,
            rgba8: PINK.to_vec(),
        },
    ));

    let mut make_material =
        |base_color: Handle<GpuTexture>, factor: [f32; 4], sampler: &GltfSampler| {
            let sampler = create_sampler(device, sampler);
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("material_bind_group"),
                layout: &material_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &assets.textures.get(base_color).view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });
            assets.materials.add(GpuMaterial {
                _base_color: base_color,
                _base_color_factor: Vec4::from_array(factor),
                bind_group,
            })
        };

    let default_material = make_material(pink_texture, [1.0; 4], &gltf_loader::DEFAULT_SAMPLER);

    let material_handles: Vec<Handle<GpuMaterial>> = scene
        .materials
        .iter()
        .map(|m| {
            let texture = m
                .base_color_image
                .map_or(pink_texture, |i| image_textures[i]);
            make_material(texture, m.base_color_factor, &m.sampler)
        })
        .collect();

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut submeshes: Vec<(Handle<GpuSubMesh>, Handle<GpuMaterial>)> = Vec::new();
    let mut mesh_ranges: Vec<Range<usize>> = Vec::new();

    for gltf_mesh in scene.meshes {
        let start = submeshes.len();
        for prim in gltf_mesh.primitives {
            let mesh = assets.meshes.add(GpuSubMesh {
                first_index: indices.len() as u32,
                index_count: prim.indices.len() as u32,
                base_vertex: vertices.len() as i32,
            });
            let material = prim
                .material
                .map_or(default_material, |i| material_handles[i]);

            vertices.extend(
                prim.positions
                    .iter()
                    .zip(&prim.normals)
                    .zip(&prim.uvs)
                    .map(|((&pos, &nor), &uv)| Vertex { pos, nor, uv }),
            );
            indices.extend(prim.indices);
            submeshes.push((mesh, material));
        }
        mesh_ranges.push(start..submeshes.len());
    }

    for gltf_instance in scene.instances {
        let transform = world.global_transforms.len() as u32;
        world.global_transforms.push(gltf_instance.world_transform);

        for &(mesh, material) in &submeshes[mesh_ranges[gltf_instance.mesh].clone()] {
            world.instances.push(MeshInstance {
                mesh,
                material,
                transform,
            });
        }
    }

    assets.vertex_buffer = Some(
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }),
    );
    assets.index_buffer = Some(
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        }),
    );

    log::debug!(
        "imported glTF: {} vertices, {} indices, {} submeshes, {} materials, {} textures",
        vertices.len(),
        indices.len(),
        assets.meshes.items.len(),
        assets.materials.items.len(),
        assets.textures.items.len(),
    );
}

fn create_material_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("material_bind_group_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn upload_texture(gpu: &Gpu, image: &GltfImage) -> GpuTexture {
    let texture = gpu.device.create_texture_with_data(
        &gpu.queue,
        &wgpu::TextureDescriptor {
            label: Some("base_color_texture"),
            size: wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &image.rgba8,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    GpuTexture {
        _texture: texture,
        view,
    }
}

fn create_sampler(device: &wgpu::Device, s: &GltfSampler) -> wgpu::Sampler {
    let filter = |f| match f {
        Filter::Nearest => wgpu::FilterMode::Nearest,
        Filter::Linear => wgpu::FilterMode::Linear,
    };
    let wrap = |w| match w {
        Wrap::Repeat => wgpu::AddressMode::Repeat,
        Wrap::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
        Wrap::ClampToEdge => wgpu::AddressMode::ClampToEdge,
    };
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("material_sampler"),
        address_mode_u: wrap(s.wrap_u),
        address_mode_v: wrap(s.wrap_v),
        mag_filter: filter(s.mag),
        min_filter: filter(s.min),
        mipmap_filter: match s.mipmap {
            Some(Filter::Linear) => wgpu::MipmapFilterMode::Linear,
            Some(Filter::Nearest) | None => wgpu::MipmapFilterMode::Nearest,
        },
        ..Default::default()
    })
}
