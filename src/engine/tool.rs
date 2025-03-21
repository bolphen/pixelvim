use super::buffer::{Buffer, FrameId, ImgIndex, PasteFrom, RegIndex};
use super::register::Register;
use super::{Message, Modifier, NormalMode, VisualMode};
use crate::algo::Brush;
use crate::color::{Color, ColorMode};
use crate::graphics::Graphics;
use crate::image::Image;
use crate::selection::Selection;
use std::collections::HashMap;

// type Trace = Vec<(i32, i32)>;
type Bounds = ((i32, i32), (i32, i32));
type Once = (i32, i32);
type Dir = (i32, i32);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    Keyboard,
    Mouse,
}

enum Staged {
    All(Selection),
    PerFrame(FrameId, HashMap<FrameId, Selection>),
}

impl Staged {
    fn new(frame: Option<FrameId>) -> Self {
        if let Some(id) = frame {
            Staged::PerFrame(id, HashMap::new())
        } else {
            Staged::All(Selection::new())
        }
    }
    fn frame_tracked(&self) -> Option<FrameId> {
        match self {
            Staged::All(_) => None,
            Staged::PerFrame(id, _) => Some(*id),
        }
    }
    fn stage(&mut self, new: &Selection) {
        match self {
            Staged::All(s) => s.extend(new),
            Staged::PerFrame(id, h) => h.entry(*id).or_insert_with(Selection::new).extend(new),
        }
    }
    fn get(&self) -> &Selection {
        match self {
            Staged::All(s) => s,
            Staged::PerFrame(id, h) => h.get(id).unwrap(),
        }
    }
    fn clear(&mut self) {
        match self {
            Staged::All(s) => s.clear(),
            Staged::PerFrame(_, h) => h.clear(),
        }
    }
}

struct TraceRecord {
    trace: Vec<(i32, i32)>,
    input: InputType,
    used: usize,
    staged: Staged,
}
impl TraceRecord {
    fn trim(&mut self) {
        self.trace.drain(0..self.used);
        self.used = 0;
    }
    fn set_frame(&mut self, frame: FrameId) {
        if let Staged::PerFrame(id, _) = &mut self.staged {
            *id = frame;
        }
    }
    fn push(&mut self, pos: (i32, i32)) {
        if self.trace.last().is_none_or(|p| *p != pos) {
            self.trace.push(pos);
        }
    }
    fn use_trace(&mut self, keep: usize) -> &[(i32, i32)] {
        let traces = &self.trace[self.used..];
        self.used = self.trace.len().saturating_sub(keep + 1);
        traces
    }
    fn stage(&mut self, new: &Selection) {
        self.staged.stage(new)
    }
    fn staged(&self) -> &Selection {
        self.staged.get()
    }
}

#[derive(Default)]
pub struct Tracker {
    trace: Option<TraceRecord>,
    pub pixel_perfect: bool,
}
impl Tracker {
    pub fn is_any_in_use(&self) -> bool {
        self.trace.is_some()
    }
    pub fn is_mouse_in_use(&self) -> bool {
        self.trace
            .as_ref()
            .is_some_and(|t| matches!(t.input, InputType::Mouse))
    }
    pub fn is_keyboard_in_use(&self) -> bool {
        self.trace
            .as_ref()
            .is_some_and(|t| matches!(t.input, InputType::Keyboard))
    }
    pub fn start(&mut self, pos: (i32, i32), input: InputType, frame: Option<FrameId>) {
        self.trace = Some(TraceRecord {
            trace: vec![pos],
            input,
            used: 0,
            staged: Staged::new(frame),
        });
    }
    pub fn frame_tracked(&self) -> Option<FrameId> {
        self.trace.as_ref()?.staged.frame_tracked()
    }
    pub fn trim(&mut self) {
        self.trace.as_mut().map(|trace| trace.trim());
    }
    pub fn set_frame(&mut self, frame: FrameId) {
        self.trace.as_mut().map(|trace| trace.set_frame(frame));
    }
    pub fn track_mouse(&mut self, pos: (i32, i32)) {
        self.trace.as_mut().map(|trace| {
            if trace.input == InputType::Mouse {
                trace.push(pos);
            }
        });
    }
    pub fn track_keyboard(&mut self, pos: (i32, i32)) {
        self.trace.as_mut().map(|trace| {
            if trace.input == InputType::Keyboard {
                trace.push(pos);
            }
        });
    }
    pub fn stop(&mut self) {
        self.trace = None;
    }
    pub fn reset_used(&mut self) {
        self.trace.as_mut().map(|trace| {
            trace.used = 0;
            trace.staged.clear();
        });
    }
    // fn get_trace(&mut self) -> Option<Trace> {
    //     let trace = &self.trace.as_ref()?.trace;
    //     Some(if self.pixel_perfect {
    //         crate::algo::pixel_perfect_filter(trace)
    //     } else {
    //         trace.to_vec()
    //     })
    // }
    fn get_bounds(&self) -> Option<Bounds> {
        let trace = &self.trace.as_ref()?.trace;
        Some((trace[0], trace[trace.len() - 1]))
    }
    fn get_once(&self) -> Option<Once> {
        let trace = &self.trace.as_ref()?.trace;
        Some(trace[trace.len() - 1])
    }
    pub fn get_dir(&self) -> Option<Dir> {
        let trace = &self.trace.as_ref()?.trace;
        let (start, end) = (trace[0], trace[trace.len() - 1]);
        Some((end.0 - start.0, end.1 - start.1))
    }
}

#[derive(PartialEq, Eq)]
pub enum Tool {
    Brush(bool),
    Rect(bool),
    Ellipse(bool),
    Flood(bool),
    Line,
    Move,
    Rotate,
}

pub struct ToolSetting {
    pub brush: Brush,
    pub flood_tolerance: u8,
}
impl ToolSetting {
    pub fn new() -> Self {
        ToolSetting {
            brush: Brush::Rect(1),
            flood_tolerance: 10,
        }
    }
}

impl Tool {
    fn toggle(&mut self) {
        match self {
            Tool::Brush(outline) | Tool::Rect(outline) | Tool::Ellipse(outline) => {
                *outline = !*outline
            }
            Tool::Flood(discon) => *discon = !*discon,
            Tool::Line | Tool::Move | Tool::Rotate => (),
        }
    }
    pub fn set_or_toggle(&mut self, tool: Tool) {
        match (&self, &tool) {
            (Tool::Brush(..), Tool::Brush(..))
            | (Tool::Rect(..), Tool::Rect(..))
            | (Tool::Ellipse(..), Tool::Ellipse(..))
            | (Tool::Flood(..), Tool::Flood(..)) => {
                self.toggle();
            }
            _ => *self = tool,
        }
    }
    pub fn set(&mut self, tool: Tool) -> bool {
        if *self != tool {
            *self = tool;
            true
        } else {
            false
        }
    }
    pub fn string_short(&self) -> &str {
        match self {
            Tool::Brush(outline) => {
                if *outline {
                    "brush"
                } else {
                    "fill. brush"
                }
            }
            Tool::Rect(outline) => {
                if *outline {
                    "outl. rect."
                } else {
                    "fill. rect."
                }
            }
            Tool::Ellipse(outline) => {
                if *outline {
                    "outl. ellipse"
                } else {
                    "fill. ellipse"
                }
            }
            Tool::Flood(discon) => {
                if *discon {
                    "disc. flood"
                } else {
                    "cont. flood"
                }
            }
            Tool::Line => "line",
            Tool::Move => "move",
            Tool::Rotate => "rotate",
        }
    }
    pub fn string(&self, setting: &ToolSetting) -> String {
        match self {
            Tool::Brush(outline) => {
                if *outline {
                    format!("brush {}", setting.brush)
                } else {
                    format!("fill. brush {}", setting.brush)
                }
            }
            Tool::Rect(outline) => {
                if *outline {
                    "outl. rect.".into()
                } else {
                    "fill. rect.".into()
                }
            }
            Tool::Ellipse(outline) => {
                if *outline {
                    "outl. ellipse".into()
                } else {
                    "fill. ellipse".into()
                }
            }
            Tool::Flood(discon) => {
                if *discon {
                    format!("disc. flood [{}]", setting.flood_tolerance)
                } else {
                    format!("cont. flood [{}]", setting.flood_tolerance)
                }
            }
            Tool::Line => format!("line {}", setting.brush),
            Tool::Move => "move".into(),
            Tool::Rotate => "rotate".into(),
        }
    }
}
macro_rules! draw_diff {
    ($diff:expr, $image:expr, $graphics:expr, $color_mode:expr $(,)*) => {
        (!$diff.is_empty()).then(|| {
            if let (Some(graphics), ColorMode::Blend(color, _)) = ($graphics, $color_mode) {
                let source = $diff
                    .crop($image.size())
                    .apply(|b| if *b { color } else { (0, 0, 0, 0).into() });
                graphics.render($image, &source)
            } else {
                let mut new = $image.clone();
                for (x, y) in $diff {
                    $color_mode.apply(new.get_unchecked_mut(x, y));
                }
                new
            }
        })
    };
}
macro_rules! draw {
    (
        $diff:expr,
        $image:expr,
        $selection:expr,
        $graphics:expr,
        $color_mode:expr $(,)*
    ) => {{
        let mut diff = $diff;
        diff.retain(|(x, y)| {
            $image.is_in_bound(x, y) && ($selection.is_empty() || $selection.contains((x, y)))
        });
        draw_diff!(diff, $image, $graphics, $color_mode)
    }};
}
impl Tool {
    pub fn normal_preview(
        &self,
        tracker: &mut Tracker,
        buffer: &mut Buffer,
        mode: NormalMode,
        color: Color,
        modifier: Option<Modifier>,
        tool_setting: &ToolSetting,
        mut graphics: Option<&mut Graphics>,
    ) {
        let color_mode = match mode {
            NormalMode::Blend(srgb) => ColorMode::Blend(color, srgb),
            NormalMode::Replace => ColorMode::Set(color),
            NormalMode::Erase => ColorMode::Clear,
        };
        let size = buffer.size();
        let brush = &tool_setting.brush;
        match self {
            Tool::Brush(outline) => {
                if let Some(record) = &mut tracker.trace {
                    use crate::algo::brush as brush_fn;
                    buffer.preview(|image, selection, sym| {
                        if *outline {
                            if !tracker.pixel_perfect {
                                let trace = record.use_trace(0);
                                let new = brush_fn(size, trace, sym, brush, true);
                                record.stage(&new);
                                let diff = record.staged().clone();
                                draw!(diff, image, selection, graphics.as_mut(), color_mode)
                            } else {
                                let trace = record.use_trace(1);
                                let trace_f = crate::algo::pixel_perfect_filter(trace);
                                let len = trace_f.len();
                                if len > 1 && trace_f[len - 2] != trace[trace.len() - 2] {
                                    record.used -= 1;
                                }
                                let d1 = brush_fn(size, &trace_f[..len - 1], sym, brush, true);
                                record.stage(&d1);
                                let mut diff = record.staged().clone();
                                if len > 1 {
                                    let d2 = brush_fn(size, &trace_f[len - 2..], sym, brush, true);
                                    diff.extend(d2);
                                }
                                draw!(diff, image, selection, graphics.as_mut(), color_mode)
                            }
                        } else {
                            let bind;
                            let trace = if tracker.pixel_perfect {
                                bind = crate::algo::pixel_perfect_filter(&record.trace);
                                &bind
                            } else {
                                &record.trace
                            };
                            let diff = brush_fn(size, trace, sym, brush, false);
                            draw!(diff, image, selection, graphics.as_mut(), color_mode)
                        }
                    });
                }
            }
            Tool::Rect(outline) => {
                if let Some(bounds) = tracker.get_bounds() {
                    buffer.preview(|image, selection, sym| {
                        draw!(
                            crate::algo::rect(size, bounds, sym, *outline),
                            image,
                            selection,
                            graphics.as_mut(),
                            color_mode,
                        )
                    });
                }
            }
            Tool::Ellipse(outline) => {
                if let Some(bounds) = tracker.get_bounds() {
                    buffer.preview(|image, selection, sym| {
                        draw!(
                            crate::algo::ellipse(size, bounds, sym, *outline),
                            image,
                            selection,
                            graphics.as_mut(),
                            color_mode,
                        )
                    });
                }
            }
            Tool::Line => {
                if let Some(bounds) = tracker.get_bounds() {
                    buffer.preview(|image, selection, sym| {
                        draw!(
                            crate::algo::line(size, bounds, sym, brush),
                            image,
                            selection,
                            graphics.as_mut(),
                            color_mode,
                        )
                    });
                }
            }
            Tool::Flood(discon) => {
                tracker.get_once().map(|once| {
                    let tolerance = modifier
                        .and_then(Modifier::to_i32)
                        .unwrap_or(tool_setting.flood_tolerance as _)
                        .min(255) as u8;
                    buffer.preview(|image, selection, sym| {
                        let diff = crate::algo::flood(
                            image,
                            (!selection.is_empty()).then_some(selection),
                            once,
                            tolerance,
                            sym,
                            *discon,
                        );
                        draw_diff!(diff, image, graphics.as_mut(), color_mode)
                    });
                });
            }
            Tool::Move => {
                if let Some(dir) = tracker.get_dir() {
                    buffer.preview(|i, _s, _sym| Some(crate::tool::r#move(i, dir)));
                }
            }
            Tool::Rotate => {
                if let Some(bounds) = tracker.get_bounds() {
                    buffer.preview(|i, _s, _sym| Some(crate::tool::rotate_bounds(i, bounds)));
                }
            }
        }
    }
    pub fn visual_preview(
        &self,
        tracker: &mut Tracker,
        buffer: &mut Buffer,
        mode: VisualMode,
        modifier: Option<Modifier>,
        tool_setting: &ToolSetting,
    ) {
        let size = buffer.size();
        let brush = &tool_setting.brush;
        match self {
            Tool::Brush(outline) => {
                if let Some(record) = &mut tracker.trace {
                    use crate::algo::brush as brush_fn;
                    buffer.preview_selection(|_i, s, sym| {
                        let diff = if *outline {
                            if !tracker.pixel_perfect {
                                let trace = record.use_trace(0);
                                let new = brush_fn(size, trace, sym, brush, true);
                                record.stage(&new);
                                record.staged().clone()
                            } else {
                                let trace = record.use_trace(1);
                                let trace_f = crate::algo::pixel_perfect_filter(trace);
                                let len = trace_f.len();
                                if len > 1 && trace_f[len - 2] != trace[trace.len() - 2] {
                                    record.used -= 1;
                                }
                                let d1 = brush_fn(size, &trace_f[..len - 1], sym, brush, true);
                                record.stage(&d1);
                                let mut diff = record.staged().clone();
                                if len > 1 {
                                    let d2 = brush_fn(size, &trace_f[len - 2..], sym, brush, true);
                                    diff.extend(d2);
                                }
                                diff
                            }
                        } else {
                            let bind;
                            let trace = if tracker.pixel_perfect {
                                bind = crate::algo::pixel_perfect_filter(&record.trace);
                                &bind
                            } else {
                                &record.trace
                            };
                            brush_fn(size, trace, sym, brush, false)
                        };
                        mode.apply(s.clone(), &diff)
                    });
                }
            }
            Tool::Rect(outline) => {
                if let Some(bounds) = tracker.get_bounds() {
                    buffer.preview_selection(|_i, s, sym| {
                        mode.apply(s.clone(), &crate::algo::rect(size, bounds, sym, *outline))
                    });
                }
            }
            Tool::Ellipse(outline) => {
                if let Some(bounds) = tracker.get_bounds() {
                    buffer.preview_selection(|_i, s, sym| {
                        mode.apply(
                            s.clone(),
                            &crate::algo::ellipse(size, bounds, sym, *outline),
                        )
                    });
                }
            }
            Tool::Line => {
                if let Some(bounds) = tracker.get_bounds() {
                    buffer.preview_selection(|_i, s, sym| {
                        mode.apply(s.clone(), &crate::algo::line(size, bounds, sym, brush))
                    });
                }
            }
            Tool::Flood(discon) => {
                tracker.get_once().map(|once| {
                    let tolerance = modifier
                        .and_then(Modifier::to_i32)
                        .unwrap_or(tool_setting.flood_tolerance as _)
                        .min(255) as u8;
                    buffer.preview_selection(|i, s, sym| {
                        mode.apply(
                            s.clone(),
                            &crate::algo::flood(i, None, once, tolerance, sym, *discon),
                        )
                    });
                });
            }
            Tool::Move => {
                (!buffer.selection().is_empty()).then(|| {
                    tracker.get_dir().map(|dir| {
                        buffer.preview_selection(|_i, s, _sym| crate::algo::r#move(s, dir));
                    })
                });
            }
            Tool::Rotate => {
                // TODO
            }
        }
    }
    pub fn visual_move(
        &mut self,
        buffer: &mut Buffer,
        dir: (i32, i32),
        amend: bool,
    ) -> Option<Message> {
        match self {
            Tool::Move => {
                if buffer.selection().is_empty() {
                    Some(Message::error("Empty selection"))
                } else {
                    (dir != (0, 0)).then(|| {
                        if amend {
                            buffer.amend_selection(|_i, s, _sym| crate::algo::r#move(s, dir));
                        } else {
                            buffer.edit_selection("move selection", |_i, s, _sym| {
                                crate::algo::r#move(s, dir)
                            });
                        }
                        self.visual_message()
                    })
                }
            }

            _ => {
                buffer.move_cursor(dir);
                None
            }
        }
    }
    pub fn normal_message(&self) -> Message {
        Message::normal(&format!("Used {}", self.string_short()))
    }
    pub fn visual_message(&self) -> Message {
        Message::normal(&format!("Used {} selection", self.string_short()))
    }
}

fn _paste(
    register: &Register,
    image: &Image,
    mut offset: (i32, i32),
    mode: NormalMode,
    graphics: Option<&mut Graphics>,
) -> (Image, Selection) {
    offset.0 += register.offset.0;
    offset.1 += register.offset.1;
    let new = if let (Some(graphics), NormalMode::Blend(_)) = (graphics, mode) {
        let mut source = image.blank();
        register.content.blit(&mut source, offset.0, offset.1);
        graphics.render(image, &source)
    } else {
        let mut new = image.clone();
        for x in 0..register.width() as i32 {
            for y in 0..register.height() as i32 {
                if register.selection.contains((x, y)) {
                    new.get_mut(x + offset.0, y + offset.1).map(|c| {
                        let color = *register.content.get_unchecked(x, y);
                        match mode {
                            NormalMode::Blend(srgb) => *c = c.blend(color, srgb),
                            NormalMode::Replace => *c = color,
                            NormalMode::Erase => *c = (0, 0, 0, 0).into(),
                        }
                    });
                }
            }
        }
        new
    };
    (new, crate::algo::r#move(&register.selection, offset))
}

pub fn paste_preview(
    register: &Register,
    buffer: &mut Buffer,
    image: &Image,
    offset: (i32, i32),
    mode: NormalMode,
    graphics: Option<&mut Graphics>,
) {
    buffer.preview_both(|_i, _s, _sym| _paste(register, image, offset, mode, graphics));
}

pub fn paste(
    register: &Register,
    from: PasteFrom,
    buffer: &mut Buffer,
    img_idx: Option<ImgIndex>,
    offset: Option<(i32, i32)>,
    mode: NormalMode,
    graphics: Option<&mut Graphics>,
) -> (Image, ImgIndex, RegIndex) {
    let (name, offset) = match offset {
        Some(offset) => ("move", offset),
        None => ("paste", (0, 0)),
    };
    let img_idx = img_idx.unwrap_or(buffer.img_idx());
    let image = buffer.session.image_from_index(img_idx);
    let (pasted, selection) = _paste(register, &image, offset, mode, graphics);
    let reg_idx = buffer.paste(name, pasted, selection, from, img_idx, offset);
    (image, img_idx, reg_idx)
}
