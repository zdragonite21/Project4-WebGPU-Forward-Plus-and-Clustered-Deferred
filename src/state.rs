use crate::{
    camera::{CameraRig, CameraSettings}, frame_clock::FrameClock, renderers::{Renderer, naive::NaiveRenderer}, scene::Scene,
};
use anyhow::bail;
use std::sync::Arc;
use winit::{
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window},
};

pub struct State {
    pub window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_format: wgpu::TextureFormat,

    pub cam_rig: CameraRig,
    scene: Scene,
    frame_clock: FrameClock,
    renderer: Box<dyn Renderer>,

    is_surface_configured: bool,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<State> {
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

        let cam_rig = CameraRig::new(&device, &CameraSettings::default(), size.width, size.height);

        let scene = Scene::new(&device);

        let frame_clock = FrameClock::new();

        let renderer = Box::new(NaiveRenderer::new(
            &device,
            render_format,
            &cam_rig.bind_group_layout,
        ));

        log::debug!("renderer created");

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            render_format,
            cam_rig,
            scene,
            frame_clock,
            renderer,
            is_surface_configured: false,
        })
    }

    fn set_cam_capture(&mut self, captured: bool) {
        self.cam_rig.controller.set_captured(captured);
        self.window.set_cursor_visible(!captured);

        if captured {
            match self
                .window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined))
            {
                Ok(()) => log::debug!("cursor captured"),
                Err(err) => {
                    self.cam_rig.controller.set_captured(false);
                    self.window.set_cursor_visible(true);
                    log::info!("failed to capture cursor: {err}");
                }
            }
        } else {
            let _ = self.window.set_cursor_grab(CursorGrabMode::None);
            log::debug!("cursor released");
        }
    }

    pub fn handle_cam_input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => self.cam_rig.controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } if self.cam_rig.controller.is_captured() => {
                self.cam_rig.controller.handle_mouse_scroll(delta);
                true
            }
            WindowEvent::MouseInput { state, button, .. }
                if *button == MouseButton::Left && *state == ElementState::Pressed =>
            {
                self.set_cam_capture(true);
                true
            }
            WindowEvent::MouseInput { state, button, .. }
                if *button == MouseButton::Left
                    && *state == ElementState::Released
                    && self.cam_rig.controller.is_captured() =>
            {
                self.set_cam_capture(false);
                true
            }
            WindowEvent::Focused(false) => {
                self.set_cam_capture(false);
                log::debug!("window lost focus, cursor released");
                true
            }
            _ => false,
        }
    }

    pub fn handle_shortcut(&mut self, event: &WindowEvent) -> bool {
        let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(key),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } = event
        else {
            return false;
        };

        match key {
            KeyCode::Enter => true,
            _ => false,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);

            self.cam_rig.resize(width, height);

            log::debug!("surface resized to {}x{}", width, height);

            self.is_surface_configured = true;
        } else {
            log::debug!("ignored zero-sized surface resize: {}x{}", width, height);
        }
    }

    fn update(&mut self, dt: instant::Duration) {
        self.cam_rig.update(&self.queue, dt);
    }

    pub fn frame(&mut self) -> anyhow::Result<()> {
        self.frame_clock.tick();
        self.window.request_redraw();

        if !self.is_surface_configured {
            log::debug!("surface not configured yet; skipping render");
            return Ok(());
        }

        let (output, reconfigure_after_present) = {
            use wgpu::CurrentSurfaceTexture::*;

            match self.surface.get_current_texture() {
                Success(surface_texture) => (surface_texture, false),
                Suboptimal(surface_texture) => (surface_texture, true),
                Timeout => {
                    log::debug!("surface texture acquisition timed out; skipping frame");
                    return Ok(());
                }
                Occluded => {
                    log::debug!("surface is occluded; skipping frame");
                    return Ok(());
                }
                Validation => {
                    bail!("surface texture acquisition failed validation; terminate");
                }
                Outdated => {
                    self.surface.configure(&self.device, &self.config);
                    log::debug!("surface outdated; reconfiguring");
                    return Ok(());
                }
                Lost => {
                    bail!("lost surface");
                }
            }
        };

        self.update(self.frame_clock.dt);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // render
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.render_format),
            ..Default::default()
        });

        self.renderer
            .draw(&mut encoder, &view, &self.cam_rig.bind_group, &self.scene)?;

        self.queue.submit([encoder.finish()]);
        self.queue.present(output);

        if reconfigure_after_present {
            log::debug!("reconfiguring surface");
            self.surface.configure(&self.device, &self.config);
        }

        Ok(())
    }
}
