use crate::input::TextInput;
use crate::util::{escape, expand_tilde, unescape};
use ratatui::crossterm::event::KeyEvent;
use std::path::PathBuf;

#[derive(Default)]
pub struct PathPrompt {
    pub input: TextInput,
    pub completions: Option<Completions>,
}

pub struct Completions {
    head: String,
    partial: String,
    quoted: bool,
    pub names: Vec<String>,
    pub selected: Option<usize>,
}

impl PathPrompt {
    pub fn edit(&mut self, key: KeyEvent) {
        self.input.handle_key(key);
        self.completions = None;
    }

    pub fn complete(&mut self, backwards: bool) {
        if let Some(c) = self.completions.as_mut() {
            if let Some(i) = c.selected {
                let n = c.names.len();
                let i = if backwards { (i + n - 1) % n } else { (i + 1) % n };
                c.selected = Some(i);
                let text = c.text_for(&c.names[i]);
                self.input.set_text(text);
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
            n => {
                let common = common_prefix(&c.names);
                if common.chars().count() > c.partial.chars().count() {
                    self.input.set_text(c.text_for(&common));
                } else {
                    let i = if backwards { n - 1 } else { 0 };
                    c.selected = Some(i);
                    self.input.set_text(c.text_for(&c.names[i]));
                }
                self.completions = Some(c);
            }
        }
    }
}

impl Completions {
    // Directories get a trailing '/'. Dotfiles only show up once you type a '.'.
    fn scan(raw: &str) -> Self {
        let quote = raw.chars().next().filter(|c| matches!(c, '"' | '\''));
        let quoted = quote.is_some();
        let body = &raw[quote.map_or(0, char::len_utf8)..];

        let (dir_raw, partial_raw) = match body.rfind('/') {
            Some(i) => body.split_at(i + 1),
            None => ("", body),
        };
        let head = raw[..raw.len() - partial_raw.len()].to_string();
        let plain = |s: &str| if quoted { s.to_string() } else { unescape(s) };
        let partial = plain(partial_raw);

        let mut completions = Self {
            head,
            partial,
            quoted,
            names: Vec::new(),
            selected: None,
        };

        if dir_raw.is_empty() && completions.partial == "~" {
            completions.names.push("~/".to_string());
            return completions;
        }

        let dir = plain(dir_raw);
        let dir = if dir.is_empty() {
            PathBuf::from(".")
        } else {
            expand_tilde(&dir)
        };
        let Ok(entries) = std::fs::read_dir(dir) else {
            return completions;
        };

        let show_hidden = completions.partial.starts_with('.');
        completions.names = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if !name.starts_with(&completions.partial) {
                    return None;
                }
                if name.starts_with('.') && !show_hidden {
                    return None;
                }
                Some(if e.path().is_dir() { name + "/" } else { name })
            })
            .collect();
        completions.names.sort_by_key(|n| n.to_lowercase());
        completions
    }

    fn text_for(&self, name: &str) -> String {
        if self.quoted {
            format!("{}{name}", self.head)
        } else {
            format!("{}{}", self.head, escape(name))
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{KeyCode, KeyModifiers};
    use std::fs;

    fn tmp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("crabpi-complete-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn prompt_with(text: &str) -> PathPrompt {
        let mut p = PathPrompt::default();
        p.input.set_text(text.to_string());
        p
    }

    #[test]
    fn single_match_completes_fully() {
        let dir = tmp_dir("single");
        fs::write(dir.join("report.pdf"), "").unwrap();
        fs::write(dir.join("notes.txt"), "").unwrap();

        let mut p = prompt_with(&format!("{}/rep", dir.display()));
        p.complete(false);
        assert_eq!(p.input.text(), format!("{}/report.pdf", dir.display()));
        assert!(p.completions.is_none());
    }

    #[test]
    fn directory_gets_slash_and_spaces_are_escaped() {
        let dir = tmp_dir("dirs");
        fs::create_dir(dir.join("my docs")).unwrap();

        let mut p = prompt_with(&format!("{}/my", dir.display()));
        p.complete(false);
        assert_eq!(p.input.text(), format!("{}/my\\ docs/", dir.display()));
    }

    #[test]
    fn quoted_paths_are_not_escaped() {
        let dir = tmp_dir("quoted");
        fs::write(dir.join("my file.txt"), "").unwrap();

        let mut p = prompt_with(&format!("\"{}/my", dir.display()));
        p.complete(false);
        assert_eq!(p.input.text(), format!("\"{}/my file.txt", dir.display()));
    }

    #[test]
    fn several_matches_extend_to_common_prefix_then_cycle() {
        let dir = tmp_dir("cycle");
        fs::write(dir.join("scan_a.pdf"), "").unwrap();
        fs::write(dir.join("scan_b.pdf"), "").unwrap();

        let base = dir.display();
        let mut p = prompt_with(&format!("{base}/s"));
        p.complete(false);
        assert_eq!(p.input.text(), format!("{base}/scan_"));
        assert_eq!(p.completions.as_ref().unwrap().names.len(), 2);
        assert_eq!(p.completions.as_ref().unwrap().selected, None);

        p.complete(false);
        assert_eq!(p.input.text(), format!("{base}/scan_a.pdf"));
        p.complete(false);
        assert_eq!(p.input.text(), format!("{base}/scan_b.pdf"));
        p.complete(false);
        assert_eq!(p.input.text(), format!("{base}/scan_a.pdf"));
        p.complete(true);
        assert_eq!(p.input.text(), format!("{base}/scan_b.pdf"));
    }

    #[test]
    fn hidden_files_need_a_dot_and_typing_resets_completions() {
        let dir = tmp_dir("hidden");
        fs::write(dir.join(".env"), "").unwrap();
        fs::write(dir.join("a.txt"), "").unwrap();
        fs::write(dir.join("b.txt"), "").unwrap();

        let base = dir.display();
        let mut p = prompt_with(&format!("{base}/"));
        p.complete(false);
        assert_eq!(p.completions.as_ref().unwrap().names, vec!["a.txt", "b.txt"]);

        p.input.set_text(format!("{base}/"));
        p.edit(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE));
        assert!(p.completions.is_none());
        p.complete(false);
        assert_eq!(p.input.text(), format!("{base}/.env"));
    }

    #[test]
    fn tilde_completes_to_home() {
        let mut p = prompt_with("~");
        p.complete(false);
        assert_eq!(p.input.text(), "~/");
    }

    #[test]
    fn no_match_leaves_text_alone() {
        let dir = tmp_dir("nomatch");
        let mut p = prompt_with(&format!("{}/zzz", dir.display()));
        p.complete(false);
        assert_eq!(p.input.text(), format!("{}/zzz", dir.display()));
        assert!(p.completions.is_none());
    }
}
