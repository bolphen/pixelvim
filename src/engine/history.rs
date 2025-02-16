use nanoserde::{DeBin, SerBin};
use std::collections::HashMap;

use super::register::Register;
use crate::algo::Selection;
use crate::color::Color;
use crate::compression::{Compressed, Compressible};
use crate::image::{Image, Symmetry};

#[derive(SerBin, DeBin)]
pub struct History {
    images: Vec<Compressed<Image>>,
    selections: Vec<Compressed<Selection>>,
    registers: Vec<Compressed<Register>>,
    edits: Vec<Edit>,
    current_edit: usize,
    current_images: Vec<Vec<Image>>,
    current_selection: Selection,
    extra: usize,
    saved: ImgIndex,
    layer_counter: usize,
    frame_counter: usize,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, SerBin, DeBin)]
pub struct ImgIndex(usize);
impl ImgIndex {
    pub fn new(i: usize) -> Self {
        ImgIndex(i)
    }
}
#[derive(Clone, Copy, SerBin, DeBin)]
struct SelIndex(usize);
#[derive(Clone, Copy, SerBin, DeBin)]
pub struct RegIndex(usize);

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, SerBin, DeBin)]
pub struct LayerId(usize);
impl LayerId {
    pub fn new(i: usize) -> Self {
        LayerId(i)
    }
}
#[derive(PartialEq, Eq, Clone, Debug, SerBin, DeBin)]
pub struct Layer {
    pub id: LayerId,
    img_idx: Vec<ImgIndex>,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, SerBin, DeBin)]
pub struct FrameId(usize);

#[derive(SerBin, DeBin)]
pub struct Edit {
    name: String,
    img_idx: ImgIndex,
    sel_idx: SelIndex,
    reg_idx: RegIndex,
    layer_info: LayerInfo,
    frame_info: FrameInfo,
    paste_info: Option<PasteInfo>,
}

struct EditBuilder {
    name: String,
    img_idx: Option<ImgIndex>,
    sel_idx: Option<SelIndex>,
    reg_idx: Option<RegIndex>,
    layer_info: LayerInfo,
    frame_info: FrameInfo,
    paste_info: Option<PasteInfo>,
}
impl EditBuilder {
    fn new(name: String, layer_info: LayerInfo, frame_info: FrameInfo) -> Self {
        EditBuilder {
            name,
            img_idx: None,
            sel_idx: None,
            reg_idx: None,
            layer_info,
            frame_info,
            paste_info: None,
        }
    }
    fn img_idx(mut self, img_idx: ImgIndex) -> Self {
        self.img_idx = Some(img_idx);
        self
    }
    fn sel_idx(mut self, sel_idx: SelIndex) -> Self {
        self.sel_idx = Some(sel_idx);
        self
    }
    fn reg_idx(mut self, reg_idx: RegIndex) -> Self {
        self.reg_idx = Some(reg_idx);
        self
    }
    fn paste_info(mut self, paste_info: PasteInfo) -> Self {
        self.paste_info = Some(paste_info);
        self
    }
    fn build(self, h: &mut History) -> Edit {
        Edit {
            name: self.name,
            img_idx: self.img_idx.unwrap_or_else(|| h.insert_image(None)),
            sel_idx: self.sel_idx.unwrap_or_else(|| h.insert_selection(None)),
            reg_idx: self.reg_idx.unwrap_or_else(|| h.insert_register(None)),
            layer_info: self.layer_info,
            frame_info: self.frame_info,
            paste_info: self.paste_info,
        }
    }
}

#[derive(Clone, SerBin, DeBin)]
pub struct LayerInfo {
    layers: Vec<Layer>,
    current_layer: usize,
}

#[derive(Clone, SerBin, DeBin)]
struct FrameInfo {
    frames: Vec<FrameId>,
    delay: Vec<u32>,
    current_frame: usize,
}

#[derive(Clone, SerBin, DeBin)]
pub struct PasteInfo {
    img_idx: ImgIndex,
    reg_idx: RegIndex,
    offset: (i32, i32),
}
impl PasteInfo {
    pub fn new(img_idx: ImgIndex, reg_idx: RegIndex, offset: (i32, i32)) -> Self {
        PasteInfo {
            img_idx,
            reg_idx,
            offset,
        }
    }
}

pub enum PasteFrom<'a> {
    RegIndex(RegIndex),
    RegIndexAmend(RegIndex),
    Register(&'a Register),
}

pub struct PasteInfoBuilder<'a> {
    img_idx: ImgIndex,
    from: PasteFrom<'a>,
    offset: (i32, i32),
}

impl<'a> PasteInfoBuilder<'a> {
    pub fn new(img_idx: ImgIndex, from: PasteFrom<'a>, offset: (i32, i32)) -> Self {
        PasteInfoBuilder {
            img_idx,
            from,
            offset,
        }
    }
    fn build(self, h: &mut History) -> PasteInfo {
        let reg_idx = match self.from {
            PasteFrom::Register(r) => h.insert_register(Some(r)),
            PasteFrom::RegIndex(reg_idx) | PasteFrom::RegIndexAmend(reg_idx) => reg_idx,
        };
        PasteInfo {
            img_idx: self.img_idx,
            reg_idx,
            offset: self.offset,
        }
    }
}

impl Edit {
    pub fn init(num_layers: usize, delay: Vec<u32>) -> Self {
        let n = num_layers;
        let m = delay.len();
        Edit::new(
            "".into(),
            ImgIndex(n * m - 1),
            SelIndex(0),
            RegIndex(0),
            LayerInfo {
                current_layer: 0,
                layers: (0..n)
                    .map(|l| Layer {
                        img_idx: (0..m).map(|f| ImgIndex(l * m + f)).collect(),
                        id: LayerId(l),
                    })
                    .collect(),
            },
            FrameInfo {
                current_frame: 0,
                delay,
                frames: (0..m).map(FrameId).collect(),
            },
        )
    }
    fn new(
        name: String,
        img_idx: ImgIndex,
        sel_idx: SelIndex,
        reg_idx: RegIndex,
        layer_info: LayerInfo,
        frame_info: FrameInfo,
    ) -> Self {
        Edit {
            name,
            img_idx,
            sel_idx,
            reg_idx,
            layer_info,
            frame_info,
            paste_info: None,
        }
    }
}

pub struct PasteData {
    pub register: Register,
    pub reg_idx: RegIndex,
    pub image: Image,
    pub img_idx: ImgIndex,
    pub offset: (i32, i32),
}

pub struct UndoRedoResult {
    pub msg: String,
    pub steps: usize,
    pub layer: usize,
    pub frame: usize,
    pub paste_data: Option<PasteData>,
}

impl History {
    pub fn new(images: Vec<Compressed<Image>>, layers: Vec<Vec<Image>>, delay: Vec<u32>) -> Self {
        let current_selection = Selection::new();
        let n = layers.len();
        let m = layers[0].len();
        History {
            images,
            selections: vec![current_selection.compress()],
            registers: vec![],
            extra: 0,
            saved: ImgIndex::new(n * m - 1),
            current_edit: 0,
            current_images: layers,
            current_selection,
            edits: vec![Edit::init(n, delay)],
            layer_counter: n,
            frame_counter: m,
        }
    }
    #[inline]
    fn current_edit(&self) -> &Edit {
        &self.edits[self.current_edit]
    }
    #[inline]
    fn img_idx(&self) -> ImgIndex {
        self.current_edit().img_idx
    }
    #[inline]
    fn sel_idx(&self) -> SelIndex {
        self.current_edit().sel_idx
    }
    #[inline]
    fn reg_idx(&self) -> RegIndex {
        self.current_edit().reg_idx
    }
    #[inline]
    fn layers(&self) -> &Vec<Layer> {
        &self.current_edit().layer_info.layers
    }
    #[inline]
    fn frames(&self) -> &Vec<FrameId> {
        &self.current_edit().frame_info.frames
    }
    #[inline]
    fn delay(&self) -> &Vec<u32> {
        &self.current_edit().frame_info.delay
    }
    #[inline]
    fn layer_id(&self, layer: usize) -> LayerId {
        self.layers()[layer].id
    }
    #[inline]
    fn frame_id(&self, frame: usize) -> FrameId {
        self.frames()[frame]
    }
    #[inline]
    fn layer_img_idx(&self, layer: usize, frame: usize) -> ImgIndex {
        self.layers()[layer].img_idx[frame]
    }
    #[inline]
    fn layer_image(&self, layer: usize, frame: usize) -> &Image {
        &self.current_images[layer][frame]
    }
    #[inline]
    fn selection(&self) -> &Selection {
        &self.current_selection
    }
    fn insert_image(&mut self, image: Option<&Image>) -> ImgIndex {
        self.images.drain(self.img_idx().0 + 1 + self.extra..);
        image.map(|i| {
            self.images.push(i.compress());
            self.extra += 1;
        });
        ImgIndex(self.images.len() - 1)
    }
    fn insert_selection(&mut self, selection: Option<&Selection>) -> SelIndex {
        self.selections.drain(self.sel_idx().0 + 1..);
        if let Some(s) = selection {
            self.selections.push(s.compress())
        }
        SelIndex(self.selections.len() - 1)
    }
    fn insert_register(&mut self, register: Option<&Register>) -> RegIndex {
        // initial reg_idx is 0 while there is no register
        if self.reg_idx().0 < self.registers.len() {
            self.registers.drain(self.reg_idx().0 + 1..);
        }
        if let Some(r) = register {
            self.registers.push(r.compress())
        }
        RegIndex(self.registers.len().saturating_sub(1))
    }
    fn insert_edit(&mut self, edit: EditBuilder) {
        self.edits.drain(self.current_edit + 1..);
        let edit = edit.build(self);
        self.edits.push(edit);
        self.extra = 0;
        self.current_edit += 1;
    }
    fn _new_frame(
        &mut self,
        frame: usize,
        images: Option<Vec<Image>>,
        delay: u32,
        layer_info: &mut LayerInfo,
        frame_info: &mut FrameInfo,
    ) {
        let images: Vec<_> = images.unwrap_or_else(|| {
            (0..self.current_images.len())
                .map(|_| self.layer_image(0, 0).blank())
                .collect()
        });
        for (l, (layer, image)) in layer_info.layers.iter_mut().zip(images).enumerate() {
            let img_idx = self.insert_image(Some(&image));
            layer.img_idx.insert(frame, img_idx);
            self.current_images[l].insert(frame, image);
        }
        let id = FrameId(self.frame_counter);
        frame_info.frames.insert(frame, id);
        frame_info.delay.insert(frame, delay);
        self.frame_counter += 1;
        frame_info.current_frame = frame;
    }
    fn _delete_frame(
        &mut self,
        frame: usize,
        layer_info: &mut LayerInfo,
        frame_info: &mut FrameInfo,
    ) {
        for l in &mut layer_info.layers {
            l.img_idx.remove(frame);
        }
        frame_info.frames.remove(frame);
        frame_info.delay.remove(frame);
        frame_info.current_frame = frame.saturating_sub(1);
        for l in &mut self.current_images {
            l.remove(frame);
        }
    }
    fn _new_layer(
        &mut self,
        layer: usize,
        layer_info: &mut LayerInfo,
        images: Option<Vec<Image>>,
    ) -> LayerId {
        let images: Vec<_> = images.unwrap_or_else(|| {
            (0..self.current_images[0].len())
                .map(|_| self.layer_image(0, 0).blank())
                .collect()
        });
        let img_idx = images.iter().map(|i| self.insert_image(Some(i))).collect();
        self.current_images.insert(layer, images);
        let id = LayerId(self.layer_counter);
        self.layer_counter += 1;
        layer_info.layers.insert(layer, Layer { id, img_idx });
        id
    }
    fn _delete_layer(&mut self, layer: usize, layer_info: &mut LayerInfo) {
        layer_info.layers.remove(layer);
        layer_info.current_layer = layer.saturating_sub(1);
        self.current_images.remove(layer);
    }
    pub fn edit_selection(&mut self, name: String, selection: Selection) {
        self.edit(name, vec![], Some(selection), None, None, None);
    }
    pub fn edit(
        &mut self,
        name: String,
        images: Vec<(usize, usize, Image)>,
        selection: Option<Selection>,
        current_layer: Option<usize>,
        current_frame: Option<usize>,
        paste_data: Option<PasteInfoBuilder>,
    ) {
        let edit = &self.edits[self.current_edit];
        let mut layer_info = edit.layer_info.clone();
        let mut frame_info = edit.frame_info.clone();
        current_layer.map(|l| layer_info.current_layer = l);
        current_frame.map(|f| frame_info.current_frame = f);
        let mut img_idx = self.insert_image(None);
        for (layer, frame, image) in images {
            img_idx = self.insert_image(Some(&image));
            layer_info.layers[layer].img_idx[frame] = img_idx;
            self.current_images[layer][frame] = image;
        }
        let mut edit = EditBuilder::new(name, layer_info, frame_info).img_idx(img_idx);
        if let Some(selection) = selection {
            edit = edit.sel_idx(self.insert_selection(Some(&selection)));
            self.current_selection = selection;
        }
        if let Some(builder) = paste_data {
            let paste_info = builder.build(self);
            edit = edit.reg_idx(paste_info.reg_idx).paste_info(paste_info);
        }
        self.insert_edit(edit);
    }
    pub fn amend(
        &mut self,
        name: Option<String>,
        images: Vec<(usize, usize, Image)>,
        selection: Option<Selection>,
        layer_info: Option<LayerInfo>,
        paste_info: Option<PasteInfo>, // amend should only pass paste_info and not the actual data
    ) {
        // cannot amend the initial edit
        if self.current_edit != 0 {
            let prev_edit = &self.edits[self.current_edit - 1];
            let prev_sel_idx = prev_edit.sel_idx;
            let edit = &mut self.edits[self.current_edit];
            let (img_idx, sel_idx) = (edit.img_idx, edit.sel_idx);
            if let Some(name) = name {
                edit.name = name;
            }
            if !images.is_empty() {
                // TODO need to properly delete previous images without deleting the extras
                self.images.drain(img_idx.0 + 1 - images.len()..);
                for (layer, frame, image) in images {
                    self.images.push(image.compress());
                    self.current_images[layer][frame] = image;
                }
                edit.img_idx = ImgIndex(self.images.len() - 1);
            } else {
                self.images.drain(img_idx.0 + 1..);
            }
            if let Some(selection) = selection {
                self.selections.drain(prev_sel_idx.0 + 1..);
                self.selections.push(selection.compress());
                edit.sel_idx = SelIndex(self.selections.len() - 1);
                self.current_selection = selection;
            } else {
                self.selections.drain(sel_idx.0 + 1..);
            }
            if let Some(new_info) = layer_info {
                edit.layer_info = new_info;
            }
            if let (Some(old_info), Some(new_info)) = (&mut edit.paste_info, paste_info) {
                old_info.img_idx = new_info.img_idx;
                old_info.offset = new_info.offset;
            }
            self.edits.drain(self.current_edit + 1..);
        }
    }
    pub fn undo_redo(&mut self, requested_steps: i32) -> UndoRedoResult {
        let old = self.current_edit;
        let old_layers = self.edits[self.current_edit].layer_info.layers.clone();
        let steps = if requested_steps < 0 {
            ((-requested_steps) as usize).min(old)
        } else {
            (requested_steps as usize).min(self.edits.len() - 1 - old)
        };
        if requested_steps < 0 {
            self.current_edit -= steps;
        } else {
            self.current_edit += steps;
        }
        let edit = &mut self.edits[self.current_edit];
        let layer = edit.layer_info.current_layer;
        let frame = edit.frame_info.current_frame;
        if old_layers != edit.layer_info.layers {
            self.current_images.clear();
            for layer in &edit.layer_info.layers {
                let images = layer
                    .img_idx
                    .iter()
                    .map(|i| self.images[i.0].decompress().expect("data corrupted"))
                    .collect();
                self.current_images.push(images);
            }
        }
        self.current_selection = self.selections[self.sel_idx().0]
            .decompress()
            .expect("data corrupted");
        let msg = match (steps, requested_steps.signum()) {
            (0, -1) => "Already at oldest change".into(),
            (1, -1) => format!("Undo {}", self.edits[old].name),
            (n, -1) => format!("Undo {n} edits"),
            (0, 1) => "Already at newest change".into(),
            (1, 1) => format!("Redo {}", self.edits[self.current_edit].name),
            (n, 1) => format!("Redo {n} edits"),
            _ => unreachable!(),
        };
        let paste_data = self.edits[self.current_edit]
            .paste_info
            .as_ref()
            .map(|p| PasteData {
                register: self.registers[p.reg_idx.0]
                    .decompress()
                    .expect("data corrupted"),
                reg_idx: p.reg_idx,
                image: self.images[p.img_idx.0]
                    .decompress()
                    .expect("data corrupted"),
                img_idx: p.img_idx,
                offset: p.offset,
            });
        UndoRedoResult {
            msg,
            steps,
            paste_data,
            layer,
            frame,
        }
    }
}

#[derive(Clone, Copy, SerBin, DeBin)]
pub struct Grid {
    pub on: bool,
    pub size: (u32, u32),
    pub color: Color,
}

#[derive(SerBin, DeBin)]
pub struct Session {
    pub history: History,
    pub visibility: HashMap<LayerId, bool>,
    pub symmetry: Symmetry,
    pub grid: Grid,
    pub current_layer: usize,
    pub current_frame: usize,
}

impl Session {
    pub fn image(&self) -> &Image {
        self.history
            .layer_image(self.current_layer, self.current_frame)
    }
    pub fn mark_saved(&mut self) {
        self.history.saved = self.history.img_idx();
    }
    pub fn is_saved(&self) -> bool {
        self.history.img_idx().0 == self.history.saved.0
    }
    pub fn mark_unsaved(&mut self) {
        self.history.saved = ImgIndex(usize::MAX);
    }
    pub fn layers(&self) -> &Vec<Layer> {
        self.history.layers()
    }
    pub fn frames(&self) -> &Vec<FrameId> {
        self.history.frames()
    }
    pub fn delay(&self) -> &Vec<u32> {
        self.history.delay()
    }
    pub fn layer_id(&self, layer: usize) -> LayerId {
        self.history.layer_id(layer)
    }
    pub fn frame_id(&self, frame: usize) -> FrameId {
        self.history.frame_id(frame)
    }
    pub fn layer_img_idx(&self, layer: usize, frame: usize) -> ImgIndex {
        self.history.layer_img_idx(layer, frame)
    }
    pub fn reg_idx(&self) -> Option<RegIndex> {
        self.history
            .current_edit()
            .paste_info
            .as_ref()
            .map(|p| p.reg_idx)
    }
    pub fn layer_image(&self, layer: usize, frame: usize) -> &Image {
        self.history.layer_image(layer, frame)
    }
    pub fn selection(&self) -> &Selection {
        self.history.selection()
    }
    pub fn layer_by_id(&self, id: LayerId) -> Option<usize> {
        self.layers().iter().position(|l| l.id == id)
    }
    pub fn frame_by_id(&self, id: FrameId) -> Option<usize> {
        self.frames().iter().position(|f| *f == id)
    }
    pub fn image_from_index(&self, img_idx: ImgIndex) -> Image {
        self.history.images[img_idx.0]
            .decompress()
            .expect("data corrupted")
    }
    pub fn insert_image(&mut self, image: &Image) -> ImgIndex {
        self.history.insert_image(Some(image))
    }
    pub fn current_images(&self) -> &Vec<Vec<Image>> {
        &self.history.current_images
    }
    pub fn is_visible(&self, id: LayerId) -> bool {
        self.visibility.get(&id).is_some_and(|v| *v)
    }
    pub fn new_layer(&mut self, layer: usize, count: usize) {
        let mut layer_info = self.history.current_edit().layer_info.clone();
        let frame_info = self.history.current_edit().frame_info.clone();
        for i in 0..count {
            let id = self.history._new_layer(layer + i, &mut layer_info, None);
            self.visibility.insert(id, true);
        }
        let edit = EditBuilder::new(
            if count == 1 {
                "create layer".into()
            } else {
                format!("create {count} layers")
            },
            layer_info,
            frame_info,
        );
        self.history.insert_edit(edit);
        self.current_layer = layer + count - 1;
    }
    pub fn delete_layer(&mut self, layer: usize) {
        let mut layer_info = self.history.current_edit().layer_info.clone();
        let frame_info = self.history.current_edit().frame_info.clone();
        if self.layers().len() == 1 {
            let id = self.history._new_layer(1, &mut layer_info, None);
            self.visibility.insert(id, true);
        } else {
            // deleting does not insert new images
            // so we insert an empty image to make sure that `is_saved` recognizes the edit
            self.history.insert_image(Some(&Image::empty()));
        }
        self.history._delete_layer(layer, &mut layer_info);
        self.current_layer = layer_info.current_layer;
        let edit = EditBuilder::new("delete layer".into(), layer_info, frame_info);
        self.history.insert_edit(edit);
    }
    pub fn merge_layer_down(&mut self, layer: usize, count: usize, srgb: bool) {
        // do software merge
        let merged = (0..self.frames().len())
            .map(|f| {
                let mut new = self.layer_image(layer - count, f).clone();
                for j in 1..=count {
                    let l = layer - count + j;
                    if self.is_visible(self.layer_id(l)) {
                        let upper = self.layer_image(l, f);
                        for x in 0..new.width() as i32 {
                            for y in 0..new.height() as i32 {
                                let c = new.get_unchecked_mut(x, y);
                                *c = c.blend(*upper.get_unchecked(x, y), srgb);
                            }
                        }
                    }
                }
                new
            })
            .collect();
        let mut layer_info = self.history.current_edit().layer_info.clone();
        let frame_info = self.history.current_edit().frame_info.clone();
        let id = self
            .history
            ._new_layer(layer + 1, &mut layer_info, Some(merged));
        self.visibility.insert(id, true);
        for _ in 0..=count {
            self.history._delete_layer(layer - count, &mut layer_info);
        }
        let edit = EditBuilder::new("merge layer".into(), layer_info, frame_info);
        self.history.insert_edit(edit);
        self.current_layer = layer - count;
    }
    pub fn new_frame(&mut self, frame: usize, count: usize) {
        let mut layer_info = self.history.current_edit().layer_info.clone();
        let mut frame_info = self.history.current_edit().frame_info.clone();
        for _ in 0..count {
            self.history
                ._new_frame(frame, None, 100, &mut layer_info, &mut frame_info);
        }
        let edit = EditBuilder::new(
            if count == 1 {
                "insert frame".into()
            } else {
                format!("insert {count} frames")
            },
            layer_info,
            frame_info,
        );
        self.history.insert_edit(edit);
        self.current_frame = frame;
    }
    pub fn duplicate_frame(&mut self, frame: usize, count: usize) {
        let mut layer_info = self.history.current_edit().layer_info.clone();
        let mut frame_info = self.history.current_edit().frame_info.clone();
        for _ in 0..count {
            let images = (self.current_images())
                .iter()
                .map(|l| l[frame].clone())
                .collect();
            self.history._new_frame(
                frame + 1,
                Some(images),
                100,
                &mut layer_info,
                &mut frame_info,
            );
        }
        let edit = EditBuilder::new(
            if count == 1 {
                "duplicate frame".into()
            } else {
                format!("duplicate frame {count} times")
            },
            layer_info,
            frame_info,
        );
        self.history.insert_edit(edit);
        self.current_frame = frame + 1;
    }
    pub fn delete_frame(&mut self, frame: usize) {
        let mut layer_info = self.history.current_edit().layer_info.clone();
        let mut frame_info = self.history.current_edit().frame_info.clone();
        if self.frames().len() == 1 {
            self.history
                ._new_frame(1, None, 100, &mut layer_info, &mut frame_info);
        } else {
            // deleting does not insert new images
            // so we insert an empty image to make sure that `is_saved` recognizes the edit
            self.history.insert_image(Some(&Image::empty()));
        }
        self.history
            ._delete_frame(frame, &mut layer_info, &mut frame_info);
        self.current_frame = frame_info.current_frame;
        let edit = EditBuilder::new("delete frame".into(), layer_info, frame_info);
        self.history.insert_edit(edit);
    }
    pub fn slice(&mut self, sliced: Vec<Vec<Image>>) {
        let mut layer_info = self.history.current_edit().layer_info.clone();
        let mut frame_info = self.history.current_edit().frame_info.clone();
        self.history
            ._delete_frame(0, &mut layer_info, &mut frame_info);
        for frame in sliced.into_iter().rev() {
            self.history
                ._new_frame(0, Some(frame), 100, &mut layer_info, &mut frame_info);
        }
        self.current_frame = frame_info.current_frame;
        let edit = EditBuilder::new("slice".into(), layer_info, frame_info);
        self.history.insert_edit(edit);
    }
}
