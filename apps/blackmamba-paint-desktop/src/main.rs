use bmp_brush::{BrushDefinition, BrushLibrary};
use bmp_core::{Camera2D, Document, Layer};
use bmp_input::{PointerKind, PointerSample, StrokeCapture};
use bmp_render::{build_render_scene, RenderDab, ScreenViewport};
use bytemuck::{Pod, Zeroable};
use std::{error::Error, sync::Arc, time::Instant};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    position: [f32; 2],
    local: [f32; 2],
    opacity: f32,
}

impl GpuVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
}

impl GpuState {
    async fn new(window: Arc<Window>) -> Result<Self, Box<dyn Error>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("BlackMamba Paint GPU device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await?;

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        let config = surface
            .get_default_config(&adapter, width, height)
            .ok_or("GPU adapter cannot present to the BlackMamba Paint window")?;
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("BlackMamba Paint dab shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("dab.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("BlackMamba Paint dab pipeline layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("BlackMamba Paint dab pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(GpuVertex::layout())],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    fn reconfigure(&self) {
        self.surface.configure(&self.device, &self.config);
    }

    fn render(
        &mut self,
        document: &Document,
        brushes: &BrushLibrary,
    ) -> Result<(), wgpu::SurfaceError> {
        let viewport = ScreenViewport::new(
            f64::from(self.config.width),
            f64::from(self.config.height),
        )
        .expect("configured GPU surface has positive dimensions");
        let scene = build_render_scene(document, document.camera, viewport, brushes)
            .expect("document camera and viewport are valid");
        let vertices = dabs_to_vertices(&scene.dabs, viewport);

        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("BlackMamba Paint frame encoder"),
            });
        let vertex_buffer = (!vertices.is_empty()).then(|| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("BlackMamba Paint dab vertices"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("BlackMamba Paint canvas pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.96,
                            g: 0.955,
                            b: 0.94,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(buffer) = &vertex_buffer {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..vertices.len() as u32, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}

struct DesktopApp {
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
    document: Document,
    layer_id: uuid::Uuid,
    brushes: BrushLibrary,
    capture: Option<StrokeCapture>,
    cursor: Option<(f64, f64)>,
    right_drag: bool,
    started_at: Instant,
}

impl DesktopApp {
    fn new() -> Self {
        let mut document = Document::new("BlackMamba Paint — Desktop Canvas");
        let layer_id = document.add_layer(Layer::new("ink"));
        let mut brushes = BrushLibrary::default();
        brushes
            .insert(BrushDefinition::pencil("graphite", "Graphite", 8.0))
            .expect("built-in brush id is valid");

        Self {
            window: None,
            gpu: None,
            document,
            layer_id,
            brushes,
            capture: None,
            cursor: None,
            right_drag: false,
            started_at: Instant::now(),
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn viewport(&self) -> Option<ScreenViewport> {
        let gpu = self.gpu.as_ref()?;
        ScreenViewport::new(f64::from(gpu.config.width), f64::from(gpu.config.height)).ok()
    }

    fn screen_to_world(&self, screen_x: f64, screen_y: f64) -> Option<(f64, f64)> {
        let viewport = self.viewport()?;
        let camera = self.document.camera;
        let rotated_x = (screen_x - viewport.width * 0.5) / camera.zoom;
        let rotated_y = (screen_y - viewport.height * 0.5) / camera.zoom;
        let cosine = camera.rotation_rad.cos();
        let sine = camera.rotation_rad.sin();
        let dx = rotated_x * cosine - rotated_y * sine;
        let dy = rotated_x * sine + rotated_y * cosine;
        Some((camera.x + dx, camera.y + dy))
    }

    fn pointer_sample(&self, screen_x: f64, screen_y: f64) -> Option<PointerSample> {
        let (x, y) = self.screen_to_world(screen_x, screen_y)?;
        Some(PointerSample {
            kind: PointerKind::Mouse,
            x,
            y,
            pressure: 1.0,
            tilt_x: 0.0,
            tilt_y: 0.0,
            timestamp_ms: self.started_at.elapsed().as_millis() as u64,
        })
    }

    fn pan_by_screen_delta(&mut self, dx: f64, dy: f64) {
        let camera = &mut self.document.camera;
        let rx = dx / camera.zoom;
        let ry = dy / camera.zoom;
        let cosine = camera.rotation_rad.cos();
        let sine = camera.rotation_rad.sin();
        let world_dx = rx * cosine - ry * sine;
        let world_dy = rx * sine + ry * cosine;
        camera.x -= world_dx;
        camera.y -= world_dy;
    }

    fn zoom_by(&mut self, steps: f64) {
        let factor = 1.1_f64.powf(steps.clamp(-20.0, 20.0));
        self.document.camera.zoom = (self.document.camera.zoom * factor).clamp(0.02, 5_000.0);
    }
}

impl ApplicationHandler for DesktopApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("BlackMamba Paint")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("failed to create BlackMamba Paint window: {error}");
                event_loop.exit();
                return;
            }
        };

        match pollster::block_on(GpuState::new(window.clone())) {
            Ok(gpu) => {
                self.gpu = Some(gpu);
                self.window = Some(window);
                self.request_redraw();
            }
            Err(error) => {
                eprintln!("failed to initialize BlackMamba Paint GPU: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = &self.window else {
            return;
        };
        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
                self.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let next = (position.x, position.y);
                if let Some(previous) = self.cursor {
                    if self.right_drag {
                        self.pan_by_screen_delta(next.0 - previous.0, next.1 - previous.1);
                    }
                }
                if let Some(sample) = self.pointer_sample(next.0, next.1) {
                    if let Some(capture) = &mut self.capture {
                        capture.push(sample);
                    }
                }
                self.cursor = Some(next);
                self.request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => match (state, button) {
                (ElementState::Pressed, MouseButton::Left) => {
                    let mut capture = StrokeCapture::begin(self.layer_id, "graphite");
                    if let Some((x, y)) = self.cursor {
                        if let Some(sample) = self.pointer_sample(x, y) {
                            capture.push(sample);
                        }
                    }
                    self.capture = Some(capture);
                }
                (ElementState::Released, MouseButton::Left) => {
                    if let Some(capture) = self.capture.take() {
                        let stroke = capture.finish();
                        if !stroke.points.is_empty() {
                            let _ = self.document.add_stroke(stroke);
                        }
                    }
                    self.request_redraw();
                }
                (ElementState::Pressed, MouseButton::Right) => self.right_drag = true,
                (ElementState::Released, MouseButton::Right) => self.right_drag = false,
                _ => {}
            },
            WindowEvent::MouseWheel { delta, .. } => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, vertical) => f64::from(vertical),
                    MouseScrollDelta::PixelDelta(position) => position.y / 120.0,
                };
                self.zoom_by(steps);
                self.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if let Some(gpu) = &mut self.gpu {
                    match gpu.render(&self.document, &self.brushes) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            gpu.reconfigure();
                            self.request_redraw();
                        }
                        Err(error) => eprintln!("surface presentation warning: {error}"),
                    }
                }
            }
            _ => {}
        }
    }
}

fn dabs_to_vertices(dabs: &[RenderDab], viewport: ScreenViewport) -> Vec<GpuVertex> {
    let mut vertices = Vec::with_capacity(dabs.len() * 6);
    for dab in dabs {
        let radius = f64::from(dab.radius_px.max(0.5));
        let left = screen_to_clip_x(dab.screen_x - radius, viewport.width);
        let right = screen_to_clip_x(dab.screen_x + radius, viewport.width);
        let top = screen_to_clip_y(dab.screen_y - radius, viewport.height);
        let bottom = screen_to_clip_y(dab.screen_y + radius, viewport.height);
        let opacity = dab.opacity;

        vertices.extend_from_slice(&[
            vertex(left, top, -1.0, -1.0, opacity),
            vertex(right, top, 1.0, -1.0, opacity),
            vertex(right, bottom, 1.0, 1.0, opacity),
            vertex(left, top, -1.0, -1.0, opacity),
            vertex(right, bottom, 1.0, 1.0, opacity),
            vertex(left, bottom, -1.0, 1.0, opacity),
        ]);
    }
    vertices
}

fn vertex(x: f32, y: f32, local_x: f32, local_y: f32, opacity: f32) -> GpuVertex {
    GpuVertex {
        position: [x, y],
        local: [local_x, local_y],
        opacity,
    }
}

fn screen_to_clip_x(value: f64, width: f64) -> f32 {
    (value / width * 2.0 - 1.0) as f32
}

fn screen_to_clip_y(value: f64, height: f64) -> f32 {
    (1.0 - value / height * 2.0) as f32
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = DesktopApp::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
