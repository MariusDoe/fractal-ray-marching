use crate::initialized_app::InitializedApp;
use anyhow::{Context, Result};
use pollster::block_on;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop,
    window::WindowId,
};

#[derive(Debug, Default)]
pub struct App {
    initialized: Option<InitializedApp>,
}

impl App {
    fn initialized_mut(&mut self, context: &'static str) -> &mut InitializedApp {
        self.initialized.as_mut().context(context).unwrap()
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
                self.initialized_mut("got redraw before initialization")
                    .draw()
                    .context("failed to draw")?;
            }
            WindowEvent::Resized(..) => {
                self.initialized_mut("got resize before initialization")
                    .resize()
                    .context("failed to resize")?;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.initialized_mut("got keyboard input before initialization")
                    .handle_key(&event);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.initialized_mut("got cursor movement before initialization")
                    .handle_cursor_movement(position)
                    .context("failed to handle cursor movement")?;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.initialized_mut("got mouse input before initialization")
                    .handle_mouse(button, state);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.initialized_mut("got mouse wheel before initializtation")
                    .handle_mouse_wheel(delta)
                    .context("failed to handle mouse wheel")?;
            }
            WindowEvent::Focused(focused) => self
                .initialized_mut("got focused before initialization")
                .handle_focused(focused),
            _ => {}
        }
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.initialized = Some(
            block_on(InitializedApp::init(event_loop))
                .context("failed to initialize app")
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
