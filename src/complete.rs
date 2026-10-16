use crate::input::TextInput;
use crate::util::{escape, expand_tilde, unescape};
use ratatui::crossterm::event::KeyCode;
use std::path::PathBuf;

#[derive(Default)]
pub struct PathPrompt {
    pub input: TextInput,
    pub completions: Option<Completions>,
}

pub struct Completions {
    head: String,
    partial: String,
    pub names: Vec<String>,
    pub selected: Option<usize>,
}

impl PathPrompt {
    pub fn edit(&mut self, code: KeyCode) {
        self.input.handle_key(code);
        self.completions = None;
    }

    pub fn complete(&mut self) {
        if let Some(c) = self.completions.as_mut() {
            if let Some(i) = c.selected {
                let i = (i + 1) % c.names.len();
                c.selected = Some(i);
                self.input.set_text(c.text_for(&c.names[i]));
                return;
            }
        }

        let mut c = Completions::scan(self.input.text());
        match c.names.len() {
            0 => self.completions = None,
            1 => {
                self.input.set_text(c.text_for(&c.names[0]));
                self.completions = None;
            }
            _ => {
                let common = common_prefix(&c.names);
                if common.chars().count() > c.partial.chars().count() {
                    self.input.set_text(c.text_for(&common));
                } else {
                    c.selected = Some(0);
                    self.input.set_text(c.text_for(&c.names[0]));
                }
                self.completions = Some(c);
            }
        }
    }
}

impl Completions {
    fn scan(raw: &str) -> Self {
        let (dir_raw, partial_raw) = match raw.rfind('/') {
            Some(i) => raw.split_at(i + 1),
            None => ("", raw),
        };
        let partial = unescape(partial_raw);

        let mut names = Vec::new();
        if dir_raw.is_empty() && partial == "~" {
            names.push("~/".to_string());
        } else {
            let dir = unescape(dir_raw);
            let dir = if dir.is_empty() {
                PathBuf::from(".")
            } else {
                expand_tilde(&dir)
            };
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if !name.starts_with(&partial) {
                        continue;
                    }
                    if name.starts_with('.') && !partial.starts_with('.') {
                        continue;
                    }
                    names.push(if entry.path().is_dir() { name + "/" } else { name });
                }
                names.sort_by_key(|n| n.to_lowercase());
            }
        }

        Self {
            head: dir_raw.to_string(),
            partial,
            names,
            selected: None,
        }
    }

    fn text_for(&self, name: &str) -> String {
        format!("{}{}", self.head, escape(name))
    }
}

fn common_prefix(names: &[String]) -> String {
    let first = &names[0];
    let mut end = first.len();
    for n in &names[1..] {
        let shared = first
            .char_indices()
            .zip(n.chars())
            .take_while(|((_, a), b)| a == b)
            .last()
            .map_or(0, |((i, a), _)| i + a.len_utf8());
        end = end.min(shared);
    }
    first[..end].to_string()
}
