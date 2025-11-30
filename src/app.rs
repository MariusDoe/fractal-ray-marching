use crate::state::State;
use anyhow::Context;
use pollster::block_on;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop,
    window::WindowId,
};

#[derive(Debug, Default)]
pub struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.state = Some(
            block_on(State::init(event_loop))
                .context("failed to initialize state")
                .unwrap(),
        );
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.state
                    .as_mut()
                    .context("got redraw before initialization")
                    .unwrap()
                    .draw()
                    .context("failed to draw")
                    .unwrap();
            }
            WindowEvent::Resized(..) => {
                self.state
                    .as_mut()
                    .context("got resize before initialization")
                    .unwrap()
                    .persistent
                    .resize()
                    .context("failed to resize")
                    .unwrap();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.state
                    .as_mut()
                    .context("got keyboard input before initialization")
                    .unwrap()
                    .handle_key(event);
            }
            _ => {}
        }
    }
}
