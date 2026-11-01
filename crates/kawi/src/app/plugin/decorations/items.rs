use kawi_plugin::{sanitize, DecorationRef, Position, RpcError};
use kawi_tui::frame::{DecorationSpan, PluginStyle};
use kawi_tui::text::display_width;
use ropey::Rope;

use super::super::host_methods::position_to_char;
use super::super::style::PluginStyles;

pub(super) struct Highlight {
    start: LineColumn,
    end: LineColumn,
    appearance: DecorationRef,
    pub(super) style: PluginStyle,
}

pub(super) struct PluginVirtualText {
    pub(super) line: usize,
    pub(super) text: String,
    appearance: DecorationRef,
    pub(super) style: PluginStyle,
}

pub(super) struct SignMark {
    pub(super) line: usize,
    pub(super) text: String,
    appearance: DecorationRef,
    pub(super) style: PluginStyle,
}

#[derive(Clone, Copy)]
struct LineColumn {
    line: usize,
    column: usize,
}

impl Highlight {
    pub(super) fn new(
        start: &Position,
        end: &Position,
        appearance: DecorationRef,
        style: PluginStyle,
        snapshot: &Rope,
    ) -> Result<Option<Self>, RpcError> {
        let start_offset = position_to_char(snapshot, start)?;
        let end_offset = position_to_char(snapshot, end)?;
        if start_offset > end_offset {
            return Err(RpcError::invalid_position(
                "decoration start is after its end".to_string(),
            ));
        }
        if start_offset == end_offset {
            return Ok(None);
        }
        Ok(Some(Self {
            start: line_column(snapshot, start_offset),
            end: line_column(snapshot, end_offset),
            appearance,
            style,
        }))
    }

    pub(super) fn push_spans(&self, rows: &[usize], spans: &mut Vec<DecorationSpan>) {
        for row in rows {
            if *row < self.start.line || *row > self.end.line {
                continue;
            }
            let start = if *row == self.start.line {
                self.start.column
            } else {
                0
            };
            let end = if *row == self.end.line {
                self.end.column
            } else {
                usize::MAX
            };
            if start >= end {
                continue;
            }
            spans.push(DecorationSpan {
                row: *row,
                start,
                end,
                style: self.style,
            });
        }
    }

    pub(super) fn retheme(&mut self, plugin: &str, styles: &PluginStyles) {
        retheme(&self.appearance, &mut self.style, plugin, styles);
    }
}

impl PluginVirtualText {
    pub(super) fn new(
        line: usize,
        text: &str,
        appearance: DecorationRef,
        style: PluginStyle,
        snapshot: &Rope,
    ) -> Result<Option<Self>, RpcError> {
        if line >= snapshot.len_lines() {
            return Err(RpcError::invalid_position(format!(
                "decoration line {line} is out of range"
            )));
        }
        let text = sanitize(text);
        if text.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self {
            line,
            text,
            appearance,
            style,
        }))
    }

    pub(super) fn retheme(&mut self, plugin: &str, styles: &PluginStyles) {
        retheme(&self.appearance, &mut self.style, plugin, styles);
    }
}

impl SignMark {
    pub(super) fn new(
        line: usize,
        text: &str,
        appearance: DecorationRef,
        style: PluginStyle,
        snapshot: &Rope,
    ) -> Result<Option<Self>, RpcError> {
        if line >= snapshot.len_lines() {
            return Err(RpcError::invalid_position(format!(
                "decoration line {line} is out of range"
            )));
        }
        let text = sanitize(text);
        if text.is_empty() {
            return Ok(None);
        }
        if display_width(&text) != 1 {
            return Err(RpcError::invalid_params(
                "sign text must occupy exactly one cell".to_string(),
            ));
        }
        Ok(Some(Self {
            line,
            text,
            appearance,
            style,
        }))
    }

    pub(super) fn retheme(&mut self, plugin: &str, styles: &PluginStyles) {
        retheme(&self.appearance, &mut self.style, plugin, styles);
    }
}

fn retheme(
    appearance: &DecorationRef,
    style: &mut PluginStyle,
    plugin: &str,
    styles: &PluginStyles,
) {
    if let Some(next) = styles.resolve(plugin, appearance) {
        *style = next;
    }
}

fn line_column(snapshot: &Rope, offset: usize) -> LineColumn {
    let line = snapshot.char_to_line(offset);
    LineColumn {
        line,
        column: offset.saturating_sub(snapshot.line_to_char(line)),
    }
}
