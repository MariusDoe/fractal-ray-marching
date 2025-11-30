use anyhow::{Context, Result};
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use cgmath::{InnerSpace, Matrix, Matrix4, Rad, Vector3, Zero};
use pollster::block_on;
use std::{
    borrow::Cow,
    cmp::min,
    sync::Arc,
    time::{Duration, Instant},
};
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
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
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
    last_frame_time: Instant,
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
            last_frame_time: start_time,
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

    fn update_time(&mut self) -> Duration {
        let now = Instant::now();
        let delta_time = now - self.last_frame_time;
        self.parameters.time = (now - self.start_time).as_secs_f32();
        self.last_frame_time = now;
        delta_time
    }
}

#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Parameters {
    camera_matrix: [[f32; 4]; 4],
    aspect_scale: [f32; 2],
    time: f32,
    padding: [u8; 4],
}

impl Parameters {
    fn update_aspect(&mut self, width: u32, height: u32) {
        let min = min(width, height) as f32;
        self.aspect_scale = [width as f32 / min, height as f32 / min];
    }

    fn update_camera(&mut self, camera: &Camera) {
        self.camera_matrix = *camera.to_matrix().transpose().as_ref();
    }
}

#[derive(Debug)]
struct Camera {
    position: Vector3<f32>,
    pitch: Rad<f32>,
    yaw: Rad<f32>,
}

impl Camera {
    fn position_matrix(&self) -> Matrix4<f32> {
        Matrix4::from_translation(self.position)
    }

    fn pitch_matrix(&self) -> Matrix4<f32> {
        Matrix4::from_angle_x(self.pitch)
    }

    fn yaw_matrix(&self) -> Matrix4<f32> {
        Matrix4::from_angle_y(self.yaw)
    }

    fn rotation_matrix(&self) -> Matrix4<f32> {
        self.yaw_matrix() * self.pitch_matrix()
    }

    fn to_matrix(&self) -> Matrix4<f32> {
        self.position_matrix() * self.rotation_matrix()
    }

    const MOVEMENT_PER_SECOND: f32 = 1.5;
    const ROTATION_PER_SECOND: Rad<f32> = Rad(0.5);

    fn forward(&self) -> Vector3<f32> {
        self.rotation_matrix().z.truncate()
    }

    fn right(&self) -> Vector3<f32> {
        self.yaw_matrix().x.truncate()
    }

    fn up(&self) -> Vector3<f32> {
        Vector3::unit_y()
    }

    fn update(&mut self, keys: KeyState, delta_time: Duration) {
        let seconds = delta_time.as_secs_f32();
        let movement = self.forward() * keys.forward_magnitude().into()
            + self.right() * keys.right_magnitude().into()
            + self.up() * keys.up_magnitude().into();
        if !movement.is_zero() {
            self.position += movement.normalize_to(Self::MOVEMENT_PER_SECOND * seconds);
        }
        let rotation_magnitude = Self::ROTATION_PER_SECOND * seconds;
        self.pitch += rotation_magnitude * keys.pitch_magnitude().into();
        self.yaw += rotation_magnitude * keys.yaw_magnitude().into();
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vector3::zero(),
            pitch: Rad::zero(),
            yaw: Rad::zero(),
        }
    }
}

#[derive(Debug)]
struct RenderState {
    render_pipeline: RenderPipeline,
    camera: Camera,
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
        let camera = Camera::default();
        Self {
            render_pipeline,
            camera,
        }
    }
}

bitflags! {
    #[derive(Debug, Default, Clone, Copy)]
    struct KeyState: u16 {
        const MoveForward = 1 << 0;
        const MoveBackward = 1 << 1;
        const MoveRight = 1 << 2;
        const MoveLeft = 1 << 3;
        const MoveUp = 1 << 4;
        const MoveDown = 1 << 5;
        const PitchUp = 1 << 6;
        const PitchDown = 1 << 7;
        const YawRight = 1 << 8;
        const YawLeft = 1 << 9;
    }
}

type Magnitude = i8;

impl KeyState {
    fn magnitude(&self, positive: Self, negative: Self) -> Magnitude {
        Magnitude::from(self.contains(positive)) - Magnitude::from(self.contains(negative))
    }

    fn forward_magnitude(&self) -> Magnitude {
        self.magnitude(Self::MoveForward, Self::MoveBackward)
    }

    fn right_magnitude(&self) -> Magnitude {
        self.magnitude(Self::MoveRight, Self::MoveLeft)
    }

    fn up_magnitude(&self) -> Magnitude {
        self.magnitude(Self::MoveUp, Self::MoveDown)
    }

    fn pitch_magnitude(&self) -> Magnitude {
        self.magnitude(Self::PitchDown, Self::PitchUp)
    }

    fn yaw_magnitude(&self) -> Magnitude {
        self.magnitude(Self::YawRight, Self::YawLeft)
    }
}

#[derive(Debug)]
struct State {
    persistent: PersistentState,
    render: RenderState,
    key_state: KeyState,
}

impl State {
    const CLEAR_COLOR: Color = Color::BLACK;

    async fn init(event_loop: &ActiveEventLoop) -> Result<Self> {
        let persistent = PersistentState::init(event_loop).await?;
        let render = RenderState::init(&persistent);
        let key_state = KeyState::default();
        Ok(Self {
            persistent,
            render,
            key_state,
        })
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
        let delta_time = self.persistent.update_time();
        self.render.camera.update(self.key_state, delta_time);
        self.persistent
            .parameters
            .update_camera(&self.render.camera);
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

    fn handle_movement(&mut self, key: KeyState, pressed: bool) {
        self.key_state.set(key, pressed);
    }

    fn handle_key(&mut self, event: KeyEvent) {
        let PhysicalKey::Code(code) = event.physical_key else {
            return;
        };
        let key = match code {
            KeyCode::KeyW => KeyState::MoveForward,
            KeyCode::KeyS => KeyState::MoveBackward,
            KeyCode::KeyA => KeyState::MoveLeft,
            KeyCode::KeyD => KeyState::MoveRight,
            KeyCode::ShiftLeft => KeyState::MoveDown,
            KeyCode::Space => KeyState::MoveUp,
            KeyCode::ArrowDown => KeyState::PitchDown,
            KeyCode::ArrowUp => KeyState::PitchUp,
            KeyCode::ArrowRight => KeyState::YawRight,
            KeyCode::ArrowLeft => KeyState::YawLeft,
            _ => return,
        };
        self.handle_movement(key, event.state.is_pressed());
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
            WindowEvent::KeyboardInput { event, .. } => {
                self.state
                    .as_mut()
                    .context("got keyboard input before initialization")
                    .unwrap()
                    .handle_key(event);
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
