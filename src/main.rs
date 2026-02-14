use crate::app::App;
use winit::event_loop::EventLoop;

mod app;
mod blit_graphics;
mod camera;
mod graphics;
mod held_keys;
mod initialized_app;
mod parameters;
mod persistent_graphics;
mod reloadable_graphics;
mod render_texture_config;
mod timing;
mod utils;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop
        .run_app(&mut App::default())
        .expect("event loop error")
}
