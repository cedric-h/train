use glam::{vec2, vec3, Mat4, Vec2, Vec3};
use miniquad::*;

use train::art::{Art, ArtData};
mod render;

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

struct Stage {
    mouse_pos: Vec2,
    mouse_on_ground: Vec3,
    cam_origin: Vec3,
    cam_offset: Vec3,
    track: Vec<Vec3>,
    renderer: render::Renderer,
}

impl Stage {
    fn new(ctx: &mut Context) -> Self {
        let mut art_data = read_art_data();
        let track = art_data.make_track();

        Stage {
            mouse_pos: Vec2::from(ctx.screen_size()) / 2.0,
            mouse_on_ground: track[0],
            cam_offset: vec3(0.0, 16.0, -12.0),
            cam_origin: track[0],
            renderer: render::Renderer::new(ctx, art_data),
            track,
        }
    }

    fn eye_pos(&self) -> Vec3 {
        self.cam_origin + self.cam_offset
    }
}

impl EventHandler for Stage {
    fn update(&mut self, _ctx: &mut Context) {}

    fn draw(&mut self, ctx: &mut Context) {
        self.render(ctx);
    }

    fn resize_event(&mut self, ctx: &mut Context, _: f32, _: f32) {
        self.renderer.resize(ctx);
    }

    fn mouse_motion_event(&mut self, ctx: &mut Context, x: f32, y: f32) {
        self.mouse_pos = vec2(x, y);

        let eye_pos = self.eye_pos();
        let (w, h) = ctx.screen_size();
        let out = unproject(vec2(x, h - y), self.view_proj(), vec2(w, h));
        self.mouse_on_ground = line_plane_intersect(
            eye_pos, eye_pos - out,
            Vec3::zero(), Vec3::unit_y(),
        );
    }
}

fn unproject(win: Vec2, mvp: Mat4, viewport: Vec2) -> Vec3 {
    mvp.inverse().transform_point3(vec3(
        2.0 * win.x / viewport.x - 1.0,
        2.0 * win.y / viewport.y - 1.0,
        1.0,
    ))
}

fn line_plane_intersect(line_origin: Vec3, line: Vec3, plane_origin: Vec3, plane_normal: Vec3) -> Vec3 {
    let d = (plane_origin - line_origin).dot(plane_normal) / line.dot(plane_normal);
    line_origin + line * d
}


fn main() {
    miniquad::start(conf::Conf { sample_count: 4, ..conf::Conf::default() }, |mut ctx| {
        UserData::owning(Stage::new(&mut ctx), ctx)
    });
}
