use nanoserde::{DeBin, SerBin};

pub fn is_in_bound(width: usize, height: usize, x: i32, y: i32) -> bool {
    0 <= x && x < width as i32 && 0 <= y && y < height as i32
}

#[derive(Clone, SerBin, DeBin)]
pub struct Grid<T> {
    width: usize,
    height: usize,
    data: Vec<T>,
}
impl<T: Clone> Grid<T> {
    pub fn new_from(width: usize, height: usize, data: Vec<T>) -> Self {
        Grid {
            width,
            height,
            data,
        }
    }
    pub fn new_with(width: usize, height: usize, default: T) -> Self {
        let data = vec![default; width * height];
        Grid {
            width,
            height,
            data,
        }
    }
    pub fn clear_with(&mut self, value: T) {
        self.data.iter_mut().for_each(|d| *d = value.clone());
    }
    pub fn empty() -> Self {
        Grid {
            width: 0,
            height: 0,
            data: Vec::new(),
        }
    }
}
impl<T> Grid<T> {
    pub fn is_in_bound(&self, x: i32, y: i32) -> bool {
        is_in_bound(self.width, self.height, x, y)
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    pub fn data(&self) -> &Vec<T> {
        &self.data
    }
    pub fn get(&self, x: i32, y: i32) -> Option<&T> {
        self.is_in_bound(x, y)
            .then(|| self.data.get(y as usize * self.width + x as usize))?
    }
    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut T> {
        self.is_in_bound(x, y)
            .then(|| self.data.get_mut(y as usize * self.width + x as usize))?
    }
    pub fn get_unchecked(&self, x: i32, y: i32) -> &T {
        &self.data[y as usize * self.width + x as usize]
    }
    pub fn get_unchecked_mut(&mut self, x: i32, y: i32) -> &mut T {
        &mut self.data[y as usize * self.width + x as usize]
    }
    pub fn apply<S>(&self, f: impl Fn(&T) -> S) -> Grid<S> {
        Grid {
            width: self.width,
            height: self.height,
            data: self.data.iter().map(f).collect(),
        }
    }
}
