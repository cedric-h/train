#![feature(array_map)]
#![feature(new_uninit)]
use glam::{vec2, Vec2, vec3, Vec3, Mat4};
use miniquad::*;
use std::convert::TryInto;

mod art;
use art::{Art, ArtData, INDEX_COUNT};

struct Stage {
    pipeline: Pipeline,
    bindings: Bindings,
    mouse_pos: Vec2,
    view_pos: Vec3,
}

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

impl Stage {
    pub fn new(ctx: &mut Context) -> Stage {
        let (art, art_data) = unsafe {
            use std::io::Read;
            use std::mem::size_of;
            let mut file = std::fs::File::open("train.cedset").unwrap();

            let mut art_bytes = [0; size_of::<Art>()];
            file.read_exact(&mut art_bytes).unwrap();
            let art: Art = std::mem::transmute(art_bytes);

            let mut data_bytes = Box::new([0; size_of::<ArtData>()]);
            file.read_exact(data_bytes.as_mut()).unwrap();
            let data = transmute_copy_boxed::<[u8; size_of::<ArtData>()], ArtData>(data_bytes.as_ref());

            (art, data)
        };
        let vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, &art_data.vertices);
        let index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &art_data.indices);

        let texture = Texture::from_rgba8(ctx, 16, 16, &art_data.image);
        texture.set_filter(ctx, FilterMode::Nearest);

        let bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer: index_buffer,
            images: vec![texture],
        };

        let shader = Shader::new(ctx, shader::VERTEX, shader::FRAGMENT, shader::meta()).unwrap();

        let pipeline = Pipeline::with_params(
            ctx,
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("pos", VertexFormat::Float3),
                VertexAttribute::new("norm", VertexFormat::Float3),
                VertexAttribute::new("uv", VertexFormat::Float2)
            ],
            shader,
            PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: true,
                ..Default::default()
            }
        );

        Stage {
            pipeline,
            bindings,
            mouse_pos: Vec2::from(ctx.screen_size()) / 2.0,
            view_pos: vec3(0.0, 16.0, -5.0)
        }
    }
}

fn unproject(win: Vec2, mvp: Mat4, viewport: Vec2) -> Vec3 {
    mvp.inverse().transform_point3(vec3(
        2.0 * win.x / viewport.x - 1.0,
        2.0 * win.y / viewport.y - 1.0,
        1.0,
    ))
}

impl EventHandler for Stage {
    fn update(&mut self, _ctx: &mut Context) {}

    fn draw(&mut self, ctx: &mut Context) {
        let &mut Self { view_pos, mouse_pos, .. } = self;

        let (width, height) = ctx.screen_size();
        let proj = Mat4::perspective_rh_gl(45.0f32.to_radians(), width / height, 0.01, 50.0);
        let view = Mat4::look_at_rh(
            view_pos,
            vec3(0.0, 0.0, 7.0),
            vec3(0.0, 1.0, 0.0),
        );
        let mut mvp = proj * view;

        let out = unproject(
            vec2(mouse_pos.x, height - mouse_pos.y),
            mvp,
            ctx.screen_size().into()
        );
        let l = view_pos - out;
        let n = Vec3::unit_y();
        let d = (Vec3::zero() - view_pos).dot(n) / l.dot(n);
        mvp = mvp * Mat4::from_translation(view_pos + l * d);

        ctx.begin_default_pass(Default::default());

        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&self.bindings);
        ctx.apply_uniforms(&shader::Uniforms { mvp, });
        ctx.draw(0, INDEX_COUNT.try_into().unwrap(), 1);
        ctx.end_render_pass();

        ctx.commit_frame();
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32) {
        self.mouse_pos = vec2(x, y);
    }
}

fn main() {
    miniquad::start(conf::Conf { sample_count: 4, ..conf::Conf::default() }, |mut ctx| {
        UserData::owning(Stage::new(&mut ctx), ctx)
    });
}

mod shader {
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec3 pos;
    attribute vec3 norm;
    attribute vec2 uv;

    uniform mat4 mvp;

    varying lowp vec2 texcoord;
    varying lowp vec3 normal;
    varying lowp vec3 frag_pos;

    void main() {
        gl_Position = mvp * vec4(pos, 1);
        texcoord = uv;
        normal = norm;
        frag_pos = pos;
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec2 texcoord;
    varying lowp vec3 normal;
    varying lowp vec3 frag_pos;

    uniform sampler2D tex;

    void main() {
        lowp vec3 light_dir = normalize(vec3(50.0, 50.0, 50.0) - frag_pos);
        lowp vec3 light_color = vec3(1.0, 0.912, 0.802);
        lowp float light_strength = 1.5;

        lowp vec3 diffuse = light_color * max(dot(normal, light_dir), 0.2);

        lowp vec3 ambient = light_color * 0.2;

        gl_FragColor = texture2D(tex, texcoord) * vec4((ambient + diffuse) * light_strength, 1.0);
    }"#;

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["tex".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: vec![
                    UniformDesc::new("mvp", UniformType::Mat4),
                ],
            },
        }
    }

    #[repr(C)]
    pub struct Uniforms {
        pub mvp: glam::Mat4,
    }
}
