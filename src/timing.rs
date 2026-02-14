use crate::{parameters::Parameters, utils::limited_quadratric_delta};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Timing {
    time_factor: f32,
    last_frame_time: Instant,
    last_fps_log: Instant,
    frames_since_last_fps_log: u32,
}

impl Timing {
    pub fn init() -> Self {
        let start_time = Instant::now();
        Self {
            time_factor: 1.0,
            last_frame_time: start_time,
            last_fps_log: start_time,
            frames_since_last_fps_log: 0,
        }
    }

    pub fn update(&mut self, parameters: &mut Parameters) -> Duration {
        let now = Instant::now();
        let delta_time = now - self.last_frame_time;
        self.last_frame_time = now;
        parameters.update_time(self.time_factor * delta_time.as_secs_f32());
        self.update_fps(now);
        delta_time
    }

    pub fn update_time_factor(&mut self, delta: f32) {
        self.time_factor += limited_quadratric_delta(self.time_factor, delta);
    }

    pub fn stop_time(&mut self) {
        self.time_factor = 0.0;
    }

    const FPS_LOG_INTERVAL: Duration = Duration::from_secs(1);

    fn update_fps(&mut self, now: Instant) {
        self.frames_since_last_fps_log += 1;
        let time_since_last_fps_log = now - self.last_fps_log;
        if time_since_last_fps_log >= Self::FPS_LOG_INTERVAL {
            let fps = self.frames_since_last_fps_log as f32 / time_since_last_fps_log.as_secs_f32();
            eprintln!("{fps:.1} FPS");
            self.last_fps_log = now;
            self.frames_since_last_fps_log = 0;
        }
    }
}
