use nanoserde::{DeBin, SerBin};

use crate::image::{Image, Symmetry};
use crate::selection::Selection;

#[derive(Clone, SerBin, DeBin)]
pub struct Register {
    pub content: Image,
    pub selection: Selection,
    pub offset: (i32, i32),
}
impl Register {
    pub fn new(image: &Image, selection: &Selection) -> Option<Self> {
        let selection: Vec<_> = selection
            .iter()
            .filter(|(x, y)| image.is_in_bound(*x, *y))
            .collect();
        (!selection.is_empty()).then(|| {
            let x = *selection.iter().map(|(x, _)| x).min().expect("");
            let y = *selection.iter().map(|(_, y)| y).min().expect("");
            let w = selection.iter().map(|(x, _)| x).max().expect("") - x + 1;
            let h = selection.iter().map(|(_, y)| y).max().expect("") - y + 1;
            let mut content = Image::new_with(w as _, h as _, (0, 0, 0, 0).into());
            for (x0, y0) in &selection {
                *content.get_unchecked_mut(x0 - x, y0 - y) = *image.get_unchecked(*x0, *y0)
            }
            let selection = selection.iter().map(|(x0, y0)| (x0 - x, y0 - y)).collect();
            Register {
                content,
                selection,
                offset: (x, y),
            }
        })
    }
    #[inline]
    pub fn width(&self) -> usize {
        self.content.width()
    }
    #[inline]
    pub fn height(&self) -> usize {
        self.content.height()
    }
    pub fn with_offset(mut self, offset: (i32, i32)) -> Self {
        self.offset = offset;
        self
    }
    pub fn flip_x(&self) -> Register {
        let (content, selection) =
            crate::tool::flip_x(&self.content, &self.selection, Symmetry::default());
        Register {
            content,
            selection,
            offset: self.offset,
        }
    }
    pub fn flip_y(&self) -> Register {
        let (content, selection) =
            crate::tool::flip_y(&self.content, &self.selection, Symmetry::default());
        Register {
            content,
            selection,
            offset: self.offset,
        }
    }
}
