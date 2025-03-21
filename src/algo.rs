use crate::color::Color;
use crate::grid::Grid;
use crate::image::{Image, Symmetry};
use crate::parser::Size;
use crate::selection::Selection;
use crate::utils::Rect;

type Shape = (Vec<(i32, i32)>, Rect<i32>);
#[derive(Clone)]
pub enum Brush {
    Rect(u8),
    Round(u8, Shape),
    Custom(Shape),
}

impl std::fmt::Display for Brush {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Brush::Rect(size) => f.write_fmt(format_args!("[{size}]"))?,
            Brush::Round(size, ..) => f.write_fmt(format_args!("({size})"))?,
            Brush::Custom(..) => f.write_fmt(format_args!("custom"))?,
        }
        Ok(())
    }
}

impl Brush {
    fn round(size: u8) -> Self {
        let size = size.max(1);
        let s = size as i32;
        let selection = _ellipse(((-s / 2, -s / 2), ((s - 1) / 2, (s - 1) / 2)));
        Self::Round(size, (selection.iter().collect(), selection.capacity()))
    }
    pub fn set(&mut self, shape: &str, selection: Option<Selection>) -> Result<(), ()> {
        match shape {
            "1" => {
                // size 1 has no ambiguity
                *self = Brush::Rect(1);
            }
            c if c.starts_with("rect") => {
                let v = c.trim_start_matches("rect").trim();
                let size = v
                    .parse::<u8>()
                    .map(|v| v.max(1))
                    .ok()
                    .unwrap_or_else(|| self.size());
                *self = Brush::Rect(size);
            }
            c if c.starts_with("round") => {
                let v = c.trim_start_matches("round").trim();
                let size = v
                    .parse::<u8>()
                    .map(|v| v.max(1))
                    .ok()
                    .unwrap_or_else(|| self.size());
                *self = Brush::round(size)
            }
            "%" => {
                let selection = selection.ok_or(())?;
                *self = Brush::Custom((selection.iter().collect(), selection.capacity()));
            }
            _ => Err(())?,
        }
        Ok(())
    }
    pub fn size(&self) -> u8 {
        match self {
            Brush::Rect(size) | Brush::Round(size, _) => *size,
            Brush::Custom(..) => 1,
        }
    }
    pub fn set_size(&mut self, size: u8) {
        let size = size.max(1);
        match self {
            Brush::Rect(s) => *s = size,
            Brush::Round(s, _) => {
                if size != *s {
                    *self = Brush::round(size);
                }
            }
            _ => (),
        }
    }
    pub fn size_increase(&mut self, value: u8) {
        self.set_size(self.size().saturating_add(value));
    }
    pub fn size_decrease(&mut self, value: u8) {
        self.set_size(self.size().saturating_sub(value));
    }
    fn paint_at(&self, selection: &mut Selection, pos: (i32, i32)) {
        let (x, y) = pos;
        match self {
            Brush::Rect(size) => {
                let size = *size as i32;
                selection.insert_rect(crate::utils::Rect::new(
                    x - size / 2,
                    y - size / 2,
                    size,
                    size,
                ));
            }
            Brush::Round(_, shape) | Brush::Custom(shape) => {
                selection.extend_with_bound(
                    shape.0.iter().map(|(dx, dy)| (dx + x, dy + y)),
                    shape.1.offset_by((x, y)),
                );
            }
        }
    }
}

fn _line(p0: (i32, i32), p1: (i32, i32)) -> Vec<(i32, i32)> {
    let (x0, y0) = p0;
    let (x1, y1) = p1;
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut result = Vec::new();
    let mut error = 0;
    let (mut x, mut y) = (x0, y0);
    loop {
        result.push((x, y));
        if x == x1 && y == y1 {
            break;
        }
        let e = error;
        if 2 * e - dy + 2 * dx >= 0 {
            error -= dy;
            x += sx;
        }
        if 2 * e - 2 * dy + dx <= 0 {
            error += dx;
            y += sy;
        }
    }
    result
}

fn _lasso(trace: &[(i32, i32)], brush: &Brush) -> Selection {
    let left = trace.iter().min_by_key(|p| p.0).unwrap().0;
    let right = trace.iter().max_by_key(|p| p.0).unwrap().0;
    let top = trace.iter().min_by_key(|p| p.1).unwrap().1;
    let bottom = trace.iter().max_by_key(|p| p.1).unwrap().1;
    let width = (right - left + 1) as usize;
    let height = (bottom - top + 1) as usize;
    let mut crossings = Grid::new_with(width, height, (false, 0));
    let mut result = Selection::new();
    for i in 0..trace.len() {
        let pos1 = trace[i];
        let pos2 = trace[if i < trace.len() - 1 { i + 1 } else { 0 }];
        let line = _line(pos1, pos2);
        for p in &line {
            brush.paint_at(&mut result, *p);
        }
        for j in 0..line.len() {
            let (x, y) = line[j];
            let y_prev = line[j.saturating_sub(1)].1;
            let y_next = line[(j + 1).min(line.len() - 1)].1;
            let c = crossings.get_unchecked_mut(x - left, y - top);
            c.0 = true;
            c.1 += y_next - y_prev;
        }
    }
    for y in top..=bottom {
        let mut winding_number = 0;
        for x in left..=right {
            let c = crossings.get_unchecked(x - left, y - top);
            winding_number += c.1;
            if c.0 || winding_number != 0 {
                result.insert((x, y));
            }
        }
    }
    result
}

fn _brush(trace: &[(i32, i32)], brush: &Brush) -> Selection {
    let mut result = Selection::new();
    if let Some(mut p1) = trace.first().copied() {
        for p2 in trace {
            for p in _line(p1, *p2) {
                brush.paint_at(&mut result, p);
                p1 = *p2;
            }
        }
    }
    result
}

pub fn brush(
    size: Size,
    trace: &[(i32, i32)],
    symmetry: Symmetry,
    brush: &Brush,
    outline: bool,
) -> Selection {
    let sym = symmetry.helper(size.0 as _, size.1 as _);
    let func = if outline { _brush } else { _lasso };
    let selection = (func)(trace, brush);
    let mut result = selection.clone();
    if symmetry.x {
        result.extend(selection.iter().map(|p| sym.sym_x(p)));
    }
    if symmetry.y {
        result.extend(selection.iter().map(|p| sym.sym_y(p)));
    }
    if symmetry.x && symmetry.y {
        result.extend(selection.iter().map(|p| sym.sym_x_y(p)));
    }
    result
}

fn _rect(bounds: ((i32, i32), (i32, i32))) -> Selection {
    let x0 = bounds.0.0.min(bounds.1.0);
    let y0 = bounds.0.1.min(bounds.1.1);
    let x1 = bounds.0.0.max(bounds.1.0);
    let y1 = bounds.0.1.max(bounds.1.1);
    let mut result = Selection::new();
    for x in x0..=x1 {
        for y in y0..=y1 {
            result.insert((x, y));
        }
    }
    result
}

fn _rect_outline(bounds: ((i32, i32), (i32, i32))) -> Selection {
    let x0 = bounds.0.0.min(bounds.1.0);
    let y0 = bounds.0.1.min(bounds.1.1);
    let x1 = bounds.0.0.max(bounds.1.0);
    let y1 = bounds.0.1.max(bounds.1.1);
    let mut result = Selection::new();
    for x in x0..=x1 {
        result.insert((x, y0));
        result.insert((x, y1));
    }
    for y in y0..=y1 {
        result.insert((x0, y));
        result.insert((x1, y));
    }
    result
}

pub fn rect(
    size: Size,
    bounds: ((i32, i32), (i32, i32)),
    symmetry: Symmetry,
    outline: bool,
) -> Selection {
    let sym = symmetry.helper(size.0 as _, size.1 as _);
    let func = if outline { _rect_outline } else { _rect };
    let mut result = (func)(bounds);
    if symmetry.x {
        result.extend((func)((sym.sym_x(bounds.0), sym.sym_x(bounds.1))));
    }
    if symmetry.y {
        result.extend((func)((sym.sym_y(bounds.0), sym.sym_y(bounds.1))));
    }
    if symmetry.x && symmetry.y {
        result.extend((func)((sym.sym_x_y(bounds.0), sym.sym_x_y(bounds.1))));
    }
    result
}

fn _arc(dx: i32, dy: i32) -> Vec<(i32, i32)> {
    if dy > dx {
        return _arc(dy, dx)
            .into_iter()
            .map(|(y, x)| (x, y))
            .rev()
            .collect();
    }
    let (dx2, dy2) = ((dx * dx) as f32, (dy * dy) as f32);
    let mut arc = Vec::new();
    let (mut x, mut y) = (0, dy / 2);
    let mut error = 0.; // use f32 to avoid overflow
    loop {
        arc.push((x, y));
        if x >= dx / 2 {
            break;
        }
        let ex = (2 * x - dx + 1) as f32 * dy2;
        let ey = (-2 * y + dy + 1) as f32 * dx2;
        let e1 = error + ex / 2. + ey - dy2 / 4.;
        let e2 = error + ex + ey / 2. - dx2 / 4.;
        let mut e3 = e2;
        if e2 <= 0. {
            error += ey;
            e3 += ey + dx2;
            y -= 1;
        }
        if e1 >= 0. || e3 >= 0. {
            error += ex;
            x += 1;
        }
    }
    arc
}

fn _ellipse(bounds: ((i32, i32), (i32, i32))) -> Selection {
    let x0 = bounds.0.0.min(bounds.1.0);
    let y0 = bounds.0.1.min(bounds.1.1);
    let x1 = bounds.0.0.max(bounds.1.0);
    let y1 = bounds.0.1.max(bounds.1.1);
    let dx = x1 - x0;
    let dy = y1 - y0;
    let mut result = Selection::new();
    let mut yy = dy + 1;
    for (x, y) in _arc(dx, dy) {
        if y < yy {
            for xx in x0 + x..=x1 - x {
                result.insert((xx, y0 + y));
                result.insert((xx, y1 - y));
            }
            yy = y;
        }
    }
    result
}

fn _ellipse_outline(bounds: ((i32, i32), (i32, i32))) -> Selection {
    let x0 = bounds.0.0.min(bounds.1.0);
    let y0 = bounds.0.1.min(bounds.1.1);
    let x1 = bounds.0.0.max(bounds.1.0);
    let y1 = bounds.0.1.max(bounds.1.1);
    let dx = x1 - x0;
    let dy = y1 - y0;
    let mut result = Selection::new();
    for (x, y) in _arc(dx, dy) {
        result.insert((x0 + x, y0 + y));
        result.insert((x1 - x, y0 + y));
        result.insert((x0 + x, y1 - y));
        result.insert((x1 - x, y1 - y));
    }
    result
}

pub fn ellipse(
    size: Size,
    bounds: ((i32, i32), (i32, i32)),
    symmetry: Symmetry,
    outline: bool,
) -> Selection {
    let sym = symmetry.helper(size.0 as _, size.1 as _);
    let func = if outline { _ellipse_outline } else { _ellipse };
    let mut result = (func)(bounds);
    if symmetry.x {
        result.extend((func)((sym.sym_x(bounds.0), sym.sym_x(bounds.1))));
    }
    if symmetry.y {
        result.extend((func)((sym.sym_y(bounds.0), sym.sym_y(bounds.1))));
    }
    if symmetry.x && symmetry.y {
        result.extend((func)((sym.sym_x_y(bounds.0), sym.sym_x_y(bounds.1))));
    }
    result
}

fn _line_brush(bounds: ((i32, i32), (i32, i32)), brush: &Brush) -> Selection {
    let mut result = Selection::new();
    for p in _line(bounds.0, bounds.1) {
        brush.paint_at(&mut result, p);
    }
    result
}

pub fn line(
    size: Size,
    bounds: ((i32, i32), (i32, i32)),
    symmetry: Symmetry,
    brush: &Brush,
) -> Selection {
    let sym = symmetry.helper(size.0 as _, size.1 as _);
    let mut result: Selection = (_line_brush)((bounds.0, bounds.1), brush).iter().collect();
    if symmetry.x {
        result.extend((_line_brush)(
            (sym.sym_x(bounds.0), sym.sym_x(bounds.1)),
            brush,
        ));
    }
    if symmetry.y {
        result.extend((_line_brush)(
            (sym.sym_y(bounds.0), sym.sym_y(bounds.1)),
            brush,
        ));
    }
    if symmetry.x && symmetry.y {
        result.extend((_line_brush)(
            (sym.sym_x_y(bounds.0), sym.sym_x_y(bounds.1)),
            brush,
        ));
    }
    result
}

fn _flood(image: &Image, mask: Option<&Selection>, cursor: (i32, i32), tolerance: u8) -> Selection {
    let (width, height) = (image.width(), image.height());
    let mut fringe = Vec::new();
    let mut visited = Grid::new_with(width, height, false);
    let mut result = Selection::new();
    if let Some(start) = image.get(cursor.0, cursor.1) {
        fringe.push(cursor);
        while let Some(node) = fringe.pop() {
            let node_color = {
                *visited.get_unchecked_mut(node.0, node.1) = true;
                image.get_unchecked(node.0, node.1)
            };
            if node_color.diff(start) <= tolerance {
                result.insert(node);
                for nbr in [
                    (node.0 - 1, node.1),
                    (node.0 + 1, node.1),
                    (node.0, node.1 - 1),
                    (node.0, node.1 + 1),
                ] {
                    if visited.get(nbr.0, nbr.1).is_some_and(|v| !*v)
                        && mask.is_none_or(|m| m.contains(nbr))
                    {
                        fringe.push(nbr);
                    }
                }
            }
        }
    }
    result
}

fn _flood_discon(
    image: &Image,
    mask: Option<&Selection>,
    cursor: (i32, i32),
    tolerance: u8,
) -> Selection {
    let mut result = Selection::new();
    if let Some(start) = image.get(cursor.0, cursor.1) {
        result.extend(select(image, mask, |c| c.diff(start) <= tolerance))
    }
    result
}

pub fn flood(
    image: &Image,
    mask: Option<&Selection>,
    cursor: (i32, i32),
    tolerance: u8,
    symmetry: Symmetry,
    discon: bool,
) -> Selection {
    let (x, y) = cursor;
    let (w, h) = (image.width() as i32, image.height() as i32);
    let sym = symmetry.helper(w, h);
    let func = if discon { _flood_discon } else { _flood };
    let mut result = (func)(image, mask, cursor, tolerance);
    if symmetry.x {
        result.extend((func)(image, mask, sym.sym_x((x, y)), tolerance));
    }
    if symmetry.y {
        result.extend((func)(image, mask, sym.sym_y((x, y)), tolerance));
    }
    if symmetry.x && symmetry.y {
        result.extend((func)(image, mask, sym.sym_x_y((x, y)), tolerance));
    }
    result
}

fn select<P: FnMut(Color) -> bool>(
    image: &Image,
    mask: Option<&Selection>,
    mut predicate: P,
) -> Vec<(i32, i32)> {
    if let Some(m) = mask {
        m.iter()
            .filter(|(i, j)| image.get(*i, *j).is_some_and(|c| (predicate)(*c)))
            .collect()
    } else {
        (0..image.width() as i32 * image.height() as i32)
            .map(|ij| (ij % image.width() as i32, ij / image.width() as i32))
            .filter(|(i, j)| (predicate)(*image.get_unchecked(*i, *j)))
            .collect()
    }
}

pub fn r#move(selection: &Selection, dir: (i32, i32)) -> Selection {
    let mut result = selection.clone();
    result.offset_by(dir);
    result
}

pub fn flip_x(image: &Image, selection: &Selection, symmetry: Symmetry) -> Selection {
    let width = image.width();
    let offset = if symmetry.x { symmetry.x_offset } else { 0 };
    selection
        .iter()
        .map(|(x, y)| (width as i32 - 1 - x + offset, y))
        .collect()
}

pub fn flip_y(image: &Image, selection: &Selection, symmetry: Symmetry) -> Selection {
    let height = image.height();
    let offset = if symmetry.y { symmetry.y_offset } else { 0 };
    selection
        .iter()
        .map(|(x, y)| (x, height as i32 - 1 - y + offset))
        .collect()
}

pub fn pixel_perfect_filter(trace: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut new = Vec::new();
    let (mut p0, mut p1) = (trace[0], trace[0]);
    for (i, p2) in trace.iter().enumerate() {
        let p2 = *p2;
        if i >= 2 && (p0.0 == p1.0 && p1.1 == p2.1 || p0.1 == p1.1 && p1.0 == p2.0) {
            new.pop();
            p1 = p2;
        } else {
            (p0, p1) = (p1, p2);
        }
        new.push(p2);
    }
    new
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ellipse_symmetry() {
        for i in 0..50 {
            for j in 0..i {
                let e1: Selection = _ellipse_outline(((0, 0), (i, j)))
                    .iter()
                    .map(|(y, x)| (x, y))
                    .collect();
                let e2 = _ellipse_outline(((j, i), (0, 0)));
                assert_eq!(e1.to_vec(), e2.to_vec());

                let e1: Selection = _ellipse(((0, 0), (i, j)))
                    .iter()
                    .map(|(y, x)| (x, y))
                    .collect();
                let e2 = _ellipse(((j, i), (0, 0)));
                assert_eq!(e1.to_vec(), e2.to_vec());
            }
        }
    }
}
