#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Write,
    Quit,
    WriteQuit,
    Unknown,
}

pub fn parse(text: &str) -> Command {
    match text.trim() {
        "w" => Command::Write,
        "q" => Command::Quit,
        "wq" => Command::WriteQuit,
        _ => Command::Unknown,
    }
}

pub struct CommandLine {
    text: String,
}

impl CommandLine {
    pub fn new() -> Self {
        Self {
            text: String::new(),
        }
    }

    pub fn push(&mut self, c: char) {
        self.text.push(c);
    }

    pub fn pop(&mut self) {
        self.text.pop();
    }

    pub fn clear(&mut self) {
        self.text.clear();
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

impl Default for CommandLine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_write() {
        assert_eq!(parse("w"), Command::Write);
    }

    #[test]
    fn parses_quit() {
        assert_eq!(parse("q"), Command::Quit);
    }

    #[test]
    fn parses_write_quit() {
        assert_eq!(parse("wq"), Command::WriteQuit);
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(parse("  w  "), Command::Write);
    }

    #[test]
    fn unknown_falls_through() {
        assert_eq!(parse("xyz"), Command::Unknown);
    }

    #[test]
    fn empty_is_unknown() {
        assert_eq!(parse(""), Command::Unknown);
    }

    #[test]
    fn line_pushes_chars() {
        let mut line = CommandLine::new();
        line.push('w');
        line.push('q');
        assert_eq!(line.text(), "wq");
    }

    #[test]
    fn line_pop_shrinks() {
        let mut line = CommandLine::new();
        line.push('w');
        line.pop();
        assert_eq!(line.text(), "");
    }

    #[test]
    fn line_clear_empties() {
        let mut line = CommandLine::new();
        line.push('w');
        line.clear();
        assert_eq!(line.text(), "");
    }
}
