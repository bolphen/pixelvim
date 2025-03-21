use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub struct Rect<T> {
    pub x: T,
    pub y: T,
    pub w: T,
    pub h: T,
}

impl<T> From<(T, T, T, T)> for Rect<T> {
    fn from(value: (T, T, T, T)) -> Self {
        let (x, y, w, h) = value;
        Rect { x, y, w, h }
    }
}

impl<T: Copy> Rect<T> {
    pub fn new(x: T, y: T, w: T, h: T) -> Self {
        Self { x, y, w, h }
    }
    pub fn get(&self) -> (T, T, T, T) {
        (self.x, self.y, self.w, self.h)
    }
}

impl<T> Rect<T>
where
    T: std::ops::AddAssign,
{
    pub fn offset_by(mut self, dir: (T, T)) -> Self {
        self.x += dir.0;
        self.y += dir.1;
        self
    }
}

pub fn replace_home_dir(path: &PathBuf) -> PathBuf {
    if let (Ok(path), Some(home_dir)) = (path.strip_prefix("~/"), dirs::home_dir()) {
        home_dir.join(path)
    } else {
        path.into()
    }
}

pub fn get_swap_path(path: &Path) -> Option<PathBuf> {
    let mut swap = path.to_path_buf();
    let name = path.file_name()?.to_str()?;
    swap.set_file_name(format!(".{}.swp", name));
    Some(swap)
}

#[cfg(not(target_arch = "wasm32"))]
pub struct TimeManager {
    time: std::time::Instant,
    start_time: std::time::Instant,
}

#[cfg(not(target_arch = "wasm32"))]
impl TimeManager {
    pub fn new() -> Self {
        TimeManager {
            time: std::time::Instant::now(),
            start_time: std::time::Instant::now(),
        }
    }
    pub fn update(&mut self) {
        self.time = std::time::Instant::now();
    }
    pub fn elapsed(&self) -> u32 {
        let elapsed = self.time.elapsed();
        elapsed.as_millis().min(u32::MAX as _) as u32
    }
    pub fn elapsed_abs(&self) -> u32 {
        self.time.duration_since(self.start_time).subsec_millis()
    }
}

#[cfg(target_arch = "wasm32")]
pub struct TimeManager {
    time: f32,
}

#[cfg(target_arch = "wasm32")]
impl TimeManager {
    pub fn new() -> Self {
        TimeManager { time: 0. }
    }
    pub fn update(&mut self) {
        self.time = (self.time + 1000. / 60.).rem_euclid(1000.);
    }
    pub fn elapsed(&self) -> u32 {
        17
    }
    pub fn elapsed_abs(&self) -> u32 {
        self.time as _
    }
}
