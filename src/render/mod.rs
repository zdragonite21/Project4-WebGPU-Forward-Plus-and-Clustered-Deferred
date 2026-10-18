use crate::{assets::RenderAssets, render::world::RenderWorld, world::World};

pub mod naive;
pub mod world;

pub trait Renderer {
    fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        render_resources: &RenderWorld,
        world: &World,
        render_assets: &RenderAssets,
    ) -> anyhow::Result<()>;

    fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32);
}
