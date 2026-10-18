use crate::assets::{Assets, material::GpuMaterial, mesh::GpuSubMesh, texture::GpuTexture};

pub struct RenderAssets {
    pub meshes: Assets<GpuSubMesh>,
    pub materials: Assets<GpuMaterial>,
    #[allow(unused)]
    pub textures: Assets<GpuTexture>,

    pub material_layout: Option<wgpu::BindGroupLayout>,

    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
}

impl RenderAssets {
    pub const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;

    pub fn new() -> Self {
        Self {
            meshes: Assets::new(),
            materials: Assets::new(),
            textures: Assets::new(),
            material_layout: None,
            vertex_buffer: None,
            index_buffer: None,
        }
    }
}
