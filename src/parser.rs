use crate::color::Color;

#[derive(Clone, Copy)]
pub struct OnOff(pub bool);
impl From<bool> for OnOff {
    fn from(value: bool) -> Self {
        OnOff(value)
    }
}
impl core::str::FromStr for OnOff {
    type Err = String;
    fn from_str(expr: &str) -> Result<Self, String> {
        match expr.trim() {
            "on" | "true" | "yes" | "y" => Ok(OnOff(true)),
            "off" | "false" | "no" | "n" => Ok(OnOff(false)),
            _ => Err("Cannot parse value".into()),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Int(pub i32);
impl core::str::FromStr for Int {
    type Err = String;
    fn from_str(expr: &str) -> Result<Self, String> {
        if let Ok(value) = expr.trim().parse() {
            Ok(Int(value))
        } else {
            Err("Cannot parse value".into())
        }
    }
}

#[derive(Clone, Copy)]
pub struct Float(pub f32);
impl core::str::FromStr for Float {
    type Err = String;
    fn from_str(expr: &str) -> Result<Self, String> {
        if let Ok(value) = expr.trim().parse() {
            Ok(Float(value))
        } else {
            Err("Cannot parse value".into())
        }
    }
}

#[derive(PartialEq, Eq)]
pub struct Size(pub u32, pub u32);

fn parse_size_with(expr: &str, sep: char) -> Option<Size> {
    let (left, right) = expr.split_once(sep)?;
    let w: u32 = left.trim().parse().ok()?;
    let h: u32 = right.trim().parse().ok()?;
    (w > 0 && h > 0).then_some(Size(w, h))
}

impl core::str::FromStr for Size {
    type Err = String;
    fn from_str(expr: &str) -> Result<Self, String> {
        parse_size_with(expr, ' ')
            .or_else(|| parse_size_with(expr, ','))
            .or_else(|| parse_size_with(expr, 'x'))
            .ok_or("Cannot parse size".into())
    }
}

fn parse_hex(expr: &str) -> Option<Color> {
    let mut chars = expr.trim().chars().peekable();
    let r = u8::from_str_radix(&format!("{}{}", chars.next()?, chars.next()?), 16).ok()?;
    let g = u8::from_str_radix(&format!("{}{}", chars.next()?, chars.next()?), 16).ok()?;
    let b = u8::from_str_radix(&format!("{}{}", chars.next()?, chars.next()?), 16).ok()?;
    if chars.peek() == Some(&'/') {
        chars.next();
        let a: u8 = chars.collect::<String>().parse().ok()?;
        Some(Color(r, g, b, (a.min(100) as f32 * 2.55) as u8))
    } else if chars.peek().is_some() {
        let a = u8::from_str_radix(&format!("{}{}", chars.next()?, chars.next()?), 16).ok()?;
        Some(Color(r, g, b, a))
    } else {
        Some(Color(r, g, b, 255))
    }
}

fn parse_vec(expr: &str, sep: char) -> Option<Color> {
    let exprs: Vec<_> = expr.split(sep).collect();
    match exprs.len() {
        3 => {
            let r = exprs[0].trim().parse().ok()?;
            let g = exprs[1].trim().parse().ok()?;
            let b = exprs[2].trim().parse().ok()?;
            Some(Color(r, g, b, 255))
        }
        4 => {
            let r = exprs[0].trim().parse().ok()?;
            let g = exprs[1].trim().parse().ok()?;
            let b = exprs[2].trim().parse().ok()?;
            let a = exprs[3].trim().parse().ok()?;
            Some(Color(r, g, b, a))
        }
        _ => None,
    }
}

fn parse_named(expr: &str) -> Option<Color> {
    match expr.to_lowercase().as_str() {
        "white" => Some(Color::WHITE),
        "lightgray" => Some(Color::LIGHTGRAY),
        "gray" => Some(Color::GRAY),
        "darkgray" => Some(Color::DARKGRAY),
        "black" => Some(Color::BLACK),
        "red" => Some(Color::RED),
        "orange" => Some(Color::ORANGE),
        _ => None,
    }
}

impl core::str::FromStr for Color {
    type Err = String;
    fn from_str(expr: &str) -> Result<Self, String> {
        if expr.starts_with('#') {
            parse_hex(expr.strip_prefix('#').unwrap())
        } else {
            parse_hex(expr)
                .or_else(|| parse_vec(expr, ','))
                .or_else(|| parse_vec(expr, ' '))
                .or_else(|| parse_named(expr))
        }
        .ok_or("Cannot parse color".into())
    }
}
