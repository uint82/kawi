use crate::frame::Rect;

pub struct DebugPanel<'a> {
    pub rect: Rect,
    pub title: &'a str,
    pub status: &'a str,
    pub focused: bool,
    pub rows: &'a [DebugRow],
    pub selected: Option<usize>,
    pub offset: usize,
}

pub struct DebugRow {
    pub depth: usize,
    pub label: String,
    pub kind: DebugRowKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugRowKind {
    Section,
    Thread { current: bool },
    Frame { current: bool },
    Variable { expandable: bool, expanded: bool },
    Watch,
    Breakpoint,
    FunctionBreakpoint,
    ExceptionFilter { enabled: bool },
    Console { error: bool },
    Action,
}
