use crate::scene::Scene;

pub mod naive;
pub mod resources;

pub trait Renderer {
    fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, cam_bg: &wgpu::BindGroup, scene: &Scene) -> anyhow::Result<()>;
}
