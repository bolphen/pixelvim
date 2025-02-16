use miniquad::*;
use std::path::PathBuf;

use crate::console::Console;
use crate::engine::Engine;
use crate::graphics::Graphics;

pub struct Stage {
    console: Console,
    engine: Engine,
    graphics: Graphics,
}

const DEFAULT_CONFIG: &str = include_str!("../config/pixelvim.conf");

impl Stage {
    pub fn new(config: Option<PathBuf>, paths: Vec<PathBuf>) -> Self {
        let mut stage = Stage {
            console: Console::new(1, 1),
            engine: Engine::new(),
            graphics: Graphics::new(1.),
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
        for path in paths {
            stage.engine.load_path(&path);
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
}
impl EventHandler for Stage {
    fn resize_event(&mut self, _width: f32, _height: f32) {
        self.resize();
        miniquad::window::schedule_update();
    }
    fn update(&mut self) {
        self.engine.update();
        // release textures when buffers are dropped
        for id in self.engine.dropped_buffer.drain(..) {
            self.graphics.drop_buffer(id);
        }
        if self.engine.quit_requested() {
            miniquad::window::quit();
        }
        if self.engine.needs_update() {
            miniquad::window::schedule_update();
        }
    }
    fn key_down_event(&mut self, keycode: KeyCode, keymods: KeyMods, repeat: bool) {
        self.engine.key_down_event(keycode, keymods, repeat);
        miniquad::window::schedule_update();
    }
    fn key_up_event(&mut self, keycode: KeyCode, keymods: KeyMods) {
        self.engine.key_up_event(keycode, keymods);
        miniquad::window::schedule_update();
    }
    fn char_event(&mut self, char: char, keymods: KeyMods, repeat: bool) {
        self.engine.char_event(char, keymods, repeat);
        miniquad::window::schedule_update();
    }
    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.engine.mouse_tile = self.graphics.mouse_tile((x, y));
        self.engine.screen_tile = (self.console.width() as _, self.console.height() as _);
        let dpi_scale = miniquad::window::dpi_scale();
        self.engine.set_cursor((x / dpi_scale, y / dpi_scale));
        miniquad::window::schedule_update();
    }
    fn mouse_wheel_event(&mut self, x: f32, y: f32) {
        self.engine.mouse_wheel_event(x, y);
        miniquad::window::schedule_update();
    }
    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        self.engine.mouse_tile = self.graphics.mouse_tile((x, y));
        self.engine.screen_tile = (self.console.width() as _, self.console.height() as _);
        self.engine.mouse_button_down(button);
        let dpi_scale = miniquad::window::dpi_scale();
        self.engine.set_cursor((x / dpi_scale, y / dpi_scale));
        miniquad::window::schedule_update();
    }
    fn mouse_button_up_event(&mut self, button: MouseButton, x: f32, y: f32) {
        self.engine.mouse_tile = self.graphics.mouse_tile((x, y));
        self.engine.screen_tile = (self.console.width() as _, self.console.height() as _);
        self.engine.mouse_button_up(button);
        let dpi_scale = miniquad::window::dpi_scale();
        self.engine.set_cursor((x / dpi_scale, y / dpi_scale));
        miniquad::window::schedule_update();
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
        miniquad::window::schedule_update();
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
        }
        self.graphics.flush();
    }
}
