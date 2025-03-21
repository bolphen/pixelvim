use crate::color::Color;
use crate::image::{Image, Symmetry};
use crate::selection::Selection;

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

pub fn resize_image(image: &Image, width: usize, height: usize) -> Image {
    let x_factor = image.width() as f32 / width as f32;
    let y_factor = image.height() as f32 / height as f32;
    let mut new = Image::new_with(width, height, (0, 0, 0, 0).into());
    for x in 0..width as i32 {
        let xx = ((x as f32 + 0.5) * x_factor - 0.5).round() as i32;
        for y in 0..height as i32 {
            let yy = ((y as f32 + 0.5) * y_factor - 0.5).round() as i32;
            *new.get_unchecked_mut(x, y) = *image.get_unchecked(xx, yy);
        }
    }
    new
}

pub fn scale2x(image: &Image) -> Image {
    let (width, height) = (image.width(), image.height());
    let mut new = Image::new_with(width * 2, height * 2, (0, 0, 0, 0).into());
    if height == 0 {
        return new;
    }
    for x in 0..width as i32 {
        for y in 0..height as i32 {
            let c = *image.get_unchecked(x, y);
            let l = image.get(x - 1, y);
            let r = image.get(x + 1, y);
            let u = image.get(x, y - 1);
            let d = image.get(x, y + 1);
            *new.get_unchecked_mut(2 * x, 2 * y) = l
                .and_then(|l| (u == Some(l) && r != u && d != Some(l)).then_some(*l))
                .unwrap_or(c);
            *new.get_unchecked_mut(2 * x + 1, 2 * y) = r
                .and_then(|r| (u == Some(r) && l != u && d != Some(r)).then_some(*r))
                .unwrap_or(c);
            *new.get_unchecked_mut(2 * x, 2 * y + 1) = l
                .and_then(|l| (d == Some(l) && Some(l) != u && r != d).then_some(*l))
                .unwrap_or(c);
            *new.get_unchecked_mut(2 * x + 1, 2 * y + 1) = r
                .and_then(|r| (d == Some(r) && Some(r) != u && l != d).then_some(*r))
                .unwrap_or(c);
        }
    }
    new
}

pub fn scale3x(image: &Image) -> Image {
    let (width, height) = (image.width(), image.height());
    let mut new = Image::new_with(width * 3, height * 3, (0, 0, 0, 0).into());
    if height == 0 {
        return new;
    }
    for x in 0..width as i32 {
        for y in 0..height as i32 {
            let c = *image.get_unchecked(x, y);
            let l = image.get(x - 1, y);
            let r = image.get(x + 1, y);
            let u = image.get(x, y - 1);
            let ul = image.get(x - 1, y - 1);
            let ur = image.get(x + 1, y - 1);
            let d = image.get(x, y + 1);
            let dl = image.get(x - 1, y + 1);
            let dr = image.get(x + 1, y + 1);
            *new.get_unchecked_mut(3 * x, 3 * y) = l
                .and_then(|l| (u == Some(l) && r != u && d != Some(l)).then_some(*l))
                .unwrap_or(c);
            *new.get_unchecked_mut(3 * x + 1, 3 * y) = l
                .and_then(|l| {
                    (u == Some(l) && r != u && d != Some(l) && ur != Some(&c)).then_some(*l)
                })
                .unwrap_or(
                    r.and_then(|r| {
                        (u == Some(r) && l != u && d != Some(r) && ul != Some(&c)).then_some(*r)
                    })
                    .unwrap_or(c),
                );
            *new.get_unchecked_mut(3 * x + 2, 3 * y) = r
                .and_then(|r| (u == Some(r) && l != u && d != Some(r)).then_some(*r))
                .unwrap_or(c);
            *new.get_unchecked_mut(3 * x, 3 * y + 1) = l
                .and_then(|l| {
                    ((d == Some(l) && r != d && u != Some(l) && ul != Some(&c))
                        || (u == Some(l) && r != u && d != Some(l) && dl != Some(&c)))
                    .then_some(*l)
                })
                .unwrap_or(c);
            *new.get_unchecked_mut(3 * x + 1, 3 * y + 1) = c;
            *new.get_unchecked_mut(3 * x + 2, 3 * y + 1) = r
                .and_then(|r| {
                    ((d == Some(r) && l != d && u != Some(r) && ur != Some(&c))
                        || (u == Some(r) && l != u && d != Some(r) && dr != Some(&c)))
                    .then_some(*r)
                })
                .unwrap_or(c);
            *new.get_unchecked_mut(3 * x, 3 * y + 2) = l
                .and_then(|l| (d == Some(l) && Some(l) != u && r != d).then_some(*l))
                .unwrap_or(c);
            *new.get_unchecked_mut(3 * x + 1, 3 * y + 2) = l
                .and_then(|l| {
                    (d == Some(l) && r != d && u != Some(l) && dr != Some(&c)).then_some(*l)
                })
                .unwrap_or(
                    r.and_then(|r| {
                        (d == Some(r) && l != d && u != Some(r) && dl != Some(&c)).then_some(*r)
                    })
                    .unwrap_or(c),
                );
            *new.get_unchecked_mut(3 * x + 2, 3 * y + 2) = r
                .and_then(|r| (d == Some(r) && Some(r) != u && l != d).then_some(*r))
                .unwrap_or(c);
        }
    }
    new
}

pub fn rotate_angle(image: &Image, angle: f32, center: Option<(f32, f32)>) -> Image {
    let (width, height) = (image.width(), image.height());
    let center = center.unwrap_or((width as f32 / 2., height as f32 / 2.));
    let scaled = scale3x(image);
    let mut new = image.blank();
    for x in 0..width as i32 {
        for y in 0..height as i32 {
            let c = angle.cos();
            let s = angle.sin();
            let xx = (3.
                * ((x as f32 + 0.5 - center.0) * c + (y as f32 + 0.5 - center.1) * s + center.0
                    - 1. / 3.))
                .round() as _;
            let yy = (3.
                * ((x as f32 + 0.5 - center.0) * -s + (y as f32 + 0.5 - center.1) * c + center.1
                    - 1. / 3.))
                .round() as _;
            if let Some(c) = scaled.get(xx, yy) {
                *new.get_unchecked_mut(x, y) = *c;
            }
        }
    }
    new
}
pub fn rotate_bounds(image: &Image, bounds: ((i32, i32), (i32, i32))) -> Image {
    let (width, height) = (image.width() as f32, image.height() as f32);
    let center = (width / 2., height / 2.);
    rotate_angle(
        image,
        f32::atan2(bounds.1.1 as f32 - center.1, bounds.1.0 as f32 - center.0)
            - f32::atan2(bounds.0.1 as f32 - center.1, bounds.0.0 as f32 - center.0),
        Some(center),
    )
}

pub fn resize_canvas(image: &Image, width: usize, height: usize) -> Image {
    let mut new = Image::new_with(width, height, (0, 0, 0, 0).into());
    image.blit(&mut new, 0, 0);
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
                if selection.is_empty() || selection.contains((i, j)) {
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
        new.get_mut(x, y).map(|c| *c = Color(0, 0, 0, 0));
    }
    new
}
