#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextInput {
    text: String,
    cursor: usize,
}

impl TextInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.text.chars().count();
    }

    pub fn insert_char(&mut self, character: char) {
        let byte = self.byte_index(self.cursor);
        self.text.insert(byte, character);
        self.cursor = self.cursor.saturating_add(1);
    }

    pub fn insert_text(&mut self, text: &str) {
        for character in text.chars().filter(|character| !character.is_control()) {
            self.insert_char(character);
        }
    }

    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.cursor = self.cursor.saturating_add(1).min(self.text.chars().count());
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.text.chars().count();
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = self.byte_index(self.cursor - 1);
        let end = self.byte_index(self.cursor);
        self.text.replace_range(start..end, "");
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn delete(&mut self) {
        if self.cursor >= self.text.chars().count() {
            return;
        }
        let start = self.byte_index(self.cursor);
        let end = self.byte_index(self.cursor + 1);
        self.text.replace_range(start..end, "");
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    pub fn delete_word(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let characters: Vec<char> = self.text.chars().collect();
        let mut start = self.cursor;
        while start > 0 && characters[start - 1].is_whitespace() {
            start -= 1;
        }
        while start > 0 && !characters[start - 1].is_whitespace() {
            start -= 1;
        }
        let start_byte = self.byte_index(start);
        let end_byte = self.byte_index(self.cursor);
        self.text.replace_range(start_byte..end_byte, "");
        self.cursor = start;
    }

    fn byte_index(&self, character: usize) -> usize {
        self.text
            .char_indices()
            .nth(character)
            .map(|(byte, _)| byte)
            .unwrap_or(self.text.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_and_moves_with_unicode() {
        let mut input = TextInput::new();
        input.insert_text("aé");
        input.move_left();
        input.insert_char('b');
        assert_eq!(input.text(), "abé");
        assert_eq!(input.cursor(), 2);
    }

    #[test]
    fn deletes_around_the_cursor() {
        let mut input = TextInput::new();
        input.set_text("abcd");
        input.move_left();
        input.move_left();
        input.backspace();
        assert_eq!(input.text(), "acd");
        input.delete();
        assert_eq!(input.text(), "ad");
        input.move_home();
        input.insert_char('x');
        assert_eq!(input.text(), "xad");
        input.move_end();
        input.insert_char('y');
        assert_eq!(input.text(), "xady");
    }

    #[test]
    fn deletes_the_previous_word() {
        let mut input = TextInput::new();
        input.set_text("src lib");
        input.delete_word();
        assert_eq!(input.text(), "src ");
        assert_eq!(input.cursor(), 4);
    }
}
