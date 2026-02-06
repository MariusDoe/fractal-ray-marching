use crate::{
    camera::Camera, key_state::KeyState, parameters::Parameters, utils::create_render_pipeline,
};
use anyhow::{Context, Result};
use std::{
    borrow::Cow,
    sync::Arc,
    time::{Duration, Instant},
};
use wgpu::{
    Adapter, AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
    BufferBinding, BufferBindingType, BufferDescriptor, BufferUsages, Device, DeviceDescriptor,
    Extent3d, FilterMode, Instance, InstanceDescriptor, PowerPreference, Queue, RenderPipeline,
    RequestAdapterOptions, SamplerBindingType, SamplerDescriptor, ShaderModule,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, Surface, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDimension,
    wgt::TextureViewDescriptor,
};
use winit::{dpi::PhysicalSize, event_loop::ActiveEventLoop, window::Window};

#[derive(Debug)]
pub struct PersistentState {
    pub window: Arc<Window>,
    pub surface: Surface<'static>,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub render_texture: Texture,
    pub blit_bind_group: BindGroup,
    pub blit_render_pipeline: RenderPipeline,
    pub vertex_shader: ShaderModule,
    pub parameters: Parameters,
    pub parameters_buffer: Buffer,
    pub parameters_bind_group_layout: BindGroupLayout,
    pub parameters_bind_group: BindGroup,
    pub camera: Camera,
    start_time: Instant,
    last_frame_time: Instant,
    last_fps_log: Instant,
    frames_since_last_fps_log: u32,
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
        let render_texture_format = TextureFormat::Rgba8UnormSrgb;
        let render_texture = device.create_texture(&TextureDescriptor {
            label: Some("render_texture"),
            dimension: TextureDimension::D2,
            size: Extent3d {
                width: 1920,
                height: 1080,
                ..Default::default()
            },
            mip_level_count: 1,
            sample_count: 1,
            format: render_texture_format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let render_texture_view = render_texture.create_view(&TextureViewDescriptor::default());
        let render_texture_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("render_texture_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });
        let vertex_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("vertex_shader"),
            source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("./vertex.wgsl"))),
        });
        let blit_fragment_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("blit_fragment_shader"),
            source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("./blit.wgsl"))),
        });
        let blit_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("blit_bind_group_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let blit_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("blit_bind_group"),
            layout: &blit_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&render_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&render_texture_sampler),
                },
            ],
        });
        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities.formats[0];
        let blit_render_pipeline = create_render_pipeline(
            &device,
            "blit_render_pipeline_layout",
            &blit_bind_group_layout,
            "blit_render_pipeline",
            &vertex_shader,
            &blit_fragment_shader,
            surface_format,
        );
        let parameters_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("parameters_buffer"),
            mapped_at_creation: false,
            size: size_of::<Parameters>()
                .try_into()
                .context("size_of Parameters is too large for a buffer")?,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let parameters_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("parameters_bind_group_layout"),
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
            label: Some("parameters_bind_group"),
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
            render_texture,
            blit_bind_group,
            blit_render_pipeline,
            vertex_shader,
            parameters: Parameters::default(),
            parameters_buffer,
            parameters_bind_group_layout,
            parameters_bind_group,
            camera: Camera::default(),
            start_time,
            last_frame_time: start_time,
            last_fps_log: start_time,
            frames_since_last_fps_log: 0,
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

    pub fn update(&mut self, key_state: KeyState) {
        let delta_time = self.update_time();
        self.camera.update(key_state, delta_time);
        self.parameters.update_camera(&self.camera);
        self.queue.write_buffer(
            &self.parameters_buffer,
            0,
            bytemuck::cast_slice(&[self.parameters]),
        );
    }

    const FPS_LOG_INTERVAL: Duration = Duration::from_secs(1);

    fn update_time(&mut self) -> Duration {
        let now = Instant::now();
        let delta_time = now - self.last_frame_time;
        self.parameters.update_time(now - self.start_time);
        self.last_frame_time = now;
        self.frames_since_last_fps_log += 1;
        let time_since_last_fps_log = now - self.last_fps_log;
        if time_since_last_fps_log >= Self::FPS_LOG_INTERVAL {
            let fps = self.frames_since_last_fps_log as f32 / time_since_last_fps_log.as_secs_f32();
            eprintln!("{fps:.1} FPS");
            self.last_fps_log = now;
            self.frames_since_last_fps_log = 0;
        }
        delta_time
    }
}
