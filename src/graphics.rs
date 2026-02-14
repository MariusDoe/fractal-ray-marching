use crate::{
    blit_graphics::BlitGraphics, parameters::Parameters, persistent_graphics::PersistentGraphics,
    reloadable_graphics::ReloadableGraphics, render_texture_config::RenderTextureConfig,
};
use anyhow::{Context, Ok, Result};
use wgpu::{
    BindGroup, Color, CommandEncoder, CommandEncoderDescriptor, LoadOp, Operations,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, StoreOp, SurfaceTexture,
    TextureView, TextureViewDescriptor,
};
use winit::{dpi::PhysicalPosition, event_loop::ActiveEventLoop};

#[derive(Debug)]
pub struct Graphics {
    persistent: PersistentGraphics,
    reloadable: ReloadableGraphics,
    blit: BlitGraphics,
    render_texture_config: RenderTextureConfig,
    last_cursor_position: Option<PhysicalPosition<f64>>,
}

impl Graphics {
    const CLEAR_COLOR: Color = Color::BLACK;

    pub async fn init(event_loop: &ActiveEventLoop) -> Result<Self> {
        let persistent = PersistentGraphics::init(event_loop).await?;
        let render_texture_config = RenderTextureConfig::default();
        let reloadable = ReloadableGraphics::init(&persistent)?;
        let blit = BlitGraphics::init(&persistent, &render_texture_config);
        Ok(Self {
            persistent,
            reloadable,
            blit,
            render_texture_config,
            last_cursor_position: None,
        })
    }

    pub fn try_reload(&mut self) {
        if let Err(error) = self.reload() {
            println!("{error:?}");
        }
    }

    fn reload(&mut self) -> Result<()> {
        self.reloadable = ReloadableGraphics::init(&self.persistent).context("failed to reload")?;
        Ok(())
    }

    pub fn resize(&self, parameters: &mut Parameters) -> Result<()> {
        self.persistent.resize(parameters)
    }

    pub fn update_render_texture_size(&mut self, delta: i32) {
        self.render_texture_config.update_render_texture_size(delta);
        self.blit = BlitGraphics::init(&self.persistent, &self.render_texture_config);
    }

    pub fn update_parameters_buffer(&mut self, parameters: &Parameters) {
        self.persistent.update_parameters_buffer(parameters)
    }

    pub fn move_cursor(
        &mut self,
        position: PhysicalPosition<f64>,
    ) -> Result<Option<PhysicalPosition<f64>>> {
        if self.persistent.is_cursor_grabbed
            && let Some(last_position) = self.last_cursor_position
        {
            let x = position.x - last_position.x;
            let y = position.y - last_position.y;
            self.persistent
                .window
                .set_cursor_position(last_position)
                .context("failed to lock cursor in place")?;
            Ok(Some(PhysicalPosition { x, y }))
        } else {
            self.last_cursor_position = Some(position);
            Ok(None)
        }
    }

    pub fn grab_cursor(&mut self) -> Result<()> {
        self.persistent.grab_cursor()
    }

    pub fn ungrab_cursor(&mut self) -> Result<()> {
        self.persistent.ungrab_cursor()
    }

    pub fn render(&self) -> Result<()> {
        let PersistentGraphics {
            device,
            surface,
            queue,
            window,
            ..
        } = &self.persistent;
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
        self.do_render_texture_pass(&mut encoder);
        let frame = surface
            .get_current_texture()
            .context("failed to get frame texture")?;
        self.do_blit_pass(&mut encoder, &frame);
        queue.submit(Some(encoder.finish()));
        window.pre_present_notify();
        frame.present();
        window.request_redraw();
        Ok(())
    }

    fn do_render_texture_pass(&self, encoder: &mut CommandEncoder) {
        let render_texture_view = self
            .blit
            .render_texture
            .create_view(&TextureViewDescriptor::default());
        Self::do_render_pass(
            encoder,
            "render_pass",
            &render_texture_view,
            &self.reloadable.render_pipeline,
            &self.persistent.parameters_bind_group,
        );
    }

    fn do_blit_pass(&self, encoder: &mut CommandEncoder, frame: &SurfaceTexture) {
        let frame_texture_view = frame.texture.create_view(&TextureViewDescriptor::default());
        Self::do_render_pass(
            encoder,
            "blit_render_pass",
            &frame_texture_view,
            &self.persistent.blit_render_pipeline,
            &self.blit.blit_bind_group,
        );
    }

    fn do_render_pass(
        encoder: &mut CommandEncoder,
        label: &'static str,
        view: &TextureView,
        render_pipeline: &RenderPipeline,
        bind_group: &BindGroup,
    ) {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
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
        render_pass.set_pipeline(render_pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        let vertices = 0..4; // a quad
        let single_instance = 0..1;
        render_pass.draw(vertices, single_instance);
    }
}
