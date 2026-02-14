use crate::{
    blit_graphics::BlitGraphics,
    persistent_graphics::PersistentGraphics,
    utils::{create_render_pipeline, handle_device_errors},
};
use anyhow::{Context, Result};
use std::{borrow::Cow, fs::read_to_string, path::Path};
use wgpu::{ErrorFilter, RenderPipeline, ShaderModuleDescriptor, ShaderSource};

#[derive(Debug)]
pub struct ReloadableGraphics {
    pub render_pipeline: RenderPipeline,
}

impl ReloadableGraphics {
    pub fn init(persistent: &PersistentGraphics) -> Result<Self> {
        let PersistentGraphics {
            device,
            vertex_shader,
            parameters_bind_group_layout,
            ..
        } = persistent;
        let fragment_shader_source = if cfg!(debug_assertions) {
            let fragment_shader_source_path =
                Path::new(file!()).parent().unwrap().join("./fragment.wgsl");
            Cow::Owned(
                read_to_string(fragment_shader_source_path)
                    .context("failed to read fragment shader source")?,
            )
        } else {
            Cow::Borrowed(include_str!("./fragment.wgsl"))
        };
        let fragment_shader = handle_device_errors(device, ErrorFilter::Validation, || {
            device.create_shader_module(ShaderModuleDescriptor {
                label: Some("fragment_shader"),
                source: ShaderSource::Wgsl(fragment_shader_source),
            })
        })
        .context("failed to validate fragment shader source")?;
        let render_pipeline = create_render_pipeline(
            device,
            "render_pipeline_layout",
            parameters_bind_group_layout,
            "render_pipeline",
            vertex_shader,
            &fragment_shader,
            BlitGraphics::RENDER_TEXTURE_FORMAT,
        );
        Ok(Self { render_pipeline })
    }
}
