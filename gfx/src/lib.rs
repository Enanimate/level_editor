use std::{iter, sync::Arc};

use wgpu::util::DeviceExt;
use wgpu_text::{glyph_brush::{ab_glyph::FontRef, Section, Text}, BrushBuilder, TextBrush};
use winit::{dpi::{PhysicalPosition, PhysicalSize}, event_loop::ActiveEventLoop, keyboard::KeyCode, window::Window};

use crate::{definitions::Vertex, gui::{camera::{Camera2D, Camera2DUniform}, interface::Interface}};

mod builder;
mod definitions;
pub mod gui;

pub struct RenderState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    pipeline: wgpu::RenderPipeline,
    pub window: Arc<Window>,

    size: PhysicalSize<u32>,
    interface: Interface,

    camera_2d: Camera2D,
    camera_buffer_2d: wgpu::Buffer,
    camera_bind_group_2d: wgpu::BindGroup,

    text_brush: TextBrush<FontRef<'static>>,
}

impl RenderState {
    pub async fn new(window: Arc<Window>, mut interface: Interface) -> anyhow::Result<RenderState> {

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

        let pipeline = builder::PipeLineBuilder::new(&device)
            .set_pixel_format(wgpu::TextureFormat::Bgra8UnormSrgb)
            .add_vertex_buffer_layout(Vertex::desc())
            .add_bind_group_layout(&camera_bind_group_layout_2d)
            .set_shader_module("shader.wgsl", "vs_main", "fs_main")
            .build("Render Pipeline");

        interface.init_gpu_buffers(&device, &queue, size, &config);
        

        const FONT_BYTES: &[u8] = include_bytes!("../../Comic Sans MS.ttf");

        let text_brush = BrushBuilder::using_font_bytes(FONT_BYTES)? // Use as_slice here
        .build(
            &device,
            config.width,
            config.height,
            config.format,
        );

        let mut render_state = Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            pipeline,

            size, 
            interface,

            camera_2d,
            camera_buffer_2d,
            camera_bind_group_2d,

            text_brush,
        };

        render_state.queue_all_text();

        Ok(render_state)
    }

    pub fn handle_interact(&mut self, position: PhysicalPosition<f64>) {
        let size = self.size;

        let x_percent_location = position.x as f32 / size.width as f32;
        let y_percent_location = position.y as f32 / size.height as f32;

        println!("X: {}, Y: {}", x_percent_location, y_percent_location);
        //self.interface.check_click_position(x_percent_location, y_percent_location);
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

            self.interface.update_vertices_and_queue_text(self.size, &self.queue, &self.device, &self.config);

            self.text_brush.resize_view(width as f32, height as f32, &self.queue);
            self.queue_all_text();
        }
    }

    pub fn queue_all_text(&mut self) {
        let screen_width = self.size.width as f32;
        let _screen_height = self.size.height as f32;

        let hello_text_section = Section::builder()
            .with_screen_position([screen_width / 2.0 - 50.0, 20.0])
            .with_text(vec![
                Text::new("Hello, WGPU Text 26.0.0")
                    .with_scale(30.0)
                    .with_color([1.0, 1.0, 1.0, 1.0]),
            ]);

        self.text_brush.queue(&self.device, &self.queue, &[hello_text_section]).unwrap();
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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
            label: Some("Render Pass"),
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

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group_2d, &[]);
        self.interface.render(&mut render_pass, &self.device, &self.config);

        self.text_brush.draw(&mut render_pass);

        drop(render_pass);

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
    
    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
    }
}