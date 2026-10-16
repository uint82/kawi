#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Write,
    Quit,
    WriteQuit,
    SetKeymap(String),
    SetTheme(String),
    SetShiftWidth(usize),
    Unknown,
}

pub fn parse(text: &str) -> Command {
    match text.trim() {
        "w" => Command::Write,
        "q" => Command::Quit,
        "wq" => Command::WriteQuit,
        _ => parse_set_command(text.trim()),
    }
}

fn parse_set_command(text: &str) -> Command {
    let words: Vec<&str> = text.split_whitespace().collect();
    match words.as_slice() {
        ["set", "keymap", name] => name_command(name),
        ["set", "theme", name] => theme_command(name),
        ["set", "shiftwidth", value] => shiftwidth_command(value),
        ["set", combined] => equals_command(combined),
        _ => Command::Unknown,
    }
}

fn name_command(name: &str) -> Command {
    if name.is_empty() {
        return Command::Unknown;
    }
    Command::SetKeymap(name.to_string())
}

fn theme_command(name: &str) -> Command {
    if name.is_empty() {
        return Command::Unknown;
    }
    Command::SetTheme(name.to_string())
}

fn equals_command(combined: &str) -> Command {
    if let Some(name) = combined.strip_prefix("keymap=") {
        return name_command(name);
    }
    if let Some(name) = combined.strip_prefix("theme=") {
        return theme_command(name);
    }
    if let Some(value) = combined.strip_prefix("shiftwidth=") {
        return shiftwidth_command(value);
    }
    Command::Unknown
}

fn shiftwidth_command(value: &str) -> Command {
    match value.parse::<usize>() {
        Ok(width) => Command::SetShiftWidth(width),
        Err(_) => Command::Unknown,
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
    fn parses_set_shiftwidth() {
        assert_eq!(parse("set shiftwidth 4"), Command::SetShiftWidth(4));
        assert_eq!(parse("set shiftwidth=4"), Command::SetShiftWidth(4));
        assert_eq!(parse("set shiftwidth 0"), Command::SetShiftWidth(0));
    }

    #[test]
    fn rejects_bad_shiftwidth() {
        assert_eq!(parse("set shiftwidth"), Command::Unknown);
        assert_eq!(parse("set shiftwidth four"), Command::Unknown);
        assert_eq!(parse("set shiftwidth 4 extra"), Command::Unknown);
    }

    #[test]
    fn parses_set_keymap() {
        assert_eq!(
            parse("set keymap helix"),
            Command::SetKeymap("helix".to_string())
        );
    }

    #[test]
    fn parses_set_keymap_equals() {
        assert_eq!(
            parse("set keymap=vim"),
            Command::SetKeymap("vim".to_string())
        );
    }

    #[test]
    fn parses_set_theme() {
        assert_eq!(
            parse("set theme gruvbox-dark"),
            Command::SetTheme("gruvbox-dark".to_string())
        );
    }

    #[test]
    fn parses_set_theme_equals() {
        assert_eq!(
            parse("set theme=default"),
            Command::SetTheme("default".to_string())
        );
    }

    #[test]
    fn set_theme_without_name_is_unknown() {
        assert_eq!(parse("set theme"), Command::Unknown);
        assert_eq!(parse("set theme="), Command::Unknown);
    }

    #[test]
    fn set_without_name_is_unknown() {
        assert_eq!(parse("set keymap"), Command::Unknown);
        assert_eq!(parse("set"), Command::Unknown);
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
