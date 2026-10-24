#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Write {
        force: bool,
    },
    Quit,
    WriteQuit {
        force: bool,
    },
    ForceQuit,
    QuitAll,
    ForceQuitAll,
    WriteAll,
    WriteQuitAll,
    Edit {
        name: Option<String>,
        force: bool,
    },
    WriteAs {
        name: String,
        force: bool,
    },
    DeleteBuffer {
        spec: Option<String>,
        force: bool,
    },
    ListBuffers,
    Marks,
    NextBuffer,
    PreviousBuffer,
    AlternateBuffer,
    SelectBuffer(String),
    Split {
        vertical: bool,
        name: Option<String>,
    },
    NewWindow {
        vertical: bool,
        name: Option<String>,
    },
    CloseWindow,
    OnlyWindow,
    Set {
        name: String,
        value: SetValue,
    },
    Pick(PickerCommand),
    Git(GitCommand),
    Task(TaskCommand),
    Quickfix(QuickfixCommand),
    SetError(String),
    Substitute(SubstituteCommand),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetValue {
    Assign(Option<String>),
    Toggle,
    Query,
    Reset,
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerCommand {
    Files,
    Grep,
    Buffers,
    Replace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskCommand {
    Run(String),
    List,
    Rerun,
    Cancel,
    Output,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickfixCommand {
    Open,
    Next,
    Previous,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitCommand {
    NextHunk,
    PrevHunk,
    StageHunk,
    UndoStage,
    StageBuffer,
    PreviewHunk,
    Diff,
    Refresh,
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
    "cclose",
    "clo",
    "close",
    "cnext",
    "copen",
    "cprev",
    "e",
    "edit",
    "files",
    "git",
    "ls",
    "new",
    "on",
    "only",
    "pick",
    "q",
    "q!",
    "qa",
    "qa!",
    "qf",
    "quickfix",
    "quit",
    "quit!",
    "quitall",
    "quitall!",
    "replace",
    "s",
    "set",
    "sp",
    "split",
    "substitute",
    "task",
    "vnew",
    "vs",
    "vsplit",
    "w",
    "w!",
    "wall",
    "wq",
    "wqa",
    "wqall",
    "write",
    "write!",
    "writequit",
];

pub const SET_OPTION_NAMES: &[&str] = &[
    "autoindent",
    "expandtab",
    "history",
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
        ("sp" | "split", rest) if rest.len() <= 1 => Command::Split {
            vertical: false,
            name: rest.first().map(|name| (*name).to_string()),
        },
        ("vs" | "vsplit", rest) if rest.len() <= 1 => Command::Split {
            vertical: true,
            name: rest.first().map(|name| (*name).to_string()),
        },
        ("new", rest) if rest.len() <= 1 => Command::NewWindow {
            vertical: false,
            name: rest.first().map(|name| (*name).to_string()),
        },
        ("vnew", rest) if rest.len() <= 1 => Command::NewWindow {
            vertical: true,
            name: rest.first().map(|name| (*name).to_string()),
        },
        ("clo" | "close", []) => Command::CloseWindow,
        ("on" | "only", []) => Command::OnlyWindow,
        ("pick", []) => Command::Pick(PickerCommand::Files),
        ("pick", ["file" | "files"]) => Command::Pick(PickerCommand::Files),
        ("pick", ["grep" | "live_grep" | "grep_live"]) => Command::Pick(PickerCommand::Grep),
        ("pick", ["buffer" | "buffers"]) => Command::Pick(PickerCommand::Buffers),
        ("pick", ["replace"]) => Command::Pick(PickerCommand::Replace),
        ("replace", []) => Command::Pick(PickerCommand::Replace),
        ("git", rest) => parse_git_command(rest),
        ("task", rest) => parse_task_command(rest),
        ("quickfix" | "qf", rest) => parse_quickfix_command(rest),
        ("copen", []) => Command::Quickfix(QuickfixCommand::Open),
        ("cclose", []) => Command::Quickfix(QuickfixCommand::Close),
        ("cnext", []) => Command::Quickfix(QuickfixCommand::Next),
        ("cprev", []) => Command::Quickfix(QuickfixCommand::Previous),
        ("set", rest) => set::parse(rest),
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

fn trailing_characters(word: &str) -> Command {
    Command::SetError(format!("E488: Trailing characters: {word}"))
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

fn parse_task_command(words: &[&str]) -> Command {
    let command = match words {
        [] | ["list"] => TaskCommand::List,
        ["rerun"] => TaskCommand::Rerun,
        ["cancel"] => TaskCommand::Cancel,
        ["output"] => TaskCommand::Output,
        ["close"] => TaskCommand::Close,
        ["run"] => return Command::Unknown,
        ["run", name] => TaskCommand::Run((*name).to_string()),
        [name] => TaskCommand::Run((*name).to_string()),
        _ => return Command::Unknown,
    };
    Command::Task(command)
}

fn parse_quickfix_command(words: &[&str]) -> Command {
    let command = match words {
        [] | ["open"] => QuickfixCommand::Open,
        ["next"] => QuickfixCommand::Next,
        ["prev" | "previous"] => QuickfixCommand::Previous,
        ["close"] => QuickfixCommand::Close,
        _ => return Command::Unknown,
    };
    Command::Quickfix(command)
}

fn parse_git_command(words: &[&str]) -> Command {
    let command = match words {
        [] => GitCommand::Diff,
        ["next_hunk"] => GitCommand::NextHunk,
        ["prev_hunk"] => GitCommand::PrevHunk,
        ["stage_hunk"] => GitCommand::StageHunk,
        ["undo_stage_hunk"] => GitCommand::UndoStage,
        ["stage_buffer"] => GitCommand::StageBuffer,
        ["preview_hunk"] => GitCommand::PreviewHunk,
        ["diff"] => GitCommand::Diff,
        ["refresh"] => GitCommand::Refresh,
        _ => return Command::Unknown,
    };
    Command::Git(command)
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

mod set;
mod substitute;

pub(crate) use substitute::parse_preview;
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
    fn parses_git_commands() {
        assert_eq!(parse("git"), Command::Git(GitCommand::Diff));
        assert_eq!(parse("git diff"), Command::Git(GitCommand::Diff));
        assert_eq!(parse("git next_hunk"), Command::Git(GitCommand::NextHunk));
        assert_eq!(parse("git prev_hunk"), Command::Git(GitCommand::PrevHunk));
        assert_eq!(parse("git stage_hunk"), Command::Git(GitCommand::StageHunk));
        assert_eq!(
            parse("git undo_stage_hunk"),
            Command::Git(GitCommand::UndoStage)
        );
        assert_eq!(
            parse("git stage_buffer"),
            Command::Git(GitCommand::StageBuffer)
        );
        assert_eq!(
            parse("git preview_hunk"),
            Command::Git(GitCommand::PreviewHunk)
        );
        assert_eq!(parse("git refresh"), Command::Git(GitCommand::Refresh));
        assert_eq!(parse("git nope"), Command::Unknown);
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

    fn set(name: &str, value: SetValue) -> Command {
        Command::Set {
            name: name.to_string(),
            value,
        }
    }

    fn assign(name: &str, value: &str) -> Command {
        set(name, SetValue::Assign(Some(value.to_string())))
    }

    #[test]
    fn parses_set_shiftwidth() {
        assert_eq!(parse("set shiftwidth 4"), assign("shiftwidth", "4"));
        assert_eq!(parse("set shiftwidth=4"), assign("shiftwidth", "4"));
        assert_eq!(parse("set shiftwidth 0"), assign("shiftwidth", "0"));
        assert_eq!(parse("set sw=2"), assign("sw", "2"));
    }

    #[test]
    fn parses_set_expandtab() {
        assert_eq!(
            parse("set expandtab"),
            set("expandtab", SetValue::Assign(None))
        );
        assert_eq!(parse("set noexpandtab"), set("expandtab", SetValue::Negate));
    }

    #[test]
    fn parses_search_and_indent_options() {
        assert_eq!(
            parse("set ignorecase"),
            set("ignorecase", SetValue::Assign(None))
        );
        assert_eq!(
            parse("set noignorecase"),
            set("ignorecase", SetValue::Negate)
        );
        assert_eq!(
            parse("set smartcase"),
            set("smartcase", SetValue::Assign(None))
        );
        assert_eq!(parse("set nosmartcase"), set("smartcase", SetValue::Negate));
        assert_eq!(
            parse("set autoindent"),
            set("autoindent", SetValue::Assign(None))
        );
        assert_eq!(
            parse("set noautoindent"),
            set("autoindent", SetValue::Negate)
        );
    }

    #[test]
    fn parses_set_modifiers() {
        assert_eq!(parse("set number!"), set("number", SetValue::Toggle));
        assert_eq!(parse("set number?"), set("number", SetValue::Query));
        assert_eq!(parse("set number&"), set("number", SetValue::Reset));
        assert_eq!(parse("set rnu!"), set("rnu", SetValue::Toggle));
        assert_eq!(parse("set et?"), set("et", SetValue::Query));
        assert_eq!(parse("set sw&"), set("sw", SetValue::Reset));
    }

    #[test]
    fn parses_window_commands() {
        assert_eq!(
            parse("sp"),
            Command::Split {
                vertical: false,
                name: None
            }
        );
        assert_eq!(
            parse("split src/main.rs"),
            Command::Split {
                vertical: false,
                name: Some("src/main.rs".to_string())
            }
        );
        assert_eq!(
            parse("vsplit"),
            Command::Split {
                vertical: true,
                name: None
            }
        );
        assert_eq!(
            parse("vs src/main.rs"),
            Command::Split {
                vertical: true,
                name: Some("src/main.rs".to_string())
            }
        );
        assert_eq!(
            parse("new"),
            Command::NewWindow {
                vertical: false,
                name: None
            }
        );
        assert_eq!(
            parse("vnew src/main.rs"),
            Command::NewWindow {
                vertical: true,
                name: Some("src/main.rs".to_string())
            }
        );
        assert_eq!(parse("close"), Command::CloseWindow);
        assert_eq!(parse("clo"), Command::CloseWindow);
        assert_eq!(parse("only"), Command::OnlyWindow);
        assert_eq!(parse("on"), Command::OnlyWindow);
        assert_eq!(parse("close now"), Command::Unknown);
    }

    #[test]
    fn parses_task_commands() {
        assert_eq!(parse("task"), Command::Task(TaskCommand::List));
        assert_eq!(parse("task list"), Command::Task(TaskCommand::List));
        assert_eq!(
            parse("task check"),
            Command::Task(TaskCommand::Run("check".to_string()))
        );
        assert_eq!(
            parse("task run check"),
            Command::Task(TaskCommand::Run("check".to_string()))
        );
        assert_eq!(parse("task rerun"), Command::Task(TaskCommand::Rerun));
        assert_eq!(parse("task cancel"), Command::Task(TaskCommand::Cancel));
        assert_eq!(parse("task output"), Command::Task(TaskCommand::Output));
        assert_eq!(parse("task close"), Command::Task(TaskCommand::Close));
        assert_eq!(parse("task run"), Command::Unknown);
        assert_eq!(parse("task a b"), Command::Unknown);
    }

    #[test]
    fn parses_quickfix_commands() {
        assert_eq!(parse("quickfix"), Command::Quickfix(QuickfixCommand::Open));
        assert_eq!(parse("qf open"), Command::Quickfix(QuickfixCommand::Open));
        assert_eq!(parse("cnext"), Command::Quickfix(QuickfixCommand::Next));
        assert_eq!(parse("cprev"), Command::Quickfix(QuickfixCommand::Previous));
        assert_eq!(
            parse("quickfix prev"),
            Command::Quickfix(QuickfixCommand::Previous)
        );
        assert_eq!(parse("cclose"), Command::Quickfix(QuickfixCommand::Close));
        assert_eq!(parse("quickfix nope"), Command::Unknown);
    }

    #[test]
    fn parses_picker_commands() {
        assert_eq!(parse("pick files"), Command::Pick(PickerCommand::Files));
        assert_eq!(parse("pick file"), Command::Pick(PickerCommand::Files));
        assert_eq!(parse("pick grep"), Command::Pick(PickerCommand::Grep));
        assert_eq!(parse("pick live_grep"), Command::Pick(PickerCommand::Grep));
        assert_eq!(parse("pick buffers"), Command::Pick(PickerCommand::Buffers));
        assert_eq!(parse("pick replace"), Command::Pick(PickerCommand::Replace));
        assert_eq!(parse("replace"), Command::Pick(PickerCommand::Replace));
        assert_eq!(parse("pick"), Command::Pick(PickerCommand::Files));
        assert_eq!(parse("pick nope"), Command::Unknown);
    }

    #[test]
    fn parses_set_number_options() {
        assert_eq!(parse("set number"), set("number", SetValue::Assign(None)));
        assert_eq!(parse("set nonumber"), set("number", SetValue::Negate));
        assert_eq!(
            parse("set relativenumber"),
            set("relativenumber", SetValue::Assign(None))
        );
        assert_eq!(
            parse("set norelativenumber"),
            set("relativenumber", SetValue::Negate)
        );
        assert_eq!(parse("set numberwidth=4"), assign("numberwidth", "4"));
        assert_eq!(parse("set numberwidth=1"), assign("numberwidth", "1"));
        assert_eq!(parse("set numberwidth=20"), assign("numberwidth", "20"));
    }

    #[test]
    fn leaves_invalid_values_for_the_registry() {
        assert_eq!(parse("set numberwidth=0"), assign("numberwidth", "0"));
        assert_eq!(parse("set numberwidth=abc"), assign("numberwidth", "abc"));
        assert_eq!(parse("set shiftwidth four"), assign("shiftwidth", "four"));
    }

    #[test]
    fn rejects_missing_or_trailing_set_arguments() {
        assert_eq!(parse("set"), Command::Unknown);
        assert_eq!(
            parse("set shiftwidth 4 extra"),
            Command::SetError("E488: Trailing characters: shiftwidth".to_string())
        );
    }

    #[test]
    fn parses_set_keymap() {
        assert_eq!(parse("set keymap helix"), assign("keymap", "helix"));
    }

    #[test]
    fn parses_set_keymap_equals() {
        assert_eq!(parse("set keymap=vim"), assign("keymap", "vim"));
    }

    #[test]
    fn parses_set_theme() {
        assert_eq!(
            parse("set theme gruvbox-dark"),
            assign("theme", "gruvbox-dark")
        );
    }

    #[test]
    fn parses_set_theme_equals() {
        assert_eq!(parse("set theme=default"), assign("theme", "default"));
    }

    #[test]
    fn set_without_value_is_passed_to_the_registry() {
        assert_eq!(parse("set theme"), set("theme", SetValue::Assign(None)));
        assert_eq!(parse("set keymap"), set("keymap", SetValue::Assign(None)));
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
