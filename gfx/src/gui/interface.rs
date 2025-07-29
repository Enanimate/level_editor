use glam::{Vec2, Vec3};
use wgpu::{Device, Queue, util::DeviceExt};

use wgpu_text::{glyph_brush::{ab_glyph::{FontRef, PxScale}, Section, Text}, BrushBuilder, TextBrush};
use winit::dpi::PhysicalSize;

use crate::definitions::Vertex;

pub struct Interface {
    panels: Vec<Panel>,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    brush: Option<TextBrush<FontRef<'static>>>,
    max_screen_size: Option<PhysicalSize<u32>>
}

impl Interface {
    pub fn new() -> Interface {
        Self {
            panels: Vec::new(),
            vertex_buffer: None,
            index_buffer: None,
            brush: None,
            max_screen_size: None,
        }
    }

    pub fn add_panel(&mut self, panel: Panel) {
        self.panels.push(panel);
    }

    pub fn set_maximized_size(&self) {
        //if self.max_screen_size.is_none() || 
    }

    pub(crate) fn init_gpu_buffers(
        &mut self,
        device: &Device,
        queue: &Queue,
        screen_size: PhysicalSize<u32>,
        config: &wgpu::SurfaceConfiguration
    ) {
        let indices: &[u16] = &[0, 2, 1, 1, 2, 3];

        let font_bytes = include_bytes!("../../../ComicMono.ttf");
        self.brush = Some(BrushBuilder::using_font_bytes(font_bytes)
            .unwrap()
            .build(device, config.width, config.height, config.format));

        let total_vertices_needed =
            self.panels.iter().flat_map(|panel| &panel.elements).count() * 4;
        let vertex_buffer_size =
            (total_vertices_needed * std::mem::size_of::<Vertex>()) as wgpu::BufferAddress;

        self.vertex_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.index_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            }),
        );

        self.update_vertices_and_queue_text(screen_size, queue, device, config);
    }

    pub(crate) fn update_vertices_and_queue_text(
        &mut self,
        screen_size: PhysicalSize<u32>,
        queue: &Queue,
        device: &Device,
        config: &wgpu::SurfaceConfiguration,
    ) {
        let mut vertex_offset = 0; // Keep track of the current offset in bytes
        self.brush.as_ref().unwrap().resize_view(screen_size.width as f32, screen_size.height as f32, queue);

        for panel in &mut self.panels {
            let (panel_x_min_co, panel_y_min_co, panel_x_max_co, panel_y_max_co) =
                panel.calculate_absolute_coordinates(screen_size);

            for element in &mut panel.elements {
                let new_vertices = element.calculate_vertices_relative_to_panel(
                    panel_x_min_co,
                    panel_y_min_co,
                    panel_x_max_co,
                    panel_y_max_co,
                );
                let vertex_data_slice = bytemuck::cast_slice(&new_vertices);
                let vertex_data_size = vertex_data_slice.len() as wgpu::BufferAddress;
                let text_data = Self::text_alignment(
                    element.start_coordinate.x, 
                    element.start_coordinate.y, 
                    element.end_coordinate.x, 
                    element.end_coordinate.y, 
                    panel_x_min_co, 
                    panel_y_min_co, 
                    panel_x_max_co, 
                    panel_y_max_co, 
                    screen_size,
                    &element.text_alignment.as_ref().unwrap(),
                    element.text.clone().unwrap(),
                );
                Self::text(text_data, &new_vertices, screen_size, device, config, queue, self.brush.as_mut().unwrap());

                queue.write_buffer(
                    self.vertex_buffer.as_ref().unwrap(),
                    vertex_offset,
                    vertex_data_slice,
                );

                vertex_offset += vertex_data_size; // Increment offset for the next element
            }
        }
    }

    fn text_alignment(ex_0: f32, ey_0: f32, ex_1: f32, ey_1: f32, px_0: f32, py_0: f32, px_1: f32, py_1: f32, screen_size: PhysicalSize<u32>, alignment: &Alignment, text: String) -> ((f32, f32), f32, String){
        let screen_x_center = screen_size.width as f32 / 2.0;
        let screen_y_center = screen_size.height as f32 / 2.0;
        let scale = 1.0;

        //let element_abs_y_top_center_origin = panel_y_max_center_origin - self.start_coordinate.y * (panel_y_max_center_origin - panel_y_min_center_origin);
        //let element_abs_y_bottom_center_origin = panel_y_max_center_origin - self.end_coordinate.y * (panel_y_max_center_origin - panel_y_min_center_origin);

        match (&alignment.horizontal, &alignment.vertical) {
            (HorizontalAlignment::Left, VerticalAlignment::Top) => {
                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x, y), scale, text);
            }
            (HorizontalAlignment::Left, VerticalAlignment::Center) => {
                // Not quite there, look later
                let half_y_length = ((py_1 - ey_0 * (py_1 - py_0)) - (py_1 - ey_1 * (py_1 - py_0))) / 2.0;
                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x, y + half_y_length - 30.0), scale, text);
            }
            (HorizontalAlignment::Left, VerticalAlignment::Bottom) => {
                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_1 * (py_1 - py_0));
                return ((x, y - 30.0), scale, text);
            }



            //let element_abs_x_min_center_origin = panel_x_min_center_origin + self.start_coordinate.x * (panel_x_max_center_origin - panel_x_min_center_origin);
            //let element_abs_x_max_center_origin = panel_x_min_center_origin + self.end_coordinate.x * (panel_x_max_center_origin - panel_x_min_center_origin);
            (HorizontalAlignment::Center, VerticalAlignment::Top) => {
                let text_offset = text.chars().count() as f32 * 15.0;
                let half_x_length = (px_0 + ex_1 * (px_1 - px_0)) - (px_0 + ex_0 * (px_1 - px_0));
                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x + half_x_length - text_offset, y), scale, text);
            }
            (HorizontalAlignment::Center, VerticalAlignment::Center) => {
                todo!();
                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x, y), scale, text);
            }
            (HorizontalAlignment::Center, VerticalAlignment::Bottom) => {
                todo!();
                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x, y), scale, text);
            }

            (HorizontalAlignment::Right, VerticalAlignment::Top) => {
                todo!();
                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x, y), scale, text);
            }
            (HorizontalAlignment::Right, VerticalAlignment::Center) => {
                todo!();
                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x, y), scale, text);
            }
            (HorizontalAlignment::Right, VerticalAlignment::Bottom) => {
                todo!();
                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x, y), scale, text);
            }
        }
    }

    fn text<'a>(text_data: ((f32, f32), f32, String), vertices: &[Vertex], screen_size: PhysicalSize<u32>, device: &Device, config: &wgpu::SurfaceConfiguration, queue: &Queue, brush: &mut TextBrush<FontRef<'a>>) {
        let (text_coordinate, scale, text) = text_data;

//[text_x_co + vertex_x_offset, text_y_co - vertex_y_offset]
        let text_x_co = screen_size.width as f32 / 2.0;
        let text_y_co = screen_size.height as f32 / 2.0;
        let vertex_x_offset = vertices[0].position.x;
        let vertex_y_offset = vertices[0].position.y;
        println!("{} {}", text_x_co + vertex_x_offset, text_y_co - vertex_y_offset);
        let section = Section::builder()
            .with_screen_position(text_coordinate)
            //.with_bounds([20.0, 20.0])
            .with_text(vec![
                Text::new(&text)
                    .with_scale(PxScale {x: 30.0, y: 30.0})
                    .with_color([1.0, 1.0, 1.0, 1.0]),
            ]);
        brush.queue(device, queue, [section]).unwrap();
    }

    pub(crate) fn render<'a>(&'a mut self, renderpass: &mut wgpu::RenderPass<'a>, device: &Device, config: &wgpu::SurfaceConfiguration) {
        let vertex_buffer = match &self.vertex_buffer {
            Some(buffer) => buffer,
            None => {
                eprintln!("Warning: GUI vertex buffer not initialized. Skipping Render...");
                return;
            }
        };
        renderpass.set_index_buffer(
            self.index_buffer.as_ref().unwrap().slice(..),
            wgpu::IndexFormat::Uint16,
        );

        let mut vertex_offset_in_buffer = 0;
        let vertex_size_bytes = std::mem::size_of::<Vertex>() as wgpu::BufferAddress;
        let quad_vertices_count = 4;
        for _panel in 0..self.panels.len() {
            for _element in 0..self.panels[_panel].elements.len() {
                renderpass.set_vertex_buffer(
                    0,
                    vertex_buffer.slice(
                        vertex_offset_in_buffer
                            ..(vertex_offset_in_buffer + quad_vertices_count * vertex_size_bytes),
                    ),
                );
                renderpass.draw_indexed(0..6, 0, 0..1);
                vertex_offset_in_buffer += quad_vertices_count * vertex_size_bytes;
            }
        }
        self.brush.as_mut().unwrap().draw(renderpass);
    }
}

pub struct Panel {
    elements: Vec<Element>,
    start_coordinate: Coordinate,
    end_coordinate: Coordinate,
}

impl Panel {
    pub fn new(start_coordinate: Coordinate, end_coordinate: Coordinate) -> Self {
        Self {
            elements: Vec::new(),
            start_coordinate,
            end_coordinate,
        }
    }

    pub fn add_element(&mut self, element: Element) {
        self.elements.push(element);
    }

    fn calculate_absolute_coordinates(
        &self,
        screen_size: PhysicalSize<u32>,
    ) -> (f32, f32, f32, f32) {
        let screen_width_full = screen_size.width as f32;
        let screen_height_full = screen_size.height as f32;

        let x_min_px = self.start_coordinate.x * screen_width_full;
        let x_max_px = self.end_coordinate.x * screen_width_full;
        let y_min_px = self.start_coordinate.y * screen_height_full;
        let y_max_px = self.end_coordinate.y * screen_height_full;

        let half_screen_width = screen_width_full / 2.0;
        let half_screen_height = screen_height_full / 2.0;

        let x_min_ndc = x_min_px - half_screen_width;
        let x_max_ndc = x_max_px - half_screen_width;

        let y_max_ndc = half_screen_height - y_min_px;
        let y_min_ndc = half_screen_height - y_max_px;

        (x_min_ndc, y_min_ndc, x_max_ndc, y_max_ndc)
    }
}

pub struct Element {
    start_coordinate: Coordinate,
    end_coordinate: Coordinate,
    color: Color,
    text: Option<String>,
    text_alignment: Option<Alignment>,
}

impl Element {
    pub fn new(start_coordinate: Coordinate, end_coordinate: Coordinate, color: Color) -> Self {
        Self {
            start_coordinate,
            end_coordinate,
            color,
            text: None,
            text_alignment: None,
        }
    }

    pub fn with_text(mut self, alignment: Alignment) -> Self {
        self.text = Some("test".to_string());
        self.text_alignment = Some(alignment);
        self
    }

    fn calculate_vertices_relative_to_panel(
        &mut self,
        panel_x_min_center_origin: f32,
        panel_y_min_center_origin: f32,
        panel_x_max_center_origin: f32,
        panel_y_max_center_origin: f32,
    ) -> [Vertex; 4] {

        // Convert element's local coordinates to panel's absolute coordinates (center-origin)
        let element_abs_x_min_center_origin = panel_x_min_center_origin
            + self.start_coordinate.x * (panel_x_max_center_origin - panel_x_min_center_origin);
        let element_abs_x_max_center_origin = panel_x_min_center_origin
            + self.end_coordinate.x * (panel_x_max_center_origin - panel_x_min_center_origin);

        // Your Y-axis is inverted here: y_max_center_origin is top, y_min_center_origin is bottom
        // elem_local_y_min_rel corresponds to the top of the element relative to panel's top (0.0 to 1.0)
        // elem_local_y_max_rel corresponds to the bottom of the element relative to panel's top (0.0 to 1.0)
        let element_abs_y_top_center_origin = panel_y_max_center_origin
            - self.start_coordinate.y * (panel_y_max_center_origin - panel_y_min_center_origin);
        let element_abs_y_bottom_center_origin = panel_y_max_center_origin
            - self.end_coordinate.y * (panel_y_max_center_origin - panel_y_min_center_origin);

        // Calculate the vertices for the mesh (these should match your rendering pipeline)
        // These are typically NDC, so leave them as is if your Vertex shader handles center-origin NDC.
        let vtx_x_min = element_abs_x_min_center_origin;
        let vtx_x_max = element_abs_x_max_center_origin;
        let vtx_y_top = element_abs_y_top_center_origin; // The Y coordinate for the top edge of the element
        let vtx_y_bottom = element_abs_y_bottom_center_origin; // The Y coordinate for the bottom edge of the element

        [
            Vertex {
                position: Vec2::new(vtx_x_min, vtx_y_top),
                color: self.color.into_vec3(),
            }, // Top-Left
            Vertex {
                position: Vec2::new(vtx_x_max, vtx_y_top),
                color: self.color.into_vec3(),
            }, // Top-Right
            Vertex {
                position: Vec2::new(vtx_x_min, vtx_y_bottom),
                color: self.color.into_vec3(),
            }, // Bottom-Left
            Vertex {
                position: Vec2::new(vtx_x_max, vtx_y_bottom),
                color: self.color.into_vec3(),
            }, // Bottom-Right
        ]
    }
}

pub struct Coordinate {
    x: f32,
    y: f32,
}

impl Coordinate {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

pub struct Color {
    r: f32,
    g: f32,
    b: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    fn into_vec3(&self) -> Vec3 {
        Vec3::new(self.r, self.g, self.b)
    }
}

pub struct Alignment {
    pub vertical: VerticalAlignment,
    pub horizontal: HorizontalAlignment
}

pub enum VerticalAlignment {
    Top,
    Center,
    Bottom
}

pub enum HorizontalAlignment {
    Left,
    Center,
    Right
}