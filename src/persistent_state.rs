use crate::parameters::Parameters;
use anyhow::{Context, Result};
use std::{
    borrow::Cow,
    sync::Arc,
    time::{Duration, Instant},
};
use wgpu::{
    Adapter, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
    BufferBinding, BufferBindingType, BufferDescriptor, BufferUsages, Device, DeviceDescriptor,
    Instance, InstanceDescriptor, PowerPreference, Queue, RequestAdapterOptions, ShaderModule,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, Surface,
};
use winit::{dpi::PhysicalSize, event_loop::ActiveEventLoop, window::Window};

#[derive(Debug)]
pub struct PersistentState {
    pub window: Arc<Window>,
    pub surface: Surface<'static>,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub vertex_shader: ShaderModule,
    pub parameters: Parameters,
    pub parameters_buffer: Buffer,
    pub parameters_bind_group_layout: BindGroupLayout,
    pub parameters_bind_group: BindGroup,
    start_time: Instant,
    last_frame_time: Instant,
}

impl PersistentState {
    pub async fn init(event_loop: &ActiveEventLoop) -> Result<Self> {
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
            last_frame_time: start_time,
        };
        state.resize().context("failed to resize the surface")?;
        Ok(state)
    }

    pub fn resize(&mut self) -> Result<()> {
        let PhysicalSize { width, height } = self.window.inner_size();
        let config = self
            .surface
            .get_default_config(&self.adapter, width, height)
            .context("failed to get surface config")?;
        self.surface.configure(&self.device, &config);
        self.parameters.update_aspect(width, height);
        Ok(())
    }

    pub fn update_time(&mut self) -> Duration {
        let now = Instant::now();
        let delta_time = now - self.last_frame_time;
        self.parameters.update_time(now - self.start_time);
        self.last_frame_time = now;
        delta_time
    }
}
