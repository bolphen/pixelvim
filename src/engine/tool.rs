use super::buffer::{Buffer, ImgIndex, PasteFrom, RegIndex};
use super::register::Register;
use super::{Message, Modifier, NormalMode, VisualMode};
use crate::algo::{Brush, Selection};
use crate::color::{Color, ColorMode};
use crate::image::Image;

type Trace = Vec<(i32, i32)>;
type Bounds = ((i32, i32), (i32, i32));
type Once = (i32, i32);
type Dir = (i32, i32);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    Keyboard,
    Mouse,
}
#[derive(Default)]
pub struct Tracker {
    trace: Option<(Vec<(i32, i32)>, InputType)>,
    pub pixel_perfect: bool,
}
impl Tracker {
    pub fn is_any_in_use(&self) -> bool {
        self.trace.is_some()
    }
    pub fn is_mouse_in_use(&self) -> bool {
        self.trace
            .as_ref()
            .is_some_and(|(_, input)| matches!(input, InputType::Mouse))
    }
    pub fn is_keyboard_in_use(&self) -> bool {
        self.trace
            .as_ref()
            .is_some_and(|(_, input)| matches!(input, InputType::Keyboard))
    }
    pub fn start(&mut self, pos: (i32, i32), input: InputType) {
        self.trace = Some((vec![pos], input));
    }
    pub fn trim(&mut self) {
        if let Some((trace, _)) = &mut self.trace {
            trace.drain(0..trace.len() - 1);
        }
    }
    pub fn track_mouse(&mut self, pos: (i32, i32)) {
        if let Some((trace, InputType::Mouse)) = &mut self.trace {
            if trace.last().is_none_or(|p| *p != pos) {
                trace.push(pos);
            }
        }
    }
    pub fn track_keyboard(&mut self, pos: (i32, i32)) {
        if let Some((trace, InputType::Keyboard)) = &mut self.trace {
            if trace.last().is_none_or(|p| *p != pos) {
                trace.push(pos);
            }
        }
    }
    pub fn stop(&mut self) {
        self.trace = None;
    }
    fn get_trace(&self) -> Option<Trace> {
        self.trace.as_ref().map(|(trace, _)| {
            if self.pixel_perfect {
                crate::algo::pixel_perfect_filter(trace)
            } else {
                trace.to_vec()
            }
        })
    }
    fn get_bounds(&self) -> Option<Bounds> {
        self.trace
            .as_ref()
            .map(|(trace, _)| (trace[0], trace[trace.len() - 1]))
    }
    fn get_once(&self) -> Option<Once> {
        self.trace.as_ref().map(|(trace, _)| trace[trace.len() - 1])
    }
    pub fn get_dir(&self) -> Option<Dir> {
        self.trace.as_ref().map(|(trace, _)| {
            let (start, end) = (trace[0], trace[trace.len() - 1]);
            (end.0 - start.0, end.1 - start.1)
        })
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
            Tool::Line | Tool::Move => (),
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
        }
    }
    pub fn normal_preview(
        &self,
        tracker: &Tracker,
        buffer: &mut Buffer,
        mode: NormalMode,
        color: Color,
        modifier: Option<Modifier>,
        tool_setting: &ToolSetting,
    ) {
        let color_mode = match mode {
            NormalMode::Blend(srgb) => ColorMode::Blend(color, srgb),
            NormalMode::Replace => ColorMode::Set(color),
            NormalMode::Erase => ColorMode::Clear,
        };
        match self {
            Tool::Brush(outline) => {
                if let Some(trace) = tracker.get_trace() {
                    buffer.preview(
                        |i, s, sym| {
                            crate::tool::brush(
                                i,
                                s,
                                trace,
                                color_mode,
                                sym,
                                &tool_setting.brush,
                                *outline,
                            )
                        },
                        *outline && !tracker.pixel_perfect,
                    );
                }
            }
            Tool::Rect(outline) => {
                if let Some(bounds) = tracker.get_bounds() {
                    buffer.preview(
                        |i, s, sym| crate::tool::rect(i, s, bounds, color_mode, sym, *outline),
                        false,
                    );
                }
            }
            Tool::Ellipse(outline) => {
                if let Some(bounds) = tracker.get_bounds() {
                    buffer.preview(
                        |i, s, sym| crate::tool::ellipse(i, s, bounds, color_mode, sym, *outline),
                        false,
                    );
                }
            }
            Tool::Flood(discon) => {
                tracker.get_once().map(|once| {
                    let tolerance = modifier
                        .and_then(Modifier::to_i32)
                        .unwrap_or(tool_setting.flood_tolerance as _)
                        .min(255) as u8;
                    buffer.preview(
                        |i, s, sym| {
                            crate::tool::flood(i, s, once, tolerance, color_mode, sym, *discon)
                        },
                        false,
                    );
                });
            }
            Tool::Line => {
                if let Some(bounds) = tracker.get_bounds() {
                    buffer.preview(
                        |i, s, sym| {
                            crate::tool::line(i, s, bounds, color_mode, sym, &tool_setting.brush)
                        },
                        false,
                    );
                }
            }
            Tool::Move => {
                if let Some(dir) = tracker.get_dir() {
                    buffer.preview(|i, _s, _sym| Some(crate::tool::r#move(i, dir)), false);
                }
            }
        }
    }
    pub fn visual_preview(
        &self,
        tracker: &Tracker,
        buffer: &mut Buffer,
        mode: VisualMode,
        modifier: Option<Modifier>,
        tool_setting: &ToolSetting,
    ) {
        match self {
            Tool::Brush(outline) => {
                if let Some(trace) = tracker.get_trace() {
                    buffer.preview_selection(|i, s, sym| {
                        mode.apply(
                            s.clone(),
                            &crate::algo::brush(i, trace, sym, &tool_setting.brush, *outline),
                        )
                    });
                }
            }
            Tool::Rect(outline) => {
                if let Some(bounds) = tracker.get_bounds() {
                    buffer.preview_selection(|i, s, sym| {
                        mode.apply(s.clone(), &crate::algo::rect(i, bounds, sym, *outline))
                    });
                }
            }
            Tool::Ellipse(outline) => {
                if let Some(bounds) = tracker.get_bounds() {
                    buffer.preview_selection(|i, s, sym| {
                        mode.apply(s.clone(), &crate::algo::ellipse(i, bounds, sym, *outline))
                    });
                }
            }
            Tool::Line => {
                if let Some(bounds) = tracker.get_bounds() {
                    buffer.preview_selection(|i, s, sym| {
                        mode.apply(
                            s.clone(),
                            &crate::algo::line(i, bounds, sym, &tool_setting.brush),
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
        }
    }
    pub fn visual_use(
        &mut self,
        tracker: &Tracker,
        buffer: &mut Buffer,
        mode: VisualMode,
        modifier: Option<Modifier>,
        tool_setting: &ToolSetting,
    ) -> Option<Message> {
        match &self {
            Tool::Brush(outline) => tracker.get_trace().map(|trace| {
                buffer.edit_selection(
                    &format!("{} selection", self.string_short(),),
                    |i, s, sym| {
                        mode.apply(
                            s.clone(),
                            &crate::algo::brush(i, trace, sym, &tool_setting.brush, *outline),
                        )
                    },
                );
                self.visual_message()
            }),
            Tool::Rect(outline) => tracker.get_bounds().map(|bounds| {
                buffer.edit_selection(
                    &format!("{} selection", self.string_short(),),
                    |i, s, sym| mode.apply(s.clone(), &crate::algo::rect(i, bounds, sym, *outline)),
                );
                self.visual_message()
            }),
            Tool::Ellipse(outline) => tracker.get_bounds().map(|bounds| {
                buffer.edit_selection(
                    &format!("{} selection", self.string_short(),),
                    |i, s, sym| {
                        mode.apply(s.clone(), &crate::algo::ellipse(i, bounds, sym, *outline))
                    },
                );
                self.visual_message()
            }),
            Tool::Line => tracker.get_bounds().map(|bounds| {
                buffer.edit_selection(
                    &format!("{} selection", self.string_short(),),
                    |i, s, sym| {
                        mode.apply(
                            s.clone(),
                            &crate::algo::line(i, bounds, sym, &tool_setting.brush),
                        )
                    },
                );
                self.visual_message()
            }),
            Tool::Flood(discon) => tracker.get_once().map(|once| {
                let tolerance = modifier
                    .and_then(Modifier::to_i32)
                    .unwrap_or(tool_setting.flood_tolerance as _)
                    .min(255) as u8;
                buffer.edit_selection(
                    &format!("{} selection", self.string_short(),),
                    |i, s, sym| {
                        mode.apply(
                            s.clone(),
                            &crate::algo::flood(i, None, once, tolerance, sym, *discon),
                        )
                    },
                );
                self.visual_message()
            }),
            Tool::Move => {
                if buffer.selection().is_empty() {
                    Some(Message::error("Empty selection"))
                } else {
                    tracker.get_dir().map(|dir| {
                        (dir != (0, 0)).then(|| {
                            buffer.edit_selection(
                                &format!("{} selection", self.string_short(),),
                                |_i, s, _sym| crate::algo::r#move(s, dir),
                            );
                            self.visual_message()
                        })
                    })?
                }
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
    fn visual_message(&self) -> Message {
        Message::normal(&format!("Used {} selection", self.string_short()))
    }
}

fn _paste(
    register: &Register,
    image: &Image,
    mut offset: (i32, i32),
    mode: NormalMode,
) -> (Image, Selection) {
    offset.0 += register.offset.0;
    offset.1 += register.offset.1;
    let mut new = image.clone();
    for x in 0..register.width() as i32 {
        for y in 0..register.height() as i32 {
            if register.selection.contains(&(x, y)) {
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
    (new, crate::algo::r#move(&register.selection, offset))
}

pub fn paste_preview(
    register: &Register,
    buffer: &mut Buffer,
    image: &Image,
    offset: (i32, i32),
    mode: NormalMode,
) {
    buffer.preview_both(|_i, _s, _sym| _paste(register, image, offset, mode));
}

pub fn paste(
    register: &Register,
    from: PasteFrom,
    buffer: &mut Buffer,
    img_idx: Option<ImgIndex>,
    offset: Option<(i32, i32)>,
    mode: NormalMode,
) -> (Image, ImgIndex, RegIndex) {
    let (name, offset) = match offset {
        Some(offset) => ("move", offset),
        None => ("paste", (0, 0)),
    };
    let img_idx = img_idx.unwrap_or(buffer.img_idx());
    let image = buffer.session.image_from_index(img_idx);
    let (pasted, selection) = _paste(register, &image, offset, mode);
    let reg_idx = buffer.paste(name, pasted, selection, from, img_idx, offset);
    (image, img_idx, reg_idx)
}
