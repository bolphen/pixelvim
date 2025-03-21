#![allow(clippy::option_map_unit_fn)]
mod algo;
mod color;
mod command;
mod compression;
mod console;
mod debug;
mod engine;
mod error;
mod format;
mod graphics;
mod grid;
mod image;
mod input;
mod mapping;
mod parser;
mod selection;
mod stage;
mod tool;
mod utils;

#[cfg(all(feature = "lua", not(target_arch = "wasm32")))]
mod lua;

use std::path::PathBuf;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[global_allocator]
pub static ALLOCATOR: debug::Allocator = debug::Allocator::new(std::alloc::System);

#[derive(Default)]
pub struct Config {
    pub prefer_x11: bool,
    pub transparent_window: bool,
    pub config_file: Option<PathBuf>,
    pub new_file: Option<PathBuf>,
}

fn load_icon() -> Option<miniquad::conf::Icon> {
    use crate::format::load_png;
    let small_png = load_png(&include_bytes!("../assets/icon16x16.png")[..]).ok()?;
    let small = small_png.raw_data()[..].try_into().ok()?;
    let medium_png = load_png(&include_bytes!("../assets/icon32x32.png")[..]).ok()?;
    let medium = medium_png.raw_data()[..].try_into().ok()?;
    let big_png = load_png(&include_bytes!("../assets/icon64x64.png")[..]).ok()?;
    let big = big_png.raw_data()[..].try_into().ok()?;
    Some(miniquad::conf::Icon { small, medium, big })
}

pub fn init(config: Config, paths: Vec<PathBuf>) {
    let linux_backend = if config.prefer_x11 {
        miniquad::conf::LinuxBackend::X11WithWaylandFallback
    } else {
        miniquad::conf::LinuxBackend::WaylandWithX11Fallback
    };
    let conf = miniquad::conf::Conf {
        window_title: "pixelvim".into(),
        high_dpi: true,
        icon: load_icon(),
        platform: miniquad::conf::Platform {
            blocking_event_loop: true,
            linux_wm_class: "pixelvim",
            linux_backend,
            framebuffer_alpha: config.transparent_window,
            ..Default::default()
        },
        ..Default::default()
    };

    miniquad::start(conf, || {
        Box::new(stage::Stage::new(
            config.config_file,
            config.new_file,
            paths,
        ))
    });
}
