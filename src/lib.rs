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
mod stage;
mod tool;
mod utils;

#[cfg(all(feature = "lua", not(target_arch = "wasm32")))]
mod lua;

use std::path::PathBuf;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[global_allocator]
pub static ALLOCATOR: debug::Allocator = debug::Allocator::new(std::alloc::System);

pub fn init(config: Option<PathBuf>, paths: Vec<PathBuf>) {
    let conf = miniquad::conf::Conf {
        window_title: "pixelvim".into(),
        high_dpi: true,
        platform: miniquad::conf::Platform {
            blocking_event_loop: true,
            ..Default::default()
        },
        ..Default::default()
    };

    miniquad::start(conf, || Box::new(stage::Stage::new(config, paths)));
}
