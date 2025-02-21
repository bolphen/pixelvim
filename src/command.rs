use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::color::Color;
use crate::mapping::{Action, KeyOrChar};
use crate::parser::{Float, Int, OnOff, Size};

pub enum ColorOrIndex {
    Color(Color),
    Index(usize),
}

impl std::str::FromStr for ColorOrIndex {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        {
            Ok(if let Some(s) = s.trim().strip_prefix('$') {
                if let Ok(i) = s.parse() {
                    ColorOrIndex::Index(i)
                } else {
                    Err("Cannot parse color")?
                }
            } else {
                ColorOrIndex::Color(s.parse()?)
            })
        }
    }
}
pub enum Setting {
    Color(ColorOrIndex),
    XSymOffset(Int),
    YSymOffset(Int),
    GridSize(Size),
    GridColor(Color),
    Background(Color),
    VisualColor(Color),
    VisualAlpha(Float),
    Scale(Float),
    Toggleable(Toggleable, OnOff),
    BrushSize(Int),
    BrushShape(String),
    FloodTolerance(Int),
    AnimationSpeed(Float),
}

#[derive(Clone, Copy)]
pub enum Toggleable {
    Fullscreen,
    Picker,
    Checker,
    Palette,
    Grid,
    XSym,
    YSym,
    Srgb,
    Animation,
    AnimationStrip,
    AnimationEditAll,
    Tile,
    Debug,
}

pub struct Settings {
    settings: HashMap<
        &'static str,
        (
            &'static str,
            Option<Toggleable>,
            Rc<dyn for<'a> Fn(&'a str) -> Result<Setting, String>>,
        ),
    >,
}
impl Settings {
    fn onoff_setting(
        mut self,
        name: &'static str,
        description: &'static str,
        setting: Toggleable,
    ) -> Self {
        self.settings.insert(
            name,
            (
                description,
                Some(setting),
                Rc::new(move |v| Ok(Setting::Toggleable(setting, v.parse()?))),
            ),
        );
        self
    }
    fn setting<F>(mut self, name: &'static str, description: &'static str, parser: F) -> Self
    where
        F: Fn(&str) -> Result<Setting, String> + 'static,
    {
        self.settings
            .insert(name, (description, None, Rc::new(parser)));
        self
    }
    pub fn parse(&self, expr: &str) -> Result<Setting, String> {
        let expr = expr.trim();
        let (s, value) = expr.split_once('=').unwrap_or((expr, "true"));
        let s = s.trim();
        let value = value.trim();
        let settings = self.settings.keys().copied().collect::<Vec<_>>();
        if let Match::Single(s) = match_keyword(s, &settings[..], true) {
            (self.settings.get(s).expect("").2)(value)
        } else {
            Err(format!("Unknown setting: {}", s))
        }
    }
    pub fn parse_toggleable(&self, expr: &str) -> Result<Setting, String> {
        let expr = expr.trim();
        let settings = self.settings.keys().copied().collect::<Vec<_>>();
        if let Match::Single(s) = match_keyword(expr, &settings[..], true) {
            let setting = self.settings.get(s).expect("");
            Ok(Setting::Toggleable(
                setting.1.ok_or(format!("Cannot toggle {}", s))?,
                OnOff(true),
            ))
        } else {
            Err(format!("Unknown setting: {}", expr))
        }
    }
    pub fn match_keyword(&self, expr: &str, toggleable_only: bool) -> Match {
        let settings = self
            .settings
            .iter()
            .filter_map(|(s, (_, b, _))| (!toggleable_only || b.is_some()).then_some(*s))
            .collect::<Vec<_>>();
        match_keyword(expr, &settings[..], false)
    }
    pub fn help(&self, s: &str) -> Option<String> {
        self.settings.get(s).map(|(d, _, _)| d.to_string())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            settings: HashMap::new(),
        }
        .setting("color", "Set main color", |value| {
            Ok(Setting::Color(value.parse()?))
        })
        .setting("background", "Set background color", |value| {
            Ok(Setting::Background(value.parse()?))
        })
        .setting("visual/color", "Set visual mode color", |value| {
            Ok(Setting::VisualColor(value.parse()?))
        })
        .setting("visual/alpha", "Set visual mode alpha", |value| {
            Ok(Setting::VisualAlpha(value.parse()?))
        })
        .setting("scale/ui", "Set UI scale", |value| {
            let scale: Float = value.parse()?;
            if scale.0 >= 1. && scale.0 <= 5. {
                Ok(Setting::Scale(scale))
            } else {
                Err("Not in the range 1..5")?
            }
        })
        .onoff_setting("fullscreen", "Toggle fullscreen", Toggleable::Fullscreen)
        .onoff_setting("picker", "Toggle color picker", Toggleable::Picker)
        .onoff_setting("checker", "Toggle checker", Toggleable::Checker)
        .onoff_setting("palette", "Toggle palette", Toggleable::Palette)
        .onoff_setting(
            "x-sym",
            "Buffer toggle symmetry in X-axis",
            Toggleable::XSym,
        )
        .onoff_setting(
            "y-sym",
            "Buffer toggle symmetry in Y-axis",
            Toggleable::YSym,
        )
        .setting(
            "x-sym/offset",
            "Buffer set symmetry offset in X-axis",
            |value| Ok(Setting::XSymOffset(value.parse()?)),
        )
        .setting(
            "y-sym/offset",
            "Buffer set symmetry offset in Y-axis",
            |value| Ok(Setting::YSymOffset(value.parse()?)),
        )
        .onoff_setting("grid", "Buffer toggle grid", Toggleable::Grid)
        .setting("grid/size", "Buffer set grid size", |value| {
            Ok(Setting::GridSize(value.parse()?))
        })
        .setting("grid/color", "Buffer set grid color", |value| {
            Ok(Setting::GridColor(value.parse()?))
        })
        .onoff_setting("srgb", "SRGB color profile", Toggleable::Srgb)
        .setting("brush/size", "Brush size", |value| {
            Ok(Setting::BrushSize(value.parse()?))
        })
        .setting("brush/shape", "Brush shape", |value| {
            Ok(Setting::BrushShape(value.into()))
        })
        .setting("flood/tolerance", "Flood tolerance", |value| {
            Ok(Setting::FloodTolerance(value.parse()?))
        })
        .onoff_setting("animation", "Buffer animation", Toggleable::Animation)
        .onoff_setting(
            "animation/edit/all",
            "Buffer edit all frames at the same time",
            Toggleable::AnimationEditAll,
        )
        .setting("animation/speed", "Buffer animation speed", |value| {
            Ok(Setting::AnimationSpeed(value.parse()?))
        })
        .onoff_setting(
            "animation/strip",
            "Show full animation strip",
            Toggleable::AnimationStrip,
        )
        .onoff_setting("tile", "Display in tile mode", Toggleable::Tile)
        .onoff_setting("debug", "Memory usage", Toggleable::Debug)
    }
}

pub struct KeyMapEntry {
    pub key: KeyOrChar,
    pub key_down: Vec<Action>,
    pub key_up: Vec<Action>,
}

impl std::str::FromStr for KeyMapEntry {
    type Err = String;
    fn from_str(expr: &str) -> Result<Self, String> {
        let (key, acts);
        if let Some(rest) = expr.strip_prefix("' '") {
            key = KeyOrChar::Char(' ');
            acts = rest;
        } else if let Some((k, rest)) = expr.split_once(' ') {
            key = k.trim().parse()?;
            acts = rest;
        } else {
            return Err("Cannot parse key map")?;
        }
        let mut key_down = Vec::new();
        let mut key_up = Vec::new();
        if let Some((down, up)) = acts.split_once(" KEYUP ") {
            for act in down.split(" THEN ") {
                key_down.push(act.trim().parse()?);
            }
            for act in up.split(" THEN ") {
                key_up.push(act.trim().parse()?);
            }
        } else {
            for act in acts.split(" THEN ") {
                key_down.push(act.trim().parse()?);
            }
        }
        Ok(KeyMapEntry {
            key,
            key_down,
            key_up,
        })
    }
}

#[derive(Clone, Copy)]
pub enum Modifier {
    Num(i32),
    Go(Option<i32>),
    _Unknown,
}

impl Modifier {
    pub fn push_digit(self, i: i32) -> Option<Self> {
        match self {
            Modifier::Num(n) => {
                if n < 100000000 {
                    Some(Modifier::Num(10 * n + i))
                } else {
                    Some(self)
                }
            }
            // Modifier::Go(None) => (i > 0).then_some(Modifier::Go(Some(i))),
            // Modifier::Go(Some(n)) => {
            //     if n < 100000000 {
            //         Some(Modifier::Go(Some(10 * n + i)))
            //     } else {
            //         Some(self)
            //     }
            // }
            _ => None,
        }
    }
    pub fn to_i32(self) -> Option<i32> {
        match self {
            Modifier::Num(n) => Some(n),
            _ => None,
        }
    }
}

pub enum Command {
    Quit { forced: bool },
    QuitAll { forced: bool },
    Help,
    Buffer(String),
    New(Option<Size>),
    Edit(PathBuf),
    Write { path: Option<PathBuf>, forced: bool },
    PrintWorkingDir,
    ChangeDir(PathBuf),
    Resize(Size),
    // setting parsing are delayed
    Set(String),
    Toggle(String),
    Fit,
    Map(KeyMapEntry),
    MapNormal(KeyMapEntry),
    MapVisual(KeyMapEntry),
    Source(PathBuf),
    Undo(Int),
    Redo(Int),
    Crop,
    Reduce,
    Quantize(Int),
    FlipHorizontal,
    FlipVertical,
    LayerGoAbove(Int),
    LayerGoBelow(Int),
    LayerNewAbove(Int),
    LayerNewBelow(Int),
    LayerMergeDown(Int),
    LayerDelete,
    LayerToggle,
    FrameGoLeft(Int),
    FrameGoRight(Int),
    FrameNewLeft(Int),
    FrameNewRight(Int),
    FrameDuplicate(Int),
    FrameDelete,
    Slice(Int),
    SelectAll,
    SelectInvert,
    SelectClear,
    PaletteAdd(Option<Color>), // None means current color
    PaletteDelete(ColorOrIndex),
    PaletteClear,
    PaletteSort,
    PaletteBuild,
    PaletteGradient(Color, Color, Int),
    RunScript(PathBuf, Option<Modifier>),
    Yank,
    Paste,
    Cut,
    Delete,
    PasteSystem,
}
impl Command {
    pub fn modify(self, modifier: Option<Modifier>) -> Self {
        match self {
            Self::Undo(..) => Self::Undo(Int(modifier.and_then(Modifier::to_i32).unwrap_or(1))),
            Self::Redo(..) => Self::Redo(Int(modifier.and_then(Modifier::to_i32).unwrap_or(1))),
            Self::LayerGoAbove(..) => {
                Self::LayerGoAbove(Int(modifier.and_then(Modifier::to_i32).unwrap_or(1)))
            }
            Self::LayerGoBelow(..) => {
                Self::LayerGoBelow(Int(modifier.and_then(Modifier::to_i32).unwrap_or(1)))
            }
            Self::FrameGoLeft(..) => {
                Self::FrameGoLeft(Int(modifier.and_then(Modifier::to_i32).unwrap_or(1)))
            }
            Self::FrameGoRight(..) => {
                Self::FrameGoRight(Int(modifier.and_then(Modifier::to_i32).unwrap_or(1)))
            }
            Self::LayerNewAbove(..) => {
                Self::LayerNewAbove(Int(modifier.and_then(Modifier::to_i32).unwrap_or(1)))
            }
            Self::LayerNewBelow(..) => {
                Self::LayerNewBelow(Int(modifier.and_then(Modifier::to_i32).unwrap_or(1)))
            }
            Self::LayerMergeDown(..) => {
                Self::LayerMergeDown(Int(modifier.and_then(Modifier::to_i32).unwrap_or(1)))
            }
            Self::FrameNewLeft(..) => {
                Self::FrameNewLeft(Int(modifier.and_then(Modifier::to_i32).unwrap_or(1)))
            }
            Self::FrameNewRight(..) => {
                Self::FrameNewRight(Int(modifier.and_then(Modifier::to_i32).unwrap_or(1)))
            }
            Self::FrameDuplicate(..) => {
                Self::FrameDuplicate(Int(modifier.and_then(Modifier::to_i32).unwrap_or(1)))
            }
            Self::Slice(..) => Self::Slice(Int(modifier.and_then(Modifier::to_i32).unwrap_or(1))),
            Self::Quantize(..) => {
                Self::Quantize(Int(modifier.and_then(Modifier::to_i32).unwrap_or(32)))
            }
            Self::RunScript(path, ..) => Self::RunScript(path, modifier),
            _ => self,
        }
    }
}
pub enum Match<'a> {
    None,
    Single(&'a str),
    Multiple(Vec<&'a str>),
}
impl Match<'_> {
    pub fn is(&self, s: &str) -> bool {
        if let Match::Single(m) = self {
            *m == s
        } else {
            false
        }
    }
}
// if greedy is on, [key, key/a, key/b] will match to key
// parsing should use greedy=true
// tab completion should use greedy=false
fn match_keyword<'a>(cmd: &str, opts: &[&'a str], greedy: bool) -> Match<'a> {
    opts.iter()
        .find(|o| **o == cmd)
        .map(|o| Match::Single(o))
        .unwrap_or_else(|| {
            let mut pass: Vec<_> = opts
                .iter()
                .enumerate()
                .filter_map(|(i, o)| o.starts_with(cmd).then_some(i))
                .collect();
            match pass.len() {
                0 => Match::None,
                1 => Match::Single(opts[pass[0]]),
                _ => {
                    pass.sort_by_key(|i| opts[*i].len());
                    if greedy
                        && pass[1..]
                            .iter()
                            .all(|i| opts[*i].starts_with(&format!("{}/", opts[pass[0]])))
                    {
                        Match::Single(opts[pass[0]])
                    } else {
                        Match::Multiple(pass.into_iter().map(|i| opts[i]).collect())
                    }
                }
            }
        })
}
pub struct Commands {
    commands: HashMap<
        &'static str,
        (
            &'static str,
            Rc<dyn for<'a> Fn(&'a str, bool) -> Result<Command, String>>,
        ),
    >,
}
impl Commands {
    fn command<F>(mut self, cmd: &'static str, description: &'static str, parser: F) -> Self
    where
        F: Fn(&str, bool) -> Result<Command, String> + 'static,
    {
        self.commands.insert(cmd, (description, Rc::new(parser)));
        self
    }
    pub fn parse(&self, expr: &str) -> Result<Command, String> {
        let expr = expr.trim_start();
        if expr.starts_with('#') {
            Ok(Command::PaletteAdd(Some(expr.parse()?)))
        } else {
            let (cmd, mut args) = expr.split_once(' ').unwrap_or((expr, ""));
            let (cmd, forced) = cmd
                .strip_suffix('!')
                .map(|c| (c, true))
                .unwrap_or((cmd, false));
            args = args.trim();
            let commands = self.commands.keys().copied().collect::<Vec<_>>();
            if let Match::Single(c) = match_keyword(cmd, &commands[..], true) {
                (self.commands.get(c).expect("").1)(args, forced)
            } else {
                Err(format!("Unknown command: {}", cmd))
            }
        }
    }
    pub fn match_keyword(&self, expr: &str) -> Match {
        let commands = self.commands.keys().copied().collect::<Vec<_>>();
        match_keyword(expr, &commands[..], false)
    }
    pub fn help(&self, cmd: &str) -> Option<String> {
        self.commands.get(cmd).map(|(d, _)| d.to_string())
    }
}

fn none_if_empty<T>(args: &str) -> Result<Option<T>, String>
where
    T: std::str::FromStr<Err = String>,
{
    Ok(if args.is_empty() {
        None
    } else {
        Some(args.parse()?)
    })
}

fn escape_path(path: &str) -> PathBuf {
    crate::utils::replace_home_dir(&path.replace("//", "/").replace("\\ ", " ").into())
}

impl Default for Commands {
    fn default() -> Self {
        Commands {
            commands: HashMap::new(),
        }
        .command("q", "Close current buffer", |_, forced| {
            Ok(Command::Quit { forced })
        })
        .command("quit", "Close current buffer", |_, forced| {
            Ok(Command::Quit { forced })
        })
        .command("qall", "Close all buffers and quit", |_, forced| {
            Ok(Command::QuitAll { forced })
        })
        .command("new", "Create new image", |args, _| {
            Ok(Command::New(none_if_empty(args)?))
        })
        .command("help", "Show help", |_, _| Ok(Command::Help))
        .command("buffer", "Switch to buffer", |args, _| {
            Ok(Command::Buffer(args.into()))
        })
        .command("edit", "Open image from disk", |args, _| {
            (!args.is_empty())
                .then(|| Command::Edit(escape_path(args)))
                .ok_or("No file name".into())
        })
        .command("source", "Load config file from disk", |args, _| {
            (!args.is_empty())
                .then(|| Command::Source(escape_path(args)))
                .ok_or("No file name".into())
        })
        .command("run", "Run lua script from disk", |args, _| {
            (!args.is_empty())
                .then(|| Command::RunScript(escape_path(args), None))
                .ok_or("No file name".into())
        })
        .command("write", "Save image to disk", |args, forced| {
            let path = (!args.is_empty()).then_some(args.into());
            Ok(Command::Write { path, forced })
        })
        .command("pwd", "Print working directory", |_, _| {
            Ok(Command::PrintWorkingDir)
        })
        .command("cd", "Change working directory", |args, _| {
            if args.is_empty() {
                Ok(Command::ChangeDir(escape_path("~")))
            } else {
                Ok(Command::ChangeDir(escape_path(args)))
            }
        })
        .command("resize", "Resize image", |args, _| {
            Ok(Command::Resize(args.parse()?))
        })
        .command("fit", "Fit image to view", |_, _| Ok(Command::Fit))
        .command("set", "Change setting (to on)", |args, _| {
            Ok(Command::Set(args.into()))
        })
        .command("unset", "Change setting to off", |args, _| {
            Ok(Command::Set(format!("{args}=false")))
        })
        .command("toggle", "Toggle setting", |args, _| {
            Ok(Command::Toggle(args.into()))
        })
        .command("map", "Map key to actions", |args, _| {
            Ok(Command::Map(args.parse()?))
        })
        .command(
            "map/normal",
            "Map key to actions in normal mode",
            |args, _| Ok(Command::MapNormal(args.parse()?)),
        )
        .command(
            "map/visual",
            "Map key to actions in visual mode",
            |args, _| Ok(Command::MapVisual(args.parse()?)),
        )
        .command("undo", "Undo", |args, _| {
            Ok(Command::Undo(none_if_empty(args)?.unwrap_or(Int(1))))
        })
        .command("redo", "Redo", |args, _| {
            Ok(Command::Redo(none_if_empty(args)?.unwrap_or(Int(1))))
        })
        .command("flip/horizontal", "Flip horizontally", |_, _| {
            Ok(Command::FlipHorizontal)
        })
        .command("flip/vertical", "Flip vertically", |_, _| {
            Ok(Command::FlipVertical)
        })
        .command("crop", "Crop", |_, _| Ok(Command::Crop))
        .command(
            "reduce",
            "Reduce using only colors from the palette",
            |_, _| Ok(Command::Reduce),
        )
        .command(
            "quantize",
            "Reduce the number of colors using the NeuQuant algorithm",
            |args, _| Ok(Command::Quantize(none_if_empty(args)?.unwrap_or(Int(32)))),
        )
        .command("layer/go/above", "Go to layer above", |args, _| {
            Ok(Command::LayerGoAbove(
                none_if_empty(args)?.unwrap_or(Int(1)),
            ))
        })
        .command("layer/go/below", "Go to layer below", |args, _| {
            Ok(Command::LayerGoBelow(
                none_if_empty(args)?.unwrap_or(Int(1)),
            ))
        })
        .command("layer/new/above", "Create new layer above", |args, _| {
            Ok(Command::LayerNewAbove(
                none_if_empty(args)?.unwrap_or(Int(1)),
            ))
        })
        .command("layer/new/below", "Create new layer below", |args, _| {
            Ok(Command::LayerNewBelow(
                none_if_empty(args)?.unwrap_or(Int(1)),
            ))
        })
        .command(
            "layer/merge/down",
            "Merge with the layer below",
            |args, _| {
                Ok(Command::LayerMergeDown(
                    none_if_empty(args)?.unwrap_or(Int(1)),
                ))
            },
        )
        .command("layer/delete", "Delete current layer", |_, _| {
            Ok(Command::LayerDelete)
        })
        .command("layer/toggle", "Toggle current layer visibility", |_, _| {
            Ok(Command::LayerToggle)
        })
        .command("frame/go/left", "Go to frame on the left", |args, _| {
            Ok(Command::FrameGoLeft(none_if_empty(args)?.unwrap_or(Int(1))))
        })
        .command("frame/go/right", "Go to frame on the right", |args, _| {
            Ok(Command::FrameGoRight(
                none_if_empty(args)?.unwrap_or(Int(1)),
            ))
        })
        .command(
            "frame/new/left",
            "Insert new frame on the left",
            |args, _| {
                Ok(Command::FrameNewLeft(
                    none_if_empty(args)?.unwrap_or(Int(1)),
                ))
            },
        )
        .command(
            "frame/new/right",
            "Insert new frame on the right",
            |args, _| {
                Ok(Command::FrameNewRight(
                    none_if_empty(args)?.unwrap_or(Int(1)),
                ))
            },
        )
        .command("frame/duplicate", "Duplicate current frame", |args, _| {
            Ok(Command::FrameDuplicate(
                none_if_empty(args)?.unwrap_or(Int(1)),
            ))
        })
        .command("frame/delete", "Delete current frame", |_, _| {
            Ok(Command::FrameDelete)
        })
        .command("slice", "Slice into frames", |args, _| {
            Ok(Command::Slice(none_if_empty(args)?.unwrap_or(Int(1))))
        })
        .command("select/all", "Select all", |_, _| Ok(Command::SelectAll))
        .command("select/invert", "Invert selection", |_, _| {
            Ok(Command::SelectInvert)
        })
        .command("select/clear", "Clear selection", |_, _| {
            Ok(Command::SelectClear)
        })
        .command("palette/add", "Add color to palette", |args, _| {
            Ok(Command::PaletteAdd(none_if_empty(args)?))
        })
        .command("palette/delete", "Delete color from palette", |args, _| {
            Ok(Command::PaletteDelete(args.parse()?))
        })
        .command("palette/clear", "Clear palette", |_, _| {
            Ok(Command::PaletteClear)
        })
        .command("palette/sort", "Sort palette", |_, _| {
            Ok(Command::PaletteSort)
        })
        .command("palette/build", "Build palette from image", |_, _| {
            Ok(Command::PaletteBuild)
        })
        .command("palette/gradient", "Add gradient to palette", |args, _| {
            let (c1, rest) = args.split_once(' ').ok_or("Cannot parse command")?;
            let c1 = c1.parse()?;
            let rest = rest.trim_start();
            let (c2, rest) = rest.split_once(' ').ok_or("Cannot parse command")?;
            Ok(Command::PaletteGradient(c1, c2.parse()?, rest.parse()?))
        })
        .command("yank", "Yank", |_, _| Ok(Command::Yank))
        .command("paste", "Paste", |_, _| Ok(Command::Paste))
        .command("cut", "Cut", |_, _| Ok(Command::Cut))
        .command("delete", "Delete", |_, _| Ok(Command::Delete))
        .command("paste/system", "Paste from system clipboard", |_, _| {
            Ok(Command::PasteSystem)
        })
    }
}
