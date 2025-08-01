use std::{iter, sync::{Arc, Mutex}};

use glam::{Vec2, Vec3};
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

use crate::{definitions::{GuiState, GuiUniform, Vertex}, gui::{camera::{Camera2D, Camera2DUniform}, interface::Interface}};

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

    gui_uniform_buffer: wgpu::Buffer,
    gui_material_bind_group_layout: wgpu::BindGroupLayout,
    gui_material_bind_group: wgpu::BindGroup,
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

        let initial_uniform_data = GuiUniform { use_texture: 0 };
        let gui_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GUI Uniform Buffer"),
            contents: bytemuck::cast_slice(&[initial_uniform_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let diffuse_bytes = include_bytes!("../../folder-1484.png");
        let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        let diffuse_rgba = diffuse_image.to_rgba8();

        use image::GenericImageView;
        let dimensions = diffuse_image.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            // All textures are stored as 3D, we represent our 2D texture
            // by setting depth to 1.
            depth_or_array_layers: 1,
        };
        let diffuse_texture = device.create_texture(
            &wgpu::TextureDescriptor {
                size: texture_size,
                mip_level_count: 1, // We'll talk about this a little later
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                // Most images are stored using sRGB, so we need to reflect that here.
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
                // COPY_DST means that we want to copy data to this texture
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: Some("diffuse_texture"),
                // This is the same as with the SurfaceConfig. It
                // specifies what texture formats can be used to
                // create TextureViews for this texture. The base
                // texture format (Rgba8UnormSrgb in this case) is
                // always supported. Note that using a different
                // texture format is not supported on the WebGL2
                // backend.
                view_formats: &[],
            }
        );

        queue.write_texture(
            // Tells wgpu where to copy the pixel data
            wgpu::TexelCopyTextureInfo {
                texture: &diffuse_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            // The actual pixel data
            &diffuse_rgba,
            // The layout of the texture
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        let diffuse_texture_view = diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let gui_material_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0, //@binding(0) for our GuiUniform
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer { 
                            ty: wgpu::BufferBindingType::Uniform, 
                            has_dynamic_offset: false, 
                            min_binding_size: None 
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1, //@binding(1) for our texture
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None
                    }
                ],
                label: Some("texture_bind_group_layout"),
            });

        let gui_material_bind_group = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
                label: Some("GUI Material Bind Group"),
                layout: &gui_material_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: gui_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                    }
                ],
            }
        );

        let ui_pipeline = builder::PipeLineBuilder::new(&device)
            .set_pixel_format(wgpu::TextureFormat::Bgra8UnormSrgb)
            .add_vertex_buffer_layout(Vertex::desc())
            .add_bind_group_layout(&camera_bind_group_layout_2d)
            .add_bind_group_layout(&gui_material_bind_group_layout)
            .set_shader_module("ui_shader.wgsl", "vs_main", "fs_main")
            .build("Render Pipeline");

        let preview_pipeline = builder::PipeLineBuilder::new(&device)
            .set_pixel_format(wgpu::TextureFormat::Bgra8UnormSrgb)
            .add_vertex_buffer_layout(Vertex::desc())
            .set_shader_module("preview_shader.wgsl", "vs_main", "fs_main")
            .build("Preview Pipeline");

        let triangle_vertices = [
            Vertex { position: Vec2::new(0.0, 0.5), color: Vec3::new(0.0, 0.0, 1.0), tex_coords: Vec2::new(0.0, 0.0) },  // Top (green)
            Vertex { position: Vec2::new(-0.5, -0.5), color: Vec3::new(0.0, 1.0, 0.0), tex_coords: Vec2::new(0.0, 0.0) }, // Bottom-left (blue)
            Vertex { position: Vec2::new(0.5, -0.5), color: Vec3::new(1.0, 0.0, 0.0), tex_coords: Vec2::new(0.0, 0.0) }, // Bottom-right (yellow)
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
            gui_uniform_buffer,
            gui_material_bind_group_layout,
            gui_material_bind_group,
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

    pub fn render(&self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder")
        });

        let interface_guard = self.interface_arc.lock().unwrap();

        {
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
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.ui_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group_2d, &[]);
            render_pass.set_bind_group(1, &self.gui_material_bind_group, &[]);

            let (vertex_buffer, index_buffer) = match (interface_guard.vertex_buffer.as_ref(), interface_guard.index_buffer.as_ref()) {
                (Some(vb), Some(ib)) =>(vb, ib),
                _ => {
                    eprintln!("Warning: GUI vertex/index buffers are not initialized.");
                    return Ok(());
                }
            };

            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            let mut vertex_offset_in_buffer = 0;
            let vertex_size_bytes = std::mem::size_of::<Vertex>() as wgpu::BufferAddress;
            let quad_vertices_count = 4;
            let quad_indices_count = 6;

            for panel in interface_guard.panels.iter() {
                for element in panel.elements.iter() {
                    let use_texture_uniform_value = if element.is_textured {
                        1u32
                    } else {
                        0u32
                    };

                    self.queue.write_buffer(
                        &self.gui_uniform_buffer, 
                        0, 
                    bytemuck::cast_slice(&[GuiUniform { use_texture: use_texture_uniform_value }]),
                    );

                    render_pass.set_vertex_buffer(
                        0, 
                        vertex_buffer.slice(vertex_offset_in_buffer..(vertex_offset_in_buffer + quad_vertices_count * vertex_size_bytes),
                    ));

                    render_pass.draw_indexed(0..quad_indices_count, 0, 0..1);

                    vertex_offset_in_buffer += quad_vertices_count * vertex_size_bytes;
                }
            }

            interface_guard.draw_text_brush(&mut render_pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn old_render(&mut self) -> Result<(), wgpu::SurfaceError> {
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
        //render_pass.set_bind_group(1, &self.diffuse_bind_group, &[]);
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