mod internal;

use crate::color::Color;
use crate::engine::{BufferId, FrameId, LayerId};
use crate::image::Image;
use crate::utils::Rect;
use internal::GraphicsInternal;
use std::collections::HashMap;

use internal::{RTexture, Texture};

pub struct Graphics {
    composed: HashMap<BufferId, HashMap<FrameId, (RTexture, RTexture)>>,
    textures: HashMap<BufferId, HashMap<(LayerId, FrameId), Texture>>,
    selections: HashMap<BufferId, Texture>,
    blank: Texture,
    cursors: Texture,
    scale: f32,
    internal: GraphicsInternal,

    // for hardware render
    render: RTexture,
    render_copy: RTexture,
    source: Texture,
}
impl Graphics {
    fn screen_size() -> (f32, f32) {
        let (w, h) = miniquad::window::screen_size();
        let dpi_scale = miniquad::window::dpi_scale();
        (w / dpi_scale, h / dpi_scale)
    }
    pub fn fit_screen(&self, screen: (f32, f32)) -> (usize, usize) {
        (
            (screen.0 / self.scale / self.font_size().0) as usize,
            (screen.1 / self.scale / self.font_size().1) as usize,
        )
    }
    pub fn mouse_tile(&self, cursor: (f32, f32)) -> (i32, i32) {
        let dpi_scale = miniquad::window::dpi_scale();
        (
            (cursor.0 / dpi_scale / self.scale / self.font_size().0).floor() as i32,
            (cursor.1 / dpi_scale / self.scale / self.font_size().1).floor() as i32,
        )
    }
    pub fn scale(&self) -> f32 {
        self.scale
    }
    pub fn drop_buffer(&mut self, id: BufferId) {
        if let Some(composed) = self.composed.remove(&id) {
            for (_, (rtex, copy)) in composed {
                self.internal.delete_rtex(rtex);
                self.internal.delete_rtex(copy);
            }
        }
        if let Some(textures) = self.textures.remove(&id) {
            for (_, tex) in textures {
                self.internal.delete_tex(tex);
            }
        }
        if let Some(tex) = self.selections.remove(&id) {
            self.internal.delete_tex(tex);
        }
    }
    pub fn toggle_srgb(&mut self, value: Option<bool>) {
        if cfg!(not(target_arch = "wasm32")) {
            let value = value.unwrap_or(!self.srgb());
            if value != self.srgb() {
                // apparently it's not possible to change texture format on the GPU
                // so we delete all previously created textures and rebuild
                for (_, composed) in self.composed.drain() {
                    for (_, (rtex, copy)) in composed {
                        self.internal.delete_rtex(rtex);
                        self.internal.delete_rtex(copy);
                    }
                }
                for (_, textures) in self.textures.drain() {
                    for (_, tex) in textures {
                        self.internal.delete_tex(tex);
                    }
                }
                self.internal.set_srgb(value);
            }
        }
    }
    pub fn srgb(&self) -> bool {
        self.internal.srgb()
    }
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }
    pub fn font_size(&self) -> (f32, f32) {
        self.internal.font_size()
    }
}
impl Graphics {
    pub fn new(scale: f32) -> Self {
        let mut internal = GraphicsInternal::new();
        let cursors_img = crate::format::load_png(&include_bytes!("../../assets/cursor.png")[..])
            .expect("Unable to load cursors");
        let cursors = internal.new_tex();
        internal.update_tex(cursors.id, 32, 32, Some(cursors_img.raw_data()));
        Graphics {
            scale,
            composed: HashMap::new(),
            textures: HashMap::new(),
            selections: HashMap::new(),
            blank: internal.new_tex(),
            cursors,
            render: internal.new_rtex(),
            render_copy: internal.new_rtex(),
            source: internal.new_tex(),
            internal,
        }
    }
    pub fn draw_cursor(&mut self, rect: Rect<f32>) {
        let (mut x, mut y, w, h) = rect.get();
        x += 1.;
        y += 1.;
        let shadow_rect = Rect::new(x, y, w, h);
        self.internal
            .draw_tex_to_screen(&self.cursors, shadow_rect, Some((0, 0, 0, 128).into()));
        self.internal.draw_tex_to_screen(&self.cursors, rect, None);
    }
    pub fn texture_update(
        &mut self,
        id: BufferId,
        layer_id: LayerId,
        frame_id: FrameId,
        width: u32,
        height: u32,
        data: &[u8],
    ) {
        let textures = self.textures.entry(id).or_default();
        let tex = textures
            .entry((layer_id, frame_id))
            .or_insert_with(|| self.internal.new_tex());
        self.internal.update_tex(tex.id, width, height, Some(data));
    }
    pub fn composed_update(
        &mut self,
        id: BufferId,
        frame_id: FrameId,
        layers: Vec<LayerId>,
        width: u32,
        height: u32,
    ) -> Image {
        let frames = self.composed.entry(id).or_default();
        let (composed, copy) = frames
            .entry(frame_id)
            .or_insert_with(|| (self.internal.new_rtex(), self.internal.new_rtex()));
        self.internal.update_tex(composed.id, width, height, None);
        self.internal.update_tex(copy.id, width, height, None);
        self.internal.clear_rtex(composed, (0, 0, 0, 0).into());
        self.internal.clear_rtex(copy, (0, 0, 0, 0).into());
        for layer_id in layers {
            if let Some(tex) = self
                .textures
                .get(&id)
                .and_then(|t| t.get(&(layer_id, frame_id)))
            {
                self.internal.blend_tex_on_rtex(copy.id, tex, composed);
                self.internal.copy_rtex_to_rtex(composed, copy);
            }
        }
        let data = self.internal.read_tex(composed.id, width, height);
        Image::new_from_bytes(width as _, height as _, data)
    }
    pub fn selection_update(&mut self, id: BufferId, width: u32, height: u32, data: &[u8]) {
        let tex = self
            .selections
            .entry(id)
            .or_insert_with(|| self.internal.new_tex_1());
        self.internal.update_tex(tex.id, width, height, Some(data));
    }
    pub fn draw_rect_outline_fancy(
        &mut self,
        rect: Rect<f32>,
        b: f32,
        color1: Option<Color>,
        color2: Option<Color>,
    ) {
        let (x, y, w, h) = rect.get();
        self.draw_rect_outline(rect, 3. * b, color1);
        self.draw_rect_outline(Rect::new(x - b, y - b, w + 2. * b, h + 2. * b), b, color2);
    }
    pub fn draw_rect_filled(&mut self, rect: Rect<f32>, color: Option<Color>) {
        self.internal.draw_tex_to_screen(&self.blank, rect, color);
    }
    pub fn draw_checker(
        &mut self,
        rect: Rect<f32>,
        offset: (f32, f32),
        color1: Color,
        color2: Color,
    ) {
        self.internal.draw_checker(rect, offset, color1, color2);
    }
    pub fn draw_grid(&mut self, rect: Rect<f32>, color: Color, size: (f32, f32)) {
        self.internal.draw_grid(rect, color, size);
    }
    pub fn draw_rect_outline(&mut self, rect: Rect<f32>, b: f32, color: Option<Color>) {
        let (x, y, w, h) = rect.get();
        self.internal.draw_tex_to_screen(
            &self.blank,
            Rect::new(x - b, y - b, w + 2. * b, b),
            color,
        );
        self.internal.draw_tex_to_screen(
            &self.blank,
            Rect::new(x - b, y - b, b, h + 2. * b),
            color,
        );
        self.internal.draw_tex_to_screen(
            &self.blank,
            Rect::new(x - b, y + h, w + 2. * b, b),
            color,
        );
        self.internal.draw_tex_to_screen(
            &self.blank,
            Rect::new(x + w, y - b, b, h + 2. * b),
            color,
        );
    }
    pub fn draw_buffer_texture(
        &mut self,
        id: BufferId,
        frame: FrameId,
        rect: Rect<f32>,
        color: Option<Color>,
    ) {
        if let Some(composed) = self.composed.get(&id).and_then(|texs| texs.get(&frame)) {
            self.internal.draw_rtex_to_screen(&composed.0, rect, color);
        }
    }
    pub fn draw_selection(&mut self, id: BufferId, rect: Rect<f32>, color: Color, time: f32) {
        if let Some(texture) = self.selections.get(&id) {
            self.internal.draw_selection(texture, rect, color, time);
        }
    }
    pub fn draw_console(&mut self, console: &crate::console::Console, x: f32, y: f32) {
        let (w, h) = (
            self.scale * self.font_size().0,
            self.scale * self.font_size().1,
        );
        let mut batch = Vec::with_capacity(52 * 128);
        let mut c = 0;
        for j in 0..console.height() as i32 {
            for i in 0..console.width() as i32 {
                let tile = console.get_unchecked(i, j);
                if tile.0 != 0 || tile.2.3 != 0 {
                    let rect = Rect::new(w * (i as f32 + x), h * (j as f32 + y), w, h);
                    let index = self
                        .internal
                        .index_to_vertices(tile.0, rect, tile.1, tile.2);
                    batch.extend_from_slice(&index);
                    c += 1;
                    if c == 128 {
                        self.internal.draw_console_vertices(&batch[..], c);
                        batch.clear();
                        c = 0;
                    }
                }
            }
        }
        if c > 0 {
            self.internal.draw_console_vertices(&batch[..], c);
        }
    }
    pub fn clear_screen(&mut self, color: Color) {
        self.internal.clear_screen(color);
    }
    pub fn flush(&mut self) {
        self.internal.flush();
    }
    pub fn render(&mut self, image: &Image, source: &Image) -> Image {
        let (w, h) = (image.width() as _, image.height() as _);
        self.internal
            .update_tex(self.render.id, w, h, Some(image.raw_data()));
        self.internal.update_tex(self.render_copy.id, w, h, None);
        self.internal
            .copy_rtex_to_rtex(&self.render, &self.render_copy);
        self.internal
            .update_tex(self.source.id, w, h, Some(source.raw_data()));
        self.internal
            .blend_tex_on_rtex(self.render_copy.id, &self.source, &self.render);
        let data = self.internal.read_tex(self.render.id, w, h);
        Image::new_from_bytes(w as _, h as _, data)
    }
}
