use nanoserde::{DeBin, SerBin};

use crate::console::Console;
#[repr(C)]
#[derive(PartialEq, Clone, Copy, Eq, Hash, SerBin, DeBin, Debug)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);
impl From<(u8, u8, u8)> for Color {
    fn from(value: (u8, u8, u8)) -> Self {
        Self(value.0, value.1, value.2, 255)
    }
}
impl From<(u8, u8, u8, u8)> for Color {
    fn from(value: (u8, u8, u8, u8)) -> Self {
        let alpha = value.3;
        if alpha == 0 {
            Color(0, 0, 0, 0)
        } else {
            Color(value.0, value.1, value.2, alpha)
        }
    }
}
impl From<[u8; 4]> for Color {
    fn from(value: [u8; 4]) -> Self {
        let alpha = value[3];
        if alpha == 0 {
            Color(0, 0, 0, 0)
        } else {
            Color(value[0], value[1], value[2], alpha)
        }
    }
}
impl From<Color> for (u8, u8, u8, u8) {
    fn from(val: Color) -> Self {
        (val.0, val.1, val.2, val.3)
    }
}
impl From<Color> for [u8; 4] {
    fn from(val: Color) -> Self {
        [val.0, val.1, val.2, val.3]
    }
}

impl std::cmp::PartialOrd for Color {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Color {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.hue().total_cmp(&other.hue()) {
            std::cmp::Ordering::Less => std::cmp::Ordering::Less,
            std::cmp::Ordering::Greater => std::cmp::Ordering::Greater,
            std::cmp::Ordering::Equal => self.brightness().total_cmp(&other.brightness()).reverse(),
        }
    }
}

impl Color {
    pub fn diff(&self, other: &Color) -> u8 {
        let s = self.to_linear(false);
        let o = other.to_linear(false);
        (((s[0] * s[3] - o[0] * o[3]).abs())
            .max((s[1] * s[3] - o[1] * o[3]).abs())
            .max((s[2] * s[3] - o[2] * o[3]).abs())
            .max((s[3] - o[3]).abs())
            * 255.)
            .round() as _
    }
    pub const WHITE: Color = Color(255, 255, 255, 255);
    pub const LIGHTGRAY: Color = Color(192, 192, 192, 255);
    pub const GRAY: Color = Color(128, 128, 128, 255);
    pub const DARKGRAY: Color = Color(64, 64, 64, 255);
    pub const BLACK: Color = Color(0, 0, 0, 255);
    pub const RED: Color = Color(255, 0, 0, 255);
    pub const ORANGE: Color = Color(255, 128, 0, 255);
    pub fn alpha(self, a: u8) -> Color {
        (self.0, self.1, self.2, a).into()
    }
    pub fn hex(&self) -> String {
        let mut result = format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2);
        if self.3 != 255 {
            result.push_str(&format!("/{:02.0}", self.3 as f32 / 2.55))
        }
        result
    }
    pub fn print(&self, console: &mut Console, x: i32, y: i32, fg: Color, bg: Color, str: &str) {
        console.print(str, x, y, Some(self.marker()), (*self).into());
        console.print(
            &format!(" {}", self.hex()),
            x + str.chars().count() as i32,
            y,
            fg.into(),
            bg.into(),
        );
    }
    pub fn brightness(&self) -> f32 {
        let color: [f32; 4] = (*self).to_linear(false);
        (0.299 * color[0] + 0.587 * color[1] + 0.114 * color[2]) * color[3] + (1. - color[3])
    }
    pub fn chroma(&self) -> f32 {
        let color: [f32; 4] = (*self).to_linear(false);
        let max = color[0].max(color[1]).max(color[2]);
        let min = color[0].min(color[1]).min(color[2]);
        (max - min) * color[3]
    }
    pub fn hue(&self) -> f32 {
        let color: [f32; 4] = (*self).to_linear(false);
        let max = color[0].max(color[1]).max(color[2]);
        let min = color[0].min(color[1]).min(color[2]);
        if max == min {
            0.
        } else if max == color[0] {
            ((color[1] - color[2]) / (max - min)).rem_euclid(6.) / 6.
        } else if max == color[1] {
            ((color[2] - color[0]) / (max - min) + 2.).rem_euclid(6.) / 6.
        } else {
            ((color[0] - color[1]) / (max - min) + 4.).rem_euclid(6.) / 6.
        }
    }
    pub fn marker(&self) -> Color {
        if self.brightness() > 0.5 {
            Color::BLACK
        } else {
            Color::WHITE
        }
    }
    #[inline]
    pub fn to_linear(self, srgb: bool) -> [f32; 4] {
        if srgb {
            [
                (self.0 as f32 / 255.).decode(),
                (self.1 as f32 / 255.).decode(),
                (self.2 as f32 / 255.).decode(),
                (self.3 as f32 / 255.),
            ]
        } else {
            [
                (self.0 as f32 / 255.),
                (self.1 as f32 / 255.),
                (self.2 as f32 / 255.),
                (self.3 as f32 / 255.),
            ]
        }
    }
    #[inline]
    pub fn from_linear(value: [f32; 4], srgb: bool) -> Self {
        if srgb {
            (
                (value[0].clamp(0., 1.).encode() * 255.).round() as _,
                (value[1].clamp(0., 1.).encode() * 255.).round() as _,
                (value[2].clamp(0., 1.).encode() * 255.).round() as _,
                (value[3].clamp(0., 1.) * 255.).round() as _,
            )
                .into()
        } else {
            (
                (value[0].clamp(0., 1.) * 255.).round() as _,
                (value[1].clamp(0., 1.) * 255.).round() as _,
                (value[2].clamp(0., 1.) * 255.).round() as _,
                (value[3].clamp(0., 1.) * 255.).round() as _,
            )
                .into()
        }
    }
    pub fn blend(&self, color: Color, srgb: bool) -> Color {
        let dst: [f32; 4] = (*self).to_linear(srgb);
        let src: [f32; 4] = color.to_linear(srgb);
        let alpha = dst[3] * (1. - src[3]) + src[3];
        if alpha < 1. / 255. {
            (0, 0, 0, 0).into()
        } else {
            let c_dst = dst[3] * (1. - src[3]) / alpha;
            let c_src = src[3] / alpha;
            Color::from_linear(
                [
                    (dst[0] * c_dst + src[0] * c_src),
                    (dst[1] * c_dst + src[1] * c_src),
                    (dst[2] * c_dst + src[2] * c_src),
                    alpha,
                ],
                srgb,
            )
        }
    }
    // this uses mixing in the correct linear space, but also avoids the uneven brightness perception
    // caused by sRGB encoding
    // we first take even mixes in the sRGB space, decode them to linear space and project to the
    // line segment, and then encode them back to sRGB
    // unfortunately this means inconsistent results for colors with alpha
    // e.g. reasonably, mixing ffffff and 000000/00 should get a linear progression in alpha
    // but blending this on 000000 produces the uneven result which we don't want
    // without srgb we do get consistent result (consistently wrong, that is)
    pub fn mix(&self, color: Color, t: f32, srgb: bool) -> Color {
        if srgb {
            let color1 = self;
            let color2 = &color;
            let l1 = color1.to_linear(true);
            let l2 = color2.to_linear(true);
            let v = [l2[0] - l1[0], l2[1] - l1[1], l2[2] - l1[2]];
            let d2 = (v[0]).powi(2) + (v[1]).powi(2) + (v[2]).powi(2);
            if d2 == 0. {
                Color::from_linear(mixf4(l1, l2, t), false)
            } else {
                let p1 = color1.to_linear(false);
                let p2 = color2.to_linear(false);
                let alpha = mixf(p1[3], p2[3], t);
                if alpha < 1. / 255. {
                    (0, 0, 0, 0).into()
                } else {
                    let p = [
                        mixf(p1[0] * p1[3] / alpha, p2[0] * p2[3] / alpha, t),
                        mixf(p1[1] * p1[3] / alpha, p2[1] * p2[3] / alpha, t),
                        mixf(p1[2] * p1[3] / alpha, p2[2] * p2[3] / alpha, t),
                    ];
                    let s = ((p[0].decode() - l1[0]) * v[0]
                        + (p[1].decode() - l1[1]) * v[1]
                        + (p[2].decode() - l1[2]) * v[2])
                        / d2;
                    let a = mixf(p1[3], p2[3], s);
                    Color::from_linear(
                        [
                            mixf(l1[0] * l1[3] / a, l2[0] * l2[3] / a, s),
                            mixf(l1[1] * l1[3] / a, l2[1] * l2[3] / a, s),
                            mixf(l1[2] * l1[3] / a, l2[2] * l2[3] / a, s),
                            alpha,
                        ],
                        true,
                    )
                }
            }
        } else {
            let p1 = self.to_linear(false);
            let p2 = color.to_linear(false);
            let alpha = mixf(p1[3], p2[3], t);
            if alpha < 1. / 255. {
                (0, 0, 0, 0).into()
            } else {
                Color::from_linear(
                    [
                        mixf(p1[0] * p1[3] / alpha, p2[0] * p2[3] / alpha, t),
                        mixf(p1[1] * p1[3] / alpha, p2[1] * p2[3] / alpha, t),
                        mixf(p1[2] * p1[3] / alpha, p2[2] * p2[3] / alpha, t),
                        alpha,
                    ],
                    false,
                )
            }
        }
    }
}

mod srgb {
    // shadow the constants
    const U: f32 = 0.04045;
    const V: f32 = 0.0031308;
    const A: f32 = 12.92;
    const C: f32 = 0.055;
    const GAMMA: f32 = 2.4;
    pub trait Srgb {
        fn decode(self) -> f32;
        fn encode(self) -> f32;
    }
    impl Srgb for f32 {
        #[inline]
        fn decode(self) -> f32 {
            let u = self;
            if u <= U {
                u / A
            } else {
                ((u + C) / (1. + C)).powf(GAMMA)
            }
        }
        #[inline]
        fn encode(self) -> f32 {
            let v = self;
            if v <= V {
                A * v
            } else {
                (1. + C) * v.powf(1. / GAMMA) - C
            }
        }
    }
}
use srgb::Srgb;

#[derive(Clone, Copy)]
pub enum ColorMode {
    Set(Color),
    Blend(Color, bool),
    Clear,
}

impl ColorMode {
    pub fn apply(&self, pixel: &mut Color) {
        *pixel = match self {
            ColorMode::Set(color) => *color,
            ColorMode::Blend(color, srgb) => pixel.blend(*color, *srgb),
            ColorMode::Clear => Color(0, 0, 0, 0),
        }
    }
}

#[inline]
fn mixf(a: f32, b: f32, t: f32) -> f32 {
    a * (1. - t) + b * t
}
#[inline]
fn mixf4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        mixf(a[0], b[0], t),
        mixf(a[1], b[1], t),
        mixf(a[2], b[2], t),
        mixf(a[3], b[3], t),
    ]
}

pub fn gradient(color1: Color, color2: Color, count: u32, srgb: bool) -> Vec<Color> {
    (0..count + 2)
        .map(|i| color1.mix(color2, i as f32 / (count as f32 + 1.), srgb))
        .collect()
}
