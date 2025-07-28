use std::{env::current_dir, fs};

use wgpu::{
    BindGroupLayout, BlendState, ColorTargetState, ColorWrites, DepthBiasState, DepthStencilState, Device, Face, FragmentState, FrontFace, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, StencilState, TextureFormat, VertexBufferLayout, VertexState
};

pub(crate) struct PipeLineBuilder<'a> {
    shader_filename: String,
    vertex_entry: String,
    fragment_entry: String,
    pixel_format: wgpu::TextureFormat,
    vertex_buffer_layouts: Vec<VertexBufferLayout<'static>>,
    bind_group_layouts: Vec<&'a wgpu::BindGroupLayout>,
    device: &'a Device,
}

impl <'a> PipeLineBuilder <'a> {
    pub(crate) fn new(device: &'a Device) -> Self {
        PipeLineBuilder {
            shader_filename: "empty".to_string(),
            vertex_entry: "empty".to_string(),
            fragment_entry: "empty".to_string(),
            pixel_format: TextureFormat::Rgba8Unorm,
            vertex_buffer_layouts: Vec::new(),
            bind_group_layouts: Vec::new(),
            device: device,
        }
    }

    fn reset(&mut self) {
        self.vertex_buffer_layouts.clear();
    }

    pub(crate) fn set_shader_module(&mut self, shader_filename: &str, vertex_entry: &str, fragment_entry: &str) -> &mut Self {
        self.shader_filename = shader_filename.to_string();
        self.vertex_entry = vertex_entry.to_string();
        self.fragment_entry = fragment_entry.to_string();

        self
    }

    pub(crate) fn add_vertex_buffer_layout(&mut self, layout: VertexBufferLayout<'static>) -> &mut Self {
        self.vertex_buffer_layouts.push(layout);
        self
    }

    pub(crate) fn add_bind_group_layout(&mut self, layout: &'a BindGroupLayout) -> &mut Self {
        self.bind_group_layouts.push(layout);
        self
    }

    pub(crate) fn set_pixel_format(&mut self, pixel_format: TextureFormat) -> &mut Self {
        self.pixel_format = pixel_format;
        self
    }

    pub(crate) fn build(&mut self, label: &str) -> RenderPipeline {

        let pipeline_layout_descriptor = PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &self.bind_group_layouts,
            push_constant_ranges: &[],
        };

        let mut filepath = current_dir().unwrap();
        filepath.push(self.shader_filename.as_str());
        let filepath = filepath.into_os_string().into_string().unwrap();
        let source_code = fs::read_to_string(filepath).expect("Can't read source code!");

        let shader_module_descriptor = ShaderModuleDescriptor {
            label: Some("Shader Module"),
            source: ShaderSource::Wgsl(source_code.into()),
        };

        let shader_module = self.device.create_shader_module(shader_module_descriptor);

        let pipeline_layout = self.device.create_pipeline_layout(&pipeline_layout_descriptor);

        let render_targets = [Some(ColorTargetState {
            format: self.pixel_format,
            blend: Some(BlendState::REPLACE),
            write_mask: ColorWrites::ALL,
        })];

        let _depth_stencil = DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        };

        let pipeline_descriptor = RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&pipeline_layout),

            vertex: VertexState {
                module: &shader_module,
                entry_point:Some(&self.vertex_entry),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &self.vertex_buffer_layouts,
            },

            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some(&self.fragment_entry),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &render_targets,
            }),

            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        };

        let pipeline = self.device.create_render_pipeline(&pipeline_descriptor);

        self.reset();

        pipeline
    }
}