pub struct UI {
    pub palette: Palette,
    pub layers: Layers,
}
impl UI {
    pub fn new() -> Self {
        UI {
            palette: Palette::new(),
            layers: Layers::new(),
        }
    }
}
pub struct Palette {
    pub each_row: usize,
    pub from_left: i32,
    pub from_bottom: i32,
}
impl Palette {
    fn new() -> Self {
        Palette {
            each_row: 8,
            from_left: 1,
            from_bottom: 5,
        }
    }
}
pub struct Layers {
    pub from_right: i32,
    pub from_bottom: i32,
}
impl Layers {
    fn new() -> Self {
        Layers {
            from_right: 1,
            from_bottom: 3,
        }
    }
}

pub enum LayerClick {
    Toggle,
    Frame(usize),
    None,
}

pub struct TextViewer {
    lines: Vec<String>,
    pub scroll: i32,
}
impl TextViewer {
    pub fn new(s: &str) -> Self {
        TextViewer {
            lines: s.lines().map(|l| l.into()).collect(),
            scroll: 0,
        }
    }
    pub fn lines(&self) -> &Vec<String> {
        &self.lines
    }
    pub fn scroll(&mut self, value: i32) {
        self.scroll = (self.scroll + value).clamp(0, self.lines.len() as i32 - 10);
    }
    pub fn scroll_to_start(&mut self) {
        self.scroll = 0;
    }
    pub fn scroll_to_end(&mut self) {
        self.scroll = self.lines.len() as i32 - 10;
    }
}
