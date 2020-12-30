use glam::{vec2, vec3, Mat4, Vec2, Vec3};
use miniquad::*;

use train::art::{Art, ArtData};

mod render;

/// I wanted to name this `train` but that's what the crate's named.
/// This is an abstraction for modelling a complex vehicle with linked
/// train cars as a Vec<Car>.
mod cars;

fn read_art_data() -> Box<ArtData> {
    use std::io::Read;
    const SIZE: usize = std::mem::size_of::<ArtData>();

    let mut data_bytes = Box::new([0; SIZE]);
    std::fs::File::open("train.cedset").unwrap().read_exact(data_bytes.as_mut()).unwrap();

    unsafe {
        unsafe fn transmute_copy_boxed<T, U>(src: &T) -> Box<U> {
            let src = src as *const T as *const U;
            let layout = std::alloc::Layout::new::<U>();
            let dst = std::alloc::alloc(layout) as *mut U;

            if dst.is_null() {
                std::alloc::handle_alloc_error(layout)
            } else {
                src.copy_to(dst, 1);
                Box::from_raw(dst)
            }
        }

        transmute_copy_boxed::<[u8; SIZE], ArtData>(data_bytes.as_ref())
    }
}

#[derive(Default, Debug)]
struct RenderQueue(Vec<(Art, Mat4)>);
impl RenderQueue {
    fn draw_mat4(&mut self, art: Art, mat: Mat4) {
        self.0.push((art, mat));
    }

    fn draw(&mut self, art: Art, pos: Vec2, rot: Rot) {
        use std::f32::consts::FRAC_PI_2;

        self.draw_mat4(
            art,
            Mat4::from_translation(vec3(pos.x, 0.0, pos.y))
                * Mat4::from_rotation_y(FRAC_PI_2 - rot.0),
        )
    }

    fn clear_draws(&mut self) {
        self.0.clear();
    }
}

struct Stage {
    mouse_pos: Vec2,
    mouse_on_ground: Vec3,
    cam_origin: Vec3,
    cam_offset: Vec3,
    track: Vec<Vec2>,
    renderer: render::Renderer,
    render_queue: RenderQueue,
    train: cars::Cars,
}

impl Stage {
    fn new(ctx: &mut Context) -> Self {
        let mut art_data = read_art_data();
        let track = art_data.make_track();

        Stage {
            mouse_pos: Vec2::from(ctx.screen_size()) / 2.0,
            mouse_on_ground: Vec3::zero(),
            cam_offset: Vec3::zero(),
            cam_origin: Vec3::zero(),
            renderer: render::Renderer::new(ctx, art_data),
            render_queue: RenderQueue(Vec::with_capacity(1000)),
            train: cars::Cars::default(),
            track,
        }
    }

    fn eye_pos(&self) -> Vec3 {
        self.cam_origin + self.cam_offset
    }

    fn track_point(&self, distance: f32) -> Vec2 {
        let mut so_far = 0.0;
        for pair in self.track.windows(2) {
            if let &[left, right] = pair {
                let len = (left - right).length();
                if distance < so_far + len {
                    return left.lerp(right, (distance - so_far) / len);
                } else {
                    so_far += len;
                }
            }
        }

        panic!("no points?");
    }
}

impl EventHandler for Stage {
    fn update(&mut self, ctx: &mut Context) {
        let eye_pos = self.eye_pos();
        let (w, h) = ctx.screen_size();
        let (x, y) = self.mouse_pos.into();
        let out = unproject(vec2(x, h - y), self.view_proj(), vec2(w, h));
        self.mouse_on_ground =
            line_plane_intersect(eye_pos, eye_pos - out, Vec3::zero(), Vec3::unit_y());

        let mut rq = std::mem::take(&mut self.render_queue);
        rq.clear_draws();
        self.draw_train(&mut rq);
        self.render_queue = rq;
    }

    fn draw(&mut self, ctx: &mut Context) {
        self.render(ctx);
    }

    fn resize_event(&mut self, ctx: &mut Context, _: f32, _: f32) {
        self.renderer.resize(ctx);
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32) {
        self.mouse_pos = vec2(x, y);
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Rot(f32);
impl Rot {
    fn vec2(self) -> Vec2 {
        let (y, x) = self.0.sin_cos();
        vec2(x, y)
    }

    fn from_vec2(Vec2 { x, y }: Vec2) -> Self {
        Self(y.atan2(x))
    }

    #[allow(dead_code)]
    fn apply(self, v: Vec2) -> Vec2 {
        let len = v.length();
        let angle = Rot::from_vec2(v);
        Rot(angle.0 + self.0).vec2() * len
    }

    #[allow(dead_code)]
    fn unapply(self, v: Vec2) -> Vec2 {
        let len = v.length();
        let angle = Rot::from_vec2(v);
        Rot(angle.0 - self.0).vec2() * len
    }
}

fn ground_vec2(Vec2 { x, y }: Vec2) -> Vec3 {
    vec3(x, 0.0, y)
}

fn unproject(win: Vec2, mvp: Mat4, viewport: Vec2) -> Vec3 {
    mvp.inverse().transform_point3(vec3(
        2.0 * win.x / viewport.x - 1.0,
        2.0 * win.y / viewport.y - 1.0,
        1.0,
    ))
}

fn line_plane_intersect(
    line_origin: Vec3,
    line: Vec3,
    plane_origin: Vec3,
    plane_normal: Vec3,
) -> Vec3 {
    let d = (plane_origin - line_origin).dot(plane_normal) / line.dot(plane_normal);
    line_origin + line * d
}

fn main() {
    miniquad::start(conf::Conf { sample_count: 4, ..conf::Conf::default() }, |mut ctx| {
        UserData::owning(Stage::new(&mut ctx), ctx)
    });
}
