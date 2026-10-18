use crate::assets::{
    Assets, Handle,
    material::GpuMaterial,
    mesh::{GpuSubMesh, Vertex},
    texture::GpuTexture,
};

pub struct RenderAssets {
    pub meshes: Assets<GpuSubMesh>,
    pub materials: Assets<GpuMaterial>,
    #[allow(unused)]
    pub textures: Assets<GpuTexture>,

    pub material_layout: wgpu::BindGroupLayout,

    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    vertex_count: u32,
    index_count: u32,
}

impl RenderAssets {
    pub const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;
    const MAX_VERTICES: u32 = 1 << 20;
    const MAX_INDICES: u32 = 1 << 22;

    pub fn new(device: &wgpu::Device) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: Self::MAX_VERTICES as u64 * size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: Self::MAX_INDICES as u64 * size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            meshes: Assets::new(),
            materials: Assets::new(),
            textures: Assets::new(),
            material_layout: GpuMaterial::create_layout(device),
            vertex_buffer,
            index_buffer,
            vertex_count: 0,
            index_count: 0,
        }
    }

    pub fn add_mesh(
        &mut self,
        queue: &wgpu::Queue,
        vertices: &[Vertex],
        indices: &[u32],
    ) -> Handle<GpuSubMesh> {
        let new_vertex_count = self.vertex_count + vertices.len() as u32;
        let new_index_count = self.index_count + indices.len() as u32;
        assert!(new_vertex_count <= Self::MAX_VERTICES, "vertex buffer full");
        assert!(new_index_count <= Self::MAX_INDICES, "index buffer full");

        queue.write_buffer(
            &self.vertex_buffer,
            self.vertex_count as u64 * size_of::<Vertex>() as u64,
            bytemuck::cast_slice(vertices),
        );
        queue.write_buffer(
            &self.index_buffer,
            self.index_count as u64 * size_of::<u32>() as u64,
            bytemuck::cast_slice(indices),
        );

        let handle = self.meshes.add(GpuSubMesh {
            first_index: self.index_count,
            index_count: indices.len() as u32,
            base_vertex: self.vertex_count as i32,
        });

        self.vertex_count = new_vertex_count;
        self.index_count = new_index_count;
        handle
    }
}
