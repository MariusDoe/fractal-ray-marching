use crate::key_state::KeyState;
use cgmath::{Angle, InnerSpace, Matrix4, Rad, Vector3, Zero, num_traits::clamp};
use std::{f32::consts::FRAC_PI_2, time::Duration};

#[derive(Debug)]
pub struct Camera {
    movement_per_second: f32,
    position: Vector3<f32>,
    pitch: Rad<f32>,
    yaw: Rad<f32>,
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
        self.movement_per_second *= (delta * 0.05).exp();
    }

    pub fn update(&mut self, keys: KeyState, delta_time: Duration) {
        let seconds = delta_time.as_secs_f32();
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
            position: Vector3::zero(),
            pitch: Rad::zero(),
            yaw: Rad::zero(),
        }
    }
}
