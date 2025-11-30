use crate::{key_state::KeyState, persistent_state::PersistentState, render_state::RenderState};
use anyhow::{Context, Result};
use wgpu::{
    Color, CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, TextureViewDescriptor,
};
use winit::{
    event::KeyEvent,
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Debug)]
pub struct State {
    pub persistent: PersistentState,
    render: RenderState,
    key_state: KeyState,
}

impl State {
    const CLEAR_COLOR: Color = Color::BLACK;

    pub async fn init(event_loop: &ActiveEventLoop) -> Result<Self> {
        let persistent = PersistentState::init(event_loop).await?;
        let render = RenderState::init(&persistent);
        let key_state = KeyState::default();
        Ok(Self {
            persistent,
            render,
            key_state,
        })
    }

    pub fn draw(&mut self) -> Result<()> {
        let mut encoder = self
            .persistent
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());
        let delta_time = self.persistent.update_time();
        self.render.camera.update(self.key_state, delta_time);
        self.persistent
            .parameters
            .update_camera(&self.render.camera);
        self.persistent.queue.write_buffer(
            &self.persistent.parameters_buffer,
            0,
            bytemuck::cast_slice(&[self.persistent.parameters]),
        );
        {
            let view = self
                .persistent
                .render_texture
                .create_view(&TextureViewDescriptor::default());
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Self::CLEAR_COLOR),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.render.render_pipeline);
            render_pass.set_bind_group(0, &self.persistent.parameters_bind_group, &[]);
            render_pass.draw(0..4, 0..1);
        }
        let frame = self
            .persistent
            .surface
            .get_current_texture()
            .context("failed to get frame texture")?;
        {
            let view = frame.texture.create_view(&TextureViewDescriptor::default());
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("blit pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Self::CLEAR_COLOR),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.persistent.blit_render_pipeline);
            render_pass.set_bind_group(0, &self.persistent.blit_bind_group, &[]);
            render_pass.draw(0..4, 0..1);
        }
        self.persistent.queue.submit(Some(encoder.finish()));
        self.persistent.window.pre_present_notify();
        frame.present();
        self.persistent.window.request_redraw();
        Ok(())
    }

    fn handle_movement(&mut self, key: KeyState, pressed: bool) {
        self.key_state.set(key, pressed);
    }

    pub fn handle_key(&mut self, event: KeyEvent) {
        let PhysicalKey::Code(code) = event.physical_key else {
            return;
        };
        let key = match code {
            KeyCode::KeyW => KeyState::MoveForward,
            KeyCode::KeyS => KeyState::MoveBackward,
            KeyCode::KeyA => KeyState::MoveLeft,
            KeyCode::KeyD => KeyState::MoveRight,
            KeyCode::ShiftLeft => KeyState::MoveDown,
            KeyCode::Space => KeyState::MoveUp,
            KeyCode::ArrowDown => KeyState::PitchDown,
            KeyCode::ArrowUp => KeyState::PitchUp,
            KeyCode::ArrowRight => KeyState::YawRight,
            KeyCode::ArrowLeft => KeyState::YawLeft,
            _ => return,
        };
        self.handle_movement(key, event.state.is_pressed());
    }
}
