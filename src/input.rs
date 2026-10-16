use ratatui::crossterm::event::KeyCode;

#[derive(Default)]
pub struct TextInput {
    text: String,
    cursor: usize,
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

    pub fn handle_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) => {
                self.text.insert(self.cursor, c);
                self.cursor += c.len_utf8();
            }
            KeyCode::Backspace => {
                if let Some((i, _)) = self.before_cursor().char_indices().next_back() {
                    self.text.remove(i);
                    self.cursor = i;
                }
            }
            KeyCode::Left => {
                if let Some((i, _)) = self.before_cursor().char_indices().next_back() {
                    self.cursor = i;
                }
            }
            KeyCode::Right => {
                if let Some(c) = self.at_cursor() {
                    self.cursor += c.len_utf8();
                }
            }
            _ => {}
        }
    }
}
