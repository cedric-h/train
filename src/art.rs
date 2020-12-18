use glam::{Vec2, Vec3};

#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct Vertex {
    pub pos: Vec3,
    pub norm: Vec3,
    pub uv: Vec2,
}

pub struct Art {
    pub train: (i32, i32),
    pub cart: (i32, i32),
}

pub const INDEX_COUNT: usize = 25692;
pub const VERTEX_COUNT: usize = 11094;
pub const IMAGE_SIZE: usize = 16 * 16 * 4;

#[repr(C)]
pub struct ArtData {
    pub image: [u8; IMAGE_SIZE],
    pub vertices: [Vertex; VERTEX_COUNT],
    pub indices: [i16; INDEX_COUNT],
}

impl Default for ArtData {
    fn default() -> Self {
        Self {
            image: [0; IMAGE_SIZE],
            vertices: [Default::default(); VERTEX_COUNT],
            indices: [0; INDEX_COUNT],
        }
    }
}
