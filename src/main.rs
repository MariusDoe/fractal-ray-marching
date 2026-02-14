use crate::app::App;
use winit::event_loop::EventLoop;

mod app;
mod blit_state;
mod camera;
mod held_keys;
mod parameters;
mod persistent_state;
mod render_state;
mod state;
mod utils;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop
        .run_app(&mut App::default())
        .expect("event loop error")
}
