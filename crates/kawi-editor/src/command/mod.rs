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
    Cquit,
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
    SaveAs {
        name: String,
        force: bool,
    },
    RenameFile {
        source: Option<String>,
        name: String,
        force: bool,
    },
    DeleteFile {
        path: Option<String>,
        force: bool,
    },
    DeleteBuffer {
        spec: Option<String>,
        force: bool,
    },
    ListBuffers,
    Marks,
    Messages {
        clear: bool,
    },
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
    Lsp(LspCommand),
    Debug(DebugCommand),
    Explorer(ExplorerCommand),
    Terminal {
        command: Option<String>,
        vertical: bool,
    },
    Conflict,
    Format,
    Help {
        topic: Option<String>,
    },
    Health {
        topic: Option<String>,
    },
    Undo(UndoCommand),
    Substitute(SubstituteCommand),
    Map {
        text: String,
    },
    Plugin {
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetValue {
    Assign(Option<String>),
    Toggle,
    Query,
    Reset,
    Negate,
}

mod completion;
mod diagnostic;
mod file_ops;
mod parse;
mod undo;

pub use completion::CommandCompletion;
pub use diagnostic::{suggest_commands, DiagnosticCode, ParseFailure};
pub use parse::parse;

pub use undo::UndoCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerCommand {
    Files,
    Grep,
    Buffers,
    Replace,
    Symbols,
    WorkspaceSymbols,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LspCommand {
    Status,
    Start(Option<String>),
    Stop(Option<String>),
    Restart(Option<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugCommand {
    Status,
    Start(Option<String>),
    Restart,
    Stop,
    Terminate,
    Panel,
    Continue,
    Next,
    StepIn,
    StepOut,
    ToggleBreakpoint,
    BreakpointCondition(String),
    BreakpointHitCondition(String),
    BreakpointLogMessage(String),
    BreakpointClear,
    Breakpoints,
    FunctionBreakpoint(String),
    FunctionBreakpoints,
    Exceptions,
    ToggleException(String),
    Stack,
    Variables,
    ExpandVariables(i64),
    MoreVariables(i64),
    SetVariable {
        reference: i64,
        name: String,
        value: String,
    },
    Frame(i64),
    Threads,
    Thread(i64),
    Evaluate(String),
    Console,
    Watch(String),
    EditWatch(usize, String),
    Unwatch(String),
    Watches,
    Output,
    Configurations,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerCommand {
    Toggle,
    Focus,
    Reveal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitCommand {
    Status,
    Branches,
    Log {
        file: bool,
        grep: Option<String>,
        author: Option<String>,
    },
    Switch {
        target: String,
    },
    CreateBranch {
        name: String,
        base: Option<String>,
    },
    DeleteBranch {
        name: String,
        force: bool,
    },
    Commit {
        all: bool,
    },
    Amend {
        staged: bool,
    },
    Push {
        force: bool,
    },
    Fetch,
    NextHunk,
    PrevHunk,
    StageHunk,
    UndoStage,
    StageBuffer,
    PreviewHunk,
    Diff,
    Refresh,
    NextConflict,
    PrevConflict,
    ConflictOurs,
    ConflictTheirs,
    ConflictBoth,
    Continue,
    Abort,
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
    "checkhealth",
    "clo",
    "close",
    "cnext",
    "commands",
    "conflict",
    "copen",
    "cprev",
    "cq",
    "cquit",
    "debug",
    "doctor",
    "e",
    "earlier",
    "edit",
    "explorer",
    "files",
    "format",
    "git",
    "h",
    "health",
    "help",
    "imap",
    "iunmap",
    "later",
    "ls",
    "lsp",
    "map",
    "marks",
    "messages",
    "new",
    "nmap",
    "nunmap",
    "on",
    "only",
    "pick",
    "plugin",
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
    "delete",
    "delete!",
    "rename",
    "rename!",
    "replace",
    "s",
    "sav",
    "sav!",
    "saveas",
    "saveas!",
    "set",
    "sp",
    "split",
    "substitute",
    "task",
    "term",
    "terminal",
    "u",
    "undo",
    "undolist",
    "vmap",
    "vnew",
    "vterm",
    "vterminal",
    "vunmap",
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
    "auto_completion",
    "auto_pairs",
    "autoindent",
    "autosave",
    "autosave_delay",
    "color_preview",
    "completion_docs_down",
    "completion_docs_max_width",
    "completion_docs_up",
    "diagnostic_counts",
    "diagnostic_popup_max_width",
    "diagnostic_source",
    "diagnostic_underline",
    "diagnostic_virtual_severity",
    "diagnostic_virtual_text",
    "diagnostic_workspace_counts",
    "directory_confirm",
    "directory_metadata",
    "directory_show_hidden",
    "directory_sort",
    "directory_sort_directories_first",
    "directory_sort_reverse",
    "directory_trash",
    "expandtab",
    "explorer_enabled",
    "explorer_show_hidden",
    "explorer_width",
    "foldcolumn",
    "foldmethod",
    "format_on_save",
    "git_signs",
    "history",
    "hover_max_width",
    "ignorecase",
    "keyguide",
    "keyguide_delay",
    "keymap",
    "match_brackets",
    "number",
    "numberwidth",
    "overlay_margin",
    "overlay_max_height_percent",
    "overlay_min_width",
    "relativenumber",
    "shiftwidth",
    "signature_help",
    "signcolumn",
    "smartcase",
    "terminal_focus_key",
    "theme",
    "undo_persist",
];

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

    fn parse(text: &str) -> Command {
        super::parse(text).expect("command should parse")
    }

    fn parse_err(text: &str) -> ParseFailure {
        super::parse(text).expect_err("command should fail")
    }

    #[test]
    fn completion_names_parse() {
        for name in COMMAND_NAMES {
            assert!(
                !matches!(super::parse(name), Err(ParseFailure::UnknownCommand { .. })),
                "command {name} does not parse"
            );
            assert!(
                !matches!(
                    super::parse(&format!("{name} x")),
                    Err(ParseFailure::UnknownCommand { .. })
                ),
                "command {name} with an argument does not parse"
            );
        }
    }

    #[test]
    fn parses_write() {
        assert_eq!(parse("w"), Command::Write { force: false });
    }

    #[test]
    fn parses_format() {
        assert_eq!(parse("format"), Command::Format);
        assert_eq!(
            parse_err("format now"),
            ParseFailure::invalid_argument("now")
        );
    }

    #[test]
    fn parses_quit() {
        assert_eq!(parse("q"), Command::Quit);
    }

    #[test]
    fn parses_cquit() {
        assert_eq!(parse("cq"), Command::Cquit);
        assert_eq!(parse("cquit"), Command::Cquit);
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
        assert_eq!(parse_err("xyz"), ParseFailure::unknown_command("xyz"));
    }

    #[test]
    fn empty_is_unknown() {
        assert_eq!(parse_err(""), ParseFailure::EmptyInput);
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
    fn parses_save_as_and_file_commands() {
        assert_eq!(
            parse("saveas copy.txt"),
            Command::SaveAs {
                name: "copy.txt".to_string(),
                force: false
            }
        );
        assert_eq!(
            parse("sav! copy.txt"),
            Command::SaveAs {
                name: "copy.txt".to_string(),
                force: true
            }
        );
        assert_eq!(
            parse("rename next.txt"),
            Command::RenameFile {
                source: None,
                name: "next.txt".to_string(),
                force: false
            }
        );
        assert_eq!(
            parse("rename! next.txt"),
            Command::RenameFile {
                source: None,
                name: "next.txt".to_string(),
                force: true
            }
        );
        assert_eq!(
            parse("rename"),
            Command::RenameFile {
                source: None,
                name: String::new(),
                force: false
            }
        );
        assert_eq!(
            parse("delete"),
            Command::DeleteFile {
                path: None,
                force: false
            }
        );
        assert_eq!(
            parse("delete! old.txt"),
            Command::DeleteFile {
                path: Some("old.txt".to_string()),
                force: true
            }
        );
        assert_eq!(
            parse("rename old.txt next.txt"),
            Command::RenameFile {
                source: Some("old.txt".to_string()),
                name: "next.txt".to_string(),
                force: false
            }
        );
        assert_eq!(
            parse_err("rename a b c"),
            ParseFailure::invalid_argument("a b c")
        );
        assert_eq!(
            parse("delete now"),
            Command::DeleteFile {
                path: Some("now".to_string()),
                force: false
            }
        );
        assert_eq!(
            parse_err("delete a b"),
            ParseFailure::invalid_argument("a b")
        );
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
        assert_eq!(
            parse("git next_conflict"),
            Command::Git(GitCommand::NextConflict)
        );
        assert_eq!(
            parse("git prev_conflict"),
            Command::Git(GitCommand::PrevConflict)
        );
        assert_eq!(parse("git ours"), Command::Git(GitCommand::ConflictOurs));
        assert_eq!(
            parse("git theirs"),
            Command::Git(GitCommand::ConflictTheirs)
        );
        assert_eq!(parse("git both"), Command::Git(GitCommand::ConflictBoth));
        assert_eq!(parse("git continue"), Command::Git(GitCommand::Continue));
        assert_eq!(parse("git abort"), Command::Git(GitCommand::Abort));
        assert_eq!(parse("git status"), Command::Git(GitCommand::Status));
        assert_eq!(
            parse("git commit"),
            Command::Git(GitCommand::Commit { all: false })
        );
        assert_eq!(
            parse("git commit -a"),
            Command::Git(GitCommand::Commit { all: true })
        );
        assert_eq!(
            parse("git amend"),
            Command::Git(GitCommand::Amend { staged: false })
        );
        assert_eq!(
            parse("git amend staged"),
            Command::Git(GitCommand::Amend { staged: true })
        );
        assert_eq!(
            parse("git push"),
            Command::Git(GitCommand::Push { force: false })
        );
        assert_eq!(
            parse("git push --force-with-lease"),
            Command::Git(GitCommand::Push { force: true })
        );
        assert_eq!(parse("git fetch"), Command::Git(GitCommand::Fetch));
        assert_eq!(parse("git branches"), Command::Git(GitCommand::Branches));
        assert_eq!(
            parse("git branch feat/x"),
            Command::Git(GitCommand::CreateBranch {
                name: "feat/x".to_string(),
                base: None,
            })
        );
        assert_eq!(
            parse("git branch -d feat/x"),
            Command::Git(GitCommand::DeleteBranch {
                name: "feat/x".to_string(),
                force: false,
            })
        );
        assert_eq!(
            parse("git switch feat/x"),
            Command::Git(GitCommand::Switch {
                target: "feat/x".to_string(),
            })
        );
        assert_eq!(
            parse("git log %"),
            Command::Git(GitCommand::Log {
                file: true,
                grep: None,
                author: None,
            })
        );
        assert_eq!(
            parse("git log --grep fix bug"),
            Command::Git(GitCommand::Log {
                file: false,
                grep: Some("fix bug".to_string()),
                author: None,
            })
        );
        assert_eq!(
            parse_err("git nope"),
            ParseFailure::invalid_argument("nope")
        );
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
        assert_eq!(parse_err("bd a b"), ParseFailure::invalid_argument("a b"));
    }

    #[test]
    fn parses_messages_commands() {
        assert_eq!(parse("messages"), Command::Messages { clear: false });
        assert_eq!(parse("messages clear"), Command::Messages { clear: true });
        assert_eq!(
            parse_err("messages clear now"),
            ParseFailure::invalid_argument("clear now")
        );
        assert_eq!(
            parse_err("messages now"),
            ParseFailure::invalid_argument("now")
        );
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
        assert_eq!(parse_err("b"), ParseFailure::missing_argument("b"));
        assert_eq!(parse_err("bb"), ParseFailure::unknown_command("bb"));
        assert_eq!(parse_err("e x y"), ParseFailure::invalid_argument("x y"));
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
        assert_eq!(
            parse_err("close now"),
            ParseFailure::invalid_argument("now")
        );
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
        assert_eq!(parse_err("task run"), ParseFailure::invalid_argument("run"));
        assert_eq!(parse_err("task a b"), ParseFailure::invalid_argument("a b"));
    }

    #[test]
    fn parses_explorer_commands() {
        assert_eq!(
            parse("explorer"),
            Command::Explorer(ExplorerCommand::Toggle)
        );
        assert_eq!(
            parse("explorer focus"),
            Command::Explorer(ExplorerCommand::Focus)
        );
        assert_eq!(
            parse("explorer reveal"),
            Command::Explorer(ExplorerCommand::Reveal)
        );
        assert_eq!(
            parse_err("explorer unknown"),
            ParseFailure::invalid_argument("unknown")
        );
    }

    #[test]
    fn parses_terminal_commands() {
        assert_eq!(
            parse("term"),
            Command::Terminal {
                command: None,
                vertical: false
            }
        );
        assert_eq!(
            parse("terminal"),
            Command::Terminal {
                command: None,
                vertical: false
            }
        );
        assert_eq!(
            parse("term echo hello"),
            Command::Terminal {
                command: Some("echo hello".to_string()),
                vertical: false
            }
        );
        assert_eq!(
            parse("vterm"),
            Command::Terminal {
                command: None,
                vertical: true
            }
        );
        assert_eq!(
            parse("vterminal echo hello"),
            Command::Terminal {
                command: Some("echo hello".to_string()),
                vertical: true
            }
        );
    }

    #[test]
    fn parses_help_commands() {
        assert_eq!(parse("help"), Command::Help { topic: None });
        assert_eq!(parse("h"), Command::Help { topic: None });
        assert_eq!(
            parse("help option diagnostic_virtual_text"),
            Command::Help {
                topic: Some("option diagnostic_virtual_text".to_string())
            }
        );
        assert_eq!(
            parse("help getting started"),
            Command::Help {
                topic: Some("getting started".to_string())
            }
        );
    }

    #[test]
    fn parses_plugin_subcommands_without_the_command_name() {
        assert_eq!(
            parse("plugin list"),
            Command::Plugin {
                text: "list".to_string()
            }
        );
        assert_eq!(
            parse("plugin install owner/repo"),
            Command::Plugin {
                text: "install owner/repo".to_string()
            }
        );
    }

    #[test]
    fn parses_conflict_command() {
        assert_eq!(parse("conflict"), Command::Conflict);
        assert_eq!(
            parse_err("conflict now"),
            ParseFailure::invalid_argument("now")
        );
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
        assert_eq!(
            parse_err("quickfix nope"),
            ParseFailure::invalid_argument("nope")
        );
    }

    #[test]
    fn parses_health_commands() {
        assert_eq!(parse("health"), Command::Health { topic: None });
        assert_eq!(parse("checkhealth"), Command::Health { topic: None });
        assert_eq!(parse("doctor"), Command::Health { topic: None });
        assert_eq!(
            parse("health lsp"),
            Command::Health {
                topic: Some("lsp".to_string())
            }
        );
    }

    #[test]
    fn parses_debug_commands() {
        assert_eq!(parse("debug"), Command::Debug(DebugCommand::Status));
        assert_eq!(
            parse("debug start"),
            Command::Debug(DebugCommand::Start(None))
        );
        assert_eq!(
            parse("debug start rust"),
            Command::Debug(DebugCommand::Start(Some("rust".to_string())))
        );
        assert_eq!(parse("debug stop"), Command::Debug(DebugCommand::Stop));
        assert_eq!(
            parse("debug terminate"),
            Command::Debug(DebugCommand::Terminate)
        );
        assert_eq!(parse("debug panel"), Command::Debug(DebugCommand::Panel));
        assert_eq!(
            parse("debug continue"),
            Command::Debug(DebugCommand::Continue)
        );
        assert_eq!(parse("debug next"), Command::Debug(DebugCommand::Next));
        assert_eq!(parse("debug step"), Command::Debug(DebugCommand::StepIn));
        assert_eq!(parse("debug out"), Command::Debug(DebugCommand::StepOut));
        assert_eq!(
            parse("debug breakpoint"),
            Command::Debug(DebugCommand::ToggleBreakpoint)
        );
        assert_eq!(
            parse("debug breakpoints"),
            Command::Debug(DebugCommand::Breakpoints)
        );
        assert_eq!(parse("debug stack"), Command::Debug(DebugCommand::Stack));
        assert_eq!(
            parse("debug variables"),
            Command::Debug(DebugCommand::Variables)
        );
        assert_eq!(parse("debug output"), Command::Debug(DebugCommand::Output));
        assert_eq!(
            parse("debug configs"),
            Command::Debug(DebugCommand::Configurations)
        );
        assert_eq!(
            parse("debug restart"),
            Command::Debug(DebugCommand::Restart)
        );
        assert_eq!(
            parse("debug breakpoint condition x > 0"),
            Command::Debug(DebugCommand::BreakpointCondition("x > 0".to_string()))
        );
        assert_eq!(
            parse("debug breakpoint hit 3"),
            Command::Debug(DebugCommand::BreakpointHitCondition("3".to_string()))
        );
        assert_eq!(
            parse("debug breakpoint log value {x}"),
            Command::Debug(DebugCommand::BreakpointLogMessage("value {x}".to_string()))
        );
        assert_eq!(
            parse("debug breakpoint clear"),
            Command::Debug(DebugCommand::BreakpointClear)
        );
        assert_eq!(
            parse("debug function main"),
            Command::Debug(DebugCommand::FunctionBreakpoint("main".to_string()))
        );
        assert_eq!(
            parse("debug functions"),
            Command::Debug(DebugCommand::FunctionBreakpoints)
        );
        assert_eq!(
            parse("debug exceptions"),
            Command::Debug(DebugCommand::Exceptions)
        );
        assert_eq!(
            parse("debug exception uncaught"),
            Command::Debug(DebugCommand::ToggleException("uncaught".to_string()))
        );
        assert_eq!(
            parse("debug eval x + 1"),
            Command::Debug(DebugCommand::Evaluate("x + 1".to_string()))
        );
        assert_eq!(
            parse("debug console"),
            Command::Debug(DebugCommand::Console)
        );
        assert_eq!(
            parse("debug watch add x"),
            Command::Debug(DebugCommand::Watch("x".to_string()))
        );
        assert_eq!(
            parse("debug watch edit 2 x + 2"),
            Command::Debug(DebugCommand::EditWatch(2, "x + 2".to_string()))
        );
        assert_eq!(
            parse("debug watch remove x"),
            Command::Debug(DebugCommand::Unwatch("x".to_string()))
        );
        assert_eq!(
            parse("debug watches"),
            Command::Debug(DebugCommand::Watches)
        );
        assert_eq!(
            parse("debug threads"),
            Command::Debug(DebugCommand::Threads)
        );
        assert_eq!(
            parse("debug thread 2"),
            Command::Debug(DebugCommand::Thread(2))
        );
        assert_eq!(
            parse("debug expand 200"),
            Command::Debug(DebugCommand::ExpandVariables(200))
        );
        assert_eq!(
            parse("debug more 200"),
            Command::Debug(DebugCommand::MoreVariables(200))
        );
        assert_eq!(
            parse("debug set 100 x 42"),
            Command::Debug(DebugCommand::SetVariable {
                reference: 100,
                name: "x".to_string(),
                value: "42".to_string(),
            })
        );
        assert_eq!(
            parse("debug set 100 x = hello world"),
            Command::Debug(DebugCommand::SetVariable {
                reference: 100,
                name: "x".to_string(),
                value: "hello world".to_string(),
            })
        );
        assert_eq!(
            parse("debug frame 2"),
            Command::Debug(DebugCommand::Frame(2))
        );
        assert_eq!(
            parse_err("debug thread nope"),
            ParseFailure::invalid_argument("thread nope")
        );
        assert_eq!(
            parse_err("debug nope"),
            ParseFailure::invalid_argument("nope")
        );
    }

    #[test]
    fn parses_lsp_commands() {
        assert_eq!(parse("lsp"), Command::Lsp(LspCommand::Status));
        assert_eq!(parse("lsp status"), Command::Lsp(LspCommand::Status));
        assert_eq!(parse("lsp info"), Command::Lsp(LspCommand::Status));
        assert_eq!(parse("lsp start"), Command::Lsp(LspCommand::Start(None)));
        assert_eq!(
            parse("lsp start rust-analyzer"),
            Command::Lsp(LspCommand::Start(Some("rust-analyzer".to_string())))
        );
        assert_eq!(parse("lsp stop"), Command::Lsp(LspCommand::Stop(None)));
        assert_eq!(
            parse("lsp stop rust-analyzer"),
            Command::Lsp(LspCommand::Stop(Some("rust-analyzer".to_string())))
        );
        assert_eq!(
            parse("lsp restart"),
            Command::Lsp(LspCommand::Restart(None))
        );
        assert_eq!(
            parse("lsp restart rust-analyzer"),
            Command::Lsp(LspCommand::Restart(Some("rust-analyzer".to_string())))
        );
        assert_eq!(
            parse_err("lsp nope"),
            ParseFailure::invalid_argument("nope")
        );
    }

    #[test]
    fn parses_picker_commands() {
        assert_eq!(parse("pick files"), Command::Pick(PickerCommand::Files));
        assert_eq!(parse("pick file"), Command::Pick(PickerCommand::Files));
        assert_eq!(parse("pick grep"), Command::Pick(PickerCommand::Grep));
        assert_eq!(parse("pick live_grep"), Command::Pick(PickerCommand::Grep));
        assert_eq!(parse("pick buffers"), Command::Pick(PickerCommand::Buffers));
        assert_eq!(parse("pick replace"), Command::Pick(PickerCommand::Replace));
        assert_eq!(parse("pick symbols"), Command::Pick(PickerCommand::Symbols));
        assert_eq!(
            parse("pick workspace_symbols"),
            Command::Pick(PickerCommand::WorkspaceSymbols)
        );
        assert_eq!(parse("replace"), Command::Pick(PickerCommand::Replace));
        assert_eq!(parse("pick"), Command::Pick(PickerCommand::Files));
        assert_eq!(
            parse_err("pick nope"),
            ParseFailure::invalid_argument("nope")
        );
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
        assert_eq!(parse_err("set"), ParseFailure::missing_argument("set"));
        assert_eq!(
            parse_err("set shiftwidth 4 extra"),
            ParseFailure::trailing_characters("extra")
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
