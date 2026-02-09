use anyhow::Result;
use pollster::block_on;
use wgpu::{
    BindGroupLayout, Device, Error, ErrorFilter, FragmentState, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
    RenderPipeline, RenderPipelineDescriptor, ShaderModule, TextureFormat, VertexState,
};

pub fn create_render_pipeline(
    device: &Device,
    layout_label: &'static str,
    bind_group_layout: &BindGroupLayout,
    label: &'static str,
    vertex_shader: &ShaderModule,
    fragment_shader: &ShaderModule,
    texture_format: TextureFormat,
) -> RenderPipeline {
    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some(layout_label),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: VertexState {
            module: vertex_shader,
            entry_point: Some("vertex_main"),
            buffers: &[],
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: fragment_shader,
            entry_point: Some("fragment_main"),
            compilation_options: PipelineCompilationOptions::default(),
            targets: &[Some(texture_format.into())],
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
    })
}

pub fn handle_device_errors<F, R>(device: &Device, filter: ErrorFilter, f: F) -> Result<R, Error>
where
    F: FnOnce() -> R,
{
    device.push_error_scope(filter);
    let result = f();
    match block_on(device.pop_error_scope()) {
        Some(error) => Err(error),
        None => Ok(result),
    }
}

pub fn limited_quadratric_delta(
    current: f32,
    delta: f32,
    kickoff: f32,
    min: f32,
    max: f32,
    linear: f32,
) -> f32 {
    let factor = if current == 0.0 {
        kickoff
    } else {
        current.abs().clamp(min, max)
    };
    linear * delta * factor
}
