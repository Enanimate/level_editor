use glam::{Vec2, Vec3};

#[allow(dead_code)]

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub(crate) struct Vertex {
    pub(crate) position: Vec2,
    pub(crate) color: Vec3,
    pub(crate) tex_coords: Vec2,
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

impl Vertex {
    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress + std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                }
            ]
        }
    }
}

#[derive(Debug, Clone)]
pub struct UiAtlas {
    pub entries: Vec<UiAtlasTexture>,
    width: u32,
    height: u32,
}

impl UiAtlas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            entries: Vec::new(),
            width,
            height
        }
    }

    pub fn add_entry(&mut self, entry: UiAtlasTexture) {
        self.entries.push(entry.generate_tex_coords(self.width, self.height));
    }
}

#[derive(Debug, Clone)]
pub struct UiAtlasTexture {
    pub name: String,
    x_start: u32,
    y_start: u32,
    image_width: u32,
    image_height: u32,
    pub start_coord: Option<(f32, f32)>,
    pub end_coord: Option<(f32, f32)>
}

impl UiAtlasTexture {
    pub fn new(name: String, x_0: u32, y_0: u32, image_width: u32, image_height: u32) -> Self {
        Self {
            name,
            x_start: x_0,
            y_start: y_0,
            image_width,
            image_height,
            start_coord: None,
            end_coord: None,
        }
    }

    fn generate_tex_coords(mut self, width: u32, height: u32) -> Self {
        let x0 = self.x_start as f32 / width as f32;
        let y0 = self.y_start as f32 / height as f32;
        let x1 = (self.x_start + self.image_width) as f32 / width as f32;
        let y1 = (self.y_start + self.image_height) as f32 / height as f32;

        self.start_coord = Some((x0, y0));
        self.end_coord = Some((x1, y1));
        self
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable)]
pub(crate) struct GuiUniform {
    pub(crate) use_texture: u32,
}

unsafe impl bytemuck::Pod for GuiUniform {}

#[derive(Debug)]
pub enum GuiEvent {
    ChangeLayoutToFileExplorer
}

#[derive(PartialEq, Debug, Clone)]
pub enum GuiState {
    ProjectView,
    FileExplorer,
}