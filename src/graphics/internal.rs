use super::Graphics;
use crate::color::Color;
use crate::utils::Rect;
use miniquad::*;

pub(super) struct Texture {
    pub id: TextureId,
    bindings: Bindings,
}

pub(super) struct RTexture {
    pub id: TextureId,
    bindings: Bindings,
    render_pass: RenderPass,
}

const VERTEX: &str = include_str!("default.vert");
const TEX_FRAGMENT: &str = include_str!("texture.frag");
const BLEND_FRAGMENT: &str = include_str!("blend.frag");
const UI_FRAGMENT: &str = include_str!("ui.frag");
const CHECKER_FRAGMENT: &str = include_str!("checker.frag");
const GRID_FRAGMENT: &str = include_str!("grid.frag");
const SELECTION_FRAGMENT: &str = include_str!("selection.frag");

pub(super) struct GraphicsInternal {
    ctx: Box<dyn RenderingBackend>,
    font_size: (f32, f32),
    tex_pipeline: Pipeline,
    blend_pipeline: Pipeline,
    copy_pipeline: Pipeline,
    ui_pipeline: Pipeline,
    checker_pipeline: Pipeline,
    grid_pipeline: Pipeline,
    selection_pipeline: Pipeline,
    vertex_buffer: BufferId,
    index_buffer: BufferId,
    ui_bindings: Bindings,
    checker_bindings: Bindings,
    srgb: bool,
}
#[rustfmt::skip]
fn tex_vertices(x: f32, y: f32, w: f32, h: f32, tw: f32, th: f32) -> [f32; 20] {
    [
        /* pos                                               uvs */
        -1.0 + x/w * 2.,      -1.0 + y/h * 2.,      0.0,     0., 0.,
        -1.0 + (x+tw)/w * 2., -1.0 + y/h * 2.,      0.0,     1., 0.,
        -1.0 + (x+tw)/w * 2., -1.0 + (y-th)/h * 2., 0.0,     1., 1.,
        -1.0 + x/w * 2.,      -1.0 + (y-th)/h * 2., 0.0,     0., 1.,
    ]
}
fn build_pipeline(ctx: &mut Box<dyn RenderingBackend>, shader: ShaderId) -> Pipeline {
    ctx.new_pipeline(
        &[BufferLayout {
            stride: VertexFormat::Float3.size_bytes() + VertexFormat::Float2.size_bytes(),
            ..Default::default()
        }],
        &[
            VertexAttribute::new("in_pos", VertexFormat::Float3),
            VertexAttribute::new("in_uv", VertexFormat::Float2),
        ],
        shader,
        PipelineParams {
            color_blend: Some(BlendState::new(
                Equation::Add,
                BlendFactor::Value(BlendValue::SourceAlpha),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            )),
            alpha_blend: Some(BlendState::new(
                Equation::Add,
                BlendFactor::One,
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            )),
            ..Default::default()
        },
    )
}
impl GraphicsInternal {
    pub(super) fn new() -> Self {
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();
        let mut decoder = png::Decoder::new(&include_bytes!("../../assets/font.png")[..]);
        decoder.set_transformations(png::Transformations::ALPHA);
        let mut reader = decoder.read_info().unwrap();
        let info = reader.info().clone();
        let mut buf = vec![0; reader.output_buffer_size()];
        reader.next_frame(&mut buf).unwrap();
        let font = ctx.new_texture_from_data_and_format(
            &buf,
            TextureParams {
                width: info.width as _,
                height: info.height as _,
                ..Default::default()
            },
        );
        let font_size = ((info.width / 16) as f32, (info.height / 16) as f32);
        ctx.texture_set_filter(font, FilterMode::Nearest, miniquad::MipmapFilterMode::None);
        let tex_shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: TEX_FRAGMENT,
                },
                ShaderMeta {
                    images: vec!["texture".into()],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![UniformDesc::new("color", UniformType::Float4)],
                    },
                },
            )
            .unwrap();
        let blend_shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: BLEND_FRAGMENT,
                },
                ShaderMeta {
                    images: vec!["render_target".into(), "texture".into()],
                    uniforms: UniformBlockLayout { uniforms: vec![] },
                },
            )
            .unwrap();
        let ui_shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: UI_FRAGMENT,
                },
                ShaderMeta {
                    images: vec!["texture".into()],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("fg", UniformType::Float4),
                            UniformDesc::new("bg", UniformType::Float4),
                        ],
                    },
                },
            )
            .unwrap();
        let checker_shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: CHECKER_FRAGMENT,
                },
                ShaderMeta {
                    images: vec![],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("color1", UniformType::Float4),
                            UniformDesc::new("color2", UniformType::Float4),
                            UniformDesc::new("size", UniformType::Float2),
                            UniformDesc::new("offset", UniformType::Float2),
                        ],
                    },
                },
            )
            .unwrap();
        let grid_shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: GRID_FRAGMENT,
                },
                ShaderMeta {
                    images: vec![],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("color", UniformType::Float4),
                            UniformDesc::new("size", UniformType::Float2),
                            UniformDesc::new("grid_size", UniformType::Float2),
                        ],
                    },
                },
            )
            .unwrap();
        let selection_shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: SELECTION_FRAGMENT,
                },
                ShaderMeta {
                    images: vec!["texture".into()],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("color", UniformType::Float4),
                            UniformDesc::new("size", UniformType::Float2),
                            UniformDesc::new("time", UniformType::Float1),
                        ],
                    },
                },
            )
            .unwrap();
        let tex_pipeline = build_pipeline(&mut ctx, tex_shader);
        let ui_pipeline = build_pipeline(&mut ctx, ui_shader);
        let checker_pipeline = build_pipeline(&mut ctx, checker_shader);
        let grid_pipeline = build_pipeline(&mut ctx, grid_shader);
        let selection_pipeline = build_pipeline(&mut ctx, selection_shader);
        let blend_pipeline = ctx.new_pipeline(
            &[BufferLayout {
                stride: VertexFormat::Float3.size_bytes() + VertexFormat::Float2.size_bytes(),
                ..Default::default()
            }],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float3),
                VertexAttribute::new("in_uv", VertexFormat::Float2),
            ],
            blend_shader,
            PipelineParams {
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::One,
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                alpha_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::One,
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
        );
        let copy_pipeline = ctx.new_pipeline(
            &[BufferLayout {
                stride: VertexFormat::Float3.size_bytes() + VertexFormat::Float2.size_bytes(),
                ..Default::default()
            }],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float3),
                VertexAttribute::new("in_uv", VertexFormat::Float2),
            ],
            tex_shader,
            PipelineParams::default(),
        );

        #[rustfmt::skip]
        let vertices: &[f32] = &[
            /* pos               uvs */
            -1.0, -1.0, 0.0,     0.0, 0.0,
             1.0, -1.0, 0.0,     1.0, 0.0,
             1.0,  1.0, 0.0,     1.0, 1.0,
            -1.0,  1.0, 0.0,     0.0, 1.0,
        ];

        let vertex_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(vertices),
        );

        #[rustfmt::skip]
        let indices: &[u16] = &[
            0, 1, 2,  0, 2, 3,
        ];

        let index_buffer = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(indices),
        );
        let ui_bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer,
            images: vec![font],
        };
        let checker_bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer,
            images: vec![],
        };
        let mut internal = GraphicsInternal {
            ctx,
            font_size,
            tex_pipeline,
            blend_pipeline,
            copy_pipeline,
            ui_pipeline,
            checker_pipeline,
            grid_pipeline,
            vertex_buffer,
            index_buffer,
            ui_bindings,
            checker_bindings,
            selection_pipeline,
            srgb: false,
        };
        // enable srgb by default
        internal.set_srgb(true);
        internal
    }
    pub(super) fn set_srgb(&mut self, value: bool) {
        if cfg!(not(target_arch = "wasm32")) {
            if value {
                unsafe {
                    miniquad::native::gl::glEnable(miniquad::native::gl::GL_FRAMEBUFFER_SRGB)
                };
            } else {
                unsafe {
                    miniquad::native::gl::glDisable(miniquad::native::gl::GL_FRAMEBUFFER_SRGB)
                };
            }
            self.srgb = value;
        }
    }
    pub(super) fn srgb(&self) -> bool {
        self.srgb
    }
    pub(super) fn font_size(&self) -> (f32, f32) {
        self.font_size
    }
    pub(super) fn new_tex(&mut self) -> Texture {
        let format = if self.srgb {
            TextureFormat::SRGBA8
        } else {
            TextureFormat::RGBA8
        };
        let id = self.ctx.new_texture_from_data_and_format(
            &[255, 255, 255, 255],
            TextureParams {
                width: 1,
                height: 1,
                format,
                ..Default::default()
            },
        );
        self.ctx
            .texture_set_filter(id, FilterMode::Nearest, miniquad::MipmapFilterMode::None);
        let bindings = Bindings {
            vertex_buffers: vec![self.vertex_buffer],
            index_buffer: self.index_buffer,
            images: vec![id],
        };
        Texture { id, bindings }
    }
    pub(super) fn new_rtex(&mut self) -> RTexture {
        let format = if self.srgb {
            TextureFormat::SRGBA8
        } else {
            TextureFormat::RGBA8
        };
        let id = self.ctx.new_render_texture(TextureParams {
            width: 1,
            height: 1,
            format,
            ..Default::default()
        });
        self.ctx
            .texture_set_filter(id, FilterMode::Nearest, miniquad::MipmapFilterMode::None);
        let bindings = Bindings {
            vertex_buffers: vec![self.vertex_buffer],
            index_buffer: self.index_buffer,
            images: vec![id],
        };
        let render_pass = self.ctx.new_render_pass(id, None);
        RTexture {
            id,
            bindings,
            render_pass,
        }
    }
    pub(super) fn delete_tex(&mut self, tex: Texture) {
        self.ctx.delete_texture(tex.id);
    }
    pub(super) fn delete_rtex(&mut self, rtex: RTexture) {
        self.ctx.delete_texture(rtex.id);
    }
    pub(super) fn update_tex(
        &mut self,
        id: TextureId,
        width: u32,
        height: u32,
        data: Option<&[u8]>,
    ) {
        self.ctx.texture_params(id);
        if self.ctx.texture_size(id) == (width, height) {
            if let Some(data) = data {
                self.ctx.texture_update(id, data);
            }
        } else {
            self.ctx.texture_resize(id, width, height, data);
        }
    }
    pub(super) fn draw_index(&mut self, index: u32, rect: Rect, fg: Color, bg: Color) {
        let (x, y, tw, th) = rect.get();
        let (w, h) = Graphics::screen_size();
        let y = h - y;
        let (i, j) = ((index % 16) as f32 / 16., (index / 16) as f32 / 16.);
        #[rustfmt::skip]
        let vertices: &[f32] = &[
            /* pos                                               uvs */
            -1.0 + x/w * 2.,      -1.0 + y/h * 2.,      0.0,     i,          j,
            -1.0 + (x+tw)/w * 2., -1.0 + y/h * 2.,      0.0,     i + 0.0625, j,
            -1.0 + (x+tw)/w * 2., -1.0 + (y-th)/h * 2., 0.0,     i + 0.0625, j + 0.0625,
            -1.0 + x/w * 2.,      -1.0 + (y-th)/h * 2., 0.0,     i,          j + 0.0625,
        ];
        self.ctx
            .buffer_update(self.vertex_buffer, BufferSource::slice(vertices));
        self.ctx.begin_pass(None, PassAction::Nothing);
        self.ctx.apply_pipeline(&self.ui_pipeline);
        self.ctx.apply_bindings(&self.ui_bindings);
        let mut d: Vec<f32> = Vec::new();
        d.extend(fg.to_linear(self.srgb));
        d.extend(bg.to_linear(self.srgb));
        let t: [f32; 8] = d[..].try_into().expect("");
        self.ctx.apply_uniforms(UniformsSource::table(&t));
        self.ctx.draw(0, 6, 1);
        self.ctx.end_render_pass();
    }
    pub(super) fn draw_tex_to_screen(&mut self, tex: &Texture, rect: Rect, color: Option<Color>) {
        self._draw_to_screen(&tex.bindings, rect, color)
    }
    pub(super) fn draw_rtex_to_screen(
        &mut self,
        rtex: &RTexture,
        rect: Rect,
        color: Option<Color>,
    ) {
        self._draw_to_screen(&rtex.bindings, rect, color)
    }
    fn _draw_to_screen(&mut self, bindings: &Bindings, rect: Rect, color: Option<Color>) {
        let (x, y, tw, th) = rect.get();
        let (w, h) = Graphics::screen_size();
        let y = h - y;
        let vertices = tex_vertices(x, y, w, h, tw, th);
        self.ctx
            .buffer_update(self.vertex_buffer, BufferSource::slice(&vertices));
        self.ctx.begin_pass(None, PassAction::Nothing);
        self.ctx.apply_pipeline(&self.tex_pipeline);
        self.ctx.apply_bindings(bindings);
        let color = color.unwrap_or(Color::WHITE);
        let d: [f32; 4] = color.to_linear(self.srgb);
        self.ctx.apply_uniforms(UniformsSource::table(&d));
        self.ctx.draw(0, 6, 1);
        self.ctx.end_render_pass();
    }
    pub(super) fn clear_rtex(&mut self, rtex: &RTexture, color: Color) {
        let d: [f32; 4] = color.to_linear(self.srgb);
        self.ctx.begin_pass(
            Some(rtex.render_pass),
            PassAction::clear_color(d[0], d[1], d[2], d[3]),
        );
        self.ctx.end_render_pass();
    }
    pub(super) fn draw_tex_on_rtex(&mut self, base: TextureId, tex: &Texture, rtex: &RTexture) {
        let vertices = tex_vertices(0., 0., 1., -1., 1., 1.);
        self.ctx
            .buffer_update(self.vertex_buffer, BufferSource::slice(&vertices));
        self.ctx
            .begin_pass(Some(rtex.render_pass), PassAction::Nothing);
        self.ctx.apply_pipeline(&self.blend_pipeline);
        let bindings = Bindings {
            vertex_buffers: vec![self.vertex_buffer],
            index_buffer: self.index_buffer,
            images: vec![base, tex.id],
        };
        self.ctx.apply_bindings(&bindings);
        self.ctx.draw(0, 6, 1);
        self.ctx.end_render_pass();
    }
    pub(super) fn copy_rtex_to_rtex(&mut self, src: &RTexture, rtex: &RTexture) {
        let vertices = tex_vertices(0., 0., 1., -1., 1., 1.);
        self.ctx
            .buffer_update(self.vertex_buffer, BufferSource::slice(&vertices));
        self.ctx
            .begin_pass(Some(rtex.render_pass), PassAction::Nothing);
        self.ctx.apply_pipeline(&self.copy_pipeline);
        self.ctx.apply_bindings(&src.bindings);
        self.ctx.apply_uniforms(UniformsSource::table(&[1.; 4]));
        self.ctx.draw(0, 6, 1);
        self.ctx.end_render_pass();
    }
    pub(super) fn read_tex(&mut self, id: TextureId, width: u32, height: u32) -> Vec<u8> {
        let mut buf = vec![0; width as usize * height as usize * 4];
        self.ctx.texture_read_pixels(id, &mut buf);
        buf
    }
    pub(super) fn draw_checker(
        &mut self,
        rect: Rect,
        offset: (f32, f32),
        color1: Color,
        color2: Color,
    ) {
        let (x, y, tw, th) = rect.get();
        let (w, h) = Graphics::screen_size();
        let y = h - y;
        let vertices = tex_vertices(x, y, w, h, tw, th);
        self.ctx
            .buffer_update(self.vertex_buffer, BufferSource::slice(&vertices));
        self.ctx.begin_pass(None, PassAction::Nothing);
        self.ctx.apply_pipeline(&self.checker_pipeline);
        self.ctx.apply_bindings(&self.checker_bindings);
        let mut d: Vec<f32> = Vec::new();
        d.extend(color1.to_linear(self.srgb));
        d.extend(color2.to_linear(self.srgb));
        d.extend([tw, th, offset.0, offset.1]);
        let t: [f32; 12] = d[..].try_into().expect("");
        self.ctx.apply_uniforms(UniformsSource::table(&t));
        self.ctx.draw(0, 6, 1);
        self.ctx.end_render_pass();
    }
    pub(super) fn draw_grid(&mut self, rect: Rect, color: Color, size: (f32, f32)) {
        let (x, y, tw, th) = rect.get();
        let (w, h) = Graphics::screen_size();
        let y = h - y;
        let vertices = tex_vertices(x, y, w, h, tw, th);
        self.ctx
            .buffer_update(self.vertex_buffer, BufferSource::slice(&vertices));
        self.ctx.begin_pass(None, PassAction::Nothing);
        self.ctx.apply_pipeline(&self.grid_pipeline);
        self.ctx.apply_bindings(&self.checker_bindings);
        let mut d: Vec<f32> = Vec::new();
        d.extend(color.to_linear(self.srgb));
        d.extend([tw, th, size.0, size.1]);
        let t: [f32; 8] = d[..].try_into().expect("");
        self.ctx.apply_uniforms(UniformsSource::table(&t));
        self.ctx.draw(0, 6, 1);
        self.ctx.end_render_pass();
    }
    pub(super) fn draw_selection(&mut self, tex: &Texture, rect: Rect, color: Color, time: f32) {
        let (x, y, tw, th) = rect.get();
        let (w, h) = Graphics::screen_size();
        let y = h - y;
        let vertices = tex_vertices(x, y, w, h, tw, th);
        self.ctx
            .buffer_update(self.vertex_buffer, BufferSource::slice(&vertices));
        self.ctx.begin_pass(None, PassAction::Nothing);
        self.ctx.apply_pipeline(&self.selection_pipeline);
        self.ctx.apply_bindings(&tex.bindings);
        let mut d: Vec<f32> = Vec::new();
        d.extend(color.to_linear(self.srgb));
        d.extend([tw, th, time]);
        let t: [f32; 7] = d[..].try_into().expect("");
        self.ctx.apply_uniforms(UniformsSource::table(&t));
        self.ctx.draw(0, 6, 1);
        self.ctx.end_render_pass();
    }
    pub(super) fn clear_screen(&mut self, color: Color) {
        let d: [f32; 4] = color.to_linear(self.srgb);
        self.ctx
            .begin_default_pass(PassAction::clear_color(d[0], d[1], d[2], d[3]));
        self.ctx.end_render_pass();
    }
    pub(super) fn flush(&mut self) {
        self.ctx.commit_frame();
    }
}
