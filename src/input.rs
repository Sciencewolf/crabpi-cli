use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Default)]
pub struct TextInput {
    text: String,
    cursor: usize,
}

fn is_separator(c: char) -> bool {
    c.is_whitespace() || c == '/'
}

impl TextInput {
    pub fn with_text(text: impl Into<String>) -> Self {
        let text = text.into();
        let cursor = text.len();
        Self { text, cursor }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: String) {
        self.cursor = text.len();
        self.text = text;
    }

    pub fn before_cursor(&self) -> &str {
        &self.text[..self.cursor]
    }

    pub fn at_cursor(&self) -> Option<char> {
        self.text[self.cursor..].chars().next()
    }

    pub fn after_cursor(&self) -> &str {
        let skip = self.at_cursor().map_or(0, char::len_utf8);
        &self.text[self.cursor + skip..]
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);

        match key.code {
            KeyCode::Char('a') if ctrl => self.cursor = 0,
            KeyCode::Char('e') if ctrl => self.cursor = self.text.len(),
            KeyCode::Char('u') if ctrl => {
                self.text.drain(..self.cursor);
                self.cursor = 0;
            }
            KeyCode::Char('k') if ctrl => self.text.truncate(self.cursor),
            KeyCode::Char('w') if ctrl => self.delete_word_back(),
            KeyCode::Char('b') if alt => self.cursor = self.word_start(),
            KeyCode::Char('f') if alt => self.cursor = self.word_end(),
            KeyCode::Char(c) if !ctrl && !alt => {
                self.text.insert(self.cursor, c);
                self.cursor += c.len_utf8();
            }
            KeyCode::Backspace if alt => self.delete_word_back(),
            KeyCode::Backspace => {
                let prev = self.prev_boundary();
                self.text.drain(prev..self.cursor);
                self.cursor = prev;
            }
            KeyCode::Delete => {
                let next = self.next_boundary();
                self.text.drain(self.cursor..next);
            }
            KeyCode::Left if ctrl || alt => self.cursor = self.word_start(),
            KeyCode::Right if ctrl || alt => self.cursor = self.word_end(),
            KeyCode::Left => self.cursor = self.prev_boundary(),
            KeyCode::Right => self.cursor = self.next_boundary(),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.text.len(),
            _ => {}
        }
    }

    fn prev_boundary(&self) -> usize {
        self.before_cursor()
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i)
    }

    fn next_boundary(&self) -> usize {
        self.cursor + self.at_cursor().map_or(0, char::len_utf8)
    }

    fn word_start(&self) -> usize {
        let head = self.before_cursor().trim_end_matches(is_separator);
        head.rfind(is_separator).map_or(0, |i| {
            i + head[i..].chars().next().map_or(0, char::len_utf8)
        })
    }

    fn word_end(&self) -> usize {
        let tail = &self.text[self.cursor..];
        let rest = tail.trim_start_matches(is_separator);
        let skipped = tail.len() - rest.len();
        let word = rest.find(is_separator).unwrap_or(rest.len());
        self.cursor + skipped + word
    }

    fn delete_word_back(&mut self) {
        let start = self.word_start();
        self.text.drain(start..self.cursor);
        self.cursor = start;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn type_str(input: &mut TextInput, s: &str) {
        for c in s.chars() {
            input.handle_key(key(KeyCode::Char(c)));
        }
    }

    #[test]
    fn inserts_in_the_middle() {
        let mut input = TextInput::default();
        type_str(&mut input, "helo");
        input.handle_key(key(KeyCode::Left));
        input.handle_key(key(KeyCode::Char('l')));
        assert_eq!(input.text(), "hello");
        assert_eq!(input.before_cursor(), "hell");
        assert_eq!(input.at_cursor(), Some('o'));
    }

    #[test]
    fn backspace_and_delete_respect_multibyte_chars() {
        let mut input = TextInput::with_text("áéő");
        input.handle_key(key(KeyCode::Backspace));
        assert_eq!(input.text(), "áé");
        input.handle_key(key(KeyCode::Left));
        input.handle_key(key(KeyCode::Delete));
        assert_eq!(input.text(), "á");
    }

    #[test]
    fn home_end_and_line_editing() {
        let mut input = TextInput::with_text("report final.pdf");
        input.handle_key(key(KeyCode::Home));
        assert_eq!(input.before_cursor(), "");
        input.handle_key(ctrl('e'));
        assert_eq!(input.after_cursor(), "");
        input.handle_key(ctrl('u'));
        assert_eq!(input.text(), "");
    }

    #[test]
    fn ctrl_w_deletes_one_path_component() {
        let mut input = TextInput::with_text("/Users/me/docs/");
        input.handle_key(ctrl('w'));
        assert_eq!(input.text(), "/Users/me/");
        input.handle_key(ctrl('w'));
        assert_eq!(input.text(), "/Users/");
    }

    #[test]
    fn ctrl_k_cuts_to_the_end() {
        let mut input = TextInput::with_text("abcdef");
        input.handle_key(key(KeyCode::Left));
        input.handle_key(key(KeyCode::Left));
        input.handle_key(ctrl('k'));
        assert_eq!(input.text(), "abcd");
    }
}
