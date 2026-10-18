use crate::{assets::RenderAssets, render::resources::RenderResources, world::World};

pub mod naive;
pub mod resources;

pub trait Renderer {
    fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        render_resources: &RenderResources,
        world: &World,
        render_assets: &RenderAssets,
    ) -> anyhow::Result<()>;

    fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32);
}
