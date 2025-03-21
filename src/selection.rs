use crate::grid::Grid;
use crate::parser::Size;
use crate::utils::Rect;
use nanoserde::{DeBin, SerBin};

#[derive(Clone, SerBin, DeBin)]
pub struct Selection {
    offset: (i32, i32),
    grid: Grid<bool>,
    is_empty: bool,
    can_shift: bool,
}

impl Default for Selection {
    fn default() -> Self {
        Self::new()
    }
}

const CHUNK: u32 = 32;
const CHUNK_I32: i32 = CHUNK as _;

impl Selection {
    pub fn new() -> Self {
        Selection::with_capacity(Size(CHUNK, CHUNK))
    }
    pub fn with_capacity(size: Size) -> Self {
        Selection {
            offset: (0, 0),
            grid: Grid::new(size.0 as _, size.1 as _),
            is_empty: true,
            can_shift: true,
        }
    }
    pub fn capacity(&self) -> Rect<i32> {
        let (x, y) = self.offset;
        Rect::new(x, y, self.grid.width() as _, self.grid.height() as _)
    }
    pub fn with_offset(mut self, offset: (i32, i32)) -> Self {
        self.offset = offset;
        self
    }
    pub fn offset_by(&mut self, dir: (i32, i32)) -> &Self {
        self.offset.0 += dir.0;
        self.offset.1 += dir.1;
        self
    }
    #[inline]
    pub fn offset(&self) -> (i32, i32) {
        self.offset
    }
    #[inline]
    pub fn grid_width(&self) -> usize {
        self.grid.width()
    }
    #[inline]
    pub fn grid_height(&self) -> usize {
        self.grid.height()
    }
    #[inline]
    pub fn grid_data(&self) -> &Vec<bool> {
        self.grid.data()
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.is_empty
    }
    pub fn contains(&self, pos: (i32, i32)) -> bool {
        self.grid
            .get(pos.0 - self.offset.0, pos.1 - self.offset.1)
            .is_some_and(|b| *b)
    }
    pub fn retain(&mut self, mut predicate: impl FnMut((i32, i32)) -> bool) {
        self.is_empty = true;
        for x in 0..self.grid.width() as i32 {
            for y in 0..self.grid.height() as i32 {
                let b = self.grid.get_unchecked_mut(x, y);
                if *b && !(predicate)((x + self.offset.0, y + self.offset.1)) {
                    *b = false;
                }
                if *b {
                    self.is_empty = false;
                }
            }
        }
        if self.is_empty {
            self.can_shift = true;
        }
    }
    fn ensure(&mut self, pos: (i32, i32)) {
        let (x, y) = (pos.0 - self.offset.0, pos.1 - self.offset.1);
        if !self.grid.is_in_bound(x, y) {
            if self.is_empty && self.can_shift {
                let offset = (x - x.rem_euclid(CHUNK_I32), y - y.rem_euclid(CHUNK_I32));
                self.offset_by(offset);
            } else {
                let new_ox = -((CHUNK_I32 - 1 - 0.min(x)) / CHUNK_I32 * CHUNK_I32);
                let new_oy = -((CHUNK_I32 - 1 - 0.min(y)) / CHUNK_I32 * CHUNK_I32);
                let cap_w = (self.grid.width() as i32).max(x + 1) - new_ox;
                let cap_h = (self.grid.height() as i32).max(y + 1) - new_oy;
                let new_w = (cap_w as u32).div_ceil(CHUNK) * CHUNK;
                let new_h = (cap_h as u32).div_ceil(CHUNK) * CHUNK;
                let mut grid = Grid::new(new_w as _, new_h as _);
                self.grid.blit(&mut grid, -new_ox, -new_oy);
                self.grid = grid;
                self.offset_by((new_ox, new_oy));
            }
        }
        self.can_shift = false;
    }
    pub fn insert_unchecked(&mut self, pos: (i32, i32)) {
        let (x, y) = (pos.0 - self.offset.0, pos.1 - self.offset.1);
        *self.grid.get_unchecked_mut(x, y) = true;
        self.is_empty = false;
    }
    pub fn insert(&mut self, pos: (i32, i32)) {
        self.ensure(pos);
        self.insert_unchecked(pos);
    }
    pub fn insert_rect(&mut self, rect: Rect<i32>) {
        if rect.w > 0 && rect.h > 0 {
            self.ensure((rect.x, rect.y));
            self.ensure((rect.x + rect.w - 1, rect.y + rect.h - 1));
            let (x, y) = (rect.x - self.offset.0, rect.y - self.offset.1);
            Grid::new_with(rect.w as _, rect.h as _, true).blit(&mut self.grid, x, y);
            self.is_empty = false;
        }
    }
    pub fn extend_with_bound<T>(&mut self, iter: T, bound: Rect<i32>)
    where
        T: IntoIterator<Item = (i32, i32)>,
    {
        self.ensure((bound.x, bound.y));
        self.ensure((bound.x + bound.w - 1, bound.y + bound.h - 1));
        for pos in iter {
            self.insert_unchecked(pos);
        }
    }
    pub fn clear(&mut self) {
        *self = Selection::new()
    }
    pub fn iter(&self) -> SelectionRefIter {
        SelectionRefIter {
            selection: self,
            pos: 0,
        }
    }
    pub fn to_vec(&self) -> Vec<(i32, i32)> {
        self.iter().collect()
    }
    pub fn crop(&self, size: Size) -> Grid<bool> {
        let mut result = Grid::new(size.0 as _, size.1 as _);
        self.grid.blit(&mut result, self.offset.0, self.offset.1);
        result
    }
}

impl IntoIterator for Selection {
    type Item = (i32, i32);
    type IntoIter = SelectionIter;
    fn into_iter(self) -> Self::IntoIter {
        SelectionIter {
            selection: self,
            pos: 0,
        }
    }
}

impl<'a> IntoIterator for &'a Selection {
    type Item = (i32, i32);
    type IntoIter = SelectionRefIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        SelectionRefIter {
            selection: self,
            pos: 0,
        }
    }
}

impl FromIterator<(i32, i32)> for Selection {
    fn from_iter<T: IntoIterator<Item = (i32, i32)>>(iter: T) -> Self {
        let mut selection = Selection::new();
        selection.extend(iter);
        selection
    }
}

impl Extend<(i32, i32)> for Selection {
    fn extend<T: IntoIterator<Item = (i32, i32)>>(&mut self, iter: T) {
        for pos in iter {
            self.insert(pos);
        }
    }
}

pub struct SelectionRefIter<'a> {
    selection: &'a Selection,
    pos: usize,
}

pub struct SelectionIter {
    selection: Selection,
    pos: usize,
}

fn sel_iter_next(selection: &Selection, pos: &mut usize) -> Option<(i32, i32)> {
    let vec = &selection.grid.data()[*pos..];
    let width = selection.grid.width() as i32;
    let (mut x, mut y) = (*pos as i32 % width, *pos as i32 / width);
    for b in vec {
        *pos += 1;
        if *b {
            return Some((x + selection.offset.0, y + selection.offset.1));
        }
        x += 1;
        if x == width {
            x = 0;
            y += 1;
        }
    }
    None
}

impl Iterator for SelectionRefIter<'_> {
    type Item = (i32, i32);
    fn next(&mut self) -> Option<Self::Item> {
        sel_iter_next(self.selection, &mut self.pos)
    }
}

impl Iterator for SelectionIter {
    type Item = (i32, i32);
    fn next(&mut self) -> Option<Self::Item> {
        sel_iter_next(&self.selection, &mut self.pos)
    }
}
