use wgpu::{
    BindGroupLayout, Device, FragmentState, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, TextureFormat, VertexState,
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
