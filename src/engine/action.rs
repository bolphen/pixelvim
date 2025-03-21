use super::*;

impl Engine {
    pub fn do_action(&mut self, action: Action, repeat: bool) {
        match (&action, self.buffers.get_mut(self.current), repeat) {
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
                self.tracker.reset_used();
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
                Action::Visual | Action::VisualAdd | Action::VisualSub | Action::VisualToggle,
                buffer,
                false,
            ) => {
                if self.mode.is_normal() {
                    self.mode = Mode::visual(None);
                }
                self.tracker.reset_used();
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
                self.tracker.reset_used();
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
                | Action::EllipseFilled
                | Action::EllipseOutline
                | Action::Line
                | Action::Move
                | Action::Rotate,
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
                    Action::EllipseFilled => Tool::Ellipse(false),
                    Action::EllipseOutline => Tool::Ellipse(true),
                    Action::Line => Tool::Line,
                    Action::Move => Tool::Move,
                    Action::Rotate => Tool::Rotate,
                    _ => unreachable!(),
                };
                if self.tool.set(tool) {
                    self.tracker.reset_used();
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
                Action::BrushToggle
                | Action::FloodToggle
                | Action::RectToggle
                | Action::EllipseToggle,
                buffer,
                false,
            ) => {
                let tool = match action {
                    Action::BrushToggle => Tool::Brush(true),
                    Action::FloodToggle => Tool::Flood(false),
                    Action::RectToggle => Tool::Rect(false),
                    Action::EllipseToggle => Tool::Ellipse(false),
                    _ => unreachable!(),
                };
                self.tool.set_or_toggle(tool);
                self.tracker.reset_used();
                if let Some(b) = buffer {
                    b.clear_temporary()
                }
                self.mode.reset(Some(Message::normal(&format!(
                    "Change tool to {}",
                    self.tool.string_short()
                ))));
            }
            (Action::Cancel, buffer, false) => {
                if let Some(buffer) = buffer {
                    if self.tracker.is_mouse_in_use() {
                        self.mode.take_modifier();
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
                    } else if self.tracker.is_keyboard_in_use() {
                        self.commit();
                        return;
                    }
                }
            }
            (Action::Up | Action::Down | Action::Left | Action::Right, Some(buffer), _) => {
                let v = get_modifier(self.mode.take_modifier()).unwrap_or(1);
                let dir = match action {
                    Action::Right => (v, 0),
                    Action::Left => (-v, 0),
                    Action::Up => (0, -v),
                    Action::Down => (0, v),
                    _ => unreachable!(),
                };
                if dir != (0, 0) {
                    if self.tracker.is_keyboard_in_use() {
                        self.tracker.track_keyboard(buffer.move_cursor(dir));
                    } else if !self.tracker.is_any_in_use() {
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
                                None, // software render for now
                            );
                            self.mode.set_message(Message::normal("Used move"));
                        } else {
                            match self.tool {
                                Tool::Move => {
                                    if let Some(r) =
                                        Register::new(buffer.image(), buffer.selection())
                                    {
                                        buffer.animation.paused = true;
                                        let cut =
                                            crate::tool::cut(buffer.image(), buffer.selection());
                                        let img_idx = buffer.session.insert_image(&cut);
                                        let reg_idx = tool::paste(
                                            &r,
                                            PasteFrom::Register(&r),
                                            buffer,
                                            Some(img_idx),
                                            Some(dir),
                                            self.draw_mode.0,
                                            None, // software render for now
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
            }
            (Action::Insert, Some(buffer), _) => {
                let cursor = buffer.cursor();
                if self.picker {
                    buffer.picker(cursor).map(|c| self.color = c);
                } else if !self.tracker.is_any_in_use() {
                    let frame = (!buffer.edit_all).then_some(buffer.current_frame_id());
                    self.tracker.start(cursor, tool::InputType::Keyboard, frame);
                    if self.mode.is_visual() {
                        self.tool.visual_preview(
                            &mut self.tracker,
                            buffer,
                            self.draw_mode.1,
                            self.mode.take_modifier(),
                            &self.tool_setting,
                        );
                    } else {
                        self.tool.normal_preview(
                            &mut self.tracker,
                            buffer,
                            self.draw_mode.0,
                            self.color,
                            self.mode.take_modifier(),
                            &self.tool_setting,
                            None, // software render for now
                        );
                    }
                } else if self.tracker.is_keyboard_in_use() {
                    self.commit();
                }
            }
            (Action::Fit, Some(buffer), _) => {
                if !self.tracker.is_mouse_in_use() {
                    buffer.fit_to_view(self.screen);
                }
            }
            _ => {
                if !self.tracker.is_mouse_in_use() {
                    if self.tracker.is_keyboard_in_use() {
                        self.commit();
                    }
                    match (action, self.buffers.get_mut(self.current), repeat) {
                        (Action::Command, buffer, false) => {
                            buffer.map(|b| b.clear_temporary());
                            self.mode.clear_message();
                            self.overlay = Overlay::Command(Input::new(""));
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
                        (Action::DoCommand(cmd), _, _) => {
                            let parsed_cmd = self.commands.parse(&cmd);
                            if let Ok(cmd) = parsed_cmd {
                                if let Err(e) = {
                                    let modifier = self.mode.take_modifier();
                                    self.do_command(cmd.modify(modifier))
                                } {
                                    self.mode.error(&e);
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
    }
}
