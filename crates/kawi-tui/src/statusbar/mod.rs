use anyhow::Result;
use std::io::Stdout;

use crate::theme::Theme;

mod build;
mod layout;
mod paint;
mod prune;
mod segment;
mod text;

#[cfg(test)]
mod tests;

pub use crate::frame::DiagnosticSeverity;
pub use build::build_segments;
pub use layout::{PlacedSegment, StatusLine};
pub use segment::{
    ModeKind, Section, Segment, SegmentId, SegmentStyle, StatusBarConfig, REQUIRED_PRIORITY,
};

#[derive(Clone, Copy)]
pub struct GitStatusInfo<'a> {
    pub branch: &'a str,
    pub operation: Option<&'a str>,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Clone, Copy)]
pub struct StatusArea {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub active: bool,
}

pub struct StatusInfo<'a> {
    pub filename: &'a str,
    pub dirty: bool,
    pub readonly: bool,
    pub row: usize,
    pub col: usize,
    pub virtcol: usize,
    pub line_empty: bool,
    pub empty_buffer: bool,
    pub mode: &'a str,
    pub mode_kind: ModeKind,
    pub lsp: Option<&'a str>,
    pub diagnostics: Option<&'a str>,
    pub diagnostics_severity: Option<DiagnosticSeverity>,
    pub workspace_diagnostics: Option<&'a str>,
    pub workspace_diagnostics_severity: Option<DiagnosticSeverity>,
    pub external: Option<&'a str>,
    pub git: Option<GitStatusInfo<'a>>,
    pub filetype: Option<&'a str>,
    pub indent: Option<&'a str>,
    pub encoding: Option<&'a str>,
    pub eol: Option<&'a str>,
    pub selection: Option<&'a str>,
    pub macro_recording: Option<char>,
    pub percent: Option<usize>,
}

pub fn draw_status(
    out: &mut Stdout,
    info: &StatusInfo<'_>,
    area: StatusArea,
    theme: &Theme,
    config: &StatusBarConfig,
) -> Result<()> {
    let segments = build_segments(info, area.active, config);
    let line = layout::layout(&segments, area.width, &config.separator);
    paint::draw_line(out, &line, area.x, area.y, area.active, theme)
}
