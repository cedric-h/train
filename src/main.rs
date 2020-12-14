use glam::{Vec2, vec3, Vec3, Mat4, Vec4Swizzles};
use miniquad::*;
use std::convert::TryInto;

#[repr(C)]
struct Vertex {
    pos: Vec3,
    norm: Vec3,
    uv: Vec2,
}

struct Stage {
    pipeline: Pipeline,
    bindings: Bindings,
    primitive_indices: Vec<i32>,
    view_pos: Vec3,
}

impl Stage {
    pub fn new(ctx: &mut Context) -> Stage {
        let (doc, datas, images) = gltf::import("train.glb").unwrap();
        let mesh_data = doc.meshes().next().expect("no meshes");

        let mut vertices = vec![];
        let mut indices = vec![];
        let mut primitive_indices = vec![0];

        for prim in mesh_data.primitives() {
            let reader = prim.reader(|b| Some(&datas.get(b.index())?.0[..b.length()]));
            vertices.extend(
                reader
                    .read_positions()
                    .unwrap()
                    .zip(reader.read_normals().unwrap())
                    .zip(reader.read_tex_coords(0).unwrap().into_f32())
                    .map(|((pos, norm), uv)| {
                        Vertex { pos: pos.into(), norm: norm.into(), uv: uv.into() }
                    })
            );
            indices.extend(
                reader
                    .read_indices()
                    .unwrap()
                    .into_u32()
                    .map(|i| -> u16 {
                        i.try_into().unwrap()
                    })
            );
            primitive_indices.push(indices.len().try_into().unwrap());
        }

        let vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, &vertices);
        let index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &indices);

        let image = images.into_iter().next().unwrap();
        let texture = Texture::from_rgba8(ctx, image.width as _, image.height as _, &image.pixels);
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

        Stage { pipeline, bindings, primitive_indices, view_pos: vec3(0.0, 10.5, 10.5) }
    }
}

impl EventHandler for Stage {
    fn update(&mut self, _ctx: &mut Context) {}

    fn draw(&mut self, ctx: &mut Context) {
        let spin = Mat4::from_rotation_y(date::now().sin() as f32 * std::f32::consts::PI);
        let view_pos4 = spin * self.view_pos.extend(0.0);
        let view_pos = view_pos4.xyz();

        let (width, height) = ctx.screen_size();
        let proj = Mat4::perspective_rh_gl(60.0f32.to_radians(), width / height, 0.01, 50.0);
        let view = Mat4::look_at_rh(
            view_pos,
            view_pos / -3.0,
            vec3(0.0, 1.0, 0.0),
        );
        let mvp = proj * view;

        ctx.begin_default_pass(Default::default());

        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&self.bindings);
        ctx.apply_uniforms(&shader::Uniforms { mvp, view_pos, });
        for pair in self.primitive_indices.windows(2) {
            if let &[start, end] = pair {
                ctx.draw(start, end - start, 1);
            }
        }
        ctx.end_render_pass();

        ctx.commit_frame();
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

    uniform lowp vec3 view_pos;
    uniform sampler2D tex;

    void main() {
        lowp vec3 light_dir = normalize(vec3(50.0, 50.0, 50.0) - frag_pos);
        lowp vec3 light_color = vec3(1.0, 0.912, 0.802);
        lowp float light_strength = 1.5;

        lowp vec3 diffuse = light_color * max(dot(normal, light_dir), 0.2);

        lowp vec3 ambient = light_color * 0.2;

        lowp vec3 view_dir = normalize(view_pos - frag_pos);
        lowp vec3 reflect_dir = reflect(-light_dir, normal);
        lowp vec3 specular = light_color * pow(max(dot(view_dir, reflect_dir), 0.0), 32.0) * 0.4;

        gl_FragColor = texture2D(tex, texcoord) * vec4((ambient + diffuse + specular) * light_strength, 1.0);
    }"#;

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["tex".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: vec![
                    UniformDesc::new("mvp", UniformType::Mat4),
                    UniformDesc::new("view_pos", UniformType::Float3)
                ],
            },
        }
    }

    #[repr(C)]
    pub struct Uniforms {
        pub mvp: glam::Mat4,
        pub view_pos: glam::Vec3,
    }
}
