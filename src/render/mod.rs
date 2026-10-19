use glam::UVec2;

use crate::{
    render::{assets::RenderAssets, world::RenderWorld},
    world::World,
};

pub mod assets;
pub mod clustering;
pub mod forward_plus;
pub mod lights;
pub mod naive;
pub mod world;

pub trait Renderer {
    fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        render_world: &RenderWorld,
        world: &World,
        render_assets: &RenderAssets,
    ) -> anyhow::Result<()>;

    fn resize(&mut self, device: &wgpu::Device, size: UVec2);
}
