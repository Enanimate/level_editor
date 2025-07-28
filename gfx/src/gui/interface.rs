use glam::{Vec2, Vec3};
use wgpu::{util::DeviceExt, Device, Queue};
use winit::dpi::PhysicalSize;

use crate::definitions::Vertex;

pub struct Interface {
    panels: Vec<Panel>,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
}

impl Interface {
    pub fn new() -> Interface {
        Self {
            panels: Vec::new(),
            vertex_buffer: None,
            index_buffer: None,
        }
    }

    pub fn add_panel(&mut self, panel: Panel) {
        self.panels.push(panel);
    }

    pub(crate) fn init_gpu_buffers(&mut self, device: &Device, queue: &Queue, screen_size:PhysicalSize<u32>) {
        let indices: &[u16] = &[0, 2, 1, 1, 2, 3];

        let total_vertices_needed = self.panels.iter().flat_map(|panel| &panel.elements).count() * 4;
        let vertex_buffer_size = (total_vertices_needed * std::mem::size_of::<Vertex>()) as wgpu::BufferAddress;

        self.vertex_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false
        }));

        self.index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX
        }));

        self.update_vertices(screen_size, queue);
    }

    pub(crate) fn update_vertices(&self, screen_size:PhysicalSize<u32>, queue: &Queue) {
        let mut vertex_offset = 0; // Keep track of the current offset in bytes

        for panel in &self.panels {
            let (panel_x_min, panel_y_min, panel_x_max, panel_y_max) = panel.calculate_absolute_coordinates(screen_size);
            let panel_width = panel_x_max - panel_x_min;
            let panel_height = panel_y_max - panel_y_min;

            for element in &panel.elements {
                let new_vertices = element.calculate_vertices_relative_to_panel(panel_x_min, panel_y_max, panel_width, panel_height);
                let vertex_data_slice = bytemuck::cast_slice(&new_vertices);
                let vertex_data_size = vertex_data_slice.len() as wgpu::BufferAddress;

                queue.write_buffer(self.vertex_buffer.as_ref().unwrap(), vertex_offset, vertex_data_slice);

                vertex_offset += vertex_data_size; // Increment offset for the next element
            }
        }
    }

    pub(crate) fn render(&self, renderpass: &mut wgpu::RenderPass) {
        let vertex_buffer = match &self.vertex_buffer {
            Some(buffer) => buffer,
            None => { 
                eprintln!("Warning: GUI vertex buffer not initialized. Skipping Render...");
                return;
            }
        };
        renderpass.set_index_buffer(self.index_buffer.as_ref().unwrap().slice(..), wgpu::IndexFormat::Uint16);

        let mut vertex_offset_in_buffer = 0;
        let vertex_size_bytes = std::mem::size_of::<Vertex>() as wgpu::BufferAddress;
        let quad_vertices_count = 4;
        for _panel in 0..self.panels.len() {
            for _element in 0..self.panels[_panel].elements.len() {
                renderpass.set_vertex_buffer(0, vertex_buffer.slice(vertex_offset_in_buffer .. (vertex_offset_in_buffer + quad_vertices_count * vertex_size_bytes)));
                renderpass.draw_indexed(0..6, 0, 0..1);
                vertex_offset_in_buffer += quad_vertices_count * vertex_size_bytes;
            }
        }
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

    fn calculate_absolute_coordinates(&self, screen_size: PhysicalSize<u32>) -> (f32, f32, f32, f32) {
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
}

impl Element {
    pub fn new(start_coordinate: Coordinate, end_coordinate: Coordinate, color: Color) -> Self {
        Self {
            start_coordinate,
            end_coordinate,
            color,
        }
    }

    fn calculate_vertices_relative_to_panel(&self, panel_x_min: f32, panel_y_max: f32, panel_width: f32, panel_height: f32) -> [Vertex; 4] {
        let elem_local_x_min_rel = self.start_coordinate.x;
        let elem_local_x_max_rel = self.end_coordinate.x;
        let elem_local_y_min_rel = self.start_coordinate.y;
        let elem_local_y_max_rel = self.end_coordinate.y;

        let elem_px_x_min = panel_x_min + elem_local_x_min_rel * panel_width;
        let elem_px_x_max = panel_x_min + elem_local_x_max_rel * panel_width;
        let elem_px_y_max = panel_y_max - (elem_local_y_min_rel * panel_height);
        let elem_px_y_min = panel_y_max - (elem_local_y_max_rel * panel_height);

        [
            Vertex { position: Vec2::new(elem_px_x_min, elem_px_y_max), color: self.color.into_vec3() }, // Top-Left
            Vertex { position: Vec2::new(elem_px_x_max, elem_px_y_max), color: self.color.into_vec3() }, // Top-Right
            Vertex { position: Vec2::new(elem_px_x_min, elem_px_y_min), color: self.color.into_vec3() }, // Bottom-Left
            Vertex { position: Vec2::new(elem_px_x_max, elem_px_y_min), color: self.color.into_vec3() }, // Bottom-Right
        ]
    }
}

pub struct Coordinate {
    x: f32,
    y: f32,
}

impl Coordinate {
    pub fn new(x: f32, y: f32) -> Self {
        Self { 
            x, 
            y 
        }
    }
}

pub struct Color {
    r: f32,
    g: f32,
    b: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { 
            r, 
            g, 
            b, 
        }
    }

    fn into_vec3(&self) -> Vec3 {
        Vec3::new(self.r, self.g, self.b)
    }
}