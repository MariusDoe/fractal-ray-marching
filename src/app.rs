use crate::state::State;
use anyhow::{Context, Result};
use pollster::block_on;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop,
    window::WindowId,
};

#[derive(Debug, Default)]
pub struct App {
    state: Option<State>,
}

impl App {
    fn state_mut(&mut self, context: &'static str) -> &mut State {
        self.state.as_mut().context(context).unwrap()
    }

    fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: WindowEvent,
    ) -> Result<()> {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.state_mut("got redraw before initialization")
                    .draw()
                    .context("failed to draw")?;
            }
            WindowEvent::Resized(..) => {
                self.state_mut("got resize before initialization")
                    .resize()
                    .context("failed to resize")?;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.state_mut("got keyboard input before initialization")
                    .handle_key(&event)
                    .context("failed to handle keyboard input")?;
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.state_mut("got cursor movement before initialization")
                    .handle_cursor_movement(position)
                    .context("failed to handle cursor movement")?;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.state_mut("got mouse input before initialization")
                    .handle_mouse(button, state)
                    .context("failed to handle mouse input")?;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.state_mut("got mouse wheel before initializtation")
                    .handle_mouse_wheel(delta)
                    .context("failed to handle mouse wheel")?;
            }
            WindowEvent::Focused(focused) => self
                .state_mut("got focused before initialization")
                .handle_focused(focused)
                .context("failed to handle focused")?,
            _ => {}
        }
        Ok(())
    }
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
        self.handle_window_event(event_loop, event).unwrap();
    }
}
