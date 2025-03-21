use nanoserde::{DeBin, SerBin};

use std::collections::HashSet;

use crate::color::Color;
use crate::grid::Grid;
use crate::parser::Size;

pub type Image = Grid<Color>;

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
        Image::new(self.width(), self.height())
    }
    pub fn colors(&self) -> HashSet<Color> {
        self.data().iter().copied().collect()
    }
    pub fn sub_image(&self, x: usize, y: usize, w: usize, h: usize) -> Option<Image> {
        self.sub_grid(x, y, w, h)
    }
    pub fn effective_area(
        &self,
        mut predicate: impl FnMut((i32, i32), Color) -> Option<bool>,
    ) -> Option<((i32, i32), Image)> {
        let (mut x_min, mut y_min, mut x_max, mut y_max) =
            (self.width() as i32, self.height() as i32, 0i32, 0i32);
        for y in 0..self.height() as i32 {
            for x in 0..self.width() as i32 {
                if (predicate)((x, y), *self.get_unchecked(x, y))? {
                    x_min = x_min.min(x);
                    y_min = y_min.min(y);
                    x_max = x_max.max(x + 1);
                    y_max = y_max.max(y + 1);
                }
            }
        }
        let w = (x_max - x_min).max(0);
        let h = (y_max - y_min).max(0);
        Some((
            (x_min, y_min),
            self.sub_image(x_min as _, y_min as _, w as _, h as _)
                .unwrap(),
        ))
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
