pub mod naive;

pub trait Renderer {
    fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) -> anyhow::Result<()>;
}
