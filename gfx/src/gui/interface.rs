use glam::{Vec2, Vec3};
use wgpu::{Device, Queue, util::DeviceExt};

use wgpu_text::{glyph_brush::{ab_glyph::{FontRef, PxScale}, Section, Text}, BrushBuilder, TextBrush};
use winit::dpi::{PhysicalPosition, PhysicalSize};

use crate::definitions::{GuiEvent, Vertex};

pub struct Interface {
    panels: Vec<Panel>,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    brush: Option<TextBrush<FontRef<'static>>>,
}

impl Interface {
    pub fn new() -> Interface {
        Self {
            panels: Vec::new(),
            vertex_buffer: None,
            index_buffer: None,
            brush: None,
        }
    }

    pub fn add_panel(&mut self, panel: Panel) {
        self.panels.push(panel);
    }

    pub fn set_maximized_size(&self) {
        //if self.max_screen_size.is_none() || 
    }

    pub fn handle_interaction(&mut self, position: PhysicalPosition<f64>, screen_size: PhysicalSize<u32>) -> Option<GuiEvent> {

        let x_position = position.x as f32 / screen_size.width as f32;
        let y_position = position.y as f32 / screen_size.height as f32;
        for panel in &self.panels {
            match (x_position >= panel.start_coordinate.x && x_position <= panel.end_coordinate.x, y_position >= panel.start_coordinate.y && y_position <= panel.end_coordinate.y) {
                (true, true) => {
                    let rel_cursor_x = x_position - panel.start_coordinate.x;
                    let rel_cursor_y = y_position - panel.start_coordinate.y;
                    for element in &panel.elements {
                        if element.on_click.is_some() {
                            match (rel_cursor_x >= element.start_coordinate.x && rel_cursor_x <= element.end_coordinate.x, rel_cursor_y >= element.start_coordinate.y && rel_cursor_y <= element.end_coordinate.y) {
                                (true, true) => {
                                    return element.handle_click();
                                },
                                _ => ()
                            }
                        }
                    }
                }
                _ => ()
            }
        }
        None
    }

    pub fn init_gpu_buffers(
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

        self.update_vertices_and_queue_text(screen_size, queue, device);
    }

    pub fn update_vertices_and_queue_text(
        &mut self,
        screen_size: PhysicalSize<u32>,
        queue: &Queue,
        device: &Device,
    ) {
        let mut sections_to_queue: Vec<Section> = Vec::new();
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

                queue.write_buffer(
                    self.vertex_buffer.as_ref().unwrap(),
                    vertex_offset,
                    vertex_data_slice,
                );

                vertex_offset += vertex_data_size; // Increment offset for the next element

                if let (Some(text_content), Some(text_align)) = (
                    &element.text,
                    &element.text_alignment,
                ) {
                    let ((adjusted_x, adjusted_y), _scale) = Self::text_alignment(
                        element.start_coordinate.x, 
                        element.start_coordinate.y, 
                        element.end_coordinate.x, 
                        element.end_coordinate.y, 
                        panel_x_min_co, 
                        panel_y_min_co, 
                        panel_x_max_co, 
                        panel_y_max_co, 
                        screen_size,
                        text_align,
                        text_content,
                    );
                    let text_content_str = text_content.as_str();

                    let section = Section::builder()
                        .with_screen_position([adjusted_x, adjusted_y])
                        .with_text(vec![
                            Text::new(text_content_str)
                                .with_scale(PxScale {x: 30.0, y: 30.0})
                                .with_color([1.0, 1.0, 1.0, 1.0]),
                        ]);
                    sections_to_queue.push(section);
                }
            }
        }
        if !sections_to_queue.is_empty() {
            self.brush.as_mut().unwrap().queue(device, queue, sections_to_queue).unwrap();
        }
    }

    fn text_alignment(ex_0: f32, ey_0: f32, ex_1: f32, ey_1: f32, px_0: f32, py_0: f32, px_1: f32, py_1: f32, screen_size: PhysicalSize<u32>, alignment: &Alignment, text: &str) -> ((f32, f32), f32){
        let screen_x_center = screen_size.width as f32 / 2.0;
        let screen_y_center = screen_size.height as f32 / 2.0;
        let scale = 1.0;

        match (&alignment.horizontal, &alignment.vertical) {
            (HorizontalAlignment::Left, VerticalAlignment::Top) => {
                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x, y), scale);
            }
            (HorizontalAlignment::Left, VerticalAlignment::Center) => {
                // Not quite there, look later
                let half_y_length = ((py_1 - ey_0 * (py_1 - py_0)) - (py_1 - ey_1 * (py_1 - py_0))) / 2.0;
                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x, y + half_y_length - 30.0), scale);
            }
            (HorizontalAlignment::Left, VerticalAlignment::Bottom) => {
                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_1 * (py_1 - py_0));
                return ((x, y - 30.0), scale);
            }



            (HorizontalAlignment::Center, VerticalAlignment::Top) => {
                let text_offset = (text.chars().count() as f32 * 15.0) / 2.0;

                let half_x_length = ((px_0 + ex_1 * (px_1 - px_0)) - (px_0 + ex_0 * (px_1 - px_0))) / 2.0;

                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x + half_x_length - text_offset, y), scale);
            }
            (HorizontalAlignment::Center, VerticalAlignment::Center) => {
                let text_offset = (text.chars().count() as f32 * 15.0) / 2.0;

                let half_x_length = ((px_0 + ex_1 * (px_1 - px_0)) - (px_0 + ex_0 * (px_1 - px_0))) / 2.0;
                let half_y_length = ((py_1 - ey_0 * (py_1 - py_0)) - (py_1 - ey_1 * (py_1 - py_0))) / 2.0;

                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x + half_x_length - text_offset, y + half_y_length - 15.0), scale);
            }
            (HorizontalAlignment::Center, VerticalAlignment::Bottom) => {
                let text_offset = (text.chars().count() as f32 * 15.0) / 2.0;
                
                let half_x_length = ((px_0 + ex_1 * (px_1 - px_0)) - (px_0 + ex_0 * (px_1 - px_0))) / 2.0;

                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_1 * (py_1 - py_0));
                return ((x + half_x_length - text_offset, y - 15.0), scale);
            }


            
            (HorizontalAlignment::Right, VerticalAlignment::Top) => {
                let text_offset = text.chars().count() as f32 * 15.0;

                let half_x_length = (px_0 + ex_1 * (px_1 - px_0)) - (px_0 + ex_0 * (px_1 - px_0));

                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x + half_x_length - text_offset, y), scale);
            }
            (HorizontalAlignment::Right, VerticalAlignment::Center) => {
                let text_offset = text.chars().count() as f32 * 15.0;

                let half_x_length = (px_0 + ex_1 * (px_1 - px_0)) - (px_0 + ex_0 * (px_1 - px_0));
                let half_y_length = ((py_1 - ey_0 * (py_1 - py_0)) - (py_1 - ey_1 * (py_1 - py_0))) / 2.0;

                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x + half_x_length - text_offset, y + half_y_length - 15.0), scale);
            }
            (HorizontalAlignment::Right, VerticalAlignment::Bottom) => {
                let text_offset = text.chars().count() as f32 * 15.0;

                let half_x_length = (px_0 + ex_1 * (px_1 - px_0)) - (px_0 + ex_0 * (px_1 - px_0));

                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_1 * (py_1 - py_0));
                return ((x + half_x_length - text_offset, y - 15.0), scale);
            }
        }
    }

    pub(crate) fn render<'a>(&'a self, renderpass: &mut wgpu::RenderPass<'a>) {
        //let mut intfc = self.interface.lock().unwrap();
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
        self.brush.as_ref().unwrap().draw(renderpass);
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
    on_click: Option<Box<dyn Fn() -> Option<GuiEvent> + 'static>>,
}

impl Element {
    pub fn new(start_coordinate: Coordinate, end_coordinate: Coordinate, color: Color) -> Self {
        Self {
            start_coordinate,
            end_coordinate,
            color,
            text: None,
            text_alignment: None,
            on_click: None,
        }
    }

    pub fn with_fn(mut self, func: impl Fn() -> Option<GuiEvent> + 'static) -> Self {
        self.on_click = Some(Box::new(func));
        self
    }

    pub fn with_text(mut self, alignment: Alignment) -> Self {
        self.text = Some("test".to_string());
        self.text_alignment = Some(alignment);
        self
    }

    pub fn handle_click(&self) -> Option<GuiEvent> {
        if let Some(func) = &self.on_click {
            func()
        } else {
            None
        }
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