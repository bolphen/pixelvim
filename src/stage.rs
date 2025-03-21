use miniquad::*;
use std::path::PathBuf;

use crate::console::Console;
use crate::engine::Engine;
use crate::graphics::Graphics;

pub struct Stage {
    console: Console,
    engine: Engine,
    graphics: Graphics,
    cheap_update: bool,
}

const DEFAULT_CONFIG: &str = include_str!("../config/pixelvim.conf");

impl Stage {
    pub fn new(config: Option<PathBuf>, new_file: Option<PathBuf>, paths: Vec<PathBuf>) -> Self {
        let mut stage = Stage {
            console: Console::new(1, 1),
            engine: Engine::new(),
            graphics: Graphics::new(1.),
            cheap_update: false,
        };
        stage.resize();
        match config {
            Some(config) if config.as_os_str() == "-" => stage.engine.load_config(DEFAULT_CONFIG),
            Some(config) => stage.engine.load_config_from_path(&config),
            None => {
                if let Some(config) = dirs::config_dir().and_then(|d| {
                    let config = d.join("pixelvim.conf");
                    config.exists().then_some(config)
                }) {
                    stage.engine.load_config_from_path(&config);
                } else {
                    stage.engine.load_config(DEFAULT_CONFIG);
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        // open an empty canvas for the web version
        stage.engine.new_buffer_with_size(32, 32);
        for path in paths {
            stage.engine.load_path(&path);
        }
        if let Some(new_file) = new_file {
            let buffer = stage.engine.new_buffer_with_size(32, 32);
            buffer.texture_update(&mut stage.graphics);
            let _ = Engine::save_buffer_directly(buffer, Vec::new(), new_file);
        }
        stage
    }
    pub fn resize(&mut self) {
        let (width, height) = miniquad::window::screen_size();
        let dpi_scale = miniquad::window::dpi_scale();
        self.engine.screen = (width / dpi_scale, height / dpi_scale);
        let (w, h) = self.graphics.fit_screen(self.engine.screen);
        self.engine.screen_tile = (w as _, h as _);
        if w != self.console.width() || h != self.console.height() {
            self.console = Console::new(w, h);
        }
    }
    pub fn full_update(&mut self) {
        self.cheap_update = false;
        miniquad::window::schedule_update();
    }
}
impl EventHandler for Stage {
    fn update(&mut self) {
        if self.cheap_update {
            if self.engine.cheap_update() {
                self.engine.full_update(&mut self.graphics);
            }
            self.cheap_update = false;
        } else {
            self.engine.cheap_update();
            self.engine.full_update(&mut self.graphics);
        }
        if self.engine.needs_update() {
            self.cheap_update = true;
            miniquad::window::schedule_update();
        }
    }
    fn resize_event(&mut self, _width: f32, _height: f32) {
        self.resize();
        self.full_update();
    }
    fn key_down_event(&mut self, keycode: KeyCode, keymods: KeyMods, repeat: bool) {
        self.engine.key_down_event(keycode, keymods, repeat);
        self.full_update();
    }
    fn key_up_event(&mut self, keycode: KeyCode, keymods: KeyMods) {
        self.engine.key_up_event(keycode, keymods);
        self.full_update();
    }
    fn char_event(&mut self, char: char, keymods: KeyMods, repeat: bool) {
        self.engine.char_event(char, keymods, repeat);
        self.full_update();
    }
    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.engine.mouse_tile = self.graphics.mouse_tile((x, y));
        self.engine.screen_tile = (self.console.width() as _, self.console.height() as _);
        let dpi_scale = miniquad::window::dpi_scale();
        self.engine.set_cursor((x / dpi_scale, y / dpi_scale));
        self.full_update();
    }
    fn mouse_wheel_event(&mut self, x: f32, y: f32) {
        self.engine.mouse_wheel_event(x, y);
        self.full_update();
    }
    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        self.engine.mouse_tile = self.graphics.mouse_tile((x, y));
        self.engine.screen_tile = (self.console.width() as _, self.console.height() as _);
        let dpi_scale = miniquad::window::dpi_scale();
        self.engine.set_cursor((x / dpi_scale, y / dpi_scale));
        self.engine.mouse_button_down(button);
        self.full_update();
    }
    fn mouse_button_up_event(&mut self, button: MouseButton, x: f32, y: f32) {
        self.engine.mouse_tile = self.graphics.mouse_tile((x, y));
        self.engine.screen_tile = (self.console.width() as _, self.console.height() as _);
        let dpi_scale = miniquad::window::dpi_scale();
        self.engine.set_cursor((x / dpi_scale, y / dpi_scale));
        self.engine.mouse_button_up(button);
        self.full_update();
    }
    fn files_dropped_event(&mut self) {
        for i in 0..miniquad::window::dropped_file_count() {
            if let (Some(path), Some(bytes)) = (
                miniquad::window::dropped_file_path(i),
                miniquad::window::dropped_file_bytes(i),
            ) {
                self.engine.drop_file(&path, bytes);
            }
        }
        self.full_update();
    }

    fn draw(&mut self) {
        if self.engine.scale_ui() != self.graphics.scale() {
            self.graphics.set_scale(self.engine.scale_ui());
            self.resize();
        }
        self.graphics.toggle_srgb(Some(self.engine.srgb()));
        // draw texture
        self.engine.render_texture(&mut self.graphics);
        // draw ui
        if self.engine.show_ui {
            self.console
                .clear(Some((0, (0, 0, 0).into(), (0, 0, 0, 0).into())));
            self.engine.render_ui(&mut self.console);
            self.graphics.draw_console(&self.console, 0., 0.);
            if !self.engine.system_cursor {
                self.graphics.draw_cursor(
                    (
                        self.engine.mouse.0 - 16.,
                        self.engine.mouse.1 - 16.,
                        32.,
                        32.,
                    )
                        .into(),
                );
            }
        }
        self.graphics.flush();
    }
}
