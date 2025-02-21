use nanoserde::{DeBin, SerBin};

use crate::algo::Selection;
use crate::color::Color;
use crate::compression::{Compressed, Compressible};
use crate::error::EditError;
use crate::graphics::Graphics;
use crate::image::{Image, Symmetry};
use crate::parser::Size;
use crate::utils::Rect;

pub use super::history::{FrameId, ImgIndex, LayerId, PasteFrom, RegIndex};
use super::history::{Grid, History, PasteData, PasteInfo, PasteInfoBuilder, Session};

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// buffer settings that are not saved in swap files
#[derive(PartialEq, Eq, Hash, Clone, Copy, SerBin, DeBin)]
pub struct BufferId(usize);
impl BufferId {
    pub fn new(value: usize) -> Self {
        BufferId(value)
    }
}

#[derive(Default)]
pub struct Metadata {
    pub path: Option<PathBuf>,
    pub swap: Option<PathBuf>,
}

#[derive(Default)]
struct BufferCursor {
    cursor: Option<(i32, i32)>,
    mouse: (f32, f32),
}

#[derive(Clone, Copy)]
pub struct Animation {
    pub paused: bool,
    pub speed: f32,
    pub frame: usize,
    pub elapsed: f32,
}
impl Animation {
    pub fn new() -> Self {
        Animation {
            speed: 1.,
            paused: false,
            frame: 0,
            elapsed: 0.,
        }
    }
    pub fn update(&mut self, elapsed: u32, delay: &[u32]) {
        // can happen that the desired frame has been deleted
        self.frame = self.frame.min(delay.len() - 1);
        self.elapsed += elapsed as f32 * self.speed;
        while self.elapsed >= delay[self.frame] as f32 {
            self.elapsed -= delay[self.frame] as f32;
            self.frame = (self.frame + 1) % delay.len();
        }
    }
}

pub struct Buffer {
    id: BufferId,
    pub metadata: Metadata,
    pub temporary_images: HashMap<(LayerId, FrameId), Image>,
    temporary_selection: Option<Selection>,
    selection_dirty: bool,
    texture_clean: HashSet<(LayerId, FrameId)>,
    composed: HashMap<FrameId, Image>,
    pub viewport: Rect,
    cursor: BufferCursor,
    scale: f32,
    pub fit: bool,
    pub animation: Animation,
    pub edit_all: bool,
    pub session: Session,
}

impl Buffer {
    pub fn display_name(&self) -> &str {
        self.metadata
            .path
            .as_ref()
            .and_then(|path| {
                let p = path.to_str()?;
                if p.len() < 20 {
                    Some(p)
                } else {
                    path.file_name()?.to_str()
                }
            })
            .unwrap_or("[No Name]")
    }
    pub fn id(&self) -> BufferId {
        self.id
    }
    pub fn title(&self) -> String {
        if let Some(path) = &self.metadata.path {
            path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("[Error]")
                .into()
        } else {
            "[No Name]".into()
        }
    }
    pub fn scale(&self) -> f32 {
        self.scale
    }
    pub fn width(&self) -> usize {
        self.image().width()
    }
    pub fn height(&self) -> usize {
        self.image().height()
    }
    pub fn size(&self) -> Size {
        self.image().size()
    }
    pub fn composed(&self) -> &Image {
        &self.composed[&self.current_frame_id()]
    }
    pub fn gif_data(&self) -> (Vec<&Image>, Vec<u32>) {
        (
            self.frames().iter().map(|f| &self.composed[f]).collect(),
            self.session.delay().clone(),
        )
    }
    pub fn picker(&self, cursor: (i32, i32)) -> Option<Color> {
        self.composed().get(cursor.0, cursor.1).copied()
    }
    pub fn new(
        layers: Vec<Vec<Image>>,
        visibility: Option<Vec<bool>>,
        delay: Vec<u32>,
        id: BufferId,
    ) -> Self {
        let composed = HashMap::new();
        let viewport = Rect::new(
            0.,
            0.,
            layers[0][0].width() as f32 * 10.,
            layers[0][0].height() as f32 * 10.,
        );
        let n = layers.len();
        let m = layers[0].len();
        let visibility = visibility
            .map(|v| {
                v.iter()
                    .enumerate()
                    .map(|(i, v)| (LayerId::new(i), *v))
                    .collect()
            })
            .unwrap_or_else(|| (0..n).map(|i| (LayerId::new(i), true)).collect());
        let images = (0..m * n)
            .map(|index| {
                let (l, f) = (index / m, index % m);
                layers[l][f].compress()
            })
            .collect();
        let session = Session {
            history: History::new(images, layers, delay),
            symmetry: Symmetry::default(),
            grid: Grid {
                on: false,
                size: (8, 8),
                color: Color::RED,
            },
            current_layer: 0,
            current_frame: 0,
            visibility,
        };
        Buffer {
            id,

            metadata: Metadata::default(),
            viewport,
            scale: 10.,
            cursor: BufferCursor::default(),
            fit: true,

            session,
            temporary_images: HashMap::new(),
            temporary_selection: None,
            composed,

            texture_clean: HashSet::new(),
            selection_dirty: true,

            animation: Animation::new(),
            edit_all: false,
        }
    }
    pub fn image(&self) -> &Image {
        self.session.image()
    }
    pub fn display_selection(&self) -> &Selection {
        self.temporary_selection
            .as_ref()
            .unwrap_or(self.session.selection())
    }
    pub fn selection(&self) -> &Selection {
        self.session.selection()
    }
    /// check viewport correct, cursor in bound
    /// should be called when a potentially resizing change is made
    /// hence resizing edit, undo, redo
    fn sanitize(&mut self) {
        self.update_viewport();
        self.check_cursor();
        self.set_all_dirty();
    }
    pub fn coordinate(&mut self, mouse: (f32, f32)) -> (i32, i32) {
        (
            ((mouse.0 - self.viewport.x) / self.scale).floor() as i32,
            ((mouse.1 - self.viewport.y) / self.scale).floor() as i32,
        )
    }
    pub fn set_cursor(&mut self, mouse: (f32, f32)) -> Option<(i32, i32)> {
        let (x, y) = self.coordinate(mouse);
        self.cursor.mouse = mouse;
        if self.image().is_in_bound(x, y) {
            self.cursor.cursor = Some((x, y));
        } else {
            self.cursor.cursor = None;
        }
        self.cursor.cursor
    }
    pub fn zoom_at(&mut self, factor: f32, center: Option<(f32, f32)>) {
        let center = center.unwrap_or({
            (0., 0.)
            // if self.cursor.is_none() {
            //     self.center_at_cursor();
            // }
            // let cursor = self.cursor.expect("cursor not set");
            // self.viewport.relpos_to_screen(RelPos(
            //     cursor.0 as f32 / self.width() as f32,
            //     cursor.1 as f32 / self.height() as f32,
            // ))
        });
        let scale = (self.scale * factor).clamp(0.5, 128.);
        let factor = scale / self.scale;
        self.scale = scale;
        self.viewport.x = center.0 + (self.viewport.x - center.0) * factor;
        self.viewport.y = center.1 + (self.viewport.y - center.1) * factor;
        self.viewport.w = self.width() as f32 * scale;
        self.viewport.h = self.height() as f32 * scale;
        self.fit = false;
    }
    fn check_cursor(&mut self) {
        if let Some((x, y)) = self.cursor.cursor {
            if !self.image().is_in_bound(x, y) {
                self.cursor.cursor = None;
            }
        }
    }
    fn update_viewport(&mut self) {
        let (w, h) = (self.width(), self.height());
        self.viewport.w = w as f32 * self.scale;
        self.viewport.h = h as f32 * self.scale;
    }
    pub fn fit_to_view(&mut self, screen: (f32, f32)) {
        let (w, h) = (self.width(), self.height());
        self.scale = (screen.0 / w as f32).min(screen.1 / h as f32).min(32.);
        self.viewport.w = w as f32 * self.scale;
        self.viewport.h = h as f32 * self.scale;
        self.viewport.x = (screen.0 - self.viewport.w) / 2.;
        self.viewport.y = (screen.1 - self.viewport.h) / 2.;
        self.fit = true;
    }

    pub fn texture_update(&mut self, graphics: &mut Graphics) {
        let (w, h) = (self.width() as u32, self.height() as u32);
        if self.selection_dirty {
            let mut image = Image::new_with(w as _, h as _, Color(0, 0, 0, 0));
            for (x, y) in self.display_selection() {
                image.get_mut(*x, *y).map(|c| *c = Color::WHITE);
            }
            graphics.selection_update(self.id, w, h, image.raw_data());
            self.selection_dirty = false;
        }
        let mut changed_frames = HashSet::new();
        for (f, frame) in self.session.frames().iter().enumerate() {
            let mut clean = true;
            for (l, layer) in self.layers().iter().enumerate() {
                if !self.texture_clean.contains(&(*layer, *frame)) {
                    clean = false;
                    let mut image = self.session.layer_image(l, f);
                    if let Some(tmp) = self.temporary_images.get(&(*layer, *frame)) {
                        image = tmp;
                    }
                    graphics.texture_update(self.id, *layer, *frame, w, h, image.raw_data());
                    self.texture_clean.insert((*layer, *frame));
                }
            }
            if !clean {
                changed_frames.insert(*frame);
            }
        }
        for frame in changed_frames {
            let composed = graphics.composed_update(
                self.id,
                frame,
                self.layers()
                    .into_iter()
                    .filter(|l| self.session.is_visible(*l))
                    .collect(),
                w,
                h,
            );
            self.composed.insert(frame, composed);
        }
    }

    pub fn set_texture_dirty(&mut self, layer_id: LayerId, frame_id: FrameId) {
        self.texture_clean.remove(&(layer_id, frame_id));
    }
    pub fn set_all_dirty(&mut self) {
        self.selection_dirty = true;
        self.texture_clean.clear();
    }

    pub fn undo_redo(&mut self, steps: i32) -> (String, Option<PasteData>) {
        let old_layer = self.current_layer_id();
        let old_frame = self.current_frame_id();
        let result = self.session.history.undo_redo(steps);
        if result.steps > 0 {
            if result.paste_data.is_some() {
                self.session.current_layer = result.layer;
                self.session.current_frame = result.frame;
            } else {
                self.session.current_layer =
                    self.session.layer_by_id(old_layer).unwrap_or(result.layer);
                self.session.current_frame =
                    self.session.frame_by_id(old_frame).unwrap_or(result.frame);
            }
            if self.is_saved() {
                self.clean_swap();
            } else {
                self.write_swap();
            }
            self.sanitize();
        }
        (result.msg, result.paste_data)
    }
    pub fn cursor(&self) -> Option<(i32, i32)> {
        self.cursor.cursor
    }
    pub fn cursor_color(&self) -> Option<((i32, i32), Color)> {
        let (x, y) = self.cursor.cursor?;
        let color = self.composed().get(x, y)?;
        Some(((x, y), *color))
    }
    pub fn move_cursor(&mut self, dir: (i32, i32)) -> (i32, i32) {
        let (w, h) = (self.width(), self.height());
        if let Some((x, y)) = &mut self.cursor.cursor {
            *x = (*x + dir.0).clamp(0, w as i32 - 1);
            *y = (*y + dir.1).clamp(0, h as i32 - 1);
            (*x, *y)
        } else {
            let (x, y) = self.coordinate(self.cursor.mouse);
            let cursor = (x.clamp(0, w as i32 - 1), y.clamp(0, h as i32 - 1));
            self.cursor.cursor = Some(cursor);
            cursor
        }
    }
    pub fn is_saved(&self) -> bool {
        self.session.is_saved()
    }
    pub fn history_mark_saved(&mut self) {
        self.session.mark_saved();
    }
    pub fn clear_temporary(&mut self) {
        for (l, f) in self
            .temporary_images
            .drain()
            .map(|(lf, _)| lf)
            .collect::<Vec<_>>()
        {
            self.set_texture_dirty(l, f);
        }
        if self.temporary_selection.take().is_some() {
            self.selection_dirty = true;
        }
    }
    pub fn commit_temporary(&mut self, name: &str) -> Option<()> {
        let tmp = self.temporary_images.drain().collect::<Vec<_>>();
        for ((l, f), _) in &tmp {
            self.set_texture_dirty(*l, *f);
        }
        let tmp: Vec<_> = tmp
            .into_iter()
            .filter_map(|((l, f), im)| {
                Some((
                    self.session.layer_by_id(l)?,
                    self.session.frame_by_id(f)?,
                    im,
                ))
            })
            .collect();
        (!tmp.is_empty()).then(|| {
            let old_size = self.size();
            let size_changed = tmp[0].2.size() != old_size;
            self.session.history.edit(
                name.into(),
                tmp,
                None,
                Some(self.session.current_layer),
                Some(self.session.current_frame),
                None,
            );
            self.write_swap();
            if size_changed {
                self.sanitize();
            }
        })
    }
    pub fn edit_infallible<F>(&mut self, name: &str, draw_fn: F, amend: bool)
    where
        F: Clone + FnOnce(&Image, &Selection, Symmetry) -> Image,
    {
        let l = self.session.current_layer;
        let f = self.session.current_frame;
        let frames = if self.edit_all {
            (0..self.num_frames())
                .map(|f| {
                    (
                        (l, f),
                        (draw_fn.clone())(
                            self.session.layer_image(l, f),
                            self.selection(),
                            self.symmetry(),
                        ),
                    )
                })
                .collect()
        } else {
            [(
                (l, f),
                (draw_fn.clone())(self.image(), self.selection(), self.symmetry()),
            )]
            .into_iter()
            .collect()
        };
        self.batch_edit(Some(name), frames, None, amend)
            .expect("infallible");
    }
    pub fn edit<F>(&mut self, name: Option<&str>, draw_fn: F, amend: bool) -> Result<(), EditError>
    where
        F: Clone + FnOnce(&Image, &Selection, Symmetry) -> Option<Image>,
    {
        let l = self.session.current_layer;
        let f = self.session.current_frame;
        let frames = if self.edit_all {
            (0..self.num_frames())
                .filter_map(|f| {
                    (draw_fn.clone())(
                        self.session.layer_image(l, f),
                        self.selection(),
                        self.symmetry(),
                    )
                    .map(|im| ((l, f), im))
                })
                .collect()
        } else {
            (draw_fn.clone())(self.image(), self.selection(), self.symmetry())
                .map(|im| HashMap::from([((l, f), im)]))
                .unwrap_or_default()
        };
        self.batch_edit(name, frames, None, amend)
    }
    pub fn preview<F>(&mut self, draw_fn: F, accumulate: bool)
    where
        F: Clone + FnOnce(&Image, &Selection, Symmetry) -> Option<Image>,
    {
        let l = self.session.current_layer;
        let f = self.session.current_frame;
        let frames = if self.edit_all {
            (0..self.num_frames())
                .filter_map(|f| {
                    let image = self.session.layer_image(l, f);
                    let old_image = if accumulate {
                        self.temporary_images
                            .get(&(self.session.layer_id(l), self.session.frame_id(f)))
                            .unwrap_or(image)
                    } else {
                        image
                    };
                    (draw_fn.clone())(old_image, self.selection(), self.symmetry())
                        .map(|im| ((l, f), im))
                })
                .collect()
        } else {
            let old_image = if accumulate {
                self.temporary_images
                    .get(&(self.session.layer_id(l), self.session.frame_id(f)))
                    .unwrap_or(self.image())
            } else {
                self.image()
            };
            (draw_fn.clone())(old_image, self.selection(), self.symmetry())
                .map(|im| HashMap::from([((l, f), im)]))
                .unwrap_or_default()
        };
        self.batch_preview(frames, None);
    }
    pub fn edit_selection<F>(&mut self, name: &str, f: F)
    where
        F: FnOnce(&Image, &Selection, Symmetry) -> Selection,
    {
        self.clear_temporary();
        let selection = (f)(self.image(), self.selection(), self.symmetry());
        self.session.history.edit_selection(name.into(), selection);
        self.selection_dirty = true;
    }
    pub fn amend_selection<F>(&mut self, f: F)
    where
        F: FnOnce(&Image, &Selection, Symmetry) -> Selection,
    {
        let selection = (f)(self.image(), self.selection(), self.symmetry());
        self.session
            .history
            .amend(None, Vec::new(), selection.into(), None, None);
        self.selection_dirty = true;
    }
    pub fn preview_selection<F>(&mut self, f: F)
    where
        F: FnOnce(&Image, &Selection, Symmetry) -> Selection,
    {
        let selection = (f)(self.image(), self.selection(), self.symmetry());
        self.temporary_selection = Some(selection);
        self.selection_dirty = true;
    }
    pub fn preview_both<F>(&mut self, f: F)
    where
        F: FnOnce(&Image, &Selection, Symmetry) -> (Image, Selection),
    {
        let old_size = self.size();
        let (image, selection) = (f)(self.image(), self.selection(), self.symmetry());
        let size_changed = image.size() != old_size;
        self.temporary_images
            .insert((self.current_layer_id(), self.current_frame_id()), image);
        self.temporary_selection = Some(selection);
        if size_changed {
            self.sanitize();
        } else {
            self.selection_dirty = true;
            self.set_texture_dirty(self.current_layer_id(), self.current_frame_id());
        }
    }
    fn check_size_change(
        &self,
        frames: &HashMap<(usize, usize), Image>,
    ) -> Result<bool, EditError> {
        if let Some(im) = frames.values().next() {
            let n = self.num_layers();
            let m = self.num_frames();
            let new_size = im.size();
            let size_changed = new_size != self.size();
            if !size_changed {
                if frames.iter().any(|(_, im)| im.size() != new_size) {
                    Err(EditError::InvalidData)?
                }
            } else {
                // we must check that all layer and frames are present AND they have the right size
                for idx in 0..n * m {
                    let im = frames.get(&(idx / m, idx % m));
                    if im.is_none_or(|im| im.size() != new_size) {
                        Err(EditError::InvalidData)?
                    }
                }
            }
            Ok(size_changed)
        } else {
            Ok(false)
        }
    }
    pub fn batch_preview(
        &mut self,
        frames: HashMap<(usize, usize), Image>,
        selection: Option<Selection>,
    ) {
        if let Ok(size_changed) = self.check_size_change(&frames) {
            let changed_frames: HashSet<(LayerId, FrameId)> = frames
                .iter()
                .map(|((l, f), _)| (self.layers()[*l], self.frames()[*f]))
                .collect();
            let selection_changed = selection.is_some();
            for ((l, f), image) in frames {
                self.temporary_images
                    .insert((self.session.layer_id(l), self.session.frame_id(f)), image);
            }
            if size_changed {
                self.sanitize();
            } else {
                if selection_changed {
                    self.selection_dirty = true;
                }
                for (l, f) in changed_frames {
                    self.set_texture_dirty(l, f);
                }
            }
        }
    }
    pub fn batch_edit(
        &mut self,
        name: Option<&str>,
        frames: HashMap<(usize, usize), Image>,
        selection: Option<Selection>,
        amend: bool,
    ) -> Result<(), EditError> {
        self.clear_temporary();
        if !frames.is_empty() {
            let n = self.num_layers();
            let m = self.num_frames();
            let size_changed = self.check_size_change(&frames)?;
            let changed_frames: HashSet<(LayerId, FrameId)> = frames
                .iter()
                .map(|((l, f), _)| (self.layers()[*l], self.frames()[*f]))
                .collect();
            let selection_changed = selection.is_some();
            if amend {
                let images = frames
                    .into_iter()
                    .filter_map(|((l, f), im)| (l < n && f < m).then_some((l, f, im)))
                    .collect();
                self.session
                    .history
                    .amend(name.map(|s| s.into()), images, selection, None, None);
            } else {
                self.session.history.edit(
                    name.unwrap_or("edit").into(),
                    frames
                        .into_iter()
                        .filter_map(|((l, f), im)| (l < n && f < m).then_some((l, f, im)))
                        .collect(),
                    selection,
                    Some(self.session.current_layer),
                    Some(self.session.current_frame),
                    None,
                );
            }
            self.write_swap();
            if size_changed {
                self.sanitize();
            } else {
                if selection_changed {
                    self.selection_dirty = true;
                }
                for (l, f) in changed_frames {
                    self.set_texture_dirty(l, f);
                }
            }
            Ok(())
        } else {
            Err(EditError::Unchanged)
        }
    }
    pub fn paste(
        &mut self,
        name: &str,
        image: Image,
        selection: Selection,
        from: PasteFrom,
        img_idx: ImgIndex,
        offset: (i32, i32),
    ) -> RegIndex {
        self.clear_temporary();
        if let PasteFrom::RegIndexAmend(reg_idx) = from {
            let paste_info = PasteInfo::new(img_idx, reg_idx, offset);
            self.session.history.amend(
                None,
                vec![(
                    self.session.current_layer,
                    self.session.current_frame,
                    image,
                )],
                Some(selection),
                None,
                Some(paste_info),
            );
        } else {
            let paste_info = PasteInfoBuilder::new(img_idx, from, offset);
            self.session.history.edit(
                name.into(),
                vec![(
                    self.session.current_layer,
                    self.session.current_frame,
                    image,
                )],
                Some(selection),
                Some(self.session.current_layer),
                Some(self.session.current_frame),
                Some(paste_info),
            );
        }
        self.write_swap();
        self.selection_dirty = true;
        self.set_texture_dirty(self.current_layer_id(), self.current_frame_id());
        self.session.reg_idx().expect("")
    }
    pub fn img_idx(&self) -> ImgIndex {
        self.session
            .layer_img_idx(self.session.current_layer, self.session.current_frame)
    }
    pub fn new_layer(&mut self, layer: usize, count: usize) {
        if count > 0 {
            self.session.new_layer(layer, count);
            self.write_swap();
        }
    }
    pub fn new_frame(&mut self, frame: usize, count: usize) {
        if count > 0 {
            self.session.new_frame(frame, count);
            self.write_swap();
        }
    }
    pub fn duplicate_frame(&mut self, frame: usize, count: usize) {
        if count > 0 {
            self.session.duplicate_frame(frame, count);
            self.write_swap();
        }
    }
    pub fn delete_layer(&mut self, layer: usize) {
        self.session.delete_layer(layer);
        self.write_swap();
        self.set_all_dirty();
    }
    pub fn merge_layer_down(
        &mut self,
        layer: usize,
        count: usize,
        srgb: bool,
    ) -> Result<usize, EditError> {
        let actual_count = count.min(layer);
        if actual_count > 0 {
            self.session.merge_layer_down(layer, actual_count, srgb);
            self.write_swap();
            self.set_all_dirty();
            Ok(actual_count)
        } else {
            Err(EditError::Unchanged)
        }
    }
    pub fn delete_frame(&mut self, frame: usize) {
        self.session.delete_frame(frame);
        self.write_swap();
        self.set_all_dirty();
    }
    pub fn num_layers(&self) -> usize {
        self.session.layers().len()
    }
    pub fn num_frames(&self) -> usize {
        self.session.frames().len()
    }
    pub fn layers(&self) -> Vec<LayerId> {
        self.session.layers().iter().map(|l| l.id).collect()
    }
    pub fn frames(&self) -> &Vec<FrameId> {
        self.session.frames()
    }
    pub fn toggle_visibility(&mut self, layer: usize, value: Option<bool>) {
        let v = self
            .session
            .visibility
            .get_mut(&self.session.layer_id(layer))
            .expect("");
        *v = value.unwrap_or(!*v);
        self.set_all_dirty();
    }
    pub fn get_visibility(&self) -> Vec<bool> {
        self.session
            .layers()
            .iter()
            .map(|l| self.session.is_visible(l.id))
            .collect()
    }
    pub fn goto_layer_rel(&mut self, value: i32) {
        self.session.current_layer = (self.session.current_layer as i32 + value)
            .clamp(0, self.num_layers() as i32 - 1) as usize;
    }
    pub fn goto_frame_rel(&mut self, value: i32) {
        let frame = ((self.session.current_frame as i32 + value)
            .rem_euclid(self.num_frames() as i32)) as usize;
        self.set_frame(frame);
    }
    pub fn set_frame(&mut self, frame: usize) {
        if frame < self.num_frames() {
            self.session.current_frame = frame;
            self.animation.frame = frame;
            self.animation.elapsed = 0.;
        }
    }
    pub fn slice(&mut self, num_frames: usize) {
        assert!(self.num_frames() == 1);
        let current = self.session.current_images();
        let new_width = self.width() / num_frames;
        let height = self.height();
        let mut sliced = Vec::new();
        for f in 0..num_frames {
            let mut frame = Vec::new();
            for layer in current.iter() {
                let slice = layer[0]
                    .sub_image(f * new_width, 0, new_width, height)
                    .expect("slice failed");
                frame.push(slice);
            }
            sliced.push(frame);
        }
        self.session.slice(sliced);
        self.sanitize();
    }
    pub fn resize(&mut self, width: usize, height: usize) {
        let mut resized = Vec::new();
        let current = self.session.current_images();
        for (l, layer) in current.iter().enumerate() {
            for (f, frame) in layer.iter().enumerate() {
                resized.push((l, f, crate::tool::resize(frame, width, height)));
            }
        }
        self.session.history.edit(
            "resize".into(),
            resized,
            None,
            Some(self.session.current_layer),
            Some(self.session.current_frame),
            None,
        );
        self.sanitize();
    }
    pub fn crop(&mut self, width: usize, height: usize, offset: (i32, i32)) {
        self.clear_temporary();
        let (x, y) = offset;
        if x >= 0 && y >= 0 {
            let (x, y) = (x as usize, y as usize);
            if x + width <= self.width() && y + height <= self.height() {
                let mut cropped = Vec::new();
                let current = self.session.current_images();
                for (l, layer) in current.iter().enumerate() {
                    for (f, frame) in layer.iter().enumerate() {
                        cropped.push((l, f, frame.sub_image(x, y, width, height).expect("")));
                    }
                }
                let new_selection = crate::algo::r#move(self.selection(), (-offset.0, -offset.1));
                self.session.history.edit(
                    "crop".into(),
                    cropped,
                    Some(new_selection),
                    Some(self.session.current_layer),
                    Some(self.session.current_frame),
                    None,
                );
                self.sanitize();
            }
        }
    }
    pub fn write_swap(&mut self) {
        if let Some(swap) = &self.metadata.swap {
            let _ = std::fs::write(swap, self.session.compress().data());
        } else if let Some(path) = &self.metadata.path {
            if let Some(swap) =
                crate::utils::get_swap_path(path).and_then(|s| (!s.exists()).then_some(s))
            {
                self.metadata.swap = Some(swap.clone());
                let _ = std::fs::write(&swap, self.session.compress().data());
            }
        }
    }
    pub fn clean_swap(&mut self) {
        if let Some(swap) = &self.metadata.swap {
            if swap.exists() {
                let _ = std::fs::remove_file(swap);
            }
            self.metadata.swap = None;
        }
    }
    pub fn load_from_swap(swap: Vec<u8>, id: BufferId) -> Result<Self, String> {
        let session: Session = Compressed::from_data(swap).decompress()?;
        let image = session.image();
        let composed = HashMap::new();
        let viewport = Rect::new(
            0.,
            0.,
            image.width() as f32 * 10.,
            image.height() as f32 * 10.,
        );
        Ok(Buffer {
            id,

            metadata: Metadata::default(),
            viewport,
            fit: true,
            scale: 10.,
            cursor: BufferCursor::default(),

            session,
            temporary_images: HashMap::new(),
            temporary_selection: None,
            composed,
            texture_clean: HashSet::new(),
            selection_dirty: true,

            animation: Animation::new(),
            edit_all: false,
        })
    }
    pub fn do_animation(&mut self, elapsed: u32) -> bool {
        self.animation.update(elapsed, self.session.delay());
        if self.session.current_frame != self.animation.frame {
            self.session.current_frame = self.animation.frame;
            true
        } else {
            false
        }
    }
    pub fn symmetry(&self) -> Symmetry {
        self.session.symmetry
    }
    pub fn grid(&self) -> Grid {
        self.session.grid
    }
    pub fn current_frame_id(&self) -> FrameId {
        self.session.frame_id(self.session.current_frame)
    }
    pub fn current_layer_id(&self) -> LayerId {
        self.session.layer_id(self.session.current_layer)
    }
}
