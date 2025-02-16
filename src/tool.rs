use crate::algo::{Brush, Selection};
use crate::color::{Color, ColorMode};
use crate::image::{Image, Symmetry};

pub fn flip_x(image: &Image, selection: &Selection, symmetry: Symmetry) -> (Image, Selection) {
    let width = image.width();
    let height = image.height();
    let mut new = image.blank();
    let offset = if symmetry.x { symmetry.x_offset } else { 0 };
    for x in 0..width as i32 {
        for y in 0..height as i32 {
            if let Some(color) = image.get(width as i32 - 1 - x + offset, y) {
                *new.get_unchecked_mut(x, y) = *color;
            }
        }
    }
    (new, crate::algo::flip_x(image, selection, symmetry))
}

pub fn flip_y(image: &Image, selection: &Selection, symmetry: Symmetry) -> (Image, Selection) {
    let width = image.width();
    let height = image.height();
    let mut new = image.blank();
    let offset = if symmetry.y { symmetry.y_offset } else { 0 };
    for x in 0..width as i32 {
        for y in 0..height as i32 {
            if let Some(color) = image.get(x, height as i32 - 1 - y + offset) {
                *new.get_unchecked_mut(x, y) = *color;
            }
        }
    }
    (new, crate::algo::flip_y(image, selection, symmetry))
}

pub fn resize(image: &Image, width: usize, height: usize) -> Image {
    let x_factor = image.width() as f32 / width as f32;
    let y_factor = image.height() as f32 / height as f32;
    let mut new = Image::new_with(width, height, (0, 0, 0, 0).into());
    for x in 0..width as i32 {
        let xx = (x as f32 * x_factor).floor() as i32;
        for y in 0..height as i32 {
            let yy = (y as f32 * y_factor).floor() as i32;
            *new.get_unchecked_mut(x, y) = *image.get_unchecked(xx, yy);
        }
    }
    new
}

pub fn quantize(image: &Image, count: usize) -> Option<Image> {
    let neu_quant = color_quant::NeuQuant::new(1, count, image.raw_data());
    let mut new = image.clone();
    for j in 0..image.height() as i32 {
        for i in 0..image.width() as i32 {
            let color = new.get_unchecked_mut(i, j);
            let mut slice: [u8; 4] = (*color).into();
            neu_quant.map_pixel(&mut slice);
            // remove alpha that's near 0. or 1.
            if slice[3] >= 224 {
                slice[3] = 255;
            } else if slice[3] < 32 {
                slice[3] = 0;
            }
            let new_color = slice.into();
            if new_color != *color {
                *color = new_color;
            }
        }
    }
    Some(new)
}

pub fn reduce(image: &Image, selection: &Selection, palette: &[Color]) -> Option<Image> {
    (!palette.is_empty()).then(|| {
        let mut new = image.clone();
        let mut changed = false;
        for j in 0..image.height() as i32 {
            for i in 0..image.width() as i32 {
                if selection.is_empty() || selection.contains(&(i, j)) {
                    let color = new.get_unchecked_mut(i, j);
                    if color.3 > 0 {
                        let mut pal: Vec<_> = palette.iter().map(|c| (c.diff(color), c)).collect();
                        pal.sort_by_key(|(d, _)| *d);
                        let new_color = *pal[0].1;
                        if new_color != *color {
                            changed = true;
                            *color = new_color;
                        }
                    }
                }
            }
        }
        changed.then_some(new)
    })?
}

pub fn brush(
    image: &Image,
    selection: &Selection,
    trace: Vec<(i32, i32)>,
    color_mode: ColorMode,
    symmetry: Symmetry,
    brush: &Brush,
    outline: bool,
) -> Option<Image> {
    let diff: Vec<_> = crate::algo::brush(image, trace, symmetry, brush, outline)
        .into_iter()
        .filter(|(x, y)| {
            image.is_in_bound(*x, *y) && (selection.is_empty() || selection.contains(&(*x, *y)))
        })
        .collect();
    (!diff.is_empty()).then(|| {
        let mut new = image.clone();
        for (x, y) in diff {
            color_mode.apply(new.get_unchecked_mut(x, y));
        }
        new
    })
}

pub fn rect(
    image: &Image,
    selection: &Selection,
    bounds: ((i32, i32), (i32, i32)),
    color_mode: ColorMode,
    symmetry: Symmetry,
    outline: bool,
) -> Option<Image> {
    let diff: Vec<_> = crate::algo::rect(image, bounds, symmetry, outline)
        .into_iter()
        .filter(|(x, y)| {
            image.is_in_bound(*x, *y) && (selection.is_empty() || selection.contains(&(*x, *y)))
        })
        .collect();
    (!diff.is_empty()).then(|| {
        let mut new = image.clone();
        for (x, y) in diff {
            color_mode.apply(new.get_unchecked_mut(x, y));
        }
        new
    })
}

pub fn flood(
    image: &Image,
    selection: &Selection,
    cursor: (i32, i32),
    tolerance: u8,
    color_mode: ColorMode,
    symmetry: Symmetry,
    discon: bool,
) -> Option<Image> {
    let diff: Vec<_> = crate::algo::flood(
        image,
        (!selection.is_empty()).then_some(selection),
        cursor,
        tolerance,
        symmetry,
        discon,
    )
    .into_iter()
    .collect();
    (!diff.is_empty()).then(|| {
        let mut new = image.clone();
        for (x, y) in diff {
            color_mode.apply(new.get_unchecked_mut(x, y));
        }
        new
    })
}

pub fn r#move(image: &Image, dir: (i32, i32)) -> Image {
    let width = image.width();
    let height = image.height();
    let mut new = image.blank();
    for x in 0..width as i32 {
        for y in 0..height as i32 {
            image
                .get(x - dir.0, y - dir.1)
                .map(|c| *new.get_unchecked_mut(x, y) = *c);
        }
    }
    new
}

pub fn cut(image: &Image, selection: &Selection) -> Image {
    let mut new = image.clone();
    for (x, y) in selection.iter() {
        new.get_mut(*x, *y).map(|c| *c = Color(0, 0, 0, 0));
    }
    new
}
