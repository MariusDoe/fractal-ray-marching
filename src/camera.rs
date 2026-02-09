use crate::{key_state::KeyState, utils::limited_quadratric_delta};
use cgmath::{Angle, InnerSpace, Matrix3, Matrix4, Rad, Vector2, Vector3, Zero, num_traits::clamp};
use std::{f32::consts::FRAC_PI_2, time::Duration};

#[derive(Debug)]
pub struct Camera {
    movement_per_second: f32,
    orbit_angle_per_second: Rad<f32>,
    lock_yaw_mode: LockYawMode,
    lock_pitch: bool,
    position: Vector3<f32>,
    pitch: Rad<f32>,
    yaw: Rad<f32>,
}

#[derive(Debug)]
enum LockYawMode {
    None,
    Inwards,
    Right,
    Outwards,
    Left,
}

impl Camera {
    fn position_matrix(&self) -> Matrix4<f32> {
        Matrix4::from_translation(self.position)
    }

    fn pitch_matrix(&self) -> Matrix4<f32> {
        Matrix4::from_angle_x(self.pitch)
    }

    fn yaw_matrix(&self) -> Matrix4<f32> {
        Matrix4::from_angle_y(self.yaw)
    }

    fn rotation_matrix(&self) -> Matrix4<f32> {
        self.yaw_matrix() * self.pitch_matrix()
    }

    pub fn to_matrix(&self) -> Matrix4<f32> {
        self.position_matrix() * self.rotation_matrix()
    }

    const ROTATION_PER_SECOND: Rad<f32> = Rad(0.5);

    fn forward(&self) -> Vector3<f32> {
        self.yaw_matrix().z.truncate()
    }

    fn right(&self) -> Vector3<f32> {
        self.yaw_matrix().x.truncate()
    }

    fn up(&self) -> Vector3<f32> {
        Vector3::unit_y()
    }

    pub fn update_speed(&mut self, delta: f32) {
        self.movement_per_second *= (delta * 0.1).exp();
    }

    pub fn update_orbit_speed(&mut self, delta: f32) {
        self.orbit_angle_per_second += Rad(limited_quadratric_delta(
            self.orbit_angle_per_second.0,
            delta,
            0.025,
            0.0001,
            0.1,
            0.2,
        ));
    }

    pub fn reset_orbit_speed(&mut self) {
        self.orbit_angle_per_second = Rad::zero();
    }

    pub fn toggle_lock_pitch(&mut self) {
        self.lock_pitch = !self.lock_pitch;
    }

    pub fn cycle_lock_yaw_mode(&mut self, backwards: bool) {
        use LockYawMode::*;
        self.lock_yaw_mode = if backwards {
            match self.lock_yaw_mode {
                None => Left,
                Inwards => None,
                Right => Inwards,
                Outwards => Right,
                Left => Outwards,
            }
        } else {
            match self.lock_yaw_mode {
                None => Inwards,
                Inwards => Right,
                Right => Outwards,
                Outwards => Left,
                Left => None,
            }
        };
    }

    pub fn update(&mut self, keys: KeyState, delta_time: Duration) {
        let seconds = delta_time.as_secs_f32();
        self.do_movement(keys, seconds);
        self.do_orbit(seconds);
        self.do_lock_rotation();
    }

    fn do_movement(&mut self, keys: KeyState, seconds: f32) {
        let movement = self.forward() * keys.forward_magnitude().into()
            + self.right() * keys.right_magnitude().into()
            + self.up() * keys.up_magnitude().into();
        if !movement.is_zero() {
            self.position += movement.normalize_to(self.movement_per_second * seconds);
        }
        let rotation_magnitude = Self::ROTATION_PER_SECOND * seconds;
        self.add_pitch(rotation_magnitude * keys.pitch_magnitude().into());
        self.add_yaw(rotation_magnitude * keys.yaw_magnitude().into());
    }

    fn do_orbit(&mut self, seconds: f32) {
        let rotation = Matrix3::from_angle_y(self.orbit_angle_per_second * seconds);
        self.position = rotation * self.position;
    }

    fn do_lock_rotation(&mut self) {
        self.do_lock_yaw();
        self.do_lock_pitch();
    }

    fn do_lock_yaw(&mut self) {
        let offset = match self.lock_yaw_mode {
            LockYawMode::None => return,
            LockYawMode::Inwards => -Rad::full_turn() / 2.0,
            LockYawMode::Right => -Rad::full_turn() / 4.0,
            LockYawMode::Outwards => Rad::zero(),
            LockYawMode::Left => Rad::full_turn() / 4.0,
        };
        self.yaw = Rad::atan2(self.position.x, self.position.z) + offset;
    }

    fn do_lock_pitch(&mut self) {
        if !self.lock_pitch {
            return;
        }
        let xz = Vector2::new(self.position.x, self.position.z);
        let radius = xz.magnitude();
        self.pitch = Rad::atan2(self.position.y, radius);
    }

    const ROTATION_PER_PIXEL: Rad<f32> = Rad(0.0003);

    pub fn rotate_from_cursor_movement(&mut self, yaw_pixels: f32, pitch_pixels: f32) {
        self.add_pitch(Self::ROTATION_PER_PIXEL * pitch_pixels);
        self.add_yaw(Self::ROTATION_PER_PIXEL * yaw_pixels);
    }

    const MAX_PITCH: Rad<f32> = Rad(FRAC_PI_2);
    const MIN_PITCH: Rad<f32> = Rad(-Self::MAX_PITCH.0);

    fn add_pitch(&mut self, pitch: Rad<f32>) {
        self.update_pitch(self.pitch + pitch);
    }

    fn add_yaw(&mut self, yaw: Rad<f32>) {
        self.update_yaw(self.yaw + yaw);
    }

    fn update_pitch(&mut self, pitch: Rad<f32>) {
        self.pitch = clamp(pitch, Self::MIN_PITCH, Self::MAX_PITCH);
    }

    fn update_yaw(&mut self, yaw: Rad<f32>) {
        self.yaw = yaw % Rad::full_turn();
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            movement_per_second: 1.0,
            orbit_angle_per_second: Rad::zero(),
            lock_pitch: false,
            lock_yaw_mode: LockYawMode::None,
            position: Vector3::new(0.0, 0.0, -1.0),
            pitch: Rad::zero(),
            yaw: Rad::zero(),
        }
    }
}
