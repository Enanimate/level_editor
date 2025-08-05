use wgpu::{Device, Queue, util::DeviceExt};

use wgpu_text::{glyph_brush::{ab_glyph::{FontRef, PxScale}, Section, Text}, BrushBuilder, TextBrush};
use winit::dpi::{PhysicalPosition, PhysicalSize};

use crate::definitions::{GuiEvent, InteractionStyle, UiAtlas, Vertex};

pub struct Interface {
    pub panels: Vec<Panel>,
    pub(crate) vertex_buffer: Option<wgpu::Buffer>,
    pub(crate) index_buffer: Option<wgpu::Buffer>,
    brush: Option<TextBrush<FontRef<'static>>>,
    atlas: UiAtlas,
}

impl Interface {
    pub fn new(atlas: UiAtlas) -> Interface {
        Self {
            panels: Vec::new(),
            vertex_buffer: None,
            index_buffer: None,
            brush: None,
            atlas,
        }
    }

    pub fn add_panel(&mut self, panel: Panel) {
        self.panels.push(panel);
    }

    pub fn handle_interaction(&mut self, position: PhysicalPosition<f64>, screen_size: PhysicalSize<u32>, interaction_type: InteractionStyle) -> Option<(GuiEvent, (usize, usize))> {
        let x_position = position.x as f32 / screen_size.width as f32;
        let y_position = position.y as f32 / screen_size.height as f32;

        for (panel_idx, panel) in self.panels.iter().enumerate() {
            if x_position >= panel.start_coordinate.x && x_position <= panel.end_coordinate.x &&
            y_position >= panel.start_coordinate.y && y_position <= panel.end_coordinate.y {
                let rel_cursor_x = x_position - panel.start_coordinate.x;
                let rel_cursor_y = y_position - panel.start_coordinate.y;
                
                for (element_idx, element) in panel.elements.iter().enumerate() {
                    if rel_cursor_x >= element.start_coordinate.x && rel_cursor_x <= element.end_coordinate.x &&
                    rel_cursor_y >= element.start_coordinate.y && rel_cursor_y <= element.end_coordinate.y {
                        
                        if interaction_type == InteractionStyle::OnClick && element.on_click.is_some() {
                            if let Some(event) = element.handle_click(interaction_type.clone()) {
                                return Some((event, (panel_idx, element_idx)));
                            }
                        } else if interaction_type == InteractionStyle::OnHover && element.on_hover.is_some() {
                            if let Some(event) = element.handle_click(interaction_type.clone()) {
                                return Some((event, (panel_idx, element_idx)));
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn is_cursor_within_menu_panel_bounds(&self, position: PhysicalPosition<f64>, screen_size: PhysicalSize<u32>) -> bool {
        let x_position = position.x as f32 / screen_size.width as f32;
        let y_position = position.y as f32 / screen_size.height as f32;

        for panel in self.panels.iter() {
            if x_position >= panel.start_coordinate.x && x_position <= panel.end_coordinate.x &&
            y_position >= panel.start_coordinate.y && y_position <= panel.end_coordinate.y {
                return true;
            }
        } false
    }

    pub fn reset_all_element_colors(&mut self) {
        for panel in &mut self.panels {
            for element in &mut panel.elements {
                element.color = element.original_color.clone();
            }
        }
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
            (self.panels.iter().flat_map(|panel| &panel.elements).count() * 4) + (self.panels.iter().count() * 4);
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

            let mut panel_tex_coords: [[f32; 2]; 4] = [
                [0.0, 0.0],
                [0.0, 0.0],
                [0.0, 0.0],
                [0.0, 0.0],
            ];

            for entry in &self.atlas.entries {
                if entry.name == panel.texture_name {
                    panel_tex_coords = [
                        [entry.start_coord.unwrap().0, entry.start_coord.unwrap().1],
                        [entry.end_coord.unwrap().0, entry.start_coord.unwrap().1],
                        [entry.end_coord.unwrap().0, entry.end_coord.unwrap().1],
                        [entry.start_coord.unwrap().0, entry.end_coord.unwrap().1]
                    ];
                }
            }

            if panel.renderable == true {
                let panel_vertices = [
                    Vertex {
                        position: [panel_x_min_co, panel_y_max_co],
                        color: panel.color.into_vec4(),
                        tex_coords: panel_tex_coords[0]
                    }, // Top-Left
                    Vertex {
                        position: [panel_x_max_co, panel_y_max_co],
                        color: panel.color.into_vec4(),
                        tex_coords: panel_tex_coords[1]
                    }, // Top-Right
                    Vertex {
                        position: [panel_x_min_co, panel_y_min_co],
                        color: panel.color.into_vec4(),
                        tex_coords: panel_tex_coords[3]
                    }, // Bottom-Left
                    Vertex {
                        position: [panel_x_max_co, panel_y_min_co],
                        color: panel.color.into_vec4(),
                        tex_coords: panel_tex_coords[2]
                    }, // Bottom-Right
                ];

                let vertex_data_slice = bytemuck::cast_slice(&panel_vertices);
                let vertex_data_size = vertex_data_slice.len() as wgpu::BufferAddress;

                queue.write_buffer(
                    self.vertex_buffer.as_ref().unwrap(),
                    vertex_offset,
                    vertex_data_slice,
                );

                vertex_offset += vertex_data_size;
            }

            let mut tex_coords: [[f32; 2]; 4] = [
                        [0.0, 0.0],
                        [0.0, 0.0],
                        [0.0, 0.0],
                        [0.0, 0.0],
                    ];

            
            for element in &mut panel.elements {
                for entry in &self.atlas.entries {
                    if entry.name == element.texture_name {
                        tex_coords = [
                         [entry.start_coord.unwrap().0, entry.start_coord.unwrap().1],
                         [entry.end_coord.unwrap().0, entry.start_coord.unwrap().1],
                         [entry.end_coord.unwrap().0, entry.end_coord.unwrap().1],
                         [entry.start_coord.unwrap().0, entry.end_coord.unwrap().1]
                        ];
                    }
                }

                let new_vertices = element.calculate_vertices_relative_to_panel(
                    panel_x_min_co,
                    panel_y_min_co,
                    panel_x_max_co,
                    panel_y_max_co,
                    tex_coords
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
                    let text_content_str = text_content.0.as_str();

                    let section = Section::builder()
                        .with_screen_position([adjusted_x, adjusted_y])
                        .with_text(vec![
                            Text::new(text_content_str)
                                .with_scale(PxScale {x: 30.0 * text_content.1, y: 30.0 * text_content.1})
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

    fn text_alignment(ex_0: f32, ey_0: f32, ex_1: f32, ey_1: f32, px_0: f32, py_0: f32, px_1: f32, py_1: f32, screen_size: PhysicalSize<u32>, alignment: &Alignment, text: &(String, f32)) -> ((f32, f32), f32){
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
                let half_y_length = ((py_1 - ey_0 * (py_1 - py_0)) - (py_1 - ey_1 * (py_1 - py_0))) / 2.0;
                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x + (15.0 * text.1), y + half_y_length - (15.0 * text.1)), scale);
            }
            (HorizontalAlignment::Left, VerticalAlignment::Bottom) => {
                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_1 * (py_1 - py_0));
                return ((x, y - (30.0 * text.1)), scale);
            }



            (HorizontalAlignment::Center, VerticalAlignment::Top) => {
                let text_offset = (text.0.chars().count() as f32 * (15.0 * text.1)) / 2.0;

                let half_x_length = ((px_0 + ex_1 * (px_1 - px_0)) - (px_0 + ex_0 * (px_1 - px_0))) / 2.0;

                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x + half_x_length - text_offset, y), scale);
            }
            (HorizontalAlignment::Center, VerticalAlignment::Center) => {
                let text_offset = (text.0.chars().count() as f32 * (15.0 * text.1)) / 2.0;

                let half_x_length = ((px_0 + ex_1 * (px_1 - px_0)) - (px_0 + ex_0 * (px_1 - px_0))) / 2.0;
                let half_y_length = ((py_1 - ey_0 * (py_1 - py_0)) - (py_1 - ey_1 * (py_1 - py_0))) / 2.0;

                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x + half_x_length - text_offset, y + half_y_length - (15.0 * text.1)), scale);
            }
            (HorizontalAlignment::Center, VerticalAlignment::Bottom) => {
                let text_offset = (text.0.chars().count() as f32 * (15.0 * text.1)) / 2.0;
                
                let half_x_length = ((px_0 + ex_1 * (px_1 - px_0)) - (px_0 + ex_0 * (px_1 - px_0))) / 2.0;

                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_1 * (py_1 - py_0));
                return ((x + half_x_length - text_offset, y - 15.0), scale);
            }


            
            (HorizontalAlignment::Right, VerticalAlignment::Top) => {
                let text_offset = text.0.chars().count() as f32 * (15.0 * text.1);

                let half_x_length = (px_0 + ex_1 * (px_1 - px_0)) - (px_0 + ex_0 * (px_1 - px_0));

                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x + half_x_length - text_offset, y), scale);
            }
            (HorizontalAlignment::Right, VerticalAlignment::Center) => {
                let text_offset = text.0.chars().count() as f32 * (15.0 * text.1);

                let half_x_length = (px_0 + ex_1 * (px_1 - px_0)) - (px_0 + ex_0 * (px_1 - px_0));
                let half_y_length = ((py_1 - ey_0 * (py_1 - py_0)) - (py_1 - ey_1 * (py_1 - py_0))) / 2.0;

                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_0 * (py_1 - py_0));
                return ((x + half_x_length - text_offset, y + half_y_length - 15.0), scale);
            }
            (HorizontalAlignment::Right, VerticalAlignment::Bottom) => {
                let text_offset = text.0.chars().count() as f32 * (15.0 * text.1);

                let half_x_length = (px_0 + ex_1 * (px_1 - px_0)) - (px_0 + ex_0 * (px_1 - px_0));

                let x = screen_x_center + (px_0 + ex_0 * (px_1 - px_0));
                let y = screen_y_center - (py_1 - ey_1 * (py_1 - py_0));
                return ((x + half_x_length - text_offset, y - 15.0), scale);
            }
        }
    }

    pub(crate)  fn draw_text_brush<'a>( &'a self, renderpass: &mut wgpu::RenderPass<'a>) {
        if let Some(brush) = self.brush.as_ref() {
            brush.draw(renderpass);
        } else {
            eprintln!("Warning: Brush not initialized for drawing.");
        }
    }

    pub(crate) fn render<'a>(&'a self, renderpass: &mut wgpu::RenderPass<'a>) {
        let vertex_buffer = match &self.vertex_buffer {
            Some(buffer) => buffer,
            None => {
                eprintln!("Warning: GUI vertex buffer not initialized. Skipping Render...");
                return;
            }
        };
        let index_buffer = match &self.index_buffer {
            Some(buffer) => buffer,
            None => {
                eprintln!("Warning: GUI index buffer not initialized. Skipping Render...");
                return;
            }
        };
        renderpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
    
        let mut vertex_offset_in_buffer = 0;
        let vertex_size_bytes = std::mem::size_of::<Vertex>() as wgpu::BufferAddress;
        let quad_vertices_count = 4;
        let quad_indices_count = 6;
        let quad_buffer_size = quad_vertices_count * vertex_size_bytes;
    
        for panel in &self.panels {
            if panel.renderable {
                renderpass.set_vertex_buffer(
                    0,
                    vertex_buffer.slice(vertex_offset_in_buffer..(vertex_offset_in_buffer + quad_buffer_size)),
                );
                renderpass.draw_indexed(0..quad_indices_count, 0, 0..1);
                vertex_offset_in_buffer += quad_buffer_size;
            }
    
            for _element in &panel.elements {
                renderpass.set_vertex_buffer(
                    0,
                    vertex_buffer.slice(vertex_offset_in_buffer..(vertex_offset_in_buffer + quad_buffer_size)),
                );
                renderpass.draw_indexed(0..quad_indices_count, 0, 0..1);
                vertex_offset_in_buffer += quad_buffer_size;
            }
        }
    }
}

pub struct Panel {
    pub elements: Vec<Element>,
    start_coordinate: Coordinate,
    end_coordinate: Coordinate,
    renderable: bool,
    texture_name: String,
    color: Color,
}

impl Panel {
    pub fn new(start_coordinate: Coordinate, end_coordinate: Coordinate) -> Self {
        Self {
            elements: Vec::new(),
            start_coordinate,
            end_coordinate,
            renderable: false,
            texture_name: "solid".to_string(),
            color: Color::from_hex("#ffffffff"),
        }
    }

    pub fn add_element(&mut self, element: Element) {
        self.elements.push(element);
    }

    pub fn with_color(mut self, color: &str) -> Self {
        self.renderable = true;
        self.color = Color::from_hex(color);
        self
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
    pub color: Color,
    pub original_color: Color,
    text: Option<(String, f32)>,
    text_alignment: Option<Alignment>,
    on_click: Option<Box<dyn Fn() -> Option<GuiEvent> + 'static>>,
    on_hover: Option<Box<dyn Fn() -> Option<GuiEvent> + 'static>>,
    texture_name: String
}

impl Element {
    pub fn new(start_coordinate: Coordinate, end_coordinate: Coordinate, texture_name: &str) -> Self {
        Self {
            start_coordinate,
            end_coordinate,
            color: Color::from_hex("#ffffffff"),
            original_color: Color::from_hex("#ffffffff"),
            text: None,
            text_alignment: None,
            on_click: None,
            on_hover: None,
            texture_name: texture_name.to_string(),
        }
    }

    pub fn with_fn(mut self, func: impl Fn() -> Option<GuiEvent> + 'static, style: InteractionStyle) -> Self {
        if style == InteractionStyle::OnClick {
            self.on_click = Some(Box::new(func));
        } else if style == InteractionStyle::OnHover {
            self.on_hover = Some(Box::new(func));
        }
        self
    }

    pub fn with_color(mut self, color: &str) -> Self {
        let new_color = Color::from_hex(color);
        self.color = new_color.clone();
        self.original_color = new_color;
        self
    }

    pub fn with_text(mut self, alignment: Alignment, text: &str, scale: f32) -> Self {
        self.text = Some((text.to_string(), scale));
        self.text_alignment = Some(alignment);
        self
    }

    pub fn handle_click(&self, interaction_type: InteractionStyle) -> Option<GuiEvent> {
        let function_src = if interaction_type == InteractionStyle::OnClick {
            &self.on_click
        } else {
            &self.on_hover
        };
        if let Some(func) = function_src {
            func()
        } else {
            None
        }
    }

    pub fn with_temp_color(&mut self, color: &str) {
        let new_color = Color::from_hex(color);
        self.color = new_color;
    }

    fn calculate_vertices_relative_to_panel(
        &mut self,
        panel_x_min_center_origin: f32,
        panel_y_min_center_origin: f32,
        panel_x_max_center_origin: f32,
        panel_y_max_center_origin: f32,
        tex_coords: [[f32; 2]; 4]
    ) -> [Vertex; 4] {

        // Convert element's local coordinates to panel's absolute coordinates (center-origin)
        let element_abs_x_min_center_origin = panel_x_min_center_origin
            + self.start_coordinate.x * (panel_x_max_center_origin - panel_x_min_center_origin);
        let element_abs_x_max_center_origin = panel_x_min_center_origin
            + self.end_coordinate.x * (panel_x_max_center_origin - panel_x_min_center_origin);

        // Y-axis is inverted here: y_max_center_origin is top, y_min_center_origin is bottom
        // elem_local_y_min_rel corresponds to the top of the element relative to panel's top (0.0 to 1.0)
        // elem_local_y_max_rel corresponds to the bottom of the element relative to panel's top (0.0 to 1.0)
        let element_abs_y_top_center_origin = panel_y_max_center_origin
            - self.start_coordinate.y * (panel_y_max_center_origin - panel_y_min_center_origin);
        let element_abs_y_bottom_center_origin = panel_y_max_center_origin
            - self.end_coordinate.y * (panel_y_max_center_origin - panel_y_min_center_origin);

        let vtx_x_min = element_abs_x_min_center_origin;
        let vtx_x_max = element_abs_x_max_center_origin;
        let vtx_y_top = element_abs_y_top_center_origin; // The Y coordinate for the top edge of the element
        let vtx_y_bottom = element_abs_y_bottom_center_origin; // The Y coordinate for the bottom edge of the element

        [
            Vertex {
                position: [vtx_x_min, vtx_y_top],
                color: self.color.into_vec4(),
                tex_coords: tex_coords[0]
            }, // Top-Left
            Vertex {
                position: [vtx_x_max, vtx_y_top],
                color: self.color.into_vec4(),
                tex_coords: tex_coords[1]
            }, // Top-Right
            Vertex {
                position: [vtx_x_min, vtx_y_bottom],
                color: self.color.into_vec4(),
                tex_coords: tex_coords[3]
            }, // Bottom-Left
            Vertex {
                position: [vtx_x_max, vtx_y_bottom],
                color: self.color.into_vec4(),
                tex_coords: tex_coords[2]
            }, // Bottom-Right
        ]
    }
}

pub struct Coordinate {
    pub x: f32,
    pub y: f32,
}

impl Coordinate {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone)]
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    fn into_vec4(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn from_hex(hex_color: &str) -> Self {
        if let Some(hex) = hex_color.strip_prefix("#") {
            let red = u32::from_str_radix(&hex[0..2], 16).unwrap() as f32 / 255.0;
            let green = u32::from_str_radix(&hex[2..4], 16).unwrap() as f32 / 255.0;
            let blue = u32::from_str_radix(&hex[4..6], 16).unwrap() as f32 / 255.0;
            let alpha = u32::from_str_radix(&hex[6..8], 16).unwrap() as f32 / 255.0;

            let (corrected_r, corrected_g, corrected_b) = Self::srgb_correction(red, green, blue);
            
            Self {
                r: corrected_r,
                g: corrected_g,
                b: corrected_b,
                a: alpha
            }
        } else {
            log::error!("Provided parameter was not hex!");
            panic!()
        }
    }

    fn srgb_correction(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
        let mut linear_color = (0.0, 0.0, 0.0);

        if x <= 0.04045 {
            linear_color.0 = x / 12.92;
        } else {
            linear_color.0 = ((x + 0.055) / 1.055).powf(2.4);
        }

        if y <= 0.04045 {
            linear_color.1 = y / 12.92;
        } else {
            linear_color.1 = ((y + 0.055) / 1.055).powf(2.4);
        }

        if z <= 0.04045 {
            linear_color.2 = z / 12.92;
        } else {
            linear_color.2 = ((z + 0.055) / 1.055).powf(2.4);
        }

        linear_color
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