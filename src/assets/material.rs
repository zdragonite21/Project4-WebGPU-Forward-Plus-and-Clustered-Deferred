use glam::Vec4;

use crate::assets::{Handle, texture::GpuTexture};

pub struct GpuMaterial {
    pub _base_color: Handle<GpuTexture>,
    pub _base_color_factor: Vec4,
    pub bind_group: wgpu::BindGroup,
}
