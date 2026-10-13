use std::sync::Arc;
use winit::window::Window;

pub struct State {
    window: Arc<Window>,
    // surface: wgpu::Surface<'static>,
    // device: wgpu::Device,
    // queue: wgpu::Queue,
    // config: wgpu::SurfaceConfiguration,
    // render_format: wgpu::TextureFormat,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<State> {
        Ok(Self {
            window,
        })
    }

    pub fn resize(&mut self, _width: u32, _height: u32) {

    }

    pub fn render(&mut self) {
        self.window.request_redraw();
    }
}
