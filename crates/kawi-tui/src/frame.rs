use crate::statusbar::StatusInfo;
use crate::theme::Theme;
use kawi_editor::Viewport;
use kawi_syntax::SyntaxSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl Rect {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> usize {
        self.x.saturating_add(self.width)
    }

    pub fn bottom(&self) -> usize {
        self.y.saturating_add(self.height)
    }

    pub fn contains(&self, column: usize, row: usize) -> bool {
        column >= self.x && column < self.right() && row >= self.y && row < self.bottom()
    }
}

pub struct Frame<'a> {
    pub panes: &'a [PaneFrame<'a>],
    pub separators: &'a [Rect],
    pub pager: Option<&'a [String]>,
    pub completion: Option<CompletionMenu<'a>>,
    pub picker: Option<PickerMenu<'a>>,
    pub replace: Option<ReplaceMenu<'a>>,
    pub task: Option<TaskPanel<'a>>,
    pub viewport: &'a Viewport,
    pub message: Option<&'a str>,
    pub prompt: Option<PromptMenu<'a>>,
    pub command: Option<&'a str>,
    pub search: Option<&'a str>,
    pub search_delimiter: char,
    pub search_status: Option<&'a str>,
    pub showcmd: Option<String>,
    pub cursor_style: CursorStyle,
    pub theme: &'a Theme,
}

pub struct TaskPanel<'a> {
    pub rect: Rect,
    pub title: &'a str,
    pub status: &'a str,
    pub lines: &'a [TaskLine<'a>],
}

pub struct TaskLine<'a> {
    pub text: &'a str,
    pub stderr: bool,
}

pub struct PaneFrame<'a> {
    pub rect: Rect,
    pub active: bool,
    pub lines: &'a [String],
    pub gutter: &'a [String],
    pub signs: Option<&'a [Option<Sign>]>,
    pub diagnostic_spans: &'a [DiagnosticSpan],
    pub search_spans: &'a [SearchSpan],
    pub preview_spans: &'a [SearchSpan],
    pub syntax_spans: &'a [SyntaxSpan],
    pub selection: Option<Selection>,
    pub viewport: Viewport,
    pub cursor_col: usize,
    pub row: usize,
    pub tabstop: usize,
    pub status: &'a StatusInfo<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    Git(GitSign),
    Diagnostic(DiagnosticSeverity),
}

impl Sign {
    pub fn text(self) -> &'static str {
        match self {
            Self::Git(sign) => sign.text(),
            Self::Diagnostic(severity) => severity.sign(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

impl DiagnosticSeverity {
    pub fn sign(self) -> &'static str {
        match self {
            Self::Error => "E",
            Self::Warning => "W",
            Self::Information => "I",
            Self::Hint => "H",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticSpan {
    pub row: usize,
    pub start: usize,
    pub end: usize,
    pub severity: DiagnosticSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitSign {
    Add,
    Change,
    Delete,
    TopDelete,
    ChangeDelete,
    Untracked,
    StagedAdd,
    StagedChange,
    StagedDelete,
    StagedTopDelete,
    StagedChangeDelete,
}

impl GitSign {
    pub fn text(self) -> &'static str {
        match self {
            Self::Add | Self::Change | Self::StagedAdd | Self::StagedChange => "┃",
            Self::Delete | Self::StagedDelete => "▁",
            Self::TopDelete | Self::StagedTopDelete => "▔",
            Self::ChangeDelete | Self::StagedChangeDelete => "~",
            Self::Untracked => "┆",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    Block,
    Bar,
    Underline,
}

pub struct PromptMenu<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub cursor: usize,
}

pub struct CompletionMenu<'a> {
    pub items: &'a [String],
    pub selected: Option<usize>,
}

pub struct PickerMenu<'a> {
    pub title: &'a str,
    pub query: &'a str,
    pub cursor: usize,
    pub items: &'a [String],
    pub selected: Option<usize>,
    pub loading: bool,
    pub status: Option<&'a str>,
    pub replace_hint: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplaceStage {
    Search,
    Replacement,
    Preview,
}

pub struct ReplaceMenu<'a> {
    pub stage: ReplaceStage,
    pub query: &'a str,
    pub replacement: &'a str,
    pub cursor: usize,
    pub items: &'a [String],
    pub selected: Option<usize>,
    pub loading: bool,
    pub status: Option<&'a str>,
    pub matches: usize,
    pub files: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    Range {
        start: (usize, usize),
        end: (usize, usize),
    },
    Block {
        top: usize,
        left: usize,
        bottom: usize,
        right: usize,
        ragged: bool,
    },
}

pub struct SearchSpan {
    pub row: usize,
    pub start: usize,
    pub end: usize,
    pub current: bool,
}
