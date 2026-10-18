use glam::Vec4;
use std::{marker::PhantomData, ops::Range};
use wgpu::util::DeviceExt;

pub mod gltf_loader;

use crate::{
    assets::gltf_loader::{Filter, GltfImage, GltfSampler, GltfScene, Wrap},
    gpu::Gpu,
    world::{MeshInstance, World},
};

pub struct Handle<T> {
    index: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Handle<T> {}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}
impl<T> Eq for Handle<T> {}

pub struct Assets<T> {
    items: Vec<T>,
}

impl<T> Assets<T> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
    pub fn add(&mut self, asset: T) -> Handle<T> {
        self.items.push(asset);

        Handle {
            index: self.items.len() - 1,
            _marker: PhantomData,
        }
    }
    pub fn get(&self, handle: Handle<T>) -> &T {
        &self.items[handle.index]
    }
}

pub struct Mesh {
    pub first_index: u32,
    pub index_count: u32,
    pub base_vertex: i32,
    // pub bounds: Aabb,
}

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { texture, view }
    }
}

pub struct Material {
    pub base_color: Handle<Texture>,
    pub base_color_factor: Vec4,
    pub bind_group: wgpu::BindGroup,
}

pub const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;
pub struct RenderAssets {
    pub meshes: Assets<Mesh>,
    pub materials: Assets<Material>,
    pub textures: Assets<Texture>,

    pub material_layout: wgpu::BindGroupLayout,

    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
}

const PINK: [u8; 4] = [255, 0, 255, 255];

pub fn import_gltf(scene: GltfScene, gpu: &Gpu, world: &mut World) -> RenderAssets {
    let device = &gpu.device;

    let material_layout = create_material_layout(device);
    let mut textures = Assets::new();
    let mut materials = Assets::new();
    let mut meshes = Assets::new();

    let image_textures: Vec<Handle<Texture>> = scene
        .images
        .iter()
        .map(|image| textures.add(upload_texture(gpu, image)))
        .collect();

    let pink_texture = textures.add(upload_texture(
        gpu,
        &GltfImage {
            width: 1,
            height: 1,
            rgba8: PINK.to_vec(),
        },
    ));

    let mut make_material = |base_color: Handle<Texture>,
                             factor: [f32; 4],
                             sampler: &GltfSampler| {
        let sampler = create_sampler(device, sampler);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("material_bind_group"),
            layout: &material_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&textures.get(base_color).view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        materials.add(Material {
            base_color,
            base_color_factor: Vec4::from_array(factor),
            bind_group,
        })
    };

    let default_material = make_material(pink_texture, [1.0; 4], &gltf_loader::DEFAULT_SAMPLER);

    let material_handles: Vec<Handle<Material>> = scene
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
    let mut submeshes: Vec<(Handle<Mesh>, Handle<Material>)> = Vec::new();
    let mut mesh_ranges: Vec<Range<usize>> = Vec::new();

    for gltf_mesh in scene.meshes {
        let start = submeshes.len();
        for prim in gltf_mesh.primitives {
            let mesh = meshes.add(Mesh {
                first_index: indices.len() as u32,
                index_count: prim.indices.len() as u32,
                base_vertex: vertices.len() as i32,
            });
            let material = prim
                .material
                .map_or(default_material, |i| material_handles[i]);

            vertices.extend(prim.vertices);
            indices.extend(prim.indices);
            submeshes.push((mesh, material));
        }
        mesh_ranges.push(start..submeshes.len());
    }

    for gltf_instance in scene.instances {
        let transform = world.transforms.len() as u32;
        world.transforms.push(gltf_instance.world_transform);

        for &(mesh, material) in &submeshes[mesh_ranges[gltf_instance.mesh].clone()] {
            world.instances.push(MeshInstance {
                mesh,
                material,
                transform,
            });
        }
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Scene Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Scene Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    log::debug!(
        "imported glTF: {} vertices, {} indices, {} submeshes, {} materials, {} textures",
        vertices.len(),
        indices.len(),
        meshes.items.len(),
        materials.items.len(),
        textures.items.len(),
    );

    RenderAssets {
        meshes,
        materials,
        textures,
        material_layout,
        vertex_buffer,
        index_buffer,
    }
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

fn upload_texture(gpu: &Gpu, image: &GltfImage) -> Texture {
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
    Texture { texture, view }
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
