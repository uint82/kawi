use crate::markdown::MarkdownLine;
use crate::statusbar::{StatusBarConfig, StatusInfo};
use crate::theme::Theme;
use kawi_editor::{BracketMatch, Viewport};
use kawi_syntax::SyntaxSpan;

mod explorer;
mod keymenu;

pub use explorer::{ExplorerGit, ExplorerGitKind, ExplorerPanel, ExplorerRow, ExplorerRowKind};
pub use keymenu::{KeyColumn, KeyMenu, KeyRow};

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
    pub overlay: Option<Overlay<'a>>,
    pub picker: Option<PickerMenu<'a>>,
    pub replace: Option<ReplaceMenu<'a>>,
    pub explorer: Option<ExplorerPanel<'a>>,
    pub task: Option<TaskPanel<'a>>,
    pub terminal: Option<TerminalPanel<'a>>,
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
    pub statusline: &'a StatusBarConfig,
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

pub struct TerminalPanel<'a> {
    pub rect: Rect,
    pub title: &'a str,
    pub status: &'a str,
    pub lines: &'a [TerminalLine],
    pub cursor: Option<TerminalCursor>,
    pub focused: bool,
}

pub struct TerminalLine {
    pub spans: Vec<TerminalSpan>,
}

pub struct TerminalSpan {
    pub text: String,
    pub foreground: TerminalColor,
    pub background: TerminalColor,
    pub attrs: TerminalAttrs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalColor {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerminalAttrs {
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCursor {
    pub row: usize,
    pub col: usize,
    pub style: CursorStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualTextKind {
    Diagnostic(DiagnosticSeverity),
    Metadata,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VirtualTextPlacement {
    #[default]
    AfterLine,
    BeforeLine,
}

pub struct VirtualText {
    pub text: String,
    pub kind: VirtualTextKind,
    pub placement: VirtualTextPlacement,
}

impl VirtualText {
    pub fn diagnostic(text: impl Into<String>, severity: DiagnosticSeverity) -> Self {
        Self {
            text: text.into(),
            kind: VirtualTextKind::Diagnostic(severity),
            placement: VirtualTextPlacement::AfterLine,
        }
    }

    pub fn metadata(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: VirtualTextKind::Metadata,
            placement: VirtualTextPlacement::BeforeLine,
        }
    }
}

pub struct PaneFrame<'a> {
    pub rect: Rect,
    pub active: bool,
    pub lines: &'a [String],
    pub rows: &'a [usize],
    pub gutter: &'a [String],
    pub signs: Option<&'a [Option<Sign>]>,
    pub diagnostic_spans: &'a [DiagnosticSpan],
    pub virtual_texts: &'a [Option<VirtualText>],
    pub left_width: usize,
    pub search_spans: &'a [SearchSpan],
    pub preview_spans: &'a [SearchSpan],
    pub syntax_spans: &'a [SyntaxSpan],
    pub rulers: &'a [usize],
    pub selection: Option<Selection>,
    pub bracket_match: Option<BracketMatch>,
    pub viewport: Viewport,
    pub cursor_col: usize,
    pub cursor_screen_row: Option<usize>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
pub enum DiagnosticTag {
    Unnecessary,
    Deprecated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticSpan {
    pub row: usize,
    pub start: usize,
    pub end: usize,
    pub severity: DiagnosticSeverity,
    pub tag: Option<DiagnosticTag>,
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

pub struct Overlay<'a> {
    pub list: Option<OverlayList<'a>>,
    pub panel: Option<OverlayPanel<'a>>,
    pub key_menu: Option<KeyMenu<'a>>,
}

pub struct OverlayList<'a> {
    pub rect: Rect,
    pub items: &'a [String],
    pub selected: Option<usize>,
}

pub struct OverlayPanel<'a> {
    pub rect: Rect,
    pub lines: &'a [MarkdownLine],
    pub scroll: usize,
}

pub struct PickerMenu<'a> {
    pub title: &'a str,
    pub query: &'a str,
    pub cursor: usize,
    pub items: &'a [PickerLine<'a>],
    pub selected: Option<usize>,
    pub loading: bool,
    pub status: Option<&'a str>,
    pub replace_hint: bool,
    pub file_hint: bool,
    pub preview_enabled: bool,
    pub preview: Option<PickerPreview<'a>>,
}

pub struct PickerLine<'a> {
    pub primary: &'a str,
    pub detail: Option<&'a str>,
    pub deprecated: bool,
}

pub struct PickerPreview<'a> {
    pub title: &'a str,
    pub lines: &'a [String],
    pub first_line: usize,
    pub total_lines: usize,
    pub focus_line: Option<usize>,
    pub syntax_spans: &'a [SyntaxSpan],
    pub tabstop: usize,
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
