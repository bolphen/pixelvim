use nanoserde::{DeBin, SerBin};

#[inline]
pub fn is_in_bound(width: usize, height: usize, x: i32, y: i32) -> bool {
    0 <= x && x < width as i32 && 0 <= y && y < height as i32
}

#[derive(Clone, SerBin, DeBin)]
pub struct Grid<T> {
    width: usize,
    height: usize,
    data: Vec<T>,
}

impl<T: Clone + Default> Grid<T> {
    pub fn new(width: usize, height: usize) -> Self {
        let data = vec![Default::default(); width * height];
        Grid {
            width,
            height,
            data,
        }
    }
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
    #[inline]
    pub fn is_in_bound(&self, x: i32, y: i32) -> bool {
        is_in_bound(self.width, self.height, x, y)
    }
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }
    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }
    #[inline]
    pub fn data(&self) -> &Vec<T> {
        &self.data
    }
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Option<&T> {
        self.is_in_bound(x, y)
            .then(|| &self.data[y as usize * self.width + x as usize])
    }
    #[inline]
    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut T> {
        self.is_in_bound(x, y)
            .then(|| &mut self.data[y as usize * self.width + x as usize])
    }
    #[inline]
    pub fn get_unchecked(&self, x: i32, y: i32) -> &T {
        &self.data[y as usize * self.width + x as usize]
    }
    #[inline]
    pub fn get_unchecked_mut(&mut self, x: i32, y: i32) -> &mut T {
        &mut self.data[y as usize * self.width + x as usize]
    }
    pub fn apply<S>(&self, f: impl FnMut(&T) -> S) -> Grid<S> {
        Grid {
            width: self.width,
            height: self.height,
            data: self.data.iter().map(f).collect(),
        }
    }
}
impl<T: Clone> Grid<T> {
    pub fn blit(&self, target: &mut Grid<T>, x: i32, y: i32) {
        for j in 0.max(-y)..(self.height as i32).min(target.height as i32 - y) {
            let i_left = 0.max(-x);
            let i_right = (self.width as i32).min(target.width as i32 - x);
            if i_left < i_right {
                let src_left = (j * self.width as i32 + i_left) as usize;
                let src_right = (j * self.width as i32 + i_right) as usize;
                let dst_left = ((j + y) * target.width as i32 + i_left + x) as usize;
                let dst_right = ((j + y) * target.width as i32 + i_right + x) as usize;
                let dst = &mut target.data[dst_left..dst_right];
                dst.clone_from_slice(&self.data[src_left..src_right]);
            }
        }
    }
}
impl<T: Clone + Default> Grid<T> {
    pub fn sub_grid(&self, x: usize, y: usize, w: usize, h: usize) -> Option<Grid<T>> {
        if x + w <= self.width() && y + h <= self.height() {
            let mut sub = Grid::<T>::new(w, h);
            for i in 0..w as i32 {
                for j in 0..h as i32 {
                    *sub.get_unchecked_mut(i, j) =
                        self.get_unchecked(i + x as i32, j + y as i32).clone();
                }
            }
            Some(sub)
        } else {
            None
        }
    }
}
