use std::collections::BTreeMap;

use kawi_editor::Mode;

use crate::frame::{DecorationStyle, DiagnosticSeverity};
pub const REQUIRED_PRIORITY: u8 = u8::MAX;
pub const BADGE_PADDING: usize = 1;

const DEFAULT_LEFT: &[SegmentId] = &[
    SegmentId::Mode,
    SegmentId::File,
    SegmentId::Modified,
    SegmentId::ReadOnly,
    SegmentId::Git,
];

const DEFAULT_RIGHT: &[SegmentId] = &[
    SegmentId::Plugins,
    SegmentId::FileType,
    SegmentId::Encoding,
    SegmentId::Position,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModeKind {
    Normal,
    Insert,
    Replace,
    Visual,
    Command,
    Search,
}

impl ModeKind {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "normal" => Some(Self::Normal),
            "insert" => Some(Self::Insert),
            "replace" => Some(Self::Replace),
            "visual" => Some(Self::Visual),
            "command" => Some(Self::Command),
            "search" => Some(Self::Search),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Insert => "insert",
            Self::Replace => "replace",
            Self::Visual => "visual",
            Self::Command => "command",
            Self::Search => "search",
        }
    }
}

impl From<Mode> for ModeKind {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Normal => Self::Normal,
            Mode::Insert => Self::Insert,
            Mode::Replace => Self::Replace,
            Mode::Visual | Mode::VisualLine | Mode::VisualBlock => Self::Visual,
            Mode::Command => Self::Command,
            Mode::Search => Self::Search,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SegmentId {
    Mode,
    File,
    Modified,
    ReadOnly,
    Diagnostics,
    WorkspaceDiagnostics,
    Git,
    Lsp,
    Selection,
    Macro,
    FileType,
    Indent,
    Encoding,
    Eol,
    Percent,
    Position,
    Plugins,
}

impl SegmentId {
    pub const ALL: [Self; 17] = [
        Self::Mode,
        Self::File,
        Self::Modified,
        Self::ReadOnly,
        Self::Diagnostics,
        Self::WorkspaceDiagnostics,
        Self::Git,
        Self::Lsp,
        Self::Selection,
        Self::Macro,
        Self::FileType,
        Self::Indent,
        Self::Encoding,
        Self::Eol,
        Self::Percent,
        Self::Position,
        Self::Plugins,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Mode => "mode",
            Self::File => "file",
            Self::Modified => "modified",
            Self::ReadOnly => "readonly",
            Self::Diagnostics => "diagnostics",
            Self::WorkspaceDiagnostics => "workspace_diagnostics",
            Self::Git => "git",
            Self::Lsp => "lsp",
            Self::Selection => "selection",
            Self::Macro => "macro",
            Self::FileType => "filetype",
            Self::Indent => "indent",
            Self::Encoding => "encoding",
            Self::Eol => "eol",
            Self::Percent => "percent",
            Self::Position => "position",
            Self::Plugins => "plugins",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|id| id.name() == name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SegmentStyle {
    Default,
    Mode(ModeKind),
    Modified,
    ReadOnly,
    Diagnostic(DiagnosticSeverity),
    Git,
    Lsp,
    Selection,
    Macro,
    FileType,
    Indent,
    Encoding,
    Eol,
    Percent,
    Position,
    Plugin(DecorationStyle),
}

impl SegmentStyle {
    pub fn is_badge(self) -> bool {
        matches!(
            self,
            Self::Mode(_)
                | Self::Modified
                | Self::ReadOnly
                | Self::Diagnostic(_)
                | Self::Git
                | Self::Lsp
                | Self::Selection
                | Self::Macro
                | Self::Plugin(_)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub id: SegmentId,
    pub text: String,
    pub style: SegmentStyle,
    pub priority: u8,
    pub section: Section,
    pub flex: bool,
    pub min_width: usize,
    pub padding: usize,
}

impl Segment {
    pub fn is_required(&self) -> bool {
        self.priority == REQUIRED_PRIORITY
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusBarConfig {
    pub left: Vec<SegmentId>,
    pub center: Vec<SegmentId>,
    pub right: Vec<SegmentId>,
    pub separator: String,
    pub color_modes: bool,
    pub mode_labels: BTreeMap<ModeKind, String>,
}

impl Default for StatusBarConfig {
    fn default() -> Self {
        Self {
            left: DEFAULT_LEFT.to_vec(),
            center: Vec::new(),
            right: DEFAULT_RIGHT.to_vec(),
            separator: " ".to_string(),
            color_modes: false,
            mode_labels: default_mode_labels(),
        }
    }
}

fn default_mode_labels() -> BTreeMap<ModeKind, String> {
    BTreeMap::from([
        (ModeKind::Normal, "-- NORMAL --".to_string()),
        (ModeKind::Insert, "-- INSERT --".to_string()),
        (ModeKind::Replace, "-- REPLACE --".to_string()),
        (ModeKind::Visual, "-- VISUAL --".to_string()),
        (ModeKind::Command, "-- COMMAND --".to_string()),
        (ModeKind::Search, "-- SEARCH --".to_string()),
    ])
}
