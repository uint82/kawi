#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Write { force: bool },
    Quit,
    WriteQuit { force: bool },
    ForceQuit,
    QuitAll,
    ForceQuitAll,
    WriteAll,
    WriteQuitAll,
    Edit { name: Option<String>, force: bool },
    WriteAs { name: String, force: bool },
    DeleteBuffer { spec: Option<String>, force: bool },
    ListBuffers,
    Marks,
    NextBuffer,
    PreviousBuffer,
    AlternateBuffer,
    SelectBuffer(String),
    SetKeymap(String),
    SetTheme(String),
    SetShiftWidth(usize),
    SetExpandTab(bool),
    SetNumber(bool),
    SetRelativeNumber(bool),
    SetNumberWidth(usize),
    SetIgnoreCase(bool),
    SetSmartCase(bool),
    SetAutoIndent(bool),
    SetError(String),
    Substitute(SubstituteCommand),
    Unknown,
}

pub const COMMAND_NAMES: &[&str] = &[
    "b",
    "b#",
    "bd",
    "bdelete",
    "bn",
    "bnext",
    "bp",
    "bprevious",
    "buffer",
    "buffers",
    "e",
    "edit",
    "files",
    "ls",
    "q",
    "q!",
    "qa",
    "qa!",
    "quit",
    "quit!",
    "quitall",
    "quitall!",
    "s",
    "set",
    "substitute",
    "w",
    "w!",
    "wall",
    "write",
    "write!",
    "writequit",
    "wq",
    "wqa",
    "wqall",
];

pub const SET_OPTION_NAMES: &[&str] = &[
    "autoindent",
    "expandtab",
    "ignorecase",
    "keymap",
    "number",
    "numberwidth",
    "relativenumber",
    "shiftwidth",
    "smartcase",
    "theme",
];

pub fn parse(text: &str) -> Command {
    let named = parse_named(text);
    if named != Command::Unknown {
        return named;
    }
    if text.trim_start().starts_with("se") {
        return Command::Unknown;
    }
    match substitute::parse(text) {
        Some(Ok(command)) => Command::Substitute(command),
        Some(Err(message)) => Command::SetError(message),
        None => Command::Unknown,
    }
}

fn parse_named(text: &str) -> Command {
    let words: Vec<&str> = text.split_whitespace().collect();
    let Some((command, rest)) = words.split_first() else {
        return Command::Unknown;
    };
    match (*command, rest) {
        ("q" | "quit", []) => Command::Quit,
        ("wq" | "writequit" | "wq!" | "writequit!", []) => write_quit_command(command),
        ("q!" | "quit!", []) => Command::ForceQuit,
        ("qa" | "quitall", []) => Command::QuitAll,
        ("qa!" | "quitall!", []) => Command::ForceQuitAll,
        ("wall", []) => Command::WriteAll,
        ("wqa" | "wqall", []) => Command::WriteQuitAll,
        ("ls" | "buffers" | "files", []) => Command::ListBuffers,
        ("marks", []) => Command::Marks,
        ("bn" | "bnext", []) => Command::NextBuffer,
        ("bp" | "bprevious", []) => Command::PreviousBuffer,
        ("b#", []) => Command::AlternateBuffer,
        ("e" | "edit" | "e!" | "edit!", rest) if rest.len() <= 1 => {
            edit_command(command, rest.first().copied())
        }
        ("w" | "write" | "w!" | "write!", rest) if rest.len() <= 1 => {
            write_command(command, rest.first().copied())
        }
        ("bd" | "bdelete" | "bd!" | "bdelete!", rest) if rest.len() <= 1 => {
            delete_command(command, rest.first().copied())
        }
        ("b" | "buffer", ["#"]) => Command::AlternateBuffer,
        ("b" | "buffer", [arg]) => Command::SelectBuffer(arg.to_string()),
        ("set", rest) => parse_set_command(rest),
        _ => Command::Unknown,
    }
}

fn write_command(command: &str, name: Option<&str>) -> Command {
    let (force, command) = split_bang(command);
    if !matches!(command, "w" | "write") {
        return Command::Unknown;
    }
    match name {
        Some(name) => Command::WriteAs {
            name: name.to_string(),
            force,
        },
        None => Command::Write { force },
    }
}

fn write_quit_command(command: &str) -> Command {
    let (force, command) = split_bang(command);
    if !matches!(command, "wq" | "writequit") {
        return Command::Unknown;
    }
    Command::WriteQuit { force }
}

fn delete_command(command: &str, spec: Option<&str>) -> Command {
    let (force, command) = split_bang(command);
    if !matches!(command, "bd" | "bdelete") {
        return Command::Unknown;
    }
    Command::DeleteBuffer {
        spec: spec.map(str::to_string),
        force,
    }
}

fn split_bang(command: &str) -> (bool, &str) {
    match command.strip_suffix('!') {
        Some(command) => (true, command),
        None => (false, command),
    }
}

fn edit_command(command: &str, name: Option<&str>) -> Command {
    let (force, command) = split_bang(command);
    if !matches!(command, "e" | "edit") {
        return Command::Unknown;
    }
    Command::Edit {
        name: name.map(str::to_string),
        force,
    }
}

fn parse_set_command(words: &[&str]) -> Command {
    match words {
        ["keymap", name] => name_command(name),
        ["theme", name] => theme_command(name),
        ["shiftwidth", value] => shiftwidth_command(value),
        ["expandtab"] => Command::SetExpandTab(true),
        ["noexpandtab"] => Command::SetExpandTab(false),
        ["number"] => Command::SetNumber(true),
        ["nonumber"] => Command::SetNumber(false),
        ["relativenumber"] => Command::SetRelativeNumber(true),
        ["norelativenumber"] => Command::SetRelativeNumber(false),
        ["ignorecase"] => Command::SetIgnoreCase(true),
        ["noignorecase"] => Command::SetIgnoreCase(false),
        ["smartcase"] => Command::SetSmartCase(true),
        ["nosmartcase"] => Command::SetSmartCase(false),
        ["autoindent"] => Command::SetAutoIndent(true),
        ["noautoindent"] => Command::SetAutoIndent(false),
        [combined] => equals_command(combined),
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
    if let Some(value) = combined.strip_prefix("numberwidth=") {
        return numberwidth_command(value);
    }
    Command::Unknown
}

fn shiftwidth_command(value: &str) -> Command {
    match value.parse::<usize>() {
        Ok(width) => Command::SetShiftWidth(width),
        Err(_) => Command::Unknown,
    }
}

fn numberwidth_command(value: &str) -> Command {
    match value.parse::<i64>() {
        Ok(width) if width < 1 => Command::SetError(format!(
            "E487: Argument must be positive: numberwidth={value}"
        )),
        Ok(width) if width > 20 => {
            Command::SetError(format!("E474: Invalid argument: numberwidth={value}"))
        }
        Ok(width) => Command::SetNumberWidth(width as usize),
        Err(_) => Command::SetError(format!(
            "E521: Number required after =: numberwidth={value}"
        )),
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

    pub fn replace(&mut self, text: &str) {
        self.text.clear();
        self.text.push_str(text);
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

mod substitute;

pub use substitute::{
    SubstituteAddress, SubstituteCommand, SubstituteFlag, SubstituteFlags, SubstituteForm,
    SubstituteRange,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_names_parse() {
        for name in COMMAND_NAMES {
            if *name == "set" {
                continue;
            }
            assert!(
                parse(name) != Command::Unknown || parse(&format!("{name} x")) != Command::Unknown,
                "command {name} does not parse"
            );
        }
    }

    #[test]
    fn parses_write() {
        assert_eq!(parse("w"), Command::Write { force: false });
    }

    #[test]
    fn parses_quit() {
        assert_eq!(parse("q"), Command::Quit);
    }

    #[test]
    fn parses_write_quit() {
        assert_eq!(parse("wq"), Command::WriteQuit { force: false });
        assert_eq!(parse("wq!"), Command::WriteQuit { force: true });
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(parse("  w  "), Command::Write { force: false });
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
    fn parses_quit_variants() {
        assert_eq!(parse("q!"), Command::ForceQuit);
        assert_eq!(parse("qa"), Command::QuitAll);
        assert_eq!(parse("qa!"), Command::ForceQuitAll);
    }

    #[test]
    fn parses_write_all_variants() {
        assert_eq!(parse("wall"), Command::WriteAll);
        assert_eq!(parse("wqa"), Command::WriteQuitAll);
        assert_eq!(parse("wqall"), Command::WriteQuitAll);
    }

    #[test]
    fn parses_edit_commands() {
        assert_eq!(
            parse("e"),
            Command::Edit {
                name: None,
                force: false
            }
        );
        assert_eq!(
            parse("e!"),
            Command::Edit {
                name: None,
                force: true
            }
        );
        assert_eq!(
            parse("e src/main.rs"),
            Command::Edit {
                name: Some("src/main.rs".to_string()),
                force: false
            }
        );
        assert_eq!(
            parse("edit! src/main.rs"),
            Command::Edit {
                name: Some("src/main.rs".to_string()),
                force: true
            }
        );
        assert_eq!(
            parse("e #"),
            Command::Edit {
                name: Some("#".to_string()),
                force: false
            }
        );
    }

    #[test]
    fn parses_write_as_commands() {
        assert_eq!(
            parse("w copy.txt"),
            Command::WriteAs {
                name: "copy.txt".to_string(),
                force: false
            }
        );
        assert_eq!(
            parse("write! copy.txt"),
            Command::WriteAs {
                name: "copy.txt".to_string(),
                force: true
            }
        );
        assert_eq!(parse("write"), Command::Write { force: false });
        assert_eq!(parse("w!"), Command::Write { force: true });
    }

    #[test]
    fn parses_delete_buffer_commands() {
        assert_eq!(
            parse("bd"),
            Command::DeleteBuffer {
                spec: None,
                force: false
            }
        );
        assert_eq!(
            parse("bdelete! 2"),
            Command::DeleteBuffer {
                spec: Some("2".to_string()),
                force: true
            }
        );
        assert_eq!(
            parse("bdelete a.txt"),
            Command::DeleteBuffer {
                spec: Some("a.txt".to_string()),
                force: false
            }
        );
        assert_eq!(parse("bd a b"), Command::Unknown);
    }

    #[test]
    fn parses_buffer_commands() {
        assert_eq!(parse("ls"), Command::ListBuffers);
        assert_eq!(parse("buffers"), Command::ListBuffers);
        assert_eq!(parse("files"), Command::ListBuffers);
        assert_eq!(parse("bn"), Command::NextBuffer);
        assert_eq!(parse("bnext"), Command::NextBuffer);
        assert_eq!(parse("bp"), Command::PreviousBuffer);
        assert_eq!(parse("bprevious"), Command::PreviousBuffer);
        assert_eq!(parse("b#"), Command::AlternateBuffer);
        assert_eq!(parse("b 3"), Command::SelectBuffer("3".to_string()));
        assert_eq!(parse("buffer 3"), Command::SelectBuffer("3".to_string()));
        assert_eq!(parse("buffer #"), Command::AlternateBuffer);
    }

    #[test]
    fn rejects_unknown_buffer_commands() {
        assert_eq!(parse("b"), Command::Unknown);
        assert_eq!(parse("bb"), Command::Unknown);
        assert_eq!(parse("e x y"), Command::Unknown);
    }

    #[test]
    fn parses_set_shiftwidth() {
        assert_eq!(parse("set shiftwidth 4"), Command::SetShiftWidth(4));
        assert_eq!(parse("set shiftwidth=4"), Command::SetShiftWidth(4));
        assert_eq!(parse("set shiftwidth 0"), Command::SetShiftWidth(0));
    }

    #[test]
    fn parses_set_expandtab() {
        assert_eq!(parse("set expandtab"), Command::SetExpandTab(true));
        assert_eq!(parse("set noexpandtab"), Command::SetExpandTab(false));
    }

    #[test]
    fn parses_search_and_indent_options() {
        assert_eq!(parse("set ignorecase"), Command::SetIgnoreCase(true));
        assert_eq!(parse("set noignorecase"), Command::SetIgnoreCase(false));
        assert_eq!(parse("set smartcase"), Command::SetSmartCase(true));
        assert_eq!(parse("set nosmartcase"), Command::SetSmartCase(false));
        assert_eq!(parse("set autoindent"), Command::SetAutoIndent(true));
        assert_eq!(parse("set noautoindent"), Command::SetAutoIndent(false));
    }

    #[test]
    fn parses_set_number_options() {
        assert_eq!(parse("set number"), Command::SetNumber(true));
        assert_eq!(parse("set nonumber"), Command::SetNumber(false));
        assert_eq!(
            parse("set relativenumber"),
            Command::SetRelativeNumber(true)
        );
        assert_eq!(
            parse("set norelativenumber"),
            Command::SetRelativeNumber(false)
        );
        assert_eq!(parse("set numberwidth=4"), Command::SetNumberWidth(4));
        assert_eq!(parse("set numberwidth=1"), Command::SetNumberWidth(1));
        assert_eq!(parse("set numberwidth=20"), Command::SetNumberWidth(20));
    }

    #[test]
    fn rejects_bad_numberwidth() {
        assert_eq!(
            parse("set numberwidth=0"),
            Command::SetError("E487: Argument must be positive: numberwidth=0".to_string())
        );
        assert_eq!(
            parse("set numberwidth=21"),
            Command::SetError("E474: Invalid argument: numberwidth=21".to_string())
        );
        assert_eq!(
            parse("set numberwidth=abc"),
            Command::SetError("E521: Number required after =: numberwidth=abc".to_string())
        );
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
