use crate::{
    assets::{gltf_importer, gltf_loader},
    frame_clock::FrameClock,
    gpu::Gpu,
    input::{CursorState, Input},
    render::{
        Renderer,
        assets::RenderAssets,
        forward_plus::ForwardPlusRenderer,
        lights::{GpuLights, LightsComputePass},
        naive::NaiveRenderer,
        world::RenderWorld,
    },
    settings::{RenderMode, Settings},
    ui::Ui,
    world::World,
};
use std::sync::Arc;
use winit::{
    event::{DeviceEvent, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window},
};

pub struct Engine {
    pub window: Arc<Window>,
    gpu: Gpu,
    world: World,
    assets: RenderAssets,
    render_world: RenderWorld,
    renderer: Box<dyn Renderer>,
    lights_pass: LightsComputePass,
    input: Input,
    cursor: CursorState,
    frame_clock: FrameClock,
    settings: Settings,
    render_mode: RenderMode,
    ui: Ui,
}

impl Engine {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Engine> {
        let gpu = Gpu::new(window.clone()).await?;
        log::debug!("gpu context created");

        let mut world = World::new(gpu.size().x, gpu.size().y);
        log::debug!("world created");

        let mut assets = RenderAssets::new(&gpu.device);

        let sponza = gltf_loader::load_gltf("scenes/sponza/Sponza.gltf").await?;
        gltf_importer::import_gltf(sponza, &gpu, &mut world, &mut assets);
        log::debug!("sponza imported");

        let render_world = RenderWorld::new(&gpu.device, &world);

        let renderer = Box::new(ForwardPlusRenderer::new(
            &gpu.device,
            gpu.render_format,
            &render_world,
            &assets,
            gpu.size().x,
            gpu.size().y,
        ));
        log::debug!("renderer created");

        let lights_pass = LightsComputePass::new(&gpu.device, &render_world);

        let input = Input::default();
        let cursor = CursorState::default();
        let frame_clock = FrameClock::new();
        let ui = Ui::new(&window, &gpu);

        let render_mode = RenderMode::ForwardPlus;

        let settings = Settings {
            light_count: render_world.lights.num_lights,
            render_mode,
        };

        Ok(Self {
            window,
            gpu,
            world,
            assets,
            render_world,
            renderer,
            lights_pass,
            input,
            cursor,
            frame_clock,
            settings,
            render_mode,
            ui,
        })
    }

    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.input.on_mouse_motion(delta.0, delta.1);
            }
            _ => (),
        }
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        if !self.cursor.grabbed && self.ui.on_window_event(&self.window, event) {
            return false;
        }

        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        repeat,
                        ..
                    },
                ..
            } => {
                if *key == KeyCode::Escape {
                    return true;
                }
                self.input.on_key(*key, state.is_pressed(), *repeat)
            }

            WindowEvent::MouseInput { state, button, .. } => {
                self.input.on_mouse_button(*button, state.is_pressed())
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.on_scroll(delta);
            }
            WindowEvent::Focused(false) => {
                self.input.clear_held();
                self.cursor.want_grab = false;
            }
            _ => return false,
        }
        false
    }

    fn update_capture(&mut self) {
        match (self.cursor.grabbed, self.cursor.want_grab) {
            (false, true) => {
                match self
                    .window
                    .set_cursor_grab(CursorGrabMode::Locked)
                    .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined))
                {
                    Ok(()) => {
                        self.window.set_cursor_visible(false);
                        self.cursor.grabbed = true;
                        self.cursor.visible = false;
                        log::debug!("cursor captured");
                    }
                    Err(err) => {
                        self.window.set_cursor_visible(true);
                        self.cursor.want_grab = false;
                        self.cursor.visible = true;
                        log::info!("failed to capture cursor: {err}");
                    }
                }
            }
            (true, false) => {
                let _ = self.window.set_cursor_grab(CursorGrabMode::None);
                self.window.set_cursor_visible(true);
                self.cursor.grabbed = false;
                self.cursor.visible = true;
                log::debug!("cursor released");
            }
            _ => (),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);

        self.world.cam_rig.resize(self.gpu.size());
        self.renderer.resize(&self.gpu.device, self.gpu.size());
    }

    fn draw_ui(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) -> Vec<wgpu::CommandBuffer> {
        self.ui.draw(&self.window, &self.gpu, encoder, view, |ctx| {
            egui::Window::new("Settings").show(ctx, |ui| {
                ui.add(
                    egui::Slider::new(
                        &mut self.settings.light_count,
                        0..=GpuLights::MAX_NUM_LIGHTS,
                    )
                    .text("num lights"),
                );

                egui::ComboBox::from_label("render mode").show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.settings.render_mode, RenderMode::Naive, "naive");
                    ui.selectable_value(
                        &mut self.settings.render_mode,
                        RenderMode::ForwardPlus,
                        "forward+",
                    );
                });
            });
        })
    }

    fn update_renderer(&mut self) {
        if self.render_mode != self.settings.render_mode {
            self.renderer = match self.settings.render_mode {
                RenderMode::ForwardPlus => Box::new(ForwardPlusRenderer::new(
                    &self.gpu.device,
                    self.gpu.render_format,
                    &self.render_world,
                    &self.assets,
                    self.gpu.size().x,
                    self.gpu.size().y,
                )),
                RenderMode::Naive => Box::new(NaiveRenderer::new(
                    &self.gpu.device,
                    self.gpu.render_format,
                    &self.render_world,
                    &self.assets,
                    self.gpu.size().x,
                    self.gpu.size().y,
                )),
            };
            self.render_mode = self.settings.render_mode;
        }
    }

    pub fn frame(&mut self) -> anyhow::Result<()> {
        // start frame
        self.window.request_redraw();

        let Some(frame) = self.gpu.acquire_frame()? else {
            return Ok(());
        };

        // frame
        self.frame_clock.tick();
        self.world
            .update(&self.input, &mut self.cursor, &self.frame_clock);
        self.render_world
            .upload(&self.gpu.queue, &self.world, &self.settings);

        self.update_renderer();

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // render
        self.lights_pass.dispatch(&mut encoder, &self.render_world);

        self.renderer.draw(
            &mut encoder,
            &frame.view,
            &self.render_world,
            &self.world,
            &self.assets,
        )?;

        let ui_cmd_bufs = self.draw_ui(&mut encoder, &frame.view);

        // finish frame
        self.gpu
            .queue
            .submit(ui_cmd_bufs.into_iter().chain([encoder.finish()]));
        self.gpu.present(frame);

        self.update_capture();
        self.input.end_frame();

        Ok(())
    }
}
