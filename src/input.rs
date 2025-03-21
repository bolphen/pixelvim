use crate::color::Color;
use crate::command::{Commands, Match, Settings};

use std::ops::Range;

pub struct CompletionData<'a> {
    pub buffers: Vec<&'a str>,
    pub palette: &'a Vec<Color>,
    pub commands: &'a Commands,
    pub settings: &'a Settings,
}

impl CompletionData<'_> {
    fn match_cmd(&self, expr: &str) -> Match {
        self.commands.match_keyword(expr)
    }
    fn match_setting(&self, expr: &str, toggleable_only: bool) -> Match {
        self.settings.match_keyword(expr, toggleable_only)
    }
    fn complete_cmd(&self, input: &mut Input, to_complete: String) {
        let len = to_complete.len();
        match self.match_cmd(&to_complete) {
            Match::Single(cmd) => {
                input.text.drain(input.cursor - len..input.cursor);
                input.text.insert_str(input.cursor - len, cmd);
                input.text.insert(input.cursor - len + cmd.len(), ' ');
                input.cursor = input.cursor - len + cmd.len() + 1;
            }
            Match::Multiple(opts) => {
                let mut opts = opts
                    .iter()
                    .map(|s| Entry((*s).to_string(), self.commands.help(s), None))
                    .collect::<Vec<_>>();
                opts.sort_by_key(|e| e.0.to_ascii_lowercase());
                input.completions = Some((input.cursor - len..input.cursor, 0, opts));
            }
            _ => (),
        }
    }
    fn complete_setting(&self, input: &mut Input, to_complete: String, toggleable_only: bool) {
        let len = to_complete.len();
        match self.match_setting(&to_complete, toggleable_only) {
            Match::Single(setting) => {
                input.text.drain(input.cursor - len..input.cursor);
                input.text.insert_str(input.cursor - len, setting);
                if !toggleable_only {
                    input.text.insert(input.cursor - len + setting.len(), '=');
                    input.cursor = input.cursor - len + setting.len() + 1;
                } else {
                    input.cursor = input.cursor - len + setting.len();
                }
            }
            Match::Multiple(opts) => {
                let mut opts = opts
                    .iter()
                    .map(|s| Entry((*s).to_string(), self.settings.help(s), None))
                    .collect::<Vec<_>>();
                opts.sort_by_key(|e| e.0.to_ascii_lowercase());
                input.completions = Some((input.cursor - len..input.cursor, 0, opts));
            }
            _ => (),
        }
    }
    fn complete_path(&self, input: &mut Input, to_complete: String, exts: Option<Vec<&str>>) {
        let to_complete = to_complete.as_str();
        if to_complete == "~" {
            input.text.insert(input.cursor, '/');
            input.cursor += 1;
        } else {
            let (root, part) = if let Some(i) = to_complete.rfind('/') {
                to_complete.split_at(i + 1)
            } else {
                (".", to_complete)
            };
            let root = crate::utils::replace_home_dir(&root.into());
            let show_hidden = part.starts_with('.');
            let part_len = part.len();
            if let Ok(read_dir) = std::fs::read_dir(root) {
                let mut opts: Vec<_> = read_dir
                    .filter_map(|f| {
                        let f = f.ok()?.path();
                        let mut file = f.file_name()?.to_str()?.replace(' ', "\\ ");
                        let mut ext = "";
                        if f.is_dir() {
                            file.push('/');
                        } else {
                            ext = f.extension().and_then(|e| e.to_str()).unwrap_or("");
                        }
                        ((show_hidden || !file.starts_with("."))
                            && file.starts_with(part)
                            && (f.is_dir()
                                || exts
                                    .as_ref()
                                    .is_none_or(|exts| exts.iter().any(|e| ext.ends_with(e)))))
                        .then_some(Entry(file, None, None))
                    })
                    .collect();
                match opts.len() {
                    0 => (),
                    1 => {
                        input.text.drain((input.cursor - part_len)..input.cursor);
                        input.text.insert_str(input.cursor - part_len, &opts[0].0);
                        input.cursor = input.cursor - part_len + opts[0].0.len();
                    }
                    _ => {
                        opts.sort_by_key(|e| e.0.to_ascii_lowercase());
                        input.completions =
                            Some(((input.cursor - part.len())..input.cursor, 0, opts));
                    }
                }
            }
        }
    }
    fn complete_palette(&self, input: &mut Input, to_complete: String) {
        let len = to_complete.len();
        let opts: Vec<_> = self
            .palette
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let s = format!("${i}");
                s.starts_with(&to_complete)
                    .then_some(Entry(s, None, Some(*c)))
            })
            .collect();
        if !opts.is_empty() {
            input.completions = Some((input.cursor - len..input.cursor, 0, opts));
        }
    }
}

pub struct Entry(pub String, pub Option<String>, pub Option<Color>);
impl Entry {
    pub fn len(&self) -> usize {
        self.0.chars().count()
            + self.1.as_ref().map(|s| s.chars().count() + 3).unwrap_or(0)
            + self.2.map(|_| 5).unwrap_or(0)
    }
}

pub struct Input {
    text: String,
    cursor: usize,
    yank: String,
    pub completions: Option<(Range<usize>, usize, Vec<Entry>)>,
}
impl Input {
    pub fn new(text: &str) -> Self {
        Input {
            text: text.into(),
            cursor: text.len(),
            yank: String::new(),
            completions: None,
        }
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn len(&self) -> usize {
        self.text.chars().count()
    }
    pub fn cursor(&self) -> usize {
        self.text[..self.cursor].chars().count()
    }
    pub fn insert(&mut self, char: char) {
        self.text.insert(self.cursor, char);
        self.cursor += char.len_utf8();
        self.completions = None;
    }
    pub fn insert_str(&mut self, str: &str) {
        self.text.insert_str(self.cursor, str);
        self.cursor += str.len();
        self.completions = None;
    }
    pub fn home(&mut self) {
        self.cursor = 0;
        self.completions = None;
    }
    fn prev(&self) -> Option<char> {
        self.text[..self.cursor].chars().next_back()
    }
    fn next(&self) -> Option<char> {
        self.text[self.cursor..].chars().next()
    }
    pub fn left(&mut self) {
        self.prev().map(|ch| self.cursor -= ch.len_utf8());
        self.completions = None;
    }
    pub fn right(&mut self) {
        self.next().map(|ch| self.cursor += ch.len_utf8());
        self.completions = None;
    }
    pub fn end(&mut self) {
        self.cursor = self.text.len();
        self.completions = None;
    }
    pub fn kill(&mut self) {
        self.yank.clear();
        self.yank.extend(self.text.drain(self.cursor..));
        self.completions = None;
    }
    pub fn yank(&mut self) {
        self.text.insert_str(self.cursor, &self.yank);
        self.cursor += self.yank.len();
        self.completions = None;
    }
    pub fn backspace(&mut self) {
        self.prev().map(|ch| {
            self.text.drain(self.cursor - ch.len_utf8()..self.cursor);
            self.cursor -= ch.len_utf8()
        });
        self.completions = None;
    }
    pub fn delete(&mut self) {
        if let Some(ch) = self.next() {
            self.text.drain(self.cursor..self.cursor + ch.len_utf8());
        }
        self.completions = None;
    }
    pub fn get_completions(&mut self, data: &CompletionData, colon: Option<usize>) {
        let cmd = self.text[colon.unwrap_or(0)..self.cursor].trim_start();
        match cmd.split_once(' ') {
            None => data.complete_cmd(self, cmd.into()),
            Some((cmd, to_complete)) => {
                let cmd = cmd.trim_end_matches('!');
                let to_complete = to_complete.trim_start();
                match data.match_cmd(cmd) {
                    Match::Single("set") => {
                        if let Some((setting, value)) = to_complete.split_once('=') {
                            if data.match_setting(setting, false).is("color") {
                                let value = value.trim_start();
                                data.complete_palette(self, value.into());
                            }
                        } else {
                            data.complete_setting(self, to_complete.into(), false);
                        }
                    }
                    Match::Single("unset") | Match::Single("toggle") => {
                        data.complete_setting(self, to_complete.into(), true);
                    }
                    Match::Single("buffer") => {
                        let len = to_complete.len();
                        let opts: Vec<_> = data
                            .buffers
                            .iter()
                            .filter(|&s| s.starts_with(to_complete))
                            .map(|s| Entry(s.to_string(), None, None))
                            .collect();
                        if !opts.is_empty() {
                            self.completions = Some((self.cursor - len..self.cursor, 0, opts));
                        }
                    }
                    Match::Single("edit") | Match::Single("write") => {
                        data.complete_path(
                            self,
                            to_complete.replace("\\ ", " "),
                            vec!["png", "gif", "ase", "aseprite", "swp"].into(),
                        );
                    }
                    Match::Single("run") => {
                        data.complete_path(
                            self,
                            to_complete.replace("\\ ", " "),
                            vec!["lua"].into(),
                        );
                    }
                    Match::Single("source") => {
                        data.complete_path(self, to_complete.replace("\\ ", " "), None);
                    }
                    Match::Single("cd") => {
                        data.complete_path(self, to_complete.replace("\\ ", " "), Some(vec![]));
                    }
                    Match::Single("palette/delete") => {
                        data.complete_palette(self, to_complete.into());
                    }
                    // recursive call to complete commands in key mappings
                    Match::Single(c) if c.starts_with("map") => {
                        let to_complete = to_complete
                            .rsplit_once(" KEYUP ")
                            .map(|(_, r)| r)
                            .unwrap_or(to_complete);
                        let to_complete = to_complete
                            .rsplit_once(" THEN ")
                            .map(|(_, r)| r)
                            .unwrap_or(to_complete);
                        if let Some((_, to_complete)) = to_complete.split_once(':') {
                            self.get_completions(data, Some(self.cursor - to_complete.len()));
                        }
                    }
                    _ => (),
                }
            }
        }
    }
    pub fn complete(&mut self) {
        if let Some((range, id, completions)) = &mut self.completions {
            if let Some(completion) = completions.get(*id) {
                self.text.drain(range.clone());
                self.text.insert_str(range.start, &completion.0);
                self.cursor = range.start + completion.0.len();
                range.end = self.cursor;
            }
        }
    }
    pub fn prev_completion(&mut self, data: &CompletionData) {
        if let Some((_, id, completions)) = &mut self.completions {
            *id = (*id + completions.len() - 1) % completions.len();
        } else {
            self.get_completions(data, None);
        }
        self.complete();
    }
    pub fn next_completion(&mut self, data: &CompletionData) {
        if let Some((_, id, completions)) = &mut self.completions {
            *id = (*id + 1) % completions.len();
        } else {
            self.get_completions(data, None);
        }
        self.complete();
    }
}
