use super::{DisplayMode, Engine, Mode, Modifier, Tool};

use crate::color::Color;
use crate::console::Console;
use crate::graphics::Graphics;
use crate::utils::Rect;

impl Engine {
    pub fn render_ui(&self, console: &mut Console) {
        let w = console.width() as i32;
        let h = console.height() as i32;

        if let Mode::Help(view) = &self.mode {
            let help_string = format!("pixelvim v{}", crate::VERSION);
            console.print(&help_string, 1, 0, Color::WHITE.into(), None);
            for (i, line) in view.lines().iter().enumerate() {
                if i as i32 >= view.scroll {
                    console.print(
                        line,
                        1,
                        i as i32 + 2 - view.scroll,
                        Color::LIGHTGRAY.into(),
                        None,
                    );
                }
            }
        } else {
            if self.debug {
                let mem = crate::ALLOCATOR.allocated();
                let mem_info = format!("{}MB {}KB", mem / (1024 * 1024), mem / 1024 % (1024),);
                console.print(&mem_info, 6, 2, Color::WHITE.into(), None);
            }

            if self.show_palette || self.picker {
                if self.palette.is_empty() {
                    console.print(
                        "empty palette",
                        self.ui.palette.from_left,
                        h - 1 - self.ui.palette.from_bottom,
                        Color::LIGHTGRAY.into(),
                        self.background.into(),
                    );
                } else {
                    let n_rows = self.palette.len().div_ceil(self.ui.palette.each_row) as i32;
                    for (i, c) in self.palette.iter().enumerate() {
                        console.print(
                            "    ",
                            4 * (i % self.ui.palette.each_row) as i32 + self.ui.palette.from_left,
                            (i / self.ui.palette.each_row) as i32 + h
                                - n_rows
                                - self.ui.palette.from_bottom,
                            c.marker().into(),
                            (*c).into(),
                        );
                    }
                }
            }
            if self.buffers.len() > 1 {
                let mut x = 0;
                for (i, b) in self.buffers.iter().enumerate() {
                    let title =
                        format!(" {}{} ", b.title(), if b.is_saved() { "" } else { " [+]" });
                    console.print(
                        &title,
                        x,
                        0,
                        if i == self.current {
                            Color::WHITE
                        } else {
                            Color::GRAY
                        }
                        .into(),
                        if i == self.current {
                            Color::GRAY
                        } else {
                            self.background
                        }
                        .into(),
                    );
                    x += title.chars().count() as i32;
                }
            }
            self.color.print(
                console,
                1,
                h - 3,
                Color::WHITE,
                self.background,
                &self
                    .palette
                    .iter()
                    .position(|c| *c == self.color)
                    .map(|i| format!("{i:3} "))
                    .unwrap_or("    ".into()),
            );
            let mut tool_string = String::new();
            if let Some((color, index)) = self.mouse_on_palette() {
                color.print(
                    console,
                    1,
                    h - 4,
                    Color::WHITE,
                    self.background,
                    &format!("{index:3} "),
                );
            } else if self.picker {
                if let Some(buffer) = self.buffers.get(self.current) {
                    if let Some((_, color)) = buffer.cursor_color() {
                        color.print(
                            console,
                            1,
                            h - 4,
                            Color::WHITE,
                            self.background,
                            &self
                                .palette
                                .iter()
                                .position(|c| *c == color)
                                .map(|i| format!("{i:3} "))
                                .unwrap_or("    ".into()),
                        );
                    } else {
                        console.print(
                            "picker",
                            1,
                            h - 4,
                            Color::WHITE.into(),
                            self.background.into(),
                        );
                    }
                }
            } else {
                tool_string.push_str(&self.tool_string());
                if !matches!(self.tool, Tool::Move) {
                    if let Some(buffer) = self.buffers.get(self.current) {
                        if buffer.symmetry().x {
                            tool_string.push_str(" \u{2190}\u{2192}");
                        }
                        if buffer.symmetry().y {
                            tool_string.push_str(" \u{2195}");
                        }
                    }
                }
                console.print(
                    &tool_string,
                    1,
                    h - 4,
                    Color::WHITE.into(),
                    self.background.into(),
                );
            }
            if let Some(buffer) = self.buffers.get(self.current) {
                let mut left_info = String::new();
                left_info.push_str(buffer.display_name());
                if !buffer.is_saved() {
                    left_info.push_str(" [+]")
                }
                console.print(
                    &left_info,
                    1,
                    h - 2,
                    Color::WHITE.into(),
                    self.background.into(),
                );
                let mut right_info = String::new();
                if let (Tool::Move, Some(dir)) = (&self.tool, self.tracker.get_dir()) {
                    right_info.push_str(&format!("(move {} {}) ", dir.0, dir.1));
                }
                if let Some((x, y)) = buffer.cursor() {
                    right_info.push_str(&format!("{} {}", x + 1, y + 1));
                } else {
                    right_info.push('-');
                }
                right_info.push_str(&format!(" [{} {}]", buffer.width(), buffer.height()));
                console.print(
                    &right_info,
                    w - 1 - right_info.len() as i32,
                    h - 2,
                    Color::WHITE.into(),
                    self.background.into(),
                );
                let mut timeline_info = String::new();
                if buffer.edit_all {
                    timeline_info.push_str("all");
                } else if buffer.num_frames() > 1 && !buffer.animation.paused {
                    timeline_info.push_str("one");
                }
                console.print(
                    &timeline_info,
                    w - 1 - timeline_info.len() as i32,
                    h - 3,
                    Color::WHITE.into(),
                    self.background.into(),
                );
                let len = buffer.num_layers().to_string().len();
                let visibility = buffer.get_visibility();
                let mouse = self.mouse_on_layer().map(|(l, _)| l);
                for (i, vis) in visibility.into_iter().enumerate() {
                    let si = i.to_string();
                    let y = h - i as i32 - self.ui.layers.from_bottom - 1;
                    console.print(
                        &format!(
                            " {} {}{i} {} ",
                            if vis { "<>" } else { "--" },
                            " ".repeat(len - si.len()),
                            "\u{2219}".repeat(buffer.num_frames()),
                        ),
                        w - 6 - (len + buffer.num_frames()) as i32 - self.ui.layers.from_right,
                        y,
                        if i == buffer.session.current_layer {
                            Color::DARKGRAY
                        } else {
                            Color::WHITE
                        }
                        .into(),
                        if i == buffer.session.current_layer {
                            Color::LIGHTGRAY
                        } else if mouse.is_some_and(|l| l == i) {
                            Color::GRAY
                        } else {
                            self.background
                        }
                        .into(),
                    );
                    // if i == buffer.session.current_layer {
                    console.print(
                        "\u{2219}",
                        w - 1 + buffer.session.current_frame as i32
                            - (buffer.num_frames()) as i32
                            - self.ui.layers.from_right,
                        y,
                        if i == buffer.session.current_layer {
                            Color::WHITE
                        } else {
                            Color::DARKGRAY
                        }
                        .into(),
                        if i == buffer.session.current_layer {
                            Color::DARKGRAY
                        } else {
                            Color::LIGHTGRAY
                        }
                        .into(),
                    );
                    // }
                }
            } else {
                console.print(
                    "[No Buffer]",
                    1,
                    h - 2,
                    Color::WHITE.into(),
                    self.background.into(),
                );
                let s = "type :help<Enter> for help";
                console.print(
                    &s[..10],
                    (w - s.len() as i32) / 2,
                    h / 2,
                    Color::LIGHTGRAY.into(),
                    None,
                );
                console.print(
                    &s[10..17],
                    (w - s.len() as i32) / 2 + 10,
                    h / 2,
                    Color::GRAY.into(),
                    None,
                );
                console.print(
                    &s[17..],
                    (w - s.len() as i32) / 2 + 17,
                    h / 2,
                    Color::LIGHTGRAY.into(),
                    None,
                );
            }
            match &self.mode {
                Mode::Command(input, _) => {
                    console.print(":", 1, h - 1, Color::WHITE.into(), self.background.into());
                    console.print(
                        input.text(),
                        2,
                        h - 1,
                        Color::WHITE.into(),
                        self.background.into(),
                    );
                    if let Some(c) = &input.completions {
                        let len = (c.2.len()).min(h as usize - 2);
                        let high = c.1 + (c.2.len() - c.1).min(len);
                        let low = high - len.min(high);
                        let w = (low..high).map(|i| c.2[i].len()).max().unwrap();
                        for i in low..high {
                            let (fg, bg) = if i == c.1 {
                                (self.background, Color::WHITE)
                            } else {
                                (Color::LIGHTGRAY, Color::GRAY)
                            };
                            let x = 1 + c.0.start as i32;
                            let y = h - 2 - (len + low - 1 - i) as i32;
                            if let Some(color) = c.2[i].2 {
                                console.print(
                                    &format!(
                                        " {}{}",
                                        c.2[i].0,
                                        " ".repeat(w + 1 - c.2[i].0.chars().count())
                                    ),
                                    x,
                                    y,
                                    fg.into(),
                                    bg.into(),
                                );
                                console.print("    ", x + w as i32 - 3, y, None, color.into());
                            } else if let Some(help) = &c.2[i].1 {
                                console.print(
                                    &format!(
                                        " {} ({}){}",
                                        c.2[i].0,
                                        help,
                                        " ".repeat(
                                            w - 2 - c.2[i].0.chars().count() - help.chars().count()
                                        )
                                    ),
                                    x,
                                    y,
                                    fg.into(),
                                    bg.into(),
                                );
                            } else {
                                console.print(
                                    &format!(
                                        " {}{}",
                                        c.2[i].0,
                                        " ".repeat(w + 1 - c.2[i].0.chars().count())
                                    ),
                                    x,
                                    y,
                                    fg.into(),
                                    bg.into(),
                                );
                            }
                        }
                    }
                    console
                        .get_mut(input.cursor() as i32 + 2, h - 1)
                        .map(|t| t.2 = Color::GRAY);
                }
                Mode::Normal {
                    message, modifier, ..
                }
                | Mode::Visual {
                    message, modifier, ..
                } => {
                    if let Some(msg) = message {
                        for (i, l) in msg.text.lines().rev().enumerate() {
                            console.print(
                                l,
                                1,
                                h - i as i32 - 1,
                                msg.color().into(),
                                self.background.into(),
                            );
                        }
                    }
                    match modifier {
                        Some(Modifier::Num(n)) => {
                            let s = n.to_string();
                            console.print(
                                &s,
                                w - 1 - s.len() as i32,
                                h - 1,
                                Color::WHITE.into(),
                                self.background.into(),
                            );
                        }
                        Some(Modifier::Go(None)) => {
                            console.print(
                                "g",
                                w - 2,
                                h - 1,
                                Color::WHITE.into(),
                                self.background.into(),
                            );
                        }
                        Some(Modifier::Go(Some(n))) => {
                            let s = n.to_string();
                            console.print(
                                &format!("{s}g"),
                                w - 2 - s.len() as i32,
                                h - 1,
                                Color::WHITE.into(),
                                self.background.into(),
                            );
                        }
                        _ => (),
                    }
                }
                #[cfg(all(feature = "lua", not(target_arch = "wasm32")))]
                Mode::Running(..) => {
                    console.print(
                        "Script running (cancel with Ctrl-C)",
                        1,
                        h - 1,
                        Color::ORANGE.into(),
                        self.background.into(),
                    );
                }
                _ => (),
            }
        }
    }
    pub fn render_texture(&mut self, graphics: &mut Graphics) {
        graphics.clear_screen(self.background);
        if let Some(buffer) = self.buffers.get_mut(self.current) {
            let scale = buffer.scale();
            buffer.texture_update(graphics);
            let margin = graphics.scale();
            let viewport = buffer.viewport;
            let (x, y, w, h) = viewport.get();
            let outline_color = match self.mode {
                Mode::Normal { .. } => Color::LIGHTGRAY,
                Mode::Visual { .. } => self.visual_color.alpha(0xff),
                Mode::Command(..) => self.background,
                _ => self.background,
            };
            let main_color = match self.mode {
                Mode::Visual { .. } => Color::ORANGE,
                _ => self.color,
            }
            .into();
            match (&self.display, buffer.animation.paused) {
                (DisplayMode::Tile, _) => {
                    if self.checker {
                        graphics.draw_checker(
                            Rect::new(x - w, y - h, 3. * w, 3. * h),
                            (1. / 3., 1. / 3.),
                            Color::GRAY,
                            Color::DARKGRAY,
                        );
                    }
                    let frame_id = buffer.frames()[buffer.session.current_frame];
                    for i in -1..=1 {
                        for j in -1..=1 {
                            graphics.draw_buffer_texture(
                                buffer.id(),
                                frame_id,
                                Rect::new(x + i as f32 * w, y + j as f32 * h, w, h),
                                None,
                            );
                        }
                    }
                }
                (DisplayMode::Expand(frames), paused) => {
                    let frames = *frames && paused;
                    if frames {
                        let current = buffer.session.current_frame as f32;
                        let m = buffer.num_frames() as f32;
                        if self.checker {
                            graphics.draw_checker(
                                Rect::new(x - current * w, y, w * m, h),
                                (current / m, 0.),
                                Color::GRAY,
                                Color::DARKGRAY,
                            );
                        }
                        for (f, id) in buffer.frames().iter().enumerate() {
                            graphics.draw_buffer_texture(
                                buffer.id(),
                                *id,
                                Rect::new(x + (f as f32 - current) * w, y, w, h),
                                None,
                            );
                        }
                    } else {
                        if self.checker {
                            graphics.draw_checker(viewport, (0., 0.), Color::GRAY, Color::DARKGRAY);
                        }
                        let frame_id = buffer.frames()[buffer.session.current_frame];
                        graphics.draw_buffer_texture(buffer.id(), frame_id, viewport, None);
                    }
                }
            }
            graphics.draw_rect_outline_fancy(viewport, margin, outline_color.into());
            if buffer.grid().on {
                let size = (
                    buffer.grid().size.0 as f32 * scale,
                    buffer.grid().size.1 as f32 * scale,
                );
                graphics.draw_grid(viewport, buffer.grid().color, size);
            }
            if buffer.symmetry().x {
                let offset = (w + buffer.symmetry().x_offset as f32 * scale) / 2.;
                graphics.draw_rect_filled(Rect::new(x + offset - 1., y, 2., h), Color::BLACK.into())
            }
            if buffer.symmetry().y {
                let offset = (h + buffer.symmetry().y_offset as f32 * scale) / 2.;
                graphics.draw_rect_filled(Rect::new(x, y + offset - 1., w, 2.), Color::BLACK.into())
            }
            let time = self.time.elapsed_abs() as f32 / 1000.;
            if self.mode.is_visual() {
                graphics.draw_selection(buffer.id(), viewport, self.visual_color, time)
            } else {
                graphics.draw_selection(buffer.id(), viewport, Color(0, 0, 0, 0), time)
            }
            if let Some(cursor) = buffer.cursor() {
                if !(matches!(self.tool, Tool::Move) || matches!(self.mode, Mode::Help(..))) {
                    let (width, height) = (buffer.width() as _, buffer.height() as _);
                    let sx = width - 1 + buffer.symmetry().x_offset - cursor.0;
                    let sy = height - 1 + buffer.symmetry().y_offset - cursor.1;
                    let sx_in_bound = 0 <= sx && sx < width;
                    let sy_in_bound = 0 <= sy && sy < height;
                    if buffer.symmetry().x && sx_in_bound {
                        graphics.draw_rect_outline(
                            Rect::new(
                                x + sx as f32 * scale,
                                y + cursor.1 as f32 * scale,
                                scale,
                                scale,
                            ),
                            margin,
                            main_color,
                        );
                    }
                    if buffer.symmetry().y && sy_in_bound {
                        graphics.draw_rect_outline(
                            Rect::new(
                                x + cursor.0 as f32 * scale,
                                y + sy as f32 * scale,
                                scale,
                                scale,
                            ),
                            margin,
                            main_color,
                        );
                    }
                    if buffer.symmetry().x && buffer.symmetry().y && sx_in_bound && sy_in_bound {
                        graphics.draw_rect_outline(
                            Rect::new(x + sx as f32 * scale, y + sy as f32 * scale, scale, scale),
                            margin,
                            main_color,
                        );
                    }
                    graphics.draw_rect_outline_fancy(
                        Rect::new(
                            x + cursor.0 as f32 * scale,
                            y + cursor.1 as f32 * scale,
                            scale,
                            scale,
                        ),
                        margin,
                        main_color,
                    );
                }
            }
        }
        if matches!(self.mode, Mode::Help(..)) {
            graphics.draw_rect_filled(
                Rect::new(0., 0., self.screen.0, self.screen.1),
                self.background.alpha(192).into(),
            );
        }
    }
}
