use glam::UVec2;
use std::sync::Arc;
use winit::window::Window;
use anyhow::bail;

pub struct GpuFrame {
    surface_texture: wgpu::SurfaceTexture,
    pub view: wgpu::TextureView,
    reconfigure: bool,
}

// gpu context
pub struct Gpu {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub render_format: wgpu::TextureFormat,
    is_surface_configured: bool,
}

impl Gpu {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        #[cfg(not(target_arch = "wasm32"))]
        let backends = wgpu::Backends::VULKAN;
        #[cfg(target_arch = "wasm32")]
        let backends = wgpu::Backends::BROWSER_WEBGPU;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                apply_limit_buckets: false,
                force_fallback_adapter: false,
            })
            .await?;

        let info = adapter.get_info();
        log::info!(
            "\ngpu: {}\nbackend: {}\ndriver: {} ({})\n",
            info.name,
            info.backend,
            info.driver,
            info.driver_info,
        );

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::defaults(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or_else(|| {
                log::info!("No sRGB surface format available; colors may appear darker");
                surface_caps.formats[0]
            });

        let render_format = surface_format.add_srgb_suffix();
        let view_formats = (render_format != surface_format)
            .then_some(vec![render_format])
            .unwrap_or_default();

        log::debug!(
            "surface capabilities:\nselected_format={:?}\nrender_format={:?}\nview_formats={:?}\npresent_modes={:?}\nalpha_modes={:?}",
            surface_format,
            render_format,
            view_formats,
            surface_caps.present_modes,
            surface_caps.alpha_modes
        );

        let config: wgpu::wgt::SurfaceConfiguration<Vec<wgpu::TextureFormat>> =
            wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: surface_caps.alpha_modes[0],
                color_space: wgpu::SurfaceColorSpace::Auto,
                view_formats,
                desired_maximum_frame_latency: 2,
            };

        Ok(Self {
            surface,
            device,
            queue,
            config,
            render_format,
            is_surface_configured: false,
        })
    }

    pub fn size(&self) -> UVec2 {
        UVec2::new(self.config.width, self.config.height)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);

            log::debug!("surface resized to {}x{}", width, height);

            self.is_surface_configured = true;
        } else {
            log::debug!("ignored zero-sized surface resize: {}x{}", width, height);
        }
    }

    pub fn acquire_frame(&self) -> anyhow::Result<Option<GpuFrame>> {
        if !self.is_surface_configured {
            log::debug!("surface not configured yet; skipping render");
            return Ok(None);
        }

        let (surface_texture, reconfigure) = {
            use wgpu::CurrentSurfaceTexture::*;

            match self.surface.get_current_texture() {
                Success(surface_texture) => (surface_texture, false),
                Suboptimal(surface_texture) => (surface_texture, true),
                Timeout => {
                    log::debug!("surface texture acquisition timed out; skipping frame");
                    return Ok(None);
                }
                Occluded => {
                    log::debug!("surface is occluded; skipping frame");
                    return Ok(None);
                }
                Validation => {
                    bail!("surface texture acquisition failed validation; terminate");
                }
                Outdated => {
                    self.surface.configure(&self.device, &self.config);
                    log::debug!("surface outdated; reconfiguring");
                    return Ok(None);
                }
                Lost => {
                    bail!("lost surface");
                }
            }
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.render_format),
                ..Default::default()
            });

        let frame = GpuFrame {
            surface_texture,
            view,
            reconfigure,
        };

        Ok(Some(frame))
    }

    pub fn present(&self, frame: GpuFrame) {
        self.queue.present(frame.surface_texture);

        if frame.reconfigure {
            log::debug!("reconfiguring surface");
            self.surface.configure(&self.device, &self.config);
        }
    }

}
