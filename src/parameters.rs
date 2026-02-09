use crate::camera::Camera;
use bytemuck::{Pod, Zeroable};
use cgmath::Matrix;
use std::{cmp::min, time::Duration};

#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Parameters {
    camera_matrix: [[f32; 4]; 4],
    aspect_scale: [f32; 2],
    time: f32,
    num_iterations: u32,
    scene_index: u32,
    padding: [u8; 12],
}

impl Parameters {
    pub fn update_aspect(&mut self, width: u32, height: u32) {
        let min = min(width, height) as f32;
        self.aspect_scale = [width as f32 / min, height as f32 / min];
    }

    pub fn update_camera(&mut self, camera: &Camera) {
        self.camera_matrix = *camera.to_matrix().transpose().as_ref();
    }

    pub fn update_time(&mut self, time: Duration) {
        self.time = time.as_secs_f32();
    }

    pub fn update_num_iterations(&mut self, delta: i32) {
        self.num_iterations = self.num_iterations.saturating_add_signed(delta);
    }

    const NUM_SCENES: u32 = 19;

    pub fn update_scene_index(&mut self, delta: i32) {
        self.scene_index =
            (self.scene_index as i32 + delta).rem_euclid(Self::NUM_SCENES as i32) as u32;
    }
}
