use glam::{vec3, Mat4};
use miniquad::*;
use train::art::{ArtData, ArtIndices};

pub struct Renderer {
    pipeline: Pipeline,
    bindings: Bindings,
    art_indices: ArtIndices,
    proj: Mat4,
    track_indices: (i32, i32),
}
impl Renderer {
    pub fn new(ctx: &mut Context, art_data: Box<ArtData>) -> Self {
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
                VertexAttribute::new("uv", VertexFormat::Float2),
            ],
            shader,
            PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: true,
                ..Default::default()
            },
        );

        Renderer {
            pipeline,
            bindings,
            proj: proj(ctx),
            art_indices: art_data.art_indices,
            track_indices: art_data.track_indices,
        }
    }

    pub fn resize(&mut self, ctx: &mut Context) {
        self.proj = proj(ctx);
    }
}

fn proj(ctx: &mut Context) -> Mat4 {
    let (width, height) = ctx.screen_size();
    Mat4::perspective_rh_gl(45.0f32.to_radians(), width / height, 0.01, 250.0)
}

impl super::Stage {
    pub fn view_proj(&self) -> Mat4 {
        let &Self { cam_offset, cam_origin, .. } = self;
        let view_pos = cam_offset + cam_origin;
        let view = Mat4::look_at_rh(view_pos, cam_origin, vec3(0.0, 1.0, 0.0));
        self.renderer.proj * view
    }

    pub fn render(&mut self, ctx: &mut Context) {
        let mut uni = shader::Uniforms::new(
            self.view_proj(),
            Mat4::identity(),
        );
        let Self { renderer, .. } = self;

        ctx.begin_default_pass(Default::default());

        ctx.apply_pipeline(&renderer.pipeline);
        ctx.apply_bindings(&renderer.bindings);
        {
            ctx.apply_uniforms(&uni);
            let (start, num) = renderer.track_indices;
            ctx.draw(start, num, 1);
        }
        for &(art, model) in &self.render_queue.0 {
            uni.set_model(model);
            ctx.apply_uniforms(&uni);

            let (start, num) = renderer.art_indices.indices(art);
            ctx.draw(start, num, 1);
        }
        ctx.end_render_pass();

        ctx.commit_frame();
    }
}

mod shader {
    use miniquad::*;
    use glam::Mat4;

    pub const VERTEX: &str = r#"#version 100
    attribute vec3 pos;
    attribute vec3 norm;
    attribute vec2 uv;

    uniform mat4 view_proj;
    uniform mat4 model;
    uniform mat4 inv_trans_model;

    varying lowp vec2 texcoord;
    varying lowp vec3 normal;
    varying lowp vec3 frag_pos;

    void main() {
        gl_Position = view_proj * model * vec4(pos, 1);
        texcoord = uv;
        normal = mat3(inv_trans_model) * norm;
        frag_pos = vec3(model * vec4(pos, 1.0));
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec2 texcoord;
    varying lowp vec3 normal;
    varying lowp vec3 frag_pos;

    uniform sampler2D tex;

    void main() {
        lowp vec3 light_dir = normalize(vec3(500.0, 500.0, 500.0) - frag_pos);
        lowp vec3 light_color = vec3(1.0, 0.912, 0.802);
        lowp float light_strength = 1.4;

        lowp vec3 diffuse = max(dot(normalize(normal), light_dir), 0.0) * light_color;

        lowp vec3 ambient = light_color * 0.3;

        gl_FragColor = texture2D(tex, texcoord) * vec4((ambient + diffuse) * light_strength, 1.0);
    }"#;

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["tex".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: vec![
                    UniformDesc::new("view_proj", UniformType::Mat4),
                    UniformDesc::new("model", UniformType::Mat4),
                    UniformDesc::new("inv_trans_model", UniformType::Mat4),
                ],
            },
        }
    }

    #[repr(C)]
    pub struct Uniforms {
        pub view_proj: Mat4,
        pub model: Mat4,
        pub inv_trans_model: Mat4,
    }

    impl Uniforms {
        pub fn new(view_proj: Mat4, model: Mat4) -> Self {
            Uniforms {
                view_proj,
                inv_trans_model: model.inverse().transpose(),
                model,
            }
        }

        pub fn set_model(&mut self, model: Mat4) {
            self.model = model;
            self.inv_trans_model = model.inverse().transpose();
        }
    }
}
