use crate::{
    frame_clock::FrameClock,
    gpu::Gpu,
    input::{CursorState, Input},
    render::{Renderer, naive::NaiveRenderer, resources::RenderResources},
    scene::Scene,
    world::World,
};
use std::sync::Arc;
use winit::{
    event::{DeviceEvent, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window},
};

pub struct App {
    pub window: Arc<Window>,
    gpu: Gpu,
    world: World,
    scene: Scene,
    render_resources: RenderResources,
    renderer: Box<dyn Renderer>,
    input: Input,
    cursor: CursorState,
    frame_clock: FrameClock,
}

impl App {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<App> {
        let gpu = Gpu::new(window.clone()).await?;
        log::debug!("gpu context created");

        let world = World::new(gpu.size().x, gpu.size().y);
        log::debug!("world created");

        let scene = Scene::new(&gpu.device);
        log::debug!("scene created");

        let render_resources = RenderResources::new(&gpu.device, &world.cam_rig);

        let renderer = Box::new(NaiveRenderer::new(
            &gpu.device,
            gpu.render_format,
            &render_resources.camera.bind_group_layout,
        ));
        log::debug!("renderer created");

        let input = Input::default();
        let cursor = CursorState::default();
        let frame_clock = FrameClock::new();

        Ok(Self {
            window,
            gpu,
            world,
            scene,
            render_resources,
            renderer,
            input,
            cursor,
            frame_clock,
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
        self.world
            .cam_rig
            .resize(self.gpu.size().x, self.gpu.size().y);
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
            .update(&self.input, &mut self.cursor, self.frame_clock.dt);
        self.render_resources.upload(&self.gpu.queue, &self.world.cam_rig);

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // render
        self.renderer.draw(
            &mut encoder,
            &frame.view,
            &self.render_resources.camera.bind_group,
            &self.scene,
        )?;

        // finish frame
        self.gpu.queue.submit([encoder.finish()]);
        self.gpu.present(frame);

        self.update_capture();
        self.input.end_frame();

        Ok(())
    }
}
