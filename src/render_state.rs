use crate::{camera::Camera, persistent_state::PersistentState, utils::create_render_pipeline};
use std::borrow::Cow;
use wgpu::{RenderPipeline, ShaderModuleDescriptor, ShaderSource};

#[derive(Debug)]
pub struct RenderState {
    pub render_pipeline: RenderPipeline,
    pub camera: Camera,
}

impl RenderState {
    pub fn init(persistent: &PersistentState) -> Self {
        let PersistentState {
            device,
            surface,
            adapter,
            vertex_shader,
            parameters_bind_group_layout,
            ..
        } = persistent;
        let fragment_shader_source = include_str!("./fragment.wgsl");
        let fragment_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("fragment shader"),
            source: ShaderSource::Wgsl(Cow::Borrowed(fragment_shader_source)),
        });
        let surface_capabilities = surface.get_capabilities(adapter);
        let surface_format = surface_capabilities.formats[0];
        let render_pipeline = create_render_pipeline(
            device,
            "render_pipeline_layout",
            parameters_bind_group_layout,
            "render_pipeline",
            vertex_shader,
            &fragment_shader,
            surface_format,
        );
        let camera = Camera::default();
        Self {
            render_pipeline,
            camera,
        }
    }
}
