use crate::color::Color;
use crate::grid::Grid;
use crate::image::{Image, Symmetry};

#[derive(Clone)]
pub enum Brush {
    Rect(u8),
    Round(u8),
    Custom(Selection),
}

impl std::fmt::Display for Brush {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Brush::Rect(size) => f.write_fmt(format_args!("[{size}]"))?,
            Brush::Round(size) => f.write_fmt(format_args!("({size})"))?,
            Brush::Custom(..) => f.write_fmt(format_args!("custom"))?,
        }
        Ok(())
    }
}

impl Brush {
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
                *self = Brush::Round(size);
            }
            "%" => {
                *self = Brush::Custom(selection.ok_or(())?);
            }
            _ => Err(())?,
        }
        Ok(())
    }
    pub fn size(&self) -> u8 {
        match self {
            Brush::Rect(size) | Brush::Round(size) => *size,
            Brush::Custom(..) => 1,
        }
    }
    pub fn at(&self, x: i32, y: i32) -> Vec<(i32, i32)> {
        match self {
            Brush::Rect(size) => {
                let size = *size as i32;
                let mut brush = Vec::new();
                for i in -size / 2..(size + 1) / 2 {
                    for j in -size / 2..(size + 1) / 2 {
                        brush.push((x + i, y + j));
                    }
                }
                brush
            }
            Brush::Round(size) => {
                let size = *size as i32;
                let mut brush = Vec::new();
                for i in -size / 2..(size / 2 + 1) {
                    for j in -size / 2..(size / 2 + 1) {
                        if 4 * (i * i + j * j) <= size * size {
                            brush.push((x + i, y + j));
                        }
                    }
                }
                brush
            }
            Brush::Custom(shape) => shape.iter().map(|(dx, dy)| (x + dx, y + dy)).collect(),
        }
    }
}

use std::collections::HashSet;
pub type Selection = HashSet<(i32, i32)>;

pub fn line(p0: (i32, i32), p1: (i32, i32)) -> Vec<(i32, i32)> {
    let (x0, y0) = p0;
    let (x1, y1) = p1;
    if x0 > x1 {
        let mut result = line(p1, p0);
        result.reverse();
        return result;
    }
    let dx = x1 - x0;
    let dy = (y1 - y0).abs();
    if dy > dx {
        return line((y0, x0), (y1, x1))
            .iter()
            .map(|(x, y)| (*y, *x))
            .collect();
    }
    let mut result = Vec::new();
    let yi = if y1 > y0 { 1 } else { -1 };
    let mut d = 2 * dy - dx;
    let mut y = y0;
    for x in x0..x1 {
        result.push((x, y));
        if d > 0 {
            y += yi;
            d -= 2 * dx;
        }
        d += 2 * dy;
    }
    result.push(p1);
    result
}

fn _lasso(trace: Vec<(i32, i32)>, brush: &Brush) -> Selection {
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
        let line = line(pos1, pos2);
        for (x, y) in &line {
            for p in brush.at(*x, *y) {
                result.insert(p);
            }
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

fn _brush(trace: Vec<(i32, i32)>, brush: &Brush) -> Selection {
    let mut result = Selection::new();
    if let Some(mut p1) = trace.first().copied() {
        for p2 in trace {
            for (x, y) in line(p1, p2) {
                for p in brush.at(x, y) {
                    result.insert(p);
                }
                p1 = p2;
            }
        }
    }
    result
}

pub fn brush(
    image: &Image,
    trace: Vec<(i32, i32)>,
    symmetry: Symmetry,
    brush: &Brush,
    outline: bool,
) -> Selection {
    let (w, h) = (image.width() as i32, image.height() as i32);
    let sym = symmetry.helper(w, h);
    let func = if outline { _brush } else { _lasso };
    let selection = (func)(trace, brush);
    let mut result = selection.clone();
    if symmetry.x {
        result.extend(selection.iter().map(|p| sym.sym_x(*p)));
    }
    if symmetry.y {
        result.extend(selection.iter().map(|p| sym.sym_y(*p)));
    }
    if symmetry.x && symmetry.y {
        result.extend(selection.iter().map(|p| sym.sym_x_y(*p)));
    }
    result
}

fn _rect(bounds: ((i32, i32), (i32, i32))) -> Selection {
    let x0 = bounds.0 .0.min(bounds.1 .0);
    let y0 = bounds.0 .1.min(bounds.1 .1);
    let x1 = bounds.0 .0.max(bounds.1 .0);
    let y1 = bounds.0 .1.max(bounds.1 .1);
    let mut result = Selection::new();
    for x in x0..=x1 {
        for y in y0..=y1 {
            result.insert((x, y));
        }
    }
    result
}

fn _rect_outline(bounds: ((i32, i32), (i32, i32))) -> Selection {
    let x0 = bounds.0 .0.min(bounds.1 .0);
    let y0 = bounds.0 .1.min(bounds.1 .1);
    let x1 = bounds.0 .0.max(bounds.1 .0);
    let y1 = bounds.0 .1.max(bounds.1 .1);
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
    image: &Image,
    bounds: ((i32, i32), (i32, i32)),
    symmetry: Symmetry,
    outline: bool,
) -> Selection {
    let (w, h) = (image.width() as i32, image.height() as i32);
    let sym = symmetry.helper(w, h);
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
                        && mask.is_none_or(|m| m.contains(&nbr))
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
        result.extend(&select(image, mask, |c| c.diff(start) <= tolerance))
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
        result.extend(&(func)(image, mask, sym.sym_x((x, y)), tolerance));
    }
    if symmetry.y {
        result.extend(&(func)(image, mask, sym.sym_y((x, y)), tolerance));
    }
    if symmetry.x && symmetry.y {
        result.extend(&(func)(image, mask, sym.sym_x_y((x, y)), tolerance));
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
            .copied()
            .collect()
    } else {
        (0..image.width() as i32 * image.height() as i32)
            .map(|ij| (ij % image.width() as i32, ij / image.width() as i32))
            .filter(|(i, j)| (predicate)(*image.get_unchecked(*i, *j)))
            .collect()
    }
}

pub fn r#move(selection: &Selection, dir: (i32, i32)) -> Selection {
    let mut result = Selection::new();
    result.extend(selection.iter().map(|(x, y)| (*x + dir.0, *y + dir.1)));
    result
}

pub fn flip_x(image: &Image, selection: &Selection, symmetry: Symmetry) -> Selection {
    let width = image.width();
    let offset = if symmetry.x { symmetry.x_offset } else { 0 };
    selection
        .iter()
        .map(|(x, y)| (width as i32 - 1 - x + offset, *y))
        .collect()
}

pub fn flip_y(image: &Image, selection: &Selection, symmetry: Symmetry) -> Selection {
    let height = image.height();
    let offset = if symmetry.y { symmetry.y_offset } else { 0 };
    selection
        .iter()
        .map(|(x, y)| (*x, height as i32 - 1 - y + offset))
        .collect()
}
