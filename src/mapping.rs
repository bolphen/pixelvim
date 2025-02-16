use miniquad::*;
use std::collections::{HashMap, HashSet};

#[derive(PartialEq, Eq, Clone)]
#[rustfmt::skip]
pub enum Action {
    Left, Right, Up, Down,

    Brush, BrushFilled, Flood, FloodAll, RectFilled, RectOutline, Move,

    BrushToggle, FloodToggle, RectToggle,

    Paste, Yank, Cut, Delete,

    Cancel, Enter,

    NormalErase, NormalReplace, NormalBlend, NormalToggle,

    VisualAdd, VisualSub, VisualToggle,

    PixelPerfect,

    Normal, Visual, Command,

    Go, TabFront, TabBack,

    DoCommand(String),
}

mod action {
    use super::Action::{self, *};

    impl std::fmt::Display for Action {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Left => f.write_str("left"),
                Right => f.write_str("right"),
                Up => f.write_str("up"),
                Down => f.write_str("down"),
                RectFilled => f.write_str("rect_filled"),
                RectOutline => f.write_str("rect_outline"),
                RectToggle => f.write_str("rect_toggle"),
                Brush => f.write_str("brush"),
                BrushFilled => f.write_str("brush_filled"),
                BrushToggle => f.write_str("brush_toggle"),
                Flood => f.write_str("flood"),
                FloodAll => f.write_str("flood_all"),
                FloodToggle => f.write_str("flood_toggle"),
                Move => f.write_str("move"),
                Paste => f.write_str("paste"),
                Yank => f.write_str("yank"),
                Cut => f.write_str("cut"),
                Delete => f.write_str("delete"),
                Cancel => f.write_str("cancel"),
                Enter => f.write_str("enter"),
                PixelPerfect => f.write_str("pixel_perfect"),
                Normal => f.write_str("normal"),
                NormalBlend => f.write_str("normal_blend"),
                NormalReplace => f.write_str("normal_replace"),
                NormalErase => f.write_str("normal_erase"),
                NormalToggle => f.write_str("normal_toggle"),
                Visual => f.write_str("visual"),
                VisualAdd => f.write_str("visual_add"),
                VisualSub => f.write_str("visual_sub"),
                VisualToggle => f.write_str("visual_toggle"),
                Command => f.write_str("command"),
                Go => f.write_str("go"),
                TabFront => f.write_str("tab_front"),
                TabBack => f.write_str("tab_back"),
                DoCommand(cmd) => f.write_str(&format!(":{cmd}")),
            }
        }
    }
    impl std::str::FromStr for Action {
        type Err = String;
        fn from_str(expr: &str) -> Result<Self, String> {
            let expr = expr.trim();
            Ok(match expr {
                "left" | "Left" => Left,
                "right" | "Right" => Right,
                "up" | "Up" => Up,
                "down" | "Down" => Down,
                "rect_filled" | "RectFilled" => RectFilled,
                "rect_outline" | "RectOutline" => RectOutline,
                "rect_toggle" | "RectToggle" => RectToggle,
                "brush" | "Brush" => Brush,
                "brush_filled" | "BrushFilled" => BrushFilled,
                "brush_toggle" | "BrushToggle" => BrushToggle,
                "flood" | "Flood" => Flood,
                "flood_all" | "FloodAll" => FloodAll,
                "flood_toggle" | "FloodToggle" => FloodToggle,
                "move" | "Move" => Move,
                "paste" | "Paste" => Paste,
                "yank" | "Yank" => Yank,
                "cut" | "Cut" => Cut,
                "delete" | "Delete" => Delete,
                "cancel" | "Cancel" => Cancel,
                "enter" | "Enter" => Enter,
                "pixel_perfect" | "PixelPerfect" => PixelPerfect,
                "normal" | "Normal" => Normal,
                "normal_blend" | "NormalBlend" => NormalBlend,
                "normal_replace" | "NormalReplace" => NormalReplace,
                "normal_erase" | "NormalErase" => NormalErase,
                "normal_toggle" | "NormalToggle" => NormalToggle,
                "visual" | "Visual" => Visual,
                "visual_add" | "VisualAdd" => VisualAdd,
                "visual_sub" | "VisualSub" => VisualSub,
                "visual_toggle" | "VisualToggle" => VisualToggle,
                "command" | "Command" => Command,
                "go" | "Go" => Go,
                "tab_front" | "TabFront" => TabFront,
                "tab_back" | "TabBack" => TabBack,
                _ => {
                    if let Some(expr) = expr.strip_prefix(':') {
                        DoCommand(expr.into())
                    } else {
                        Err("Unknown action")?
                    }
                }
            })
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct Key {
    code: KeyCode,
    ctrl: bool,
    shift: bool,
}
impl Key {
    pub fn new(code: KeyCode, ctrl: bool, shift: bool) -> Self {
        Key { code, ctrl, shift }
    }
    pub fn string(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("C-");
        }
        if self.shift {
            s.push_str("S-");
        }
        s.push_str(&keycode::keycode_to_str(self.code).to_ascii_lowercase());
        s
    }
}

mod keycode {
    use super::Key;
    use miniquad::KeyCode::{self, *};
    #[rustfmt::skip]
    pub fn char_to_keycode(char: char) -> KeyCode {
        match char {
            '0' => Key0, '1' => Key1, '2' => Key2, '3' => Key3, '4' => Key4,
            '5' => Key5, '6' => Key6, '7' => Key7, '8' => Key8, '9' => Key9,

            'a' => A, 'b' => B, 'c' => C, 'd' => D, 'e' => E, 'f' => F,
            'g' => G, 'h' => H, 'i' => I, 'j' => J, 'k' => K, 'l' => L,
            'm' => M, 'n' => N, 'o' => O, 'p' => P, 'q' => Q, 'r' => R,
            's' => S, 't' => T, 'u' => U, 'v' => V, 'w' => W, 'x' => X,
            'y' => Y, 'z' => Z,

            '\'' => Apostrophe, ',' => Comma, '-' => Minus,
            '.' => Period, '/' => Slash, ';' => Semicolon, '=' => Equal,
            '[' => LeftBracket, '\\' => Backslash, ']' => RightBracket,
            '`' => GraveAccent,

            _ => Unknown,
        }
    }
    #[rustfmt::skip]
    pub fn str_to_keycode(str: &str) -> KeyCode {
        match str {
            "f1" => F1, "f2" => F2, "f3" => F3, "f4" => F4, "f5" => F5,
            "f6" => F6, "f7" => F7, "f8" => F8, "f9" => F9, "f10" => F10,
            "f11" => F11, "f12" => F12, "f13" => F13, "f14" => F14, "f15" => F15,
            "f16" => F16, "f17" => F17, "f18" => F18, "f19" => F19, "f20" => F20,
            "f21" => F21, "f22" => F22, "f23" => F23, "f24" => F24, "f25" => F25,

            "space" => Space,
            "esc" | "escape" => Escape,
            "enter" => Enter,
            "tab" => Tab,
            "backspace" => Backspace,
            "insert" => Insert,
            "delete" => Delete,
            "right" => Right,
            "left" => Left,
            "down" => Down,
            "up" => Up,
            _ => Unknown,
        }
    }
    #[rustfmt::skip]
    pub fn keycode_to_str(k: KeyCode) -> &'static str {
        match k {
            Key0 => "0", Key1 => "1", Key2 => "2", Key3 => "3", Key4 => "4",
            Key5 => "5", Key6 => "6", Key7 => "7", Key8 => "8", Key9 => "9",

            A => "a", B => "b", C => "c", D => "d", E => "e", F => "f",
            G => "g", H => "h", I => "i", J => "j", K => "k", L => "l",
            M => "m", N => "n", O => "o", P => "p", Q => "q", R => "r",
            S => "s", T => "t", U => "u", V => "v", W => "w", X => "x",
            Y => "y", Z => "z",

            Apostrophe => "\'", Comma => ",", Minus => "-",
            Period => ".", Slash => "/", Semicolon => ";", Equal => "=",
            LeftBracket => "[", Backslash => "\\", RightBracket => "]",
            GraveAccent => "`",

            F1 => "F1", F2 => "F2", F3 => "F3", F4 => "F4", F5 => "F5",
            F6 => "F6", F7 => "F7", F8 => "F8", F9 => "F9", F10 => "F10",
            F11 => "F11", F12 => "F12", F13 => "F13", F14 => "F14", F15 => "F15",
            F16 => "F16", F17 => "F17", F18 => "F18", F19 => "F19", F20 => "F20",
            F21 => "F21", F22 => "F22", F23 => "F23", F24 => "F24", F25 => "F25",

            Space => "space",
            Escape => "esc",
            Enter => "enter",
            Tab => "tab",
            Backspace => "backspace",
            Insert => "insert",
            Delete => "delete",
            Right => "right",
            Left => "left",
            Down => "down",
            Up => "up",

            _ => "unknown",
        }
    }

    impl std::str::FromStr for Key {
        type Err = String;
        fn from_str(expr: &str) -> Result<Self, String> {
            let expr = expr.trim().to_ascii_lowercase();
            match expr.len() {
                1 => {
                    let char = expr.chars().next().expect("");
                    Ok(Key {
                        code: char_to_keycode(char),
                        ctrl: false,
                        shift: false,
                    })
                }
                _ => {
                    if expr.len() == 3 && expr.starts_with("c-") {
                        let char = expr.chars().nth(2).expect("");
                        Ok(Key {
                            code: char_to_keycode(char),
                            ctrl: true,
                            shift: false,
                        })
                    } else if expr.len() == 3 && expr.starts_with("s-") {
                        let char = expr.chars().nth(2).expect("");
                        Ok(Key {
                            code: char_to_keycode(char),
                            ctrl: false,
                            shift: true,
                        })
                    } else {
                        let code = str_to_keycode(expr.as_str());
                        Ok(Key {
                            code,
                            ctrl: false,
                            shift: false,
                        })
                    }
                }
            }
        }
    }
}

#[derive(Default)]
pub struct KeyMap {
    pub normal_map: HashMap<Key, Vec<Action>>,
    pub visual_map: HashMap<Key, Vec<Action>>,
}

impl KeyMap {
    pub fn help(&self) -> String {
        let mut text = Vec::new();
        let mut both = HashSet::new();
        for (k, normal_actions) in &self.normal_map {
            let mut line = String::new();
            if matches!(self.visual_map.get(k), Some(visual_actions) if visual_actions == normal_actions)
            {
                both.insert(k);
                let k = k.string();
                line.push_str(&format!("map {k}{}", " ".repeat(10 - k.chars().count())));
            } else {
                let k = k.string();
                line.push_str(&format!(
                    "map/normal {k}{}",
                    " ".repeat(10 - k.chars().count())
                ));
            }
            line.push_str(
                &normal_actions
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(" THEN "),
            );
            text.push(line);
        }
        for (k, visual_actions) in &self.visual_map {
            if !both.contains(k) {
                let mut line = String::new();
                let k = k.string();
                line.push_str(&format!(
                    "map/visual {k}{}",
                    " ".repeat(10 - k.chars().count())
                ));
                line.push_str(
                    &visual_actions
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<_>>()
                        .join(" THEN "),
                );
                text.push(line);
            }
        }
        text.sort_by_key(|s| s.to_ascii_lowercase());
        text.join("\n")
    }
    pub fn get_normal(&self, key: Key) -> &[Action] {
        if let Some(actions) = self.normal_map.get(&key).as_ref() {
            &actions[..]
        } else {
            &[]
        }
    }
    pub fn get_visual(&self, key: Key) -> &[Action] {
        if let Some(actions) = self.visual_map.get(&key).as_ref() {
            &actions[..]
        } else {
            &[]
        }
    }
}
