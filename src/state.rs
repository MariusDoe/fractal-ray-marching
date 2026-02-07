use crate::{key_state::KeyState, persistent_state::PersistentState, render_state::RenderState};
use anyhow::{Context, Ok, Result};
use wgpu::{
    BindGroup, Color, CommandEncoder, CommandEncoderDescriptor, LoadOp, Operations,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, StoreOp, TextureView,
    TextureViewDescriptor,
};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, NamedKey, PhysicalKey},
    window::CursorGrabMode,
};

#[derive(Debug)]
pub struct State {
    pub persistent: PersistentState,
    render: RenderState,
    key_state: KeyState,
    last_cursor_position: Option<PhysicalPosition<f64>>,
    cursor_grabbed: bool,
}

impl State {
    const CLEAR_COLOR: Color = Color::BLACK;

    pub async fn init(event_loop: &ActiveEventLoop) -> Result<Self> {
        let persistent = PersistentState::init(event_loop).await?;
        let render = RenderState::init(&persistent)?;
        let key_state = KeyState::default();
        Ok(Self {
            persistent,
            render,
            key_state,
            cursor_grabbed: false,
            last_cursor_position: None,
        })
    }

    pub fn draw(&mut self) -> Result<()> {
        self.update();
        self.render()?;
        Ok(())
    }

    fn update(&mut self) {
        self.persistent.update(self.key_state);
    }

    fn render(&mut self) -> Result<()> {
        let mut encoder = self
            .persistent
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());
        let render_texture_view = self
            .persistent
            .render_texture
            .create_view(&TextureViewDescriptor::default());
        self.do_render_pass(
            &mut encoder,
            "render_pass",
            &render_texture_view,
            &self.render.render_pipeline,
            &self.persistent.parameters_bind_group,
        );
        let frame = self
            .persistent
            .surface
            .get_current_texture()
            .context("failed to get frame texture")?;
        let frame_texture_view = frame.texture.create_view(&TextureViewDescriptor::default());
        self.do_render_pass(
            &mut encoder,
            "blit_render_pass",
            &frame_texture_view,
            &self.persistent.blit_render_pipeline,
            &self.persistent.blit_bind_group,
        );
        self.persistent.queue.submit(Some(encoder.finish()));
        self.persistent.window.pre_present_notify();
        frame.present();
        self.persistent.window.request_redraw();
        Ok(())
    }

    fn do_render_pass(
        &self,
        encoder: &mut CommandEncoder,
        label: &'static str,
        view: &TextureView,
        render_pipeline: &RenderPipeline,
        bind_group: &BindGroup,
    ) {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Self::CLEAR_COLOR),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(render_pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..4, 0..1);
    }

    fn handle_movement(&mut self, key: KeyState, pressed: bool) {
        self.key_state.set(key, pressed);
    }

    fn reload(&mut self) -> Result<()> {
        self.render = RenderState::init(&self.persistent).context("failed to reload")?;
        Ok(())
    }

    fn try_reload(&mut self) {
        if let Err(error) = self.reload() {
            println!("{error:?}");
        }
    }

    pub fn handle_key(&mut self, event: KeyEvent) -> Result<()> {
        if event.state == ElementState::Pressed {
            macro_rules! handle_keys {
                ($($key:expr => $body:stmt),* $(,)?) => {
                    $(
                        if event.logical_key == $key {
                            $body
                            return Ok(());
                        }
                    )*
                };
            }
            handle_keys!(
                NamedKey::Escape => self.ungrab_cursor()?,
                "r" => self.try_reload(),
                "+" => self.persistent.parameters.update_num_iterations(1),
                "-" => self.persistent.parameters.update_num_iterations(-1),
                "o" => self.persistent.camera.toggle_orbiting(),
                "n" => self.persistent.parameters.update_scene_index(1),
                "b" => self.persistent.parameters.update_scene_index(-1),
            );
        }
        let PhysicalKey::Code(code) = event.physical_key else {
            return Ok(());
        };
        let key = match code {
            KeyCode::KeyW => KeyState::MoveForward,
            KeyCode::KeyS => KeyState::MoveBackward,
            KeyCode::KeyA => KeyState::MoveLeft,
            KeyCode::KeyD => KeyState::MoveRight,
            KeyCode::ShiftLeft => KeyState::MoveDown,
            KeyCode::Space => KeyState::MoveUp,
            KeyCode::ArrowDown => KeyState::PitchDown,
            KeyCode::ArrowUp => KeyState::PitchUp,
            KeyCode::ArrowRight => KeyState::YawRight,
            KeyCode::ArrowLeft => KeyState::YawLeft,
            _ => return Ok(()),
        };
        self.handle_movement(key, event.state.is_pressed());
        Ok(())
    }

    pub fn handle_mouse(&mut self, button: MouseButton, state: ElementState) -> Result<()> {
        if button == MouseButton::Left && state == ElementState::Pressed {
            self.grab_cursor()?;
        }
        Ok(())
    }

    pub fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) -> Result<()> {
        let y_delta = match delta {
            MouseScrollDelta::LineDelta(_x, y) => y,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => y as f32,
        };
        self.persistent.camera.update_speed(y_delta);
        Ok(())
    }

    pub fn handle_focused(&mut self, focused: bool) -> Result<()> {
        if !focused {
            self.ungrab_cursor()?;
        }
        Ok(())
    }

    pub fn handle_cursor_movement(&mut self, position: PhysicalPosition<f64>) -> Result<()> {
        if self.cursor_grabbed
            && let Some(last_position) = self.last_cursor_position
        {
            let yaw = position.x - last_position.x;
            let pitch = position.y - last_position.y;
            self.persistent
                .camera
                .rotate_from_cursor_movement(yaw as f32, pitch as f32);
            self.persistent
                .window
                .set_cursor_position(last_position)
                .context("failed to lock cursor in place")?;
        } else {
            self.last_cursor_position = Some(position);
        }
        Ok(())
    }

    fn grab_cursor(&mut self) -> Result<()> {
        if self.cursor_grabbed {
            return Ok(());
        }
        const CURSOR_GRAB_MODE: CursorGrabMode = if cfg!(target_os = "macos") {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::Confined
        };
        let window = &*self.persistent.window;
        window
            .set_cursor_grab(CURSOR_GRAB_MODE)
            .context("failed to grab cursor")?;
        window.set_cursor_visible(false);
        self.cursor_grabbed = true;
        Ok(())
    }

    fn ungrab_cursor(&mut self) -> Result<()> {
        if !self.cursor_grabbed {
            return Ok(());
        }
        let window = &*self.persistent.window;
        window
            .set_cursor_grab(CursorGrabMode::None)
            .context("failed to ungrab cursor")?;
        window.set_cursor_visible(true);
        self.cursor_grabbed = false;
        Ok(())
    }
}
