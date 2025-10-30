use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use pollster::block_on;
use std::{borrow::Cow, cmp::min, sync::Arc, time::Instant};
use wgpu::{
    Adapter, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
    BufferBinding, BufferBindingType, BufferUsages, Color, Device, FragmentState, Instance,
    InstanceDescriptor, LoadOp, MultisampleState, Operations, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PowerPreference, PrimitiveState, PrimitiveTopology, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    RequestAdapterOptions, ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStages,
    StoreOp, Surface, VertexState,
    wgt::{BufferDescriptor, CommandEncoderDescriptor, DeviceDescriptor, TextureViewDescriptor},
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[derive(Debug)]
struct PersistentState {
    window: Arc<Window>,
    surface: Surface<'static>,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    vertex_shader: ShaderModule,
    parameters: Parameters,
    parameters_buffer: Buffer,
    parameters_bind_group_layout: BindGroupLayout,
    parameters_bind_group: BindGroup,
    start_time: Instant,
}

impl PersistentState {
    async fn init(event_loop: &ActiveEventLoop) -> Result<Self> {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("Fractals"))
                .context("failed to create window")?,
        );
        let instance = Instance::new(&InstanceDescriptor::from_env_or_default());
        let surface = instance
            .create_surface(window.clone())
            .context("failed to create surface")?;
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("failed to request adapter")?;
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default())
            .await
            .context("failed to request device")?;
        let vertex_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("vertex_shader"),
            source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("./vertex.wgsl"))),
        });
        let parameters_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            mapped_at_creation: false,
            size: size_of::<Parameters>()
                .try_into()
                .context("size_of Parameters is too large for a buffer")?,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let parameters_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let parameters_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &parameters_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &parameters_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        let start_time = Instant::now();
        let mut state = Self {
            window,
            surface,
            adapter,
            device,
            queue,
            vertex_shader,
            parameters: Parameters::default(),
            parameters_buffer,
            parameters_bind_group_layout,
            parameters_bind_group,
            start_time,
        };
        state.resize().context("failed to resize the surface")?;
        Ok(state)
    }

    fn resize(&mut self) -> Result<()> {
        let PhysicalSize { width, height } = self.window.inner_size();
        let config = self
            .surface
            .get_default_config(&self.adapter, width, height)
            .context("failed to get surface config")?;
        self.surface.configure(&self.device, &config);
        self.parameters.update_aspect(width, height);
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Parameters {
    aspect_scale: [f32; 2],
    time: f32,
    padding: [u8; 4],
}

impl Parameters {
    fn update_aspect(&mut self, width: u32, height: u32) {
        let min = min(width, height) as f32;
        self.aspect_scale = [width as f32 / min, height as f32 / min];
    }

    fn update_time(&mut self, start_time: Instant) {
        self.time = (Instant::now() - start_time).as_secs_f32();
    }
}

#[derive(Debug)]
struct RenderState {
    render_pipeline: RenderPipeline,
}

impl RenderState {
    fn init(persistent: &PersistentState) -> Self {
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
            label: None,
            bind_group_layouts: &[parameters_bind_group_layout],
            push_constant_ranges: &[],
        });
        let surface_capabilities = surface.get_capabilities(adapter);
        let surface_format = surface_capabilities.formats[0];
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
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
        Self { render_pipeline }
    }
}

#[derive(Debug)]
struct State {
    persistent: PersistentState,
    render: RenderState,
}

impl State {
    const CLEAR_COLOR: Color = Color::BLACK;

    async fn init(event_loop: &ActiveEventLoop) -> Result<Self> {
        let persistent = PersistentState::init(event_loop).await?;
        let render = RenderState::init(&persistent);
        Ok(Self { persistent, render })
    }

    fn draw(&mut self) -> Result<()> {
        let frame = self
            .persistent
            .surface
            .get_current_texture()
            .context("failed to get frame texture")?;
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .persistent
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());
        self.persistent
            .parameters
            .update_time(self.persistent.start_time);
        self.persistent.queue.write_buffer(
            &self.persistent.parameters_buffer,
            0,
            bytemuck::cast_slice(&[self.persistent.parameters]),
        );
        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
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
        self.persistent.queue.submit(Some(encoder.finish()));
        self.persistent.window.pre_present_notify();
        frame.present();
        self.persistent.window.request_redraw();
        Ok(())
    }
}

#[derive(Debug, Default)]
struct App {
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
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop
        .run_app(&mut App::default())
        .expect("event loop error")
}
