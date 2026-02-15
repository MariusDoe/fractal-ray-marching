use crate::{
    camera::Camera, graphics::Graphics, held_keys::HeldKeys, parameters::Parameters, timing::Timing,
};
use anyhow::{Context, Ok, Result};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta},
    event_loop::ActiveEventLoop,
    keyboard::NamedKey,
};

#[derive(Debug)]
pub struct InitializedApp {
    graphics: Graphics,
    held_keys: HeldKeys,
    parameters: Parameters,
    camera: Camera,
    timing: Timing,
}

impl InitializedApp {
    pub async fn init(event_loop: &ActiveEventLoop) -> Result<Self> {
        let graphics = Graphics::init(event_loop).await?;
        let mut parameters = Parameters::default();
        graphics
            .resize(&mut parameters)
            .context("failed to resize the surface")?;
        Ok(Self {
            graphics,
            held_keys: HeldKeys::default(),
            parameters,
            camera: Camera::default(),
            timing: Timing::init(),
        })
    }

    pub fn draw(&mut self) -> Result<()> {
        self.update();
        self.graphics.render()?;
        Ok(())
    }

    fn update(&mut self) {
        let delta_time = self.timing.update(&mut self.parameters);
        self.camera.update(self.held_keys, delta_time);
        self.parameters.update_camera(&self.camera);
        self.graphics.update_parameters_buffer(&self.parameters);
    }

    pub fn resize(&mut self) -> Result<()> {
        self.graphics.resize(&mut self.parameters)
    }

    pub fn handle_key(&mut self, event: &KeyEvent) {
        self.handle_held_keys(event);
        self.handle_trigger_keys(event);
    }

    fn handle_trigger_keys(&mut self, event: &KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }
        macro_rules! handle_keys {
            ($($key:expr => $body:stmt),* $(,)?) => {
                $(
                    if event.logical_key == $key {
                        $body
                        return;
                    }
                )*
            };
        }
        handle_keys!(
            NamedKey::Escape => self.graphics.ungrab_cursor(),
            "+" => self.parameters.update_num_iterations(1),
            "-" => self.parameters.update_num_iterations(-1),
            "n" => self.parameters.update_scene_index(1),
            "b" => self.parameters.update_scene_index(-1),
            "o" => self.camera.reset_orbit_speed(),
            "p" => self.camera.toggle_lock_pitch(),
            "l" => self.camera.cycle_lock_yaw_mode(false),
            "L" => self.camera.cycle_lock_yaw_mode(true),
            "t" => self.timing.stop_time(),
            "r" => self.graphics.try_reload(),
            ">" => self.graphics.update_render_texture_size(1),
            "<" => self.graphics.update_render_texture_size(-1),
        );
    }

    fn handle_held_keys(&mut self, event: &KeyEvent) {
        macro_rules! match_key {
            ($($key:expr => $held_key:expr,)* else => $default:expr $(,)?) => {
                $(if event.logical_key == $key { $held_key } else )*
                { $default }
            };
        }
        let held_key = match_key! {
            "w" => HeldKeys::MoveForward,
            "s" => HeldKeys::MoveBackward,
            "a" => HeldKeys::MoveLeft,
            "d" => HeldKeys::MoveRight,
            "q" => HeldKeys::MoveDown,
            "e" => HeldKeys::MoveUp,
            NamedKey::ArrowDown => HeldKeys::PitchDown,
            NamedKey::ArrowUp => HeldKeys::PitchUp,
            NamedKey::ArrowRight => HeldKeys::YawRight,
            NamedKey::ArrowLeft => HeldKeys::YawLeft,
            NamedKey::Shift => HeldKeys::Shift,
            NamedKey::Control => HeldKeys::Control,
            else => return,
        };
        self.held_keys.set(held_key, event.state.is_pressed());
    }

    pub fn handle_mouse(&mut self, button: MouseButton, state: ElementState) {
        if button == MouseButton::Left && state == ElementState::Pressed {
            self.graphics.grab_cursor();
        }
    }

    pub fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) -> Result<()> {
        const LINE_FACTOR: f32 = 0.5;
        let (mut x, mut y) = match delta {
            MouseScrollDelta::LineDelta(x, y) => (x * LINE_FACTOR, y * LINE_FACTOR),
            MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => (x as f32, y as f32),
        };
        if self.held_keys.is_shift_pressed() {
            x += y;
            y = 0.0;
        }
        if self.held_keys.is_control_pressed() {
            self.timing.update_time_factor(y);
        } else {
            self.camera.update_orbit_speed(x);
            self.camera.update_speed(y);
        }
        Ok(())
    }

    pub fn handle_focused(&mut self, focused: bool) {
        if !focused {
            self.graphics.ungrab_cursor();
        }
    }

    pub fn handle_cursor_movement(&mut self, position: PhysicalPosition<f64>) -> Result<()> {
        let delta = self.graphics.move_cursor(position)?;
        if let Some(delta) = delta {
            let yaw = delta.x;
            let pitch = delta.y;
            self.camera
                .rotate_from_cursor_movement(yaw as f32, pitch as f32);
        }
        Ok(())
    }
}
