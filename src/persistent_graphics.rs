use crate::{parameters::Parameters, utils::create_render_pipeline};
use anyhow::{Context, Ok, Result};
use std::{borrow::Cow, sync::Arc};
use wgpu::{
    Adapter, AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
    BufferBinding, BufferBindingType, BufferDescriptor, BufferUsages, Device, DeviceDescriptor,
    FilterMode, Instance, InstanceDescriptor, PowerPreference, Queue, RenderPipeline,
    RequestAdapterOptions, Sampler, SamplerBindingType, SamplerDescriptor, ShaderModule,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, Surface, TextureSampleType,
    TextureViewDimension,
};
use winit::{
    dpi::PhysicalSize,
    event_loop::ActiveEventLoop,
    window::{CursorGrabMode, Window},
};

#[derive(Debug)]
pub struct PersistentGraphics {
    pub window: Arc<Window>,
    pub surface: Surface<'static>,
    adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub render_texture_sampler: Sampler,
    pub blit_bind_group_layout: BindGroupLayout,
    pub blit_render_pipeline: RenderPipeline,
    pub vertex_shader: ShaderModule,
    parameters_buffer: Buffer,
    pub parameters_bind_group_layout: BindGroupLayout,
    pub parameters_bind_group: BindGroup,
    pub is_cursor_grabbed: bool,
}

impl PersistentGraphics {
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
        Ok(Self {
            window,
            surface,
            adapter,
            device,
            queue,
            render_texture_sampler,
            blit_bind_group_layout,
            blit_render_pipeline,
            vertex_shader,
            parameters_buffer,
            parameters_bind_group_layout,
            parameters_bind_group,
            is_cursor_grabbed: false,
        })
    }

    pub fn resize(&self, parameters: &mut Parameters) -> Result<()> {
        let PhysicalSize { width, height } = self.window.inner_size();
        let config = self
            .surface
            .get_default_config(&self.adapter, width, height)
            .context("failed to get surface config")?;
        self.surface.configure(&self.device, &config);
        parameters.update_aspect(width, height);
        Ok(())
    }

    pub fn update_parameters_buffer(&self, parameters: &Parameters) {
        self.queue.write_buffer(
            &self.parameters_buffer,
            0,
            bytemuck::cast_slice(&[*parameters]),
        );
    }

    pub fn grab_cursor(&mut self) -> Result<()> {
        if self.is_cursor_grabbed {
            return Ok(());
        }
        const CURSOR_GRAB_MODE: CursorGrabMode = if cfg!(target_os = "macos") {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::Confined
        };
        self.window
            .set_cursor_grab(CURSOR_GRAB_MODE)
            .context("failed to grab cursor")?;
        self.window.set_cursor_visible(false);
        self.is_cursor_grabbed = true;
        Ok(())
    }

    pub fn ungrab_cursor(&mut self) -> Result<()> {
        if !self.is_cursor_grabbed {
            return Ok(());
        }
        self.window
            .set_cursor_grab(CursorGrabMode::None)
            .context("failed to ungrab cursor")?;
        self.window.set_cursor_visible(true);
        self.is_cursor_grabbed = false;
        Ok(())
    }
}
