mod buffer;
mod command;
mod history;
mod register;
mod render;
mod tool;
mod ui;

use miniquad::*;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::algo::{Brush, Selection};
use crate::color::Color;
use crate::command::{Command, Commands, Modifier, Setting, Settings, Toggleable};
use crate::error::EditError;
use crate::error::Error;
use crate::image::Image;
use crate::input::Input;
use crate::mapping::{Action, Key, KeyMap};
use buffer::{ImgIndex, PasteFrom, RegIndex};
use register::Register;
use tool::{Tool, ToolSetting, Tracker};
use ui::{TextViewer, UI};

pub use buffer::{Buffer, BufferId, FrameId, LayerId};

fn get_modifier(modifier: Option<Modifier>) -> Option<i32> {
    modifier.and_then(Modifier::to_i32)
}

pub enum DisplayMode {
    Expand(bool),
    Tile,
}

impl DisplayMode {
    pub fn set_strip(&mut self, value: bool) {
        match self {
            DisplayMode::Expand(v) => {
                *v = value;
            }
            DisplayMode::Tile => {
                if value {
                    *self = DisplayMode::Expand(true);
                }
            }
        }
    }
    pub fn set_tile(&mut self, value: bool) {
        match self {
            DisplayMode::Expand(..) => {
                if value {
                    *self = DisplayMode::Tile;
                }
            }
            DisplayMode::Tile => {
                if !value {
                    *self = DisplayMode::Expand(false);
                }
            }
        }
    }
    pub fn toggle_strip(&mut self) -> bool {
        match self {
            DisplayMode::Expand(v) => {
                *v = !*v;
                false
            }
            DisplayMode::Tile => {
                *self = DisplayMode::Expand(true);
                true
            }
        }
    }
    pub fn toggle_tile(&mut self) -> bool {
        match self {
            DisplayMode::Tile => {
                *self = DisplayMode::Expand(false);
                false
            }
            DisplayMode::Expand(..) => {
                *self = DisplayMode::Tile;
                true
            }
        }
    }
}

#[derive(Clone, Copy)]
pub enum VisualMode {
    Add,
    Sub,
}
impl VisualMode {
    pub fn apply(&self, mut selection: Selection, diff: &Selection) -> Selection {
        match self {
            VisualMode::Add => {
                selection.extend(diff);
            }
            VisualMode::Sub => {
                selection.retain(|p| !diff.contains(p));
            }
        }
        selection
    }
    pub fn toggle(&self) -> Self {
        match self {
            VisualMode::Add => VisualMode::Sub,
            VisualMode::Sub => VisualMode::Add,
        }
    }
}
#[derive(Clone, Copy)]
pub enum NormalMode {
    Blend(bool),
    Replace,
    Erase,
}
impl NormalMode {
    pub fn toggle(&self, replace: bool, srgb: bool) -> NormalMode {
        match self {
            NormalMode::Blend(_) => {
                if replace {
                    NormalMode::Replace
                } else {
                    NormalMode::Erase
                }
            }
            NormalMode::Replace => NormalMode::Erase,
            NormalMode::Erase => NormalMode::Blend(srgb),
        }
    }
}

enum MessageType {
    Normal,
    Warning,
    Error,
}
struct Message {
    text: String,
    message_type: MessageType,
}
impl Message {
    pub fn color(&self) -> Color {
        match self.message_type {
            MessageType::Normal => Color::LIGHTGRAY,
            MessageType::Warning => Color::ORANGE,
            MessageType::Error => Color::RED,
        }
    }
    pub fn normal(text: &str) -> Self {
        Message {
            text: text.into(),
            message_type: MessageType::Normal,
        }
    }
    pub fn warning(text: &str) -> Self {
        Message {
            text: text.into(),
            message_type: MessageType::Warning,
        }
    }
    pub fn error(text: &str) -> Self {
        Message {
            text: text.into(),
            message_type: MessageType::Error,
        }
    }
}

pub struct Engine {
    mode: Mode,
    buffers: Vec<Buffer>,
    buffer_count: usize,
    current: usize,
    quit_requested: bool,
    history: Vec<String>,
    tracker: Tracker,
    palette: Vec<Color>,
    picker: bool,
    pub screen: (f32, f32),
    mouse: (f32, f32),
    pub screen_tile: (i32, i32),
    pub mouse_tile: (i32, i32),
    pub show_ui: bool,
    show_palette: bool,
    registers: HashMap<char, Register>,
    ui: UI,
    key_map: KeyMap,

    commands: Commands,
    settings: Settings,

    color: Color,
    tool: Tool,
    tool_setting: ToolSetting,

    fullscreen: bool,
    background: Color,
    visual_color: Color,
    scale_ui: f32,
    checker: bool,
    display: DisplayMode,
    srgb: bool,
    debug: bool,

    draw_mode: (NormalMode, VisualMode),
    time: crate::utils::TimeManager,

    pub dropped_buffer: Vec<BufferId>,
}

struct PrePasteData {
    register: Register,
    reg_idx: Option<RegIndex>,
    image: Image,
    img_idx: ImgIndex,
    offset: (i32, i32),
    // remember the curret layer and frame
    // cancel paste when changed
    layer: LayerId,
    frame: FrameId,
}

enum Mode {
    Normal {
        message: Option<Message>,
        modifier: Option<Modifier>,
        paste_info: Option<PrePasteData>,
    },
    Visual {
        message: Option<Message>,
        modifier: Option<Modifier>,
    },
    Command(Input, bool),
    Help(TextViewer),
    #[cfg(all(feature = "lua", not(target_arch = "wasm32")))]
    Running(crate::lua::LuaHandle, std::sync::mpsc::Sender<bool>),
    _Insert,
}
impl Mode {
    pub fn normal(message: Option<Message>) -> Self {
        Mode::Normal {
            message,
            modifier: None,
            paste_info: None,
        }
    }
    pub fn visual(message: Option<Message>) -> Self {
        Mode::Visual {
            message,
            modifier: None,
        }
    }
    pub fn command() -> Self {
        Mode::Command(Input::new(""), true)
    }
    pub fn paste(
        image: Image,
        img_idx: ImgIndex,
        message: Option<Message>,
        register: Register,
        reg_idx: Option<RegIndex>,
        offset: Option<(i32, i32)>,
        layer: LayerId,
        frame: FrameId,
    ) -> Self {
        Mode::Normal {
            message,
            modifier: None,
            paste_info: Some(PrePasteData {
                register,
                reg_idx,
                image,
                img_idx,
                offset: offset.unwrap_or((0, 0)),
                layer,
                frame,
            }),
        }
    }
    fn take_modifier(&mut self) -> Option<Modifier> {
        if let Mode::Normal { modifier, .. } | Mode::Visual { modifier, .. } = self {
            modifier.take()
        } else {
            None
        }
    }
    fn take_paste_info(&mut self) -> Option<PrePasteData> {
        if let Mode::Normal { paste_info, .. } = self {
            paste_info.take()
        } else {
            None
        }
    }
    #[cfg(all(feature = "lua", not(target_arch = "wasm32")))]
    fn take_handle(&mut self) -> Option<crate::lua::LuaHandle> {
        if let Mode::Running(..) = self {
            let mut mode = Mode::normal(None);
            std::mem::swap(self, &mut mode);
            Some(if let Mode::Running(handle, ..) = mode {
                handle
            } else {
                unreachable!()
            })
        } else {
            None
        }
    }
    fn set_modifier(&mut self, new_modifier: Option<Modifier>) {
        if let Mode::Normal { modifier, .. } | Mode::Visual { modifier, .. } = self {
            *modifier = new_modifier;
        }
    }
    fn is_normal(&self) -> bool {
        matches!(self, Mode::Normal { .. })
    }
    fn is_visual(&self) -> bool {
        matches!(self, Mode::Visual { .. })
    }
    // set all modes to normal
    fn set_to_normal(&mut self, msg: Option<Message>) {
        *self = Mode::normal(msg);
    }
    // visual => visual, other => normal
    // resets paste mode
    fn reset(&mut self, msg: Option<Message>) {
        match self {
            Mode::Visual { .. } => *self = Mode::visual(msg),
            _ => *self = Mode::normal(msg),
        }
    }
    fn message(&mut self, msg: &str) {
        self.set_message(Message::normal(msg));
    }
    fn error(&mut self, msg: &str) {
        self.set_message(Message::error(msg));
    }
    fn warning(&mut self, msg: &str) {
        self.set_message(Message::warning(msg));
    }
    fn set_message(&mut self, msg: Message) {
        if let Mode::Normal { message, .. } | Mode::Visual { message, .. } = self {
            *message = Some(msg);
        }
    }
    fn clear_message(&mut self) {
        if let Mode::Normal { message, .. } | Mode::Visual { message, .. } = self {
            *message = None;
        }
    }
}
impl Engine {
    pub fn new() -> Self {
        let srgb = true;
        Engine {
            mode: Mode::normal(None),
            buffers: Vec::new(),
            buffer_count: 0,
            current: 0,
            tracker: Tracker::default(),
            palette: Vec::new(),
            picker: false,
            quit_requested: false,
            screen: (0., 0.),
            mouse: (0., 0.),
            screen_tile: (0, 0),
            mouse_tile: (0, 0),
            history: Vec::new(),
            show_ui: true,
            show_palette: true,
            registers: HashMap::new(),
            ui: UI::new(),
            key_map: KeyMap::default(),
            commands: Commands::default(),
            settings: Settings::default(),

            tool: Tool::Brush(true),
            color: Color::LIGHTGRAY,
            tool_setting: ToolSetting::new(),

            background: Color::DARKGRAY.alpha(192),
            visual_color: Color::ORANGE.alpha(64),
            fullscreen: false,
            scale_ui: 1.,
            checker: true,
            display: DisplayMode::Expand(true),
            srgb,
            debug: false,

            draw_mode: (NormalMode::Blend(srgb), VisualMode::Add),
            time: crate::utils::TimeManager::new(),

            dropped_buffer: Vec::new(),
        }
    }
    #[cfg(all(feature = "lua", not(target_arch = "wasm32")))]
    pub fn run_script_from_path(&mut self, path: &PathBuf, modifier: Option<Modifier>) {
        if let Ok(script) = std::fs::read_to_string(path) {
            if let Err(e) = self.run_script(script, modifier) {
                self.mode.error(&e.to_string())
            }
        } else {
            self.mode.error("Unable to read script");
        }
    }
    #[cfg(any(not(feature = "lua"), target_arch = "wasm32"))]
    pub fn run_script_from_path(&mut self, _path: &PathBuf, _modifier: Option<Modifier>) {
        self.mode.error("Lua extension not supported");
    }
    #[cfg(all(feature = "lua", not(target_arch = "wasm32")))]
    pub fn run_script(&mut self, script: String, modifier: Option<Modifier>) -> Result<(), Error> {
        let buffer = self
            .active_buffer_mut()
            .map_err(|_| Error::NoActiveBuffer)?;
        let lua = crate::lua::LuaInstance::new()?;
        lua.init(buffer)?;
        lua.set_modifier(get_modifier(modifier))?;
        let (handle, interrupt) = lua.exec(script);
        self.mode = Mode::Running(handle, interrupt);
        Ok(())
    }
    #[cfg(all(feature = "lua", not(target_arch = "wasm32")))]
    pub fn finish_script(&mut self, lua_handle: crate::lua::LuaHandle) -> Result<(), Error> {
        let lua = lua_handle
            .join()
            .map_err(|_| Error::LuaError("Thread panics".into()))??;
        let buffer = self
            .active_buffer_mut()
            .map_err(|_| Error::NoActiveBuffer)?;
        let output = lua.retrieve_output()?;
        let name = output.name.as_ref().map_or("script", |s| s);
        buffer.batch_edit(Some(name), output.changed_frames, None, false)?;
        self.mode.message(&format!("Used {name}"));
        Ok(())
    }
    pub fn load_config_from_path(&mut self, path: &PathBuf) {
        if let Ok(config) = std::fs::read_to_string(path) {
            self.load_config(&config);
        } else {
            self.mode.error("Unable to read config file");
        }
    }
    pub fn load_config(&mut self, config: &str) {
        let mut errors = Vec::new();
        for (i, line) in config.lines().enumerate() {
            let line = line.split_once("--").map(|(l, _)| l).unwrap_or(line).trim();
            if !line.is_empty()
                && self
                    .commands
                    .parse(line)
                    .and_then(|cmd| self.do_command(cmd))
                    .is_err()
            {
                errors.push(i);
            }
        }
        if errors.is_empty() {
            self.mode.clear_message();
        } else {
            let numbers: Vec<_> = errors.iter().map(|i| i.to_string()).collect();
            self.mode.error(&format!(
                "Config file contains error on line {}",
                numbers.join(",")
            ));
        }
    }
    pub fn scale_ui(&self) -> f32 {
        self.scale_ui
    }
    pub fn srgb(&self) -> bool {
        self.srgb
    }
    fn toggle_srgb(&mut self, value: Option<bool>) {
        let value = value.unwrap_or(!self.srgb);
        if self.srgb != value {
            self.srgb = value;
            for buffer in &mut self.buffers {
                buffer.set_all_dirty();
            }
            if matches!(self.draw_mode.0, NormalMode::Blend(..)) {
                self.draw_mode.0 = NormalMode::Blend(value)
            }
        }
    }
    fn new_buffer(&mut self, buffer: Buffer) -> &mut Buffer {
        if self.buffers.is_empty() {
            self.buffers.push(buffer);
            &mut self.buffers[0]
        } else {
            self.buffers.insert(self.current + 1, buffer);
            self.current += 1;
            &mut self.buffers[self.current]
        }
    }
    fn new_buffer_from_image(&mut self, image: Image) -> &mut Buffer {
        self.new_buffer_from_layers(vec![vec![image]], None, vec![100])
    }
    fn new_buffer_from_layers(
        &mut self,
        layers: Vec<Vec<Image>>,
        visibility: Option<Vec<bool>>,
        delay: Vec<u32>,
    ) -> &mut Buffer {
        let mut buffer = Buffer::new(layers, visibility, delay, BufferId::new(self.buffer_count));
        self.buffer_count += 1;
        buffer.fit_to_view(self.screen);
        self.new_buffer(buffer)
    }
    /// for file path
    fn new_buffer_from_file_path(
        &mut self,
        path: &PathBuf,
        bytes: Option<Vec<u8>>,
    ) -> Result<(), String> {
        match (path.canonicalize(), &bytes) {
            (Ok(path_can), _) => {
                if let Some(i) = self.buffers.iter().position(|b| {
                    b.metadata
                        .path
                        .as_ref()
                        .is_some_and(|p| p.canonicalize().is_ok_and(|p| p == path_can))
                }) {
                    self.current = i;
                    self.mode
                        .set_to_normal(Some(Message::normal("Switch to opened buffer")));
                    return Ok(());
                }
            }
            (Err(e), None) => Err(e.to_string())?,
            _ => (),
        }
        let ext = path.extension().ok_or("Unsupported file format")?;
        if let Some(swap) = crate::utils::get_swap_path(path) {
            if swap.exists() {
                if let Ok(bytes) = std::fs::read(&swap) {
                    if let Ok(mut buffer) =
                        Buffer::load_from_swap(bytes, BufferId::new(self.buffer_count))
                    {
                        buffer.metadata.path = Some(path.into());
                        buffer.metadata.swap = Some(swap);
                        self.buffer_count += 1;
                        buffer.fit_to_view(self.screen);
                        self.new_buffer(buffer);
                        self.mode.set_to_normal(Some(Message::normal(
                            "Recovered last session from swap file",
                        )));
                        // do not delete the swap yet
                        // it will be deleted upon writing or a force quit
                        return Ok(());
                    }
                }
                // unable to recover
                // either can't read or can't decode swap
                // delete it
                let _ = std::fs::remove_file(&swap);
            }
        }
        match ext {
            e if e == "png" => {
                let bytes = match bytes {
                    Some(bytes) => bytes,
                    None => std::fs::read(path).map_err(|e| e.to_string())?,
                };
                let buffer = self.new_buffer_from_image(crate::format::load_png(&bytes[..])?);
                buffer.metadata.path = Some(path.into());
            }
            e if e == "gif" => {
                let bytes = match bytes {
                    Some(bytes) => bytes,
                    None => std::fs::read(path).map_err(|e| e.to_string())?,
                };
                let (frames, delay) = crate::format::load_gif(&bytes[..])?;
                let buffer = self.new_buffer_from_layers(vec![frames], None, delay);
                buffer.metadata.path = Some(path.into());
            }
            e if e == "ase" || e == "aseprite" => {
                let bytes = match bytes {
                    Some(bytes) => bytes,
                    None => std::fs::read(path).map_err(|e| e.to_string())?,
                };
                let (layers, visibility, delay) = crate::format::load_ase(&bytes[..])?;
                let buffer = self.new_buffer_from_layers(layers, Some(visibility), delay);
                buffer.metadata.path = Some(path.into());
            }
            e if e == "swp" => {
                let bytes = match bytes {
                    Some(bytes) => bytes,
                    None => std::fs::read(path).map_err(|e| e.to_string())?,
                };
                let mut buffer = Buffer::load_from_swap(bytes, BufferId::new(self.buffer_count))?;
                self.buffer_count += 1;
                buffer.session.mark_unsaved();
                buffer.fit_to_view(self.screen);
                self.new_buffer(buffer);
                self.mode
                    .set_to_normal(Some(Message::warning("Loaded swap file")));
                return Ok(());
            }
            e if e == "conf" => {
                if let Some(bytes) = bytes {
                    self.load_config(
                        &String::from_utf8(bytes).map_err(|_| "Unable to read config file")?,
                    );
                } else {
                    self.load_config_from_path(path);
                }
                return Ok(());
            }
            _ => Err("Unsupported file format")?,
        }
        self.mode
            .set_to_normal(Some(Message::normal("Opened image")));
        Ok(())
    }
    /// for file path or directory path
    pub fn load_path(&mut self, path: &PathBuf) {
        if path.is_dir() {
            match std::fs::read_dir(path) {
                Ok(dir) => {
                    for f in dir.filter_map(|f| f.ok()) {
                        let path = f.path();
                        if path.is_file()
                            && path.extension().is_some_and(|ext| {
                                ["png", "gif", "ase", "aseprite"].iter().any(|e| ext == *e)
                            })
                        {
                            // do not stop when one file doesn't load
                            let _ = self.new_buffer_from_file_path(&path, None);
                        }
                    }
                    self.mode
                        .set_to_normal(Some(Message::normal("Opened directory")));
                }

                Err(e) => self.mode.error(&e.to_string()),
            }
        } else if let Err(e) = self.new_buffer_from_file_path(path, None) {
            self.mode.error(&e);
        }
    }
    pub fn drop_file(&mut self, path: &PathBuf, bytes: Vec<u8>) {
        if let Err(e) = self.new_buffer_from_file_path(path, Some(bytes)) {
            self.mode.error(&e);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_buffer(&mut self, path: Option<PathBuf>, forced: bool) -> Result<(), String> {
        let mut path_is_current = false;
        if let Some(path) = &path {
            if let Ok(path_can) = path.canonicalize() {
                if let Some(i) = self.buffers.iter().position(|b| {
                    b.metadata
                        .path
                        .as_ref()
                        .is_some_and(|p| p.canonicalize().is_ok_and(|p| p == path_can))
                }) {
                    if i != self.current {
                        if !forced {
                            Err("File already opened in another buffer".to_string())?
                        } else {
                            self.buffers.remove(i);
                            if i < self.current {
                                self.current -= 1;
                            }
                        }
                    } else {
                        path_is_current = true;
                    }
                }
            }
        }
        let buffer = self.active_buffer_mut()?;
        let (path, save_to_new) = path
            .as_ref()
            .map(|p| (p, !path_is_current))
            .or(buffer.metadata.path.as_ref().map(|p| (p, false)))
            .ok_or("No file name")?;
        if save_to_new && path.exists() && !forced {
            Err("File exists. Use :w! to overwrite")?
        }
        if path.extension().is_some_and(|e| e == "png") {
            let mut data = Vec::new();
            crate::format::write_png(buffer.composed(), &mut data)?;
            std::fs::write(path, data).map_err(|e| e.to_string())?;
            buffer.metadata.path = Some(path.into());
            buffer.clean_swap();
            buffer.history_mark_saved();
            if buffer.num_layers() > 1 || buffer.num_frames() > 1 {
                self.mode
                    .warning("Saved to png. Warning: workspace contains multiple layers/frames");
            } else if save_to_new {
                self.mode.message("Saved to path");
            } else {
                self.mode.message("Saved");
            }
        } else if path.extension().is_some_and(|e| e == "gif") {
            let mut data = Vec::new();
            let (l, d) = buffer.gif_data();
            crate::format::write_gif(l, d, &mut data)?;
            std::fs::write(path, data).map_err(|e| e.to_string())?;
            buffer.metadata.path = Some(path.into());
            buffer.clean_swap();
            buffer.history_mark_saved();
            if buffer.num_layers() > 1 {
                self.mode
                    .warning("Saved to gif. Warning: workspace contains multiple layers");
            } else if save_to_new {
                self.mode.message("Saved to path");
            } else {
                self.mode.message("Saved");
            }
        } else {
            Err("Unsupported file format")?
        }
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    pub fn save_buffer(&mut self, path: Option<PathBuf>, forced: bool) -> Result<(), String> {
        let buffer = self.active_buffer_mut()?;
        let path = path.unwrap_or(if buffer.num_frames() > 1 {
            "output.gif".into()
        } else {
            "output.png".into()
        });
        if path.extension().is_some_and(|e| e == "png") {
            let mut data = Vec::new();
            crate::format::write_png(buffer.composed(), &mut data)?;
            unsafe {
                let file_type = "image/png";
                let path = path.as_os_str().to_str().unwrap();
                miniquad::native::wasm::fs::fs_save_file(
                    data.as_ptr() as _,
                    data.len() as _,
                    file_type.as_ptr() as _,
                    file_type.len() as _,
                    path.as_ptr() as _,
                    path.len() as _,
                );
            }
            buffer.metadata.path = Some(path.into());
            buffer.clean_swap();
            buffer.history_mark_saved();
            if buffer.num_layers() > 1 || buffer.num_frames() > 1 {
                self.mode
                    .warning("Saved to png. Warning: workspace contains multiple layers/frames");
            } else {
                self.mode.message("Saved");
            }
        } else if path.extension().is_some_and(|e| e == "gif") {
            let mut data = Vec::new();
            let (l, d) = buffer.gif_data();
            crate::format::write_gif(l, d, &mut data)?;
            unsafe {
                let file_type = "image/gif";
                let path = path.as_os_str().to_str().unwrap();
                miniquad::native::wasm::fs::fs_save_file(
                    data.as_ptr() as _,
                    data.len() as _,
                    file_type.as_ptr() as _,
                    file_type.len() as _,
                    path.as_ptr() as _,
                    path.len() as _,
                );
            }
            buffer.metadata.path = Some(path.into());
            buffer.clean_swap();
            buffer.history_mark_saved();
            if buffer.num_layers() > 1 {
                self.mode
                    .warning("Saved to gif. Warning: workspace contains multiple layers");
            } else {
                self.mode.message("Saved");
            }
        } else {
            Err("Unsupported file format")?
        }
        Ok(())
    }
    pub fn quit_requested(&self) -> bool {
        self.quit_requested
    }
    pub fn tool_string(&self) -> String {
        let mut s = self.tool.string(&self.tool_setting);
        let mut effects = Vec::new();
        match &self.mode {
            Mode::Normal { .. } => match self.draw_mode.0 {
                NormalMode::Replace => effects.push("replace"),
                NormalMode::Erase => effects.push("erase"),
                _ => (),
            },
            Mode::Visual { .. } => {
                if !matches!(self.tool, Tool::Move) {
                    match self.draw_mode.1 {
                        VisualMode::Add => effects.push("add"),
                        VisualMode::Sub => effects.push("sub"),
                    }
                }
            }
            _ => (),
        }
        if matches!(self.tool, Tool::Brush { .. }) && self.tracker.pixel_perfect {
            effects.push("pixel perfect")
        }
        if !effects.is_empty() {
            s.push_str(&format!(" ({})", effects.join(", ")));
        }
        s
    }
    pub fn active_buffer_mut(&mut self) -> Result<&mut Buffer, String> {
        self.buffers
            .get_mut(self.current)
            .ok_or("No active buffer".into())
    }
    pub fn key_up_event(&mut self, keycode: KeyCode, _keymods: KeyMods) {
        if keycode == KeyCode::LeftAlt || keycode == KeyCode::RightAlt {
            self.picker = false;
        }
    }
    pub fn key_down_event(&mut self, keycode: KeyCode, keymods: KeyMods, repeat: bool) {
        #[cfg(all(feature = "lua", not(target_arch = "wasm32")))]
        if let Mode::Running(_, interrupt) = &mut self.mode {
            // handle interrupt signal
            if let (KeyCode::C, true, false) = (keycode, keymods.ctrl, keymods.shift) {
                let _ = interrupt.send(true);
                self.mode.reset(Some(Message::warning("Script cancelled")));
            }
            // otherwise just ignore
            return;
        }
        self.mode.clear_message();
        if let Mode::Help(view) = &mut self.mode {
            match (keycode, keymods.ctrl, keymods.shift) {
                (KeyCode::Escape, false, false) => {
                    self.mode.set_to_normal(None);
                }
                (KeyCode::J, false, false) | (KeyCode::Down, false, false) => {
                    view.scroll(1);
                }
                (KeyCode::K, false, false) | (KeyCode::Up, false, false) => {
                    view.scroll(-1);
                }
                (KeyCode::U, true, false) | (KeyCode::PageUp, false, false) => {
                    view.scroll(-self.screen_tile.1 / 2);
                }
                (KeyCode::D, true, false) | (KeyCode::PageDown, false, false) => {
                    view.scroll(self.screen_tile.1 / 2);
                }
                (KeyCode::G, false, false) | (KeyCode::Home, false, false) => {
                    view.scroll_to_start();
                }
                (KeyCode::G, false, true) | (KeyCode::End, false, false) => {
                    view.scroll_to_end();
                }
                _ => (),
            }
            return;
        }
        if let Mode::Command(input, _) = &mut self.mode {
            let data = crate::input::CompletionData {
                buffers: self.buffers.iter().map(|b| b.display_name()).collect(),
                palette: &self.palette,
                commands: &self.commands,
                settings: &self.settings,
            };
            match (keycode, keymods.ctrl, keymods.shift) {
                (KeyCode::Enter, false, false) => {
                    if self.history.last().is_none_or(|h| h != input.text()) {
                        self.history.push(input.text().into());
                    }
                    let parsed_cmd = self.commands.parse(input.text());
                    self.mode.set_to_normal(None);
                    if let Err(e) = parsed_cmd.and_then(|cmd| self.do_command(cmd)) {
                        self.mode.error(&e);
                    }
                }
                (KeyCode::Escape, false, false) | (KeyCode::C, true, false) => {
                    self.mode.set_to_normal(None);
                }
                (KeyCode::V, true, true) => {
                    if let Some(c) = miniquad::window::clipboard_get() {
                        input.insert_str(c.as_str());
                    }
                }
                (KeyCode::Backspace, _, _) => {
                    if input.len() == 0 && !repeat {
                        self.mode.set_to_normal(None);
                    } else {
                        input.backspace();
                    }
                }
                (KeyCode::Delete, false, false) => {
                    input.delete();
                }
                (KeyCode::A, true, false) | (KeyCode::Home, false, false) => {
                    input.home();
                }
                (KeyCode::E, true, false) | (KeyCode::End, false, false) => {
                    input.end();
                }
                (KeyCode::Up, false, false) => {
                    if input.completions.is_some() {
                        input.prev_completion(&data);
                    } else {
                        let h: Vec<_> = self
                            .history
                            .iter()
                            .filter(|h| h.starts_with(&input.text()[..input.cursor()]))
                            .map(|h| crate::input::Entry(h.clone(), None, None))
                            .collect();
                        if !h.is_empty() {
                            input.completions = Some(((0..input.cursor()), 0, h));
                            input.prev_completion(&data);
                        }
                    }
                }
                (KeyCode::Down, false, false) => {
                    if input.completions.is_some() {
                        input.next_completion(&data);
                    }
                }
                (KeyCode::B, true, false) | (KeyCode::Left, false, false) => {
                    input.left();
                }
                (KeyCode::F, true, false) | (KeyCode::Right, false, false) => {
                    input.right();
                }
                (KeyCode::K, true, false) => {
                    input.kill();
                }
                (KeyCode::Y, true, false) => {
                    input.yank();
                }
                (KeyCode::Tab, false, true) => {
                    input.prev_completion(&data);
                }
                (KeyCode::Tab, false, false) => {
                    input.next_completion(&data);
                }
                _ => (),
            }
            return;
        }
        if keycode == KeyCode::LeftAlt || keycode == KeyCode::RightAlt {
            if !self.tracker.is_in_use() {
                self.picker = true;
            }
        } else {
            let key = Key::new(keycode, keymods.ctrl, keymods.shift);
            let actions = if self.mode.is_visual() {
                self.key_map.get_visual(key)
            } else {
                self.key_map.get_normal(key)
            }
            .to_vec();
            for action in &actions {
                match (action, self.buffers.get_mut(self.current), repeat) {
                    (
                        Action::Normal
                        | Action::NormalBlend
                        | Action::NormalErase
                        | Action::NormalReplace
                        | Action::NormalToggle,
                        buffer,
                        false,
                    ) => {
                        if self.mode.is_visual() {
                            self.mode = Mode::normal(None);
                        }
                        if let Some(b) = buffer {
                            b.clear_temporary()
                        }
                        self.draw_mode.0 = match action {
                            Action::Normal => self.draw_mode.0,
                            Action::NormalBlend => NormalMode::Blend(self.srgb),
                            Action::NormalErase => NormalMode::Erase,
                            Action::NormalReplace => NormalMode::Replace,
                            Action::NormalToggle => self.draw_mode.0.toggle(
                                self.color.3 != 255 || matches!(self.tool, Tool::Move),
                                self.srgb,
                            ),
                            _ => unreachable!(),
                        };
                    }
                    (
                        Action::Visual
                        | Action::VisualAdd
                        | Action::VisualSub
                        | Action::VisualToggle,
                        buffer,
                        false,
                    ) => {
                        if self.mode.is_normal() {
                            self.mode = Mode::visual(None);
                        }
                        if let Some(b) = buffer {
                            b.clear_temporary()
                        }
                        self.draw_mode.1 = match action {
                            Action::Visual => self.draw_mode.1,
                            Action::VisualAdd => VisualMode::Add,
                            Action::VisualSub => VisualMode::Sub,
                            Action::VisualToggle => self.draw_mode.1.toggle(),
                            _ => unreachable!(),
                        };
                    }
                    (Action::PixelPerfect, buffer, false) => {
                        self.tracker.pixel_perfect = !self.tracker.pixel_perfect;
                        if let Some(b) = buffer {
                            b.clear_temporary()
                        }
                    }
                    (
                        Action::Brush
                        | Action::BrushFilled
                        | Action::Flood
                        | Action::FloodAll
                        | Action::RectFilled
                        | Action::RectOutline
                        | Action::Move,
                        buffer,
                        false,
                    ) => {
                        let tool = match action {
                            Action::Brush => Tool::Brush(true),
                            Action::BrushFilled => Tool::Brush(false),
                            Action::Flood => Tool::Flood(false),
                            Action::FloodAll => Tool::Flood(true),
                            Action::RectFilled => Tool::Rect(false),
                            Action::RectOutline => Tool::Rect(true),
                            Action::Move => Tool::Move,
                            _ => unreachable!(),
                        };
                        if self.tool.set(tool) {
                            if let Some(b) = buffer {
                                b.clear_temporary()
                            }
                            self.mode.reset(Some(Message::normal(&format!(
                                "Change tool to {}",
                                self.tool.string_short()
                            ))));
                        }
                    }
                    (
                        Action::BrushToggle | Action::FloodToggle | Action::RectToggle,
                        buffer,
                        false,
                    ) => {
                        let tool = match action {
                            Action::BrushToggle => Tool::Brush(true),
                            Action::FloodToggle => Tool::Flood(false),
                            Action::RectToggle => Tool::Rect(false),
                            _ => unreachable!(),
                        };
                        self.tool.set_or_toggle(tool);
                        if let Some(b) = buffer {
                            b.clear_temporary()
                        }
                        self.mode.reset(Some(Message::normal(&format!(
                            "Change tool to {}",
                            self.tool.string_short()
                        ))));
                    }
                    (Action::Cancel, buffer, false) => {
                        self.mode.take_modifier();
                        if let Some(buffer) = buffer {
                            if self.tracker.is_in_use() {
                                self.tracker.stop();
                                buffer.clear_temporary();
                                if self.mode.is_visual() {
                                    self.mode.set_message(Message::normal(&format!(
                                        "Cancelled {} selection",
                                        self.tool.string_short()
                                    )));
                                } else {
                                    self.mode.set_message(Message::normal(&format!(
                                        "Cancelled {}",
                                        self.tool.string_short()
                                    )));
                                }
                                return;
                            }
                        }
                    }
                    _ => (),
                }
                if !self.tracker.is_in_use() {
                    match (action, self.buffers.get_mut(self.current), repeat) {
                        (Action::Command, _, false) => {
                            self.mode = Mode::command();
                            return;
                        }
                        (Action::Go, _, false) => {
                            let modifier = self.mode.take_modifier();
                            self.mode
                                .set_modifier(Some(Modifier::Go(get_modifier(modifier))));
                        }
                        (Action::TabFront, _, false) => {
                            if let Some(Modifier::Go(n)) = self.mode.take_modifier() {
                                let len = self.buffers.len();
                                if len > 0 {
                                    if let Some(n) = n {
                                        let n = n as usize;
                                        self.current = (n + len - 1) % len;
                                    } else {
                                        self.current = (self.current + 1) % len;
                                    }
                                    self.mode.set_to_normal(None);
                                }
                            }
                        }
                        (Action::TabBack, _, false) => {
                            if let Some(Modifier::Go(n)) = self.mode.take_modifier() {
                                let len = self.buffers.len();
                                if len > 0 {
                                    if let Some(n) = n {
                                        let n = n as usize;
                                        self.current = (len - n % len) % len;
                                    } else {
                                        self.current = (self.current + len - 1) % len;
                                    }
                                    self.mode.set_to_normal(None);
                                }
                            }
                        }
                        (
                            Action::Up | Action::Down | Action::Left | Action::Right,
                            Some(buffer),
                            _,
                        ) => {
                            let v = get_modifier(self.mode.take_modifier()).unwrap_or(1);
                            let dir = match action {
                                Action::Right => (v, 0),
                                Action::Left => (-v, 0),
                                Action::Up => (0, -v),
                                Action::Down => (0, v),
                                _ => unreachable!(),
                            };
                            if dir != (0, 0) {
                                if self.mode.is_visual() {
                                    if let Some(msg) = self.tool.visual_move(buffer, dir, repeat) {
                                        self.mode.set_message(msg);
                                    }
                                } else if let Mode::Normal {
                                    paste_info:
                                        Some(PrePasteData {
                                            register,
                                            offset,
                                            reg_idx,
                                            img_idx,
                                            ..
                                        }),
                                    ..
                                } = &mut self.mode
                                {
                                    buffer.animation.paused = true;
                                    offset.0 += dir.0;
                                    offset.1 += dir.1;
                                    tool::paste(
                                        register,
                                        if let Some(reg_idx) = reg_idx {
                                            if repeat {
                                                PasteFrom::RegIndexAmend(*reg_idx)
                                            } else {
                                                PasteFrom::RegIndex(*reg_idx)
                                            }
                                        } else {
                                            PasteFrom::Register(register)
                                        },
                                        buffer,
                                        Some(*img_idx),
                                        Some(*offset),
                                        self.draw_mode.0,
                                    );
                                    self.mode.set_message(Message::normal("Used move"));
                                } else {
                                    match self.tool {
                                        Tool::Move => {
                                            if let Some(r) =
                                                Register::new(buffer.image(), buffer.selection())
                                            {
                                                buffer.animation.paused = true;
                                                let cut = crate::tool::cut(
                                                    buffer.image(),
                                                    buffer.selection(),
                                                );
                                                let img_idx = buffer.session.insert_image(&cut);
                                                let reg_idx = tool::paste(
                                                    &r,
                                                    PasteFrom::Register(&r),
                                                    buffer,
                                                    Some(img_idx),
                                                    Some(dir),
                                                    self.draw_mode.0,
                                                )
                                                .2;
                                                self.mode = Mode::paste(
                                                    cut,
                                                    img_idx,
                                                    Some(Message::normal("Used move")),
                                                    r,
                                                    Some(reg_idx),
                                                    Some(dir),
                                                    buffer.current_layer_id(),
                                                    buffer.current_frame_id(),
                                                );
                                            } else {
                                                if !buffer.edit_all {
                                                    buffer.animation.paused = true;
                                                }
                                                buffer.edit_infallible(
                                                    "move",
                                                    |i, _s, _sym| crate::tool::r#move(i, dir),
                                                    repeat,
                                                );
                                                self.mode.set_message(Message::normal("Used move"));
                                            }
                                        }
                                        _ => {
                                            buffer.move_cursor(dir);
                                        }
                                    }
                                }
                            }
                        }
                        (Action::Paste, Some(buffer), false) => {
                            if let Some(r) = self.registers.get(&'"') {
                                let (image, img_idx, reg_idx) = tool::paste(
                                    r,
                                    PasteFrom::Register(r),
                                    buffer,
                                    None,
                                    None,
                                    self.draw_mode.0,
                                );
                                self.tool = Tool::Move;
                                self.mode = Mode::paste(
                                    image,
                                    img_idx,
                                    Some(Message::normal("Pasted from register \"\"")),
                                    r.clone(),
                                    Some(reg_idx),
                                    None,
                                    buffer.current_layer_id(),
                                    buffer.current_frame_id(),
                                );
                            } else {
                                self.mode.error("Register \"\" is empty");
                            }
                        }
                        (Action::Yank, Some(buffer), false) => {
                            if let Mode::Normal {
                                paste_info: Some(paste_info),
                                ..
                            } = &self.mode
                            {
                                let mut register = paste_info.register.clone();
                                register.offset.0 += paste_info.offset.0;
                                register.offset.1 += paste_info.offset.1;
                                self.registers.insert('"', register);
                                self.mode
                                    .set_message(Message::normal("Yanked to register \"\""));
                            } else if let Some(r) =
                                Register::new(buffer.image(), buffer.selection())
                            {
                                self.registers.insert('"', r);
                                self.mode
                                    .set_message(Message::normal("Yanked to register \"\""));
                            } else if let Some(cursor) = buffer.cursor() {
                                buffer.picker(cursor).map(|c| self.color = c);
                            }
                        }
                        (Action::Cut, Some(buffer), false) => {
                            let paste_info = self.mode.take_paste_info();
                            if let Some(paste_info) = paste_info {
                                let mut register = paste_info.register;
                                register.offset.0 += paste_info.offset.0;
                                register.offset.1 += paste_info.offset.1;
                                self.registers.insert('"', register);
                                buffer.edit_infallible(
                                    "cut",
                                    |_i, _s, _sym| paste_info.image,
                                    false,
                                );
                                buffer.amend_selection(|_i, _s, _sym| Selection::new());
                                self.mode
                                    .set_to_normal(Some(Message::normal("Cut to register \"\"")));
                            } else if let Some(r) =
                                Register::new(buffer.image(), buffer.selection())
                            {
                                self.registers.insert('"', r);
                                buffer
                                    .edit(
                                        Some("cut"),
                                        |i, s, _sym| Some(crate::tool::cut(i, s)),
                                        false,
                                    )
                                    .expect("infallible");
                                buffer.amend_selection(|_i, _s, _sym| Selection::new());
                                self.mode
                                    .set_to_normal(Some(Message::normal("Cut to register \"\"")));
                            }
                        }
                        (Action::Delete, Some(buffer), false) => {
                            if Register::new(buffer.image(), buffer.selection()).is_some() {
                                buffer.edit_infallible(
                                    "cut",
                                    |i, s, _sym| crate::tool::cut(i, s),
                                    false,
                                );
                                buffer.amend_selection(|_i, _s, _sym| Selection::new());
                                self.mode.set_to_normal(Some(Message::normal("Delete")));
                            }
                        }
                        (Action::DoCommand(cmd), _, _) => {
                            let parsed_cmd = self.commands.parse(cmd);
                            if let Ok(cmd) = parsed_cmd {
                                if let Err(e) = {
                                    let modifier = self.mode.take_modifier();
                                    self.do_command(cmd.modify(modifier))
                                } {
                                    self.mode.error(&e);
                                    return;
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
    }
    pub fn char_event(&mut self, char: char, keymods: KeyMods, _repeat: bool) {
        match &mut self.mode {
            // on certain platforms char_event is fired after key_down_event
            // so this hack catches and removes the extra ':' char when entering command mode
            Mode::Command(input, just_entered) => {
                if (!*just_entered || char != ':')
                    && !keymods.ctrl
                    && (char.is_alphanumeric() || (char.is_ascii() && !char.is_ascii_control()))
                {
                    input.insert(char);
                }
                *just_entered = false;
            }
            Mode::Normal { modifier, .. } | Mode::Visual { modifier, .. } => {
                if !self.tracker.is_in_use() && char.is_ascii_digit() {
                    let d = char as i32 - '0' as i32;
                    if let Some(m) = modifier.take() {
                        *modifier = m.push_digit(d);
                    } else {
                        *modifier = Some(Modifier::Num(d));
                    }
                }
            }
            Mode::Help(..) => {
                // hard-coded
                if char == ':' {
                    self.mode = Mode::command();
                }
            }
            _ => {}
        }
    }
    pub fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        if y != 0. {
            match &mut self.mode {
                Mode::Help(view) => {
                    view.scroll(-y.signum() as i32);
                }
                _ => {
                    if let Some(buffer) = self.buffers.get_mut(self.current) {
                        buffer.set_cursor(self.mouse);
                        buffer.zoom_at(1. + y.clamp(-1., 1.) / 10., Some(self.mouse));
                    }
                }
            }
        }
    }
    fn mouse_on_palette(&self) -> Option<(Color, usize)> {
        (self.show_palette || self.picker).then(|| {
            let n_rows = self.palette.len().div_ceil(self.ui.palette.each_row) as i32;
            let rel_x = self.mouse_tile.0 - self.ui.palette.from_left;
            let rel_y =
                self.mouse_tile.1 - (self.screen_tile.1 - n_rows - self.ui.palette.from_bottom);
            (rel_x >= 0 && rel_x < 4 * self.ui.palette.each_row as i32 && rel_y >= 0).then(|| {
                let index = rel_y as usize * self.ui.palette.each_row + rel_x as usize / 4;
                self.palette.get(index).map(|c| (*c, index))
            })
        })??
    }
    fn mouse_on_layer(&self) -> Option<(usize, ui::LayerClick)> {
        self.buffers.get(self.current).map(|buffer| {
            let n = buffer.num_layers() as i32;
            let m = buffer.num_frames() as i32;
            let len = n.to_string().len() as i32;
            let rel_x =
                self.mouse_tile.0 - (self.screen_tile.0 - self.ui.layers.from_right - 6 - len) + m;
            let rel_y = self.mouse_tile.1 - (self.screen_tile.1 - self.ui.layers.from_bottom - 1);
            if 0 <= rel_x && rel_x < len + m + 5 && -rel_y >= 0 && -rel_y < n {
                let click = match rel_x {
                    1 | 2 => ui::LayerClick::Toggle,
                    x if x >= 5 + len => ui::LayerClick::Frame((x - 5 - len) as _),
                    _ => ui::LayerClick::None,
                };
                Some((-rel_y as _, click))
            } else {
                None
            }
        })?
    }
    fn mouse_on_main_color(&self) -> bool {
        self.mouse_tile.0 >= 1
            && self.mouse_tile.0 <= 4
            && self.mouse_tile.1 == self.screen_tile.1 - 3
    }
    fn mouse_on_tool(&self) -> bool {
        self.mouse_tile.0 >= 1
            && self.mouse_tile.0 <= self.tool.string(&self.tool_setting).len() as i32 + 1
            && self.mouse_tile.1 == self.screen_tile.1 - 4
    }
    pub fn mouse_button_down(&mut self, button: MouseButton) {
        self.mode.clear_message();
        if matches!(button, MouseButton::Left) {
            let mut used = false;
            match self.mode {
                Mode::Command(..) => {
                    self.mode.set_to_normal(None);
                    return;
                }
                #[cfg(all(feature = "lua", not(target_arch = "wasm32")))]
                Mode::Running(..) => {
                    return;
                }
                _ => (),
            }
            // capture ui
            if !used && self.mouse_on_tool() {
                match self.tool {
                    Tool::Brush(..) => {
                        self.mode = Mode::Command(Input::new("set brush/shape="), true)
                    }
                    Tool::Flood(..) => {
                        self.mode = Mode::Command(Input::new("set flood/tolerance="), true)
                    }
                    _ => (),
                }
                used = true;
            }
            if !used && self.mouse_on_main_color() {
                self.show_palette = !self.show_palette;
                used = true;
            }
            if !used {
                self.mouse_on_layer().map(|(l, b)| {
                    let buffer = self.buffers.get_mut(self.current).expect("");
                    match b {
                        ui::LayerClick::Toggle => buffer.toggle_visibility(l, None),
                        ui::LayerClick::Frame(frame) => {
                            buffer.session.current_layer = l;
                            buffer.set_frame(frame);
                        }
                        _ => {
                            buffer.session.current_layer = l;
                        }
                    }
                    used = true;
                });
            }
            if !used {
                self.mouse_on_palette().map(|(c, _)| {
                    self.color = c;
                    used = true;
                });
            }
            if let Some(buffer) = self.buffers.get_mut(self.current) {
                let new = buffer.coordinate(self.mouse);
                if !used {
                    if self.picker {
                        buffer.picker(new).map(|c| self.color = c);
                    } else {
                        self.tracker.start(new);
                    }
                }
            }
        }
    }
    pub fn mouse_button_up(&mut self, button: MouseButton) {
        if matches!(button, MouseButton::Left) {
            if let Some(buffer) = self.buffers.get_mut(self.current) {
                match &mut self.mode {
                    Mode::Visual { message, modifier } => {
                        *message = self.tool.visual_use(
                            &self.tracker,
                            buffer,
                            self.draw_mode.1,
                            modifier.take(),
                            &self.tool_setting,
                        )
                    }
                    Mode::Normal {
                        message,
                        paste_info: None,
                        modifier,
                    } => {
                        modifier.take();
                        if !(matches!(self.tool, Tool::Move)
                            && self.tracker.get_dir() == Some((0, 0)))
                            && buffer.commit_temporary(self.tool.string_short()).is_some()
                        {
                            *message = Some(self.tool.normal_message());
                        };
                    }
                    Mode::Normal {
                        message,
                        paste_info:
                            Some(PrePasteData {
                                register,
                                reg_idx,
                                img_idx,
                                offset,
                                ..
                            }),
                        ..
                    } => {
                        if let Some(dir) = self.tracker.get_dir() {
                            if buffer.temporary_images.len() > 1 {
                                buffer.commit_temporary("move");
                                buffer.amend_selection(|_i, s, _sym| crate::algo::r#move(s, dir));
                                *message = Some(Message::normal("Used move"));
                            } else if dir != (0, 0) {
                                offset.0 += dir.0;
                                offset.1 += dir.1;
                                tool::paste(
                                    register,
                                    if let Some(reg_idx) = reg_idx {
                                        PasteFrom::RegIndex(*reg_idx)
                                    } else {
                                        PasteFrom::Register(register)
                                    },
                                    buffer,
                                    Some(*img_idx),
                                    Some(*offset),
                                    self.draw_mode.0,
                                );
                                *message = Some(Message::normal("Used move"));
                            }
                        }
                    }

                    _ => (),
                }
                buffer.clear_temporary();
                self.tracker.stop();
            }
        }
    }
    pub fn set_cursor(&mut self, mouse: (f32, f32)) {
        self.mouse = mouse;
        if let Some(buffer) = self.buffers.get_mut(self.current) {
            self.tracker.track(buffer.coordinate(self.mouse));
            buffer.set_cursor(self.mouse);
        }
    }
    pub fn update(&mut self) {
        let elapsed = self.time.elapsed();
        self.time.update();
        let mut running = false;
        #[cfg(all(feature = "lua", not(target_arch = "wasm32")))]
        if let Mode::Running(handle, _) = &mut self.mode {
            if handle.is_finished() {
                let lua_handle = self.mode.take_handle().expect("");
                if let Err(e) = self.finish_script(lua_handle) {
                    self.mode.reset(Some(Message::error(&e.to_string())));
                }
            } else {
                running = true;
            }
        }
        let mut frame_changed = false;
        if let Some(buffer) = self.buffers.get_mut(self.current) {
            // these are the updates that should be done even when the lua script is blocking
            if buffer.fit {
                buffer.fit_to_view(self.screen);
            }
            if !buffer.animation.paused {
                frame_changed = buffer.do_animation(elapsed);
            }
            if running {
                miniquad::window::set_mouse_cursor(CursorIcon::Wait);
                return;
            } else if buffer.cursor().is_some()
                && self.mouse_on_layer().is_none()
                && self.mouse_on_palette().is_none()
            {
                match (&self.mode, &self.tool) {
                    (Mode::Help(..), _) => {
                        miniquad::window::set_mouse_cursor(CursorIcon::Default);
                    }
                    (_, Tool::Move) => miniquad::window::set_mouse_cursor(CursorIcon::Move),
                    _ => miniquad::window::set_mouse_cursor(CursorIcon::Crosshair),
                }
            } else {
                miniquad::window::set_mouse_cursor(CursorIcon::Default);
            }
        }
        if let Some(buffer) = self.buffers.get_mut(self.current) {
            if frame_changed
                && !buffer.edit_all
                && self.mode.is_normal()
                && matches!(self.tool, Tool::Brush(..))
            {
                self.tracker.trim()
            }
            if let Mode::Normal { paste_info, .. } = &mut self.mode {
                if let Some(p) = paste_info {
                    let current_layer = buffer.current_layer_id();
                    let current_frame = buffer.current_frame_id();
                    if p.layer != current_layer || p.frame != current_frame {
                        *paste_info = None;
                    }
                }
            }
            if let Some(dir) = self.tracker.get_dir() {
                match &mut self.mode {
                    Mode::Visual { modifier, .. } => self.tool.visual_preview(
                        &self.tracker,
                        buffer,
                        self.draw_mode.1,
                        *modifier,
                        &self.tool_setting,
                    ),
                    Mode::Normal {
                        modifier,
                        paste_info: None,
                        ..
                    } => {
                        if let (Tool::Move, Some(r)) = (
                            &self.tool,
                            Register::new(buffer.image(), buffer.selection()),
                        ) {
                            let cut = crate::tool::cut(buffer.image(), buffer.selection());
                            let img_idx = buffer.session.insert_image(&cut);
                            tool::paste_preview(&r, buffer, &cut, dir, self.draw_mode.0);
                            self.mode = Mode::paste(
                                cut,
                                img_idx,
                                None,
                                r,
                                None,
                                None,
                                buffer.current_layer_id(),
                                buffer.current_frame_id(),
                            );
                        } else {
                            self.tool.normal_preview(
                                &self.tracker,
                                buffer,
                                self.draw_mode.0,
                                self.color,
                                *modifier,
                                &self.tool_setting,
                            );
                        }
                    }
                    Mode::Normal {
                        paste_info:
                            Some(PrePasteData {
                                image,
                                register,
                                offset,
                                ..
                            }),
                        ..
                    } => {
                        let offset = (offset.0 + dir.0, offset.1 + dir.1);
                        tool::paste_preview(register, buffer, image, offset, self.draw_mode.0);
                    }
                    _ => (),
                }
            }
        }
    }
    pub fn needs_update(&self) -> bool {
        #[cfg(all(feature = "lua", not(target_arch = "wasm32")))]
        if matches!(self.mode, Mode::Running(..)) {
            return true;
        }
        if let Some(buffer) = self.buffers.get(self.current) {
            !buffer.display_selection().is_empty()
                || (!buffer.animation.paused && buffer.num_frames() > 1)
        } else {
            false
        }
    }
}
