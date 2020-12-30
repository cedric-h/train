use glam::{Vec2, Vec3};
use std::convert::TryInto;

#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct Vertex {
    pub pos: Vec3,
    pub norm: Vec3,
    pub uv: Vec2,
}

macro_rules! art {
    ( $( $enum:ident : $field:ident ; )* ) => {
        #[derive(Copy, Clone, Debug)]
        pub enum Art { $( $enum, )* }

        #[derive(Default)]
        pub struct ArtIndices { $( $field: (i32, i32), )* }
        impl ArtIndices {
            /// Returns the indices for this Art
            pub fn indices(&self, art: Art) -> (i32, i32) {
                match art {
                    $( Art::$enum => self.$field, )*
                }
            }
        }

        #[cfg(feature = "gltf")]
        #[derive(Default)]
        pub struct ArtIndicesBuilder { $( $field: Option<(i32, i32)>, )* }

        #[cfg(feature = "gltf")]
        impl ArtIndicesBuilder {
            pub fn insert(&mut self, field: &str, start_u: usize, num_u: usize) {
                use std::convert::TryInto;
                match field {
                    $(stringify!($field) => {
                        self.$field = Some((start_u.try_into().unwrap(), num_u.try_into().unwrap()));
                    })*
                    other => panic!("{} is no mesh of ours!", other),
                }
            }

            pub fn unwrap(self) -> ArtIndices {
                ArtIndices { $( $field: self.$field.expect(concat!("no ", stringify!($field))), )* }
            }
        }
    }
}

art! {
    Cart: cart;
    Train: train;
    Wheel: wheel;
    Gun: gun;
}

#[derive(Debug, Copy, Clone, Default)]
#[cfg_attr(feature = "gltf-to-cedset", derive(serde::Deserialize))]
pub struct BezierPoint {
    pub left: (f32, f32),
    pub right: (f32, f32),
    pub pos: (f32, f32),
}
impl BezierPoint {
    pub fn centered(p: Vec2) -> Self {
        Self { left: p.into(), right: p.into(), pos: p.into() }
    }

    pub fn right(&self) -> Vec2 {
        self.right.into()
    }

    pub fn left(&self) -> Vec2 {
        self.left.into()
    }

    pub fn pos(&self) -> Vec2 {
        self.pos.into()
    }
}

#[repr(C)]
pub struct Track<const N: usize>([BezierCurve; N]);

impl<const N: usize> Default for Track<N> {
    fn default() -> Self {
        Track([Default::default(); N])
    }
}

#[derive(Default, Clone, Copy)]
#[repr(C)]
pub struct BezierCurve {
    pub start: Vec2,
    pub left: Vec2,
    pub right: Vec2,
    pub end: Vec2,
}
impl BezierCurve {
    pub fn new(start: BezierPoint, end: BezierPoint) -> Self {
        Self { start: start.pos(), left: start.right(), right: end.left(), end: end.pos() }
    }

    pub fn point(&self, t: f32) -> Vec2 {
        fn thlerp(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
            p0.lerp(p1, t).lerp(p1.lerp(p2, t), t)
        }

        let &Self { start, left, right, end } = self;

        thlerp(start, left, right, t).lerp(thlerp(left, right, end, t), t)
    }

    pub fn len(&self) -> f32 {
        (0..51)
            .map(|n| self.point(n as f32 / 50.0))
            .fold((self.start, 0.0), |(p0, len), p1| (p1, (p0 - p1).length() + len))
            .1
    }
}

impl<const N: usize> Track<N> {
    pub fn from_points(points: &[BezierPoint]) -> Self {
        let mut pairs = points.windows(2);
        let mut got = 0;
        Track([(); N].map(|_| {
            if let Some(&[start, end]) = pairs.next() {
                got += 1;
                BezierCurve::new(start, end)
            } else {
                panic!("ran out of bezier points! \nexpected: {}\ngot: {}", N, got)
            }
        }))
    }

    pub fn len(&self) -> f32 {
        self.0.iter().map(|curve| curve.len()).sum()
    }

    pub fn point(&self, t: f32) -> Vec2 {
        let total = self.len();
        let mut so_far = 0.0;
        for segment in &self.0 {
            let len = segment.len() / total;
            if so_far + len >= t {
                return segment.point((t - so_far) / len);
            }
            so_far += len;
        }

        Vec2::zero()
    }
}

pub const INDEX_COUNT: usize = 32000;
pub const VERTEX_COUNT: usize = 16000;
pub const IMAGE_SIZE: usize = 16 * 16 * 4;

#[repr(C)]
pub struct ArtData {
    pub image: [u8; IMAGE_SIZE],
    pub vertices: [Vertex; VERTEX_COUNT],
    pub last_occupied_vert: u32,
    pub last_occupied_index: u32,
    pub indices: [i16; INDEX_COUNT],
    pub art_indices: ArtIndices,
    pub track_indices: (i32, i32),
    pub track: Track<3>,
}

impl Default for ArtData {
    fn default() -> Self {
        Self {
            image: [0; IMAGE_SIZE],
            vertices: [Default::default(); VERTEX_COUNT],
            indices: [0; INDEX_COUNT],
            art_indices: Default::default(),
            track: Default::default(),
            last_occupied_vert: 0,
            last_occupied_index: 0,
            track_indices: (0, 0),
        }
    }
}

impl ArtData {
    fn add_vert(&mut self, vert: Vertex) -> i16 {
        let vert_index = self.last_occupied_vert as usize;
        self.vertices[vert_index] = vert;
        self.last_occupied_vert += 1;
        vert_index.try_into().unwrap()
    }

    fn add_index(&mut self, index: i16) {
        self.indices[self.last_occupied_index as usize] = index;
        self.last_occupied_index += 1;
    }

    fn line(&mut self, from: Vec2, to: Vec2, thickness: f32) {
        fn vert(pos: Vec2) -> Vertex {
            Vertex {
                pos: glam::vec3(pos.x, 0.0, pos.y),
                norm: Vec3::unit_y(),
                uv: Vec2::new(2.0, 0.0) / 16.0 + pos.normalize().abs() / 64.0,
            }
        }

        let normal = (from - to).normalize().perp();
        let left_from = self.add_vert(vert(from - normal * thickness / 2.0));
        let right_from = self.add_vert(vert(from + normal * thickness / 2.0));
        let left_to = self.add_vert(vert(to - normal * thickness / 2.0));
        let right_to = self.add_vert(vert(to + normal * thickness / 2.0));

        for &index in &[left_from, right_from, left_to, left_to, right_to, right_from] {
            self.add_index(index);
        }
    }

    /// Turns the Track data into geometry.
    pub fn make_track(&mut self) -> Vec<Vec2> {
        let start_index = self.last_occupied_index;

        let rails = (self.track.len() as f32) as usize;
        let mut points = Vec::with_capacity(rails);
        let mut before = self.track.point(1.0);
        for i in 0..=rails {
            let end_i = if i == rails { 1 } else { i + 1 };
            let end = self.track.point(end_i as f32 / rails as f32);
            let normal = (before - end).normalize().perp() * 1.2;

            let middle = self.track.point(i as f32 / rails as f32);
            self.line(middle + normal, middle - normal, 0.2);
            before = middle;
            points.push(middle);
        }

        let start: i32 = start_index.try_into().unwrap();
        let last: i32 = self.last_occupied_index.try_into().unwrap();
        self.track_indices = (start, last - start);

        points
    }
}
