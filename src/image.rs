use nanoserde::{DeBin, SerBin};

use std::collections::HashSet;

use crate::color::Color;
use crate::parser::Size;

pub type Image = crate::grid::Grid<Color>;

impl Image {
    pub fn size(&self) -> Size {
        Size(self.width() as _, self.height() as _)
    }
    pub fn new_from_bytes(width: usize, height: usize, bytes: Vec<u8>) -> Self {
        let (prefix, data, suffix) = unsafe { bytes.align_to::<Color>() };
        assert!(prefix.is_empty() && suffix.is_empty());
        Image::new_from(width, height, data.into())
    }
    pub fn raw_data(&self) -> &[u8] {
        let (prefix, bytes, suffix) = unsafe { self.data().align_to::<u8>() };
        assert!(prefix.is_empty() && suffix.is_empty());
        bytes
    }
    pub fn blank(&self) -> Image {
        Image::new_with(self.width(), self.height(), Color(0, 0, 0, 0))
    }
    pub fn colors(&self) -> HashSet<Color> {
        self.data().iter().copied().collect()
    }
    pub fn sub_image(&self, x: usize, y: usize, w: usize, h: usize) -> Option<Image> {
        if x + w <= self.width() && y + h <= self.height() {
            let mut sub = Image::new_with(w, h, Color(0, 0, 0, 0));
            for i in 0..w as i32 {
                for j in 0..h as i32 {
                    *sub.get_unchecked_mut(i, j) = *self.get_unchecked(i + x as i32, j + y as i32);
                }
            }
            Some(sub)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Default, SerBin, DeBin)]
pub struct Symmetry {
    pub x: bool,
    pub x_offset: i32,
    pub y: bool,
    pub y_offset: i32,
}

impl Symmetry {
    pub fn helper(&self, w: i32, h: i32) -> SymmetryHelper {
        SymmetryHelper { sym: *self, w, h }
    }
}

pub struct SymmetryHelper {
    sym: Symmetry,
    w: i32,
    h: i32,
}

impl SymmetryHelper {
    pub fn sym_x(&self, p: (i32, i32)) -> (i32, i32) {
        let (x, y) = p;
        (self.w - 1 - x + self.sym.x_offset, y)
    }
    pub fn sym_y(&self, p: (i32, i32)) -> (i32, i32) {
        let (x, y) = p;
        (x, self.h - 1 - y + self.sym.y_offset)
    }
    pub fn sym_x_y(&self, p: (i32, i32)) -> (i32, i32) {
        let (x, y) = p;
        (
            self.w - 1 - x + self.sym.x_offset,
            self.h - 1 - y + self.sym.y_offset,
        )
    }
}
