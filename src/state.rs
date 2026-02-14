use crate::{
    blit_state::BlitState, key_state::KeyState, persistent_state::PersistentState,
    render_state::RenderState,
};
use anyhow::{Context, Ok, Result};
use wgpu::{
    BindGroup, Color, CommandEncoder, CommandEncoderDescriptor, LoadOp, Operations,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, StoreOp, SurfaceTexture,
    TextureView, TextureViewDescriptor,
};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta},
    event_loop::ActiveEventLoop,
    keyboard::NamedKey,
    window::CursorGrabMode,
};

#[derive(Debug)]
pub struct State {
    pub persistent: PersistentState,
    render: RenderState,
    blit: BlitState,
    key_state: KeyState,
    last_cursor_position: Option<PhysicalPosition<f64>>,
    cursor_grabbed: bool,
}

impl State {
    const CLEAR_COLOR: Color = Color::BLACK;

    pub async fn init(event_loop: &ActiveEventLoop) -> Result<Self> {
        let persistent = PersistentState::init(event_loop).await?;
        let render = RenderState::init(&persistent)?;
        let blit = BlitState::init(&persistent);
        let key_state = KeyState::default();
        Ok(Self {
            persistent,
            render,
            blit,
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
        let PersistentState {
            device,
            surface,
            queue,
            window,
            ..
        } = &self.persistent;
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
        self.do_render_texture_pass(&mut encoder);
        let frame = surface
            .get_current_texture()
            .context("failed to get frame texture")?;
        self.do_blit_pass(&mut encoder, &frame)?;
        queue.submit(Some(encoder.finish()));
        window.pre_present_notify();
        frame.present();
        window.request_redraw();
        Ok(())
    }

    fn do_render_texture_pass(&self, encoder: &mut CommandEncoder) {
        let render_texture_view = self
            .blit
            .render_texture
            .create_view(&TextureViewDescriptor::default());
        self.do_render_pass(
            encoder,
            "render_pass",
            &render_texture_view,
            &self.render.render_pipeline,
            &self.persistent.parameters_bind_group,
        );
    }

    fn do_blit_pass(&self, encoder: &mut CommandEncoder, frame: &SurfaceTexture) -> Result<()> {
        let frame_texture_view = frame.texture.create_view(&TextureViewDescriptor::default());
        self.do_render_pass(
            encoder,
            "blit_render_pass",
            &frame_texture_view,
            &self.persistent.blit_render_pipeline,
            &self.blit.blit_bind_group,
        );
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
        let vertices = 0..4; // a quad
        let single_instance = 0..1;
        render_pass.draw(vertices, single_instance);
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

    fn update_render_texture_size(&mut self, delta: i32) {
        self.persistent.update_render_texture_size(delta);
        self.blit = BlitState::init(&self.persistent);
    }

    pub fn handle_key(&mut self, event: &KeyEvent) -> Result<()> {
        self.handle_key_state_keys(event);
        self.handle_single_press_keys(event)?;
        Ok(())
    }

    fn handle_single_press_keys(&mut self, event: &KeyEvent) -> Result<()> {
        if event.state != ElementState::Pressed {
            return Ok(());
        }
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
            "o" => self.persistent.camera.reset_orbit_speed(),
            "p" => self.persistent.camera.toggle_lock_pitch(),
            "l" => self.persistent.camera.cycle_lock_yaw_mode(false),
            "L" => self.persistent.camera.cycle_lock_yaw_mode(true),
            "n" => self.persistent.parameters.update_scene_index(1),
            "b" => self.persistent.parameters.update_scene_index(-1),
            "t" => self.persistent.stop_time(),
            ">" => self.update_render_texture_size(1),
            "<" => self.update_render_texture_size(-1),
        );
        Ok(())
    }

    fn handle_key_state_keys(&mut self, event: &KeyEvent) {
        macro_rules! match_key {
            ($($key:expr => $key_state:expr,)* else => $default:expr $(,)?) => {
                $(if event.logical_key == $key { $key_state } else )*
                { $default }
            };
        }
        let key_state = match_key! {
            "w" => KeyState::MoveForward,
            "s" => KeyState::MoveBackward,
            "a" => KeyState::MoveLeft,
            "d" => KeyState::MoveRight,
            "q" => KeyState::MoveDown,
            "e" => KeyState::MoveUp,
            NamedKey::ArrowDown => KeyState::PitchDown,
            NamedKey::ArrowUp => KeyState::PitchUp,
            NamedKey::ArrowRight => KeyState::YawRight,
            NamedKey::ArrowLeft => KeyState::YawLeft,
            NamedKey::Shift => KeyState::Shift,
            NamedKey::Control => KeyState::Control,
            else => return,
        };
        self.key_state.set(key_state, event.state.is_pressed());
    }

    pub fn handle_mouse(&mut self, button: MouseButton, state: ElementState) -> Result<()> {
        if button == MouseButton::Left && state == ElementState::Pressed {
            self.grab_cursor()?;
        }
        Ok(())
    }

    fn is_shift_pressed(&self) -> bool {
        self.key_state.contains(KeyState::Shift)
    }

    fn is_control_pressed(&self) -> bool {
        self.key_state.contains(KeyState::Control)
    }

    pub fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) -> Result<()> {
        const LINE_FACTOR: f32 = 0.5;
        let (mut x, mut y) = match delta {
            MouseScrollDelta::LineDelta(x, y) => (x * LINE_FACTOR, y * LINE_FACTOR),
            MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => (x as f32, y as f32),
        };
        if self.is_shift_pressed() {
            x += y;
            y = 0.0;
        }
        if self.is_control_pressed() {
            self.persistent.update_time_factor(y);
        } else {
            self.persistent.camera.update_orbit_speed(x);
            self.persistent.camera.update_speed(y);
        }
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
