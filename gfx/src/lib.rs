use std::{iter, sync::{Arc, Mutex}};

use glam::{Vec2, Vec3};
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

use crate::{definitions::{GuiState, Vertex}, gui::{camera::{Camera2D, Camera2DUniform}, interface::Interface}};

mod builder;
pub mod definitions;
pub mod gui;

pub struct RenderState {
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    ui_pipeline: wgpu::RenderPipeline,
    preview_pipeline: wgpu::RenderPipeline,
    pub window: Arc<Window>,

    pub size: PhysicalSize<u32>,

    camera_2d: Camera2D,
    camera_buffer_2d: wgpu::Buffer,
    camera_bind_group_2d: wgpu::BindGroup,

    triangle_vertex_buffer: wgpu::Buffer,
    interface_arc: Arc<Mutex<Interface>>,
    pub gui_state: GuiState,
}

impl RenderState {
    pub async fn new(window: Arc<Window>, interface_arc: Arc<Mutex<Interface>>) -> anyhow::Result<RenderState> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;


        let camera_2d = Camera2D::new(size.width, size.height);

        let camera_uniform_2d = Camera2DUniform {
            view_proj: camera_2d.build_view_projection_matrix().to_cols_array_2d(),
        };
        let camera_buffer_2d = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera 2D Uniform Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform_2d]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout_2d = 
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer { 
                            ty: wgpu::BufferBindingType::Uniform, 
                            has_dynamic_offset: false, 
                            min_binding_size: None 
                        },
                        count: None,
                    }
                ],
                label: Some("Camera 2D Bind Group Layout"),
            });

        let camera_bind_group_2d = device.create_bind_group(&wgpu::BindGroupDescriptor { 
            label: Some("Camera 2D Bind Group"), 
            layout: &camera_bind_group_layout_2d, 
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer_2d.as_entire_binding(),
                }
            ] 
        });

        let surface_caps = surface.get_capabilities(&adapter);

        // Shader code in this tutorial assumes an Srgb surface texture. Using a different
        // one will result all the colors comming out darker. If you want to support non
        // Srgb surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };

        let ui_pipeline = builder::PipeLineBuilder::new(&device)
            .set_pixel_format(wgpu::TextureFormat::Bgra8UnormSrgb)
            .add_vertex_buffer_layout(Vertex::desc())
            .add_bind_group_layout(&camera_bind_group_layout_2d)
            .set_shader_module("ui_shader.wgsl", "vs_main", "fs_main")
            .build("Render Pipeline");

        let preview_pipeline = builder::PipeLineBuilder::new(&device)
            .set_pixel_format(wgpu::TextureFormat::Bgra8UnormSrgb)
            .add_vertex_buffer_layout(Vertex::desc())
            .set_shader_module("preview_shader.wgsl", "vs_main", "fs_main")
            .build("Preview Pipeline");

        let triangle_vertices = [
            Vertex { position: Vec2::new(0.0, 0.5), color: Vec3::new(0.0, 0.0, 1.0) },  // Top (green)
            Vertex { position: Vec2::new(-0.5, -0.5), color: Vec3::new(0.0, 1.0, 0.0) }, // Bottom-left (blue)
            Vertex { position: Vec2::new(0.5, -0.5), color: Vec3::new(1.0, 0.0, 0.0) }, // Bottom-right (yellow)
        ];

        let triangle_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Triangle Vertex Buffer"),
            contents: bytemuck::cast_slice(&triangle_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            ui_pipeline,
            preview_pipeline,

            size,

            camera_2d,
            camera_buffer_2d,
            camera_bind_group_2d,
            triangle_vertex_buffer,
            interface_arc,
            gui_state: GuiState::ProjectView,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.size = PhysicalSize::new(width, height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;

            self.camera_2d.update_screen_size(PhysicalSize::new(width, height));
            self.queue.write_buffer(
                &self.camera_buffer_2d, 
                0, 
            bytemuck::cast_slice(&[Camera2DUniform {
                view_proj: self.camera_2d.build_view_projection_matrix().to_cols_array_2d(),
            }]));
            let mut intfc = self.interface_arc.lock().unwrap();
            intfc.update_vertices_and_queue_text(self.size, &self.queue, &self.device);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let interface = self.interface_arc.lock().unwrap();
        self.window.request_redraw();
        //let ui_group = self.interface.get_render_data();
        
        // We can't render unless the surface is configured
        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });


        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("UI Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.012,
                        g: 0.012,
                        b: 0.018,
                        a: 1.00,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.ui_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group_2d, &[]);
        interface.render(&mut render_pass);

        if self.gui_state == GuiState::ProjectView {
            render_pass.set_pipeline(&self.preview_pipeline);
            render_pass.set_viewport(self.size.width as f32 / 2.0, self.size.height as f32 / 2.0, self.size.width as f32 / 2.0, self.size.height as f32 / 2.0, 0.0, 1.0);
            render_pass.set_vertex_buffer(0, self.triangle_vertex_buffer.slice(..));
            render_pass.draw(0..3, 0..1);
        }

        drop(render_pass);

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}