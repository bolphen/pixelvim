use super::*;

impl Engine {
    pub fn do_command(&mut self, cmd: Command) -> Result<(), String> {
        match cmd {
            Command::Quit { forced } => {
                if let Some(buffer) = self.buffers.get(self.current) {
                    if !forced && !buffer.is_saved() {
                        Err("No write since last change")?
                    } else {
                        let mut buffer = self.buffers.remove(self.current);
                        buffer.clean_swap();
                        self.dropped_buffer.push(buffer.id());
                        drop(buffer);
                        if !self.buffers.is_empty() {
                            self.current = self.current.saturating_sub(1);
                        }
                        self.mode.set_to_normal(None);
                    }
                }
                if self.buffers.is_empty() {
                    self.quit_requested = true;
                }
            }
            Command::QuitAll { forced } => {
                while let Some(buffer) = self.buffers.get(self.current) {
                    if !forced && !buffer.is_saved() {
                        Err("No write since last change")?
                    } else {
                        let mut buffer = self.buffers.remove(self.current);
                        buffer.clean_swap();
                        self.dropped_buffer.push(buffer.id());
                        drop(buffer);
                        if !self.buffers.is_empty() {
                            self.current = self.current.saturating_sub(1);
                        }
                        self.mode.set_to_normal(None);
                    }
                }
                if self.buffers.is_empty() {
                    self.quit_requested = true;
                }
            }
            Command::New(size) => {
                let size = size.map(|s| (s.0 as _, s.1 as _)).unwrap_or((32, 32));
                self.new_buffer_from_image(Image::new_with(size.0, size.1, Color(0, 0, 0, 0)));
                self.mode.set_to_normal(None);
            }
            Command::Help => {
                self.mode = Mode::Help(TextViewer::new(&format!(
                    "{}\nKEYMAP\n======\n{}",
                    include_str!("../../assets/help/help.txt"),
                    self.key_map.help()
                )))
            }
            // Command::Help(entry) => match entry.as_str() {
            //     "" => {
            //         self.mode =
            //             Mode::Help(TextViewer::new(include_str!("../../assets/help/help.txt")))
            //     }
            //     "keymap" => self.mode = Mode::Help(TextViewer::new(&self.key_map.help())),
            //     _ => self.mode.error(format!("No help found for {entry}")),
            // },
            Command::Buffer(name) => {
                if !name.is_empty() {
                    if let Some(i) = self
                        .buffers
                        .iter()
                        .position(|b| b.display_name().starts_with(&name))
                    {
                        self.current = i;
                        self.mode
                            .set_to_normal(Some(Message::normal("Switch to opened buffer")));
                    }
                }
            }
            Command::Edit(path) => {
                self.load_path(&path);
            }
            Command::Source(path) => {
                self.load_config_from_path(&path);
            }
            Command::RunScript(path, modifier) => {
                self.run_script_from_path(&path, modifier);
            }
            Command::Write { path, forced } => self.save_buffer(path, forced)?,
            Command::Resize(size) => {
                if let Some(buffer) = self.buffers.get_mut(self.current) {
                    if size != buffer.size() {
                        buffer.resize(size.0 as _, size.1 as _);
                        self.mode.message("Used resize");
                    }
                }
            }
            Command::Fit => {
                let screen = self.screen;
                let buffer = self.active_buffer_mut()?;
                buffer.fit_to_view(screen);
                self.mode.message("Fit image to view");
            }
            Command::Set(expr) => match self.settings.parse(&expr)? {
                Setting::Color(color) => {
                    match color {
                        crate::command::ColorOrIndex::Color(color) => self.color = color,
                        crate::command::ColorOrIndex::Index(i) => {
                            if let Some(color) = self.palette.get(i) {
                                self.color = *color
                            } else {
                                Err("Not found in the palette")?
                            }
                        }
                    }
                    self.mode.message(&format!("  color={}", self.color.hex()));
                }
                Setting::Background(color) => {
                    self.background = color;
                    self.mode
                        .message(&format!("  background={}", self.background.hex()));
                }
                Setting::VisualColor(color) => {
                    self.visual_color = color;
                    self.mode
                        .message(&format!("  visual/color={}", self.visual_color.hex()));
                }
                Setting::VisualAlpha(alpha) => {
                    let a = alpha.0.clamp(0., 1.);
                    self.visual_color = self.visual_color.alpha((a * 255.).round() as _);
                    self.mode.message(&format!("  visual/alpha={:.2}", a));
                }
                Setting::Scale(scale) => {
                    self.scale_ui = scale.0;
                    self.mode.message(&format!("  scale/ui={}", self.scale_ui));
                }
                Setting::Toggleable(Toggleable::Fullscreen, value) => {
                    if self.fullscreen != value.0 {
                        miniquad::window::set_fullscreen(value.0);
                        self.fullscreen = value.0;
                    }
                    self.mode
                        .message(&format!("  fullscreen={}", self.fullscreen));
                }
                Setting::Toggleable(Toggleable::Checker, value) => {
                    self.checker = value.0;
                    self.mode.message(&format!("  checker={}", self.checker));
                }
                Setting::Toggleable(Toggleable::Palette, value) => {
                    self.show_palette = value.0;
                    self.mode
                        .message(&format!("  palette={}", self.show_palette));
                }
                Setting::Toggleable(Toggleable::XSym, value) => {
                    let session = &mut self.active_buffer_mut()?.session;
                    session.symmetry.x = value.0;
                }
                Setting::Toggleable(Toggleable::YSym, value) => {
                    let session = &mut self.active_buffer_mut()?.session;
                    session.symmetry.y = value.0;
                }
                Setting::XSymOffset(offset) => {
                    let session = &mut self.active_buffer_mut()?.session;
                    session.symmetry.x = true;
                    session.symmetry.x_offset = offset.0;
                }
                Setting::YSymOffset(offset) => {
                    let session = &mut self.active_buffer_mut()?.session;
                    session.symmetry.y = true;
                    session.symmetry.y_offset = offset.0;
                }
                Setting::Toggleable(Toggleable::Grid, value) => {
                    let session = &mut self.active_buffer_mut()?.session;
                    session.grid.on = value.0;
                }
                Setting::GridSize(size) => {
                    let session = &mut self.active_buffer_mut()?.session;
                    session.grid.on = true;
                    session.grid.size = (size.0, size.1);
                }
                Setting::GridColor(color) => {
                    let session = &mut self.active_buffer_mut()?.session;
                    session.grid.on = true;
                    session.grid.color = color;
                }
                Setting::Toggleable(Toggleable::Srgb, value) => {
                    self.toggle_srgb(Some(value.0));
                    self.mode.message(&format!("  srgb={}", self.srgb));
                }
                Setting::Toggleable(Toggleable::Debug, value) => {
                    self.debug = value.0;
                    self.mode.message(&format!("  debug={}", self.debug));
                }
                Setting::BrushSize(size) => match &mut self.tool_setting.brush {
                    Brush::Rect(s) => *s = size.0.clamp(1, 255) as _,
                    Brush::Round(s) => *s = size.0.clamp(1, 255) as _,
                    _ => (),
                },
                Setting::BrushShape(shape) => {
                    let selection = self.active_buffer_mut().ok().and_then(|b| {
                        Register::new(b.image(), b.selection()).map(|r| {
                            crate::algo::r#move(
                                &r.selection,
                                (-(r.width() as i32) / 2, -(r.height() as i32) / 2),
                            )
                        })
                    });
                    if self.tool_setting.brush.set(&shape, selection).is_err() {
                        self.mode.error("Unknown brush shape")
                    }
                }
                Setting::FloodTolerance(value) => {
                    let value = value.0.clamp(0, 255) as _;
                    self.tool_setting.flood_tolerance = value;
                    self.mode.message(&format!("  flood/tolerance={}", value));
                }
                Setting::AnimationSpeed(speed) => {
                    let buffer = &mut self.active_buffer_mut()?;
                    buffer.animation.speed = speed.0.clamp(0.01, 16.);
                }
                Setting::Toggleable(Toggleable::Animation, value) => {
                    let buffer = &mut self.active_buffer_mut()?;
                    buffer.animation.paused = !value.0;
                    self.time.update();
                }
                Setting::Toggleable(Toggleable::AnimationEditAll, value) => {
                    let buffer = &mut self.active_buffer_mut()?;
                    buffer.edit_all = value.0;
                }
                Setting::Toggleable(Toggleable::AnimationStrip, value) => {
                    self.display.set_strip(value.0);
                    self.mode.message(&format!("  animation/strip={}", value.0));
                }
                Setting::Toggleable(Toggleable::Tile, value) => {
                    self.display.set_tile(value.0);
                    self.mode.message(&format!("  tile={}", value.0));
                }
            },
            Command::Toggle(expr) => match self.settings.parse_toggleable(&expr)? {
                Setting::Toggleable(t, _) => match t {
                    Toggleable::Fullscreen => {
                        self.fullscreen = !self.fullscreen;
                        miniquad::window::set_fullscreen(self.fullscreen);
                        self.mode
                            .message(&format!("  fullscreen={}", self.fullscreen));
                    }
                    Toggleable::Checker => {
                        self.checker = !self.checker;
                        self.mode.message(&format!("  checker={}", self.checker));
                    }
                    Toggleable::Palette => {
                        self.show_palette = !self.show_palette;
                        self.mode
                            .message(&format!("  palette={}", self.show_palette));
                    }
                    Toggleable::XSym => {
                        let session = &mut self.active_buffer_mut()?.session;
                        session.symmetry.x = !session.symmetry.x;
                    }
                    Toggleable::YSym => {
                        let session = &mut self.active_buffer_mut()?.session;
                        session.symmetry.y = !session.symmetry.y;
                    }
                    Toggleable::Grid => {
                        let session = &mut self.active_buffer_mut()?.session;
                        session.grid.on = !session.grid.on;
                    }
                    Toggleable::Srgb => {
                        self.toggle_srgb(None);
                        self.mode.message(&format!("  srgb={}", self.srgb));
                    }
                    Toggleable::Animation => {
                        let buffer = &mut self.active_buffer_mut()?;
                        if buffer.num_frames() > 1 {
                            buffer.animation.paused = !buffer.animation.paused;
                            if buffer.animation.paused {
                                self.mode.message("Paused animation");
                            } else {
                                self.mode.message("Playing animation");
                            }
                        }
                        self.time.update();
                    }
                    Toggleable::AnimationEditAll => {
                        let buffer = &mut self.active_buffer_mut()?;
                        buffer.edit_all = !buffer.edit_all;
                    }
                    Toggleable::AnimationStrip => {
                        let value = self.display.toggle_strip();
                        self.mode.message(&format!("  animation/strip={}", value));
                    }
                    Toggleable::Tile => {
                        let value = self.display.toggle_tile();
                        self.mode.message(&format!("  tile={}", value));
                    }
                    Toggleable::Debug => {
                        self.debug = !self.debug;
                        self.mode.message(&format!("  debug={}", self.debug));
                    }
                },
                _ => self.mode.error("Setting cannot be toggled"),
            },
            Command::Map(map) => {
                self.key_map.normal_map.insert(map.key, map.actions.clone());
                self.key_map.visual_map.insert(map.key, map.actions);
            }
            Command::MapNormal(map) => {
                self.key_map.normal_map.insert(map.key, map.actions);
            }
            Command::MapVisual(map) => {
                self.key_map.visual_map.insert(map.key, map.actions);
            }
            Command::Undo(count) | Command::Redo(count) => {
                let buffer = self.active_buffer_mut()?;
                let (msg, p) = if matches!(cmd, Command::Undo(..)) {
                    buffer.undo_redo(-count.0)
                } else {
                    buffer.undo_redo(count.0)
                };
                if let Some(p) = p {
                    let current_layer = buffer.current_layer_id();
                    let current_frame = buffer.current_frame_id();
                    self.tool = Tool::Move;
                    self.mode = Mode::paste(
                        p.image,
                        p.img_idx,
                        Some(Message::normal(&msg)),
                        p.register,
                        Some(p.reg_idx),
                        Some(p.offset),
                        current_layer,
                        current_frame,
                    );
                } else {
                    self.mode.reset(Some(Message::normal(&msg)));
                }
            }
            Command::Crop => {
                let buffer = self.active_buffer_mut()?;
                if let Some(r) = Register::new(buffer.image(), buffer.selection()) {
                    buffer.crop(r.width(), r.height(), r.offset);
                } else {
                    self.mode.error("Empty selection");
                }
            }
            Command::Reduce => {
                let buffer = self
                    .buffers
                    .get_mut(self.current)
                    .ok_or("No active buffer")?;
                if self.palette.is_empty() {
                    self.mode.error("Palette is empty");
                } else {
                    let mut reduced = HashMap::new();
                    let current = buffer.session.current_images();
                    for (l, layer) in current.iter().enumerate() {
                        for (f, frame) in layer.iter().enumerate() {
                            crate::tool::reduce(frame, buffer.selection(), &self.palette)
                                .map(|new| reduced.insert((l, f), new));
                        }
                    }
                    match buffer.batch_edit(Some("reduce"), reduced, None, false) {
                        Ok(_) => self.mode.message("Used reduce"),
                        Err(EditError::Unchanged) => self.mode.message("Already reduced"),
                        _ => (),
                    }
                }
            }
            Command::Quantize(count) => {
                let buffer = self.active_buffer_mut()?;
                if count.0 < 1 {
                    self.mode.error("Wrong argument");
                } else if buffer.image().colors().len() <= count.0 as usize {
                    self.mode.message("Already quantized");
                } else if buffer
                    .edit(
                        Some("quantize"),
                        |i, _s, _sym| crate::tool::quantize(i, count.0 as _),
                        false,
                    )
                    .is_ok()
                {
                    self.mode.message("Used quantize")
                }
            }
            Command::LayerGoAbove(count) => {
                let buffer = self.active_buffer_mut()?;
                buffer.goto_layer_rel(count.0);
            }
            Command::LayerGoBelow(count) => {
                let buffer = self.active_buffer_mut()?;
                buffer.goto_layer_rel(-count.0);
            }
            Command::LayerNewAbove(count) => {
                let buffer = self.active_buffer_mut()?;
                if count.0 > 0 {
                    buffer.new_layer(buffer.session.current_layer + 1, count.0 as _);
                    self.mode.reset(Some(if count.0 == 1 {
                        Message::normal("Created layer")
                    } else {
                        Message::normal(&format!("Created {} layers", count.0))
                    }));
                }
            }
            Command::LayerNewBelow(count) => {
                let buffer = self.active_buffer_mut()?;
                if count.0 > 0 {
                    buffer.new_layer(buffer.session.current_layer, count.0 as _);
                    self.mode.reset(Some(if count.0 == 1 {
                        Message::normal("Created layer")
                    } else {
                        Message::normal(&format!("Created {} layers", count.0))
                    }));
                }
            }
            Command::LayerMergeDown(count) => {
                let srgb = self.srgb;
                let buffer = self.active_buffer_mut()?;
                if count.0 > 0 {
                    match buffer.merge_layer_down(buffer.session.current_layer, count.0 as _, srgb)
                    {
                        Ok(count) => self.mode.reset(Some(Message::normal(&format!(
                            "Merged {} layers",
                            count + 1
                        )))),
                        _ => self.mode.reset(Some(Message::error("Cannot merge layer"))),
                    }
                }
            }
            Command::LayerDelete => {
                let buffer = self.active_buffer_mut()?;
                buffer.delete_layer(buffer.session.current_layer);
                self.mode
                    .reset(Some(Message::normal("Deleted current layer")));
            }
            Command::LayerToggle => {
                let buffer = self.active_buffer_mut()?;
                buffer.toggle_visibility(buffer.session.current_layer, None);
            }
            Command::FrameGoLeft(count) => {
                let buffer = self.active_buffer_mut()?;
                buffer.goto_frame_rel(-count.0);
            }
            Command::FrameGoRight(count) => {
                let buffer = self.active_buffer_mut()?;
                buffer.goto_frame_rel(count.0);
            }
            Command::FrameNewLeft(count) => {
                let buffer = self.active_buffer_mut()?;
                if count.0 > 0 {
                    buffer.new_frame(buffer.session.current_frame, count.0 as _);
                    self.mode.reset(Some(if count.0 == 1 {
                        Message::normal("Inserted frame")
                    } else {
                        Message::normal(&format!("Inserted {} frames", count.0))
                    }));
                }
            }
            Command::FrameNewRight(count) => {
                let buffer = self.active_buffer_mut()?;
                if count.0 > 0 {
                    buffer.new_frame(buffer.session.current_frame + 1, count.0 as _);
                    self.mode.reset(Some(if count.0 == 1 {
                        Message::normal("Inserted frame")
                    } else {
                        Message::normal(&format!("Inserted {} frames", count.0))
                    }));
                }
            }
            Command::FrameDuplicate(count) => {
                let buffer = self.active_buffer_mut()?;
                if count.0 > 0 {
                    buffer.duplicate_frame(buffer.session.current_frame, count.0 as _);
                    self.mode.reset(Some(if count.0 == 1 {
                        Message::normal("Duplicate current frame")
                    } else {
                        Message::normal(&format!("Duplicate current frame {} times", count.0))
                    }));
                }
            }
            Command::FrameDelete => {
                let buffer = self.active_buffer_mut()?;
                buffer.delete_frame(buffer.session.current_frame);
                self.mode
                    .reset(Some(Message::normal("Deleted current frame")));
            }
            Command::Slice(count) => {
                let buffer = self.active_buffer_mut()?;
                if buffer.num_frames() == 1 {
                    if count.0 > 1 {
                        buffer.slice(count.0 as _);
                        self.mode.reset(Some(Message::normal(&format!(
                            "Sliced into {} frames",
                            count.0
                        ))));
                    } else {
                        self.mode.error("Slice number needs to be > 1");
                    }
                } else {
                    self.mode
                        .error("Workspace already contains multiple frames");
                }
            }
            Command::SelectAll => {
                let buffer = self.active_buffer_mut()?;
                buffer.edit_selection("select all", |i, _s, _sym| {
                    let mut selection = Selection::new();
                    for x in 0..i.width() as i32 {
                        for y in 0..i.height() as i32 {
                            selection.insert((x, y));
                        }
                    }
                    selection
                });
                self.mode.reset(Some(Message::normal("Used select all")));
            }
            Command::SelectInvert => {
                let buffer = self.active_buffer_mut()?;
                buffer.edit_selection("invert selection", |i, s, _sym| {
                    let mut selection = Selection::new();
                    for x in 0..i.width() as i32 {
                        for y in 0..i.height() as i32 {
                            if !s.contains(&(x, y)) {
                                selection.insert((x, y));
                            }
                        }
                    }
                    selection
                });
                self.mode
                    .reset(Some(Message::normal("Used invert selection")));
            }
            Command::SelectClear => {
                let buffer = self.active_buffer_mut()?;
                if !buffer.selection().is_empty() {
                    buffer.edit_selection("clear selection", |_i, _s, _sym| Selection::new());
                    self.mode
                        .set_to_normal(Some(Message::normal("Used clear selection")));
                }
            }
            Command::PaletteAdd(color) => {
                let color = color.unwrap_or(self.color);
                let mut msg = "Color added to palette".to_string();
                if let Some(i) = self.palette.iter().position(|c| *c == color) {
                    msg.push_str(&format!(" (duplicate of ${i})"));
                }
                self.palette.push(color);
                self.mode.message(&msg);
            }
            Command::PaletteDelete(color) => match color {
                crate::command::ColorOrIndex::Color(color) => {
                    if let Some(i) = self.palette.iter().position(|c| *c == color) {
                        self.palette.remove(i);
                        self.mode.message("Color deleted from palette");
                    } else {
                        self.mode.error("Color not in palette");
                    }
                }
                crate::command::ColorOrIndex::Index(index) => {
                    if index < self.palette.len() {
                        self.palette.remove(index);
                        self.mode.message("Color deleted from palette");
                    } else {
                        self.mode.error("Index not valid");
                    }
                }
            },
            Command::PaletteClear => {
                self.palette.clear();
                self.mode.message("Palette cleared");
            }
            Command::PaletteSort => self.palette.sort(),
            Command::PaletteBuild => {
                // for consistency with quantize this only uses the current layer and frame
                let buffer = self.active_buffer_mut()?;
                let mut colors = HashSet::from([(0, 0, 0, 0).into()]); // always include transparent
                colors.extend(buffer.image().colors().iter().filter(|c| c.3 != 0));
                let mut colors: Vec<_> = colors.into_iter().collect();
                colors.sort();
                self.palette.clear();
                for color in colors {
                    self.palette.push(color);
                }
                self.mode.message("Build palette from current image");
            }
            Command::PaletteGradient(color1, color2, count) => {
                if count.0 >= 2 {
                    for color in
                        crate::color::gradient(color1, color2, count.0 as u32 - 2, self.srgb)
                    {
                        self.palette.push(color);
                    }
                }
            }
            Command::FlipHorizontal => {
                let buffer = self
                    .buffers
                    .get_mut(self.current)
                    .ok_or("No active buffer")?;
                if self.mode.is_visual() {
                    buffer.edit_selection("horizontal flip selection", |i, s, sym| {
                        crate::algo::flip_x(i, s, sym)
                    });
                    self.mode.message("Used horizontal flip selection");
                } else {
                    let paste_info = self.mode.take_paste_info();
                    if let Some((mut register, image, img_idx)) =
                        if let Some(paste_info) = paste_info {
                            let mut register = paste_info.register;
                            register.offset.0 += paste_info.offset.0;
                            register.offset.1 += paste_info.offset.1;
                            Some((register.flip_x(), paste_info.image, paste_info.img_idx))
                        } else if let Some(r) = Register::new(buffer.image(), buffer.selection()) {
                            let cut = crate::tool::cut(buffer.image(), buffer.selection());
                            let img_idx = buffer.session.insert_image(&cut);
                            Some((r.flip_x(), cut, img_idx))
                        } else {
                            None
                        }
                    {
                        buffer.animation.paused = true;
                        let sym = buffer.symmetry();
                        if sym.x {
                            let offset = (
                                buffer.width() as i32 - register.offset.0 - register.width() as i32
                                    + sym.x_offset,
                                register.offset.1,
                            );
                            register = register.offset(offset);
                        }
                        let reg_idx = tool::paste(
                            &register,
                            PasteFrom::Register(&register),
                            buffer,
                            Some(img_idx),
                            None,
                            self.draw_mode.0,
                        )
                        .2;
                        self.mode = Mode::paste(
                            image,
                            img_idx,
                            Some(Message::normal("Used horizontal flip")),
                            register,
                            Some(reg_idx),
                            None,
                            buffer.current_layer_id(),
                            buffer.current_frame_id(),
                        );
                    } else {
                        if !buffer.edit_all {
                            buffer.animation.paused = true;
                        }
                        buffer.edit_infallible(
                            "horizontal flip",
                            |i, s, sym| crate::tool::flip_x(i, s, sym).0,
                            false,
                        );
                        self.mode.message("Used horizontal flip");
                    }
                }
            }
            Command::FlipVertical => {
                let buffer = self
                    .buffers
                    .get_mut(self.current)
                    .ok_or("No active buffer")?;
                if self.mode.is_visual() {
                    buffer.edit_selection("vertical flip selection", |i, s, sym| {
                        crate::algo::flip_y(i, s, sym)
                    });
                    self.mode.message("Used vertical flip selection");
                } else {
                    let paste_info = self.mode.take_paste_info();
                    if let Some((mut register, image, img_idx)) =
                        if let Some(paste_info) = paste_info {
                            let mut register = paste_info.register;
                            register.offset.0 += paste_info.offset.0;
                            register.offset.1 += paste_info.offset.1;
                            Some((register.flip_y(), paste_info.image, paste_info.img_idx))
                        } else if let Some(r) = Register::new(buffer.image(), buffer.selection()) {
                            let cut = crate::tool::cut(buffer.image(), buffer.selection());
                            let img_idx = buffer.session.insert_image(&cut);
                            Some((r.flip_y(), cut, img_idx))
                        } else {
                            None
                        }
                    {
                        buffer.animation.paused = true;
                        let sym = buffer.symmetry();
                        if sym.y {
                            let offset = (
                                register.offset.0,
                                buffer.height() as i32
                                    - register.offset.1
                                    - register.height() as i32
                                    + sym.y_offset,
                            );
                            register = register.offset(offset);
                        }
                        let reg_idx = tool::paste(
                            &register,
                            PasteFrom::Register(&register),
                            buffer,
                            Some(img_idx),
                            None,
                            self.draw_mode.0,
                        )
                        .2;
                        self.mode = Mode::paste(
                            image,
                            img_idx,
                            Some(Message::normal("Used vertical flip")),
                            register,
                            Some(reg_idx),
                            None,
                            buffer.current_layer_id(),
                            buffer.current_frame_id(),
                        );
                    } else {
                        if !buffer.edit_all {
                            buffer.animation.paused = true;
                        }
                        buffer.edit_infallible(
                            "vertical flip",
                            |i, s, sym| crate::tool::flip_y(i, s, sym).0,
                            false,
                        );
                        self.mode.message("Used vertical flip");
                    }
                }
            }
            Command::Insert => {
                let buffer = self
                    .buffers
                    .get_mut(self.current)
                    .ok_or("No active buffer")?;
                if let Some(cursor) = buffer.cursor() {
                    self.tracker.start(cursor);
                    if self.mode.is_visual() {
                        self.tool.visual_use(
                            &self.tracker,
                            buffer,
                            self.draw_mode.1,
                            self.mode.take_modifier(),
                            &self.tool_setting,
                        );
                    } else if !matches!(self.tool, Tool::Move) {
                        self.tool.normal_preview(
                            &self.tracker,
                            buffer,
                            self.draw_mode.0,
                            self.color,
                            self.mode.take_modifier(),
                            &self.tool_setting,
                        );
                        buffer.commit_temporary(self.tool.string_short());
                    }
                    self.tracker.stop();
                }
            }
            Command::Paste => {
                if let Some(buf) = miniquad::window::clipboard_get_image() {
                    if let Ok(image) = crate::format::load_png(&buf[..]) {
                        let buffer = self.new_buffer_from_image(image);
                        buffer.session.mark_unsaved();
                        self.mode
                            .reset(Some(Message::normal("Pasted from clipboard")));
                    }
                } else if let Some(path) = miniquad::window::clipboard_get() {
                    self.load_path(&path.into());
                } else {
                    self.mode.error("Cannot paste from clipboard");
                }
            }
        }
        Ok(())
    }
}
