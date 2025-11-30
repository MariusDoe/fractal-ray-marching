use crate::{camera::Camera, persistent_state::PersistentState};
use std::borrow::Cow;
use wgpu::{
    FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor,
    ShaderModuleDescriptor, ShaderSource, VertexState,
};

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
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[parameters_bind_group_layout],
            push_constant_ranges: &[],
        });
        let surface_capabilities = surface.get_capabilities(adapter);
        let surface_format = surface_capabilities.formats[0];
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: vertex_shader,
                entry_point: Some("vertex_main"),
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &fragment_shader,
                entry_point: Some("fragment_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(surface_format.into())],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                cull_mode: None,
                ..Default::default()
            },
            multisample: MultisampleState::default(),
            depth_stencil: None,
            multiview: None,
            cache: None,
        });
        let camera = Camera::default();
        Self {
            render_pipeline,
            camera,
        }
    }
}
