use std::collections::BTreeMap;

use kawi_editor::Editor;
use kawi_plugin::{
    sanitize, Decoration as ProtocolDecoration, DecorationStyle as PluginStyle, Position, RpcError,
};
use kawi_tui::frame::{DecorationSpan, DecorationStyle, PluginSign, Sign, VirtualText};
use kawi_tui::text::display_width;
use ropey::Rope;

use super::host_methods::position_to_char;
use super::style::frame_style;

#[derive(Default)]
pub(crate) struct PluginDecorations {
    sets: BTreeMap<(String, usize), DecorationSet>,
}

struct DecorationSet {
    revision: u64,
    highlights: Vec<Highlight>,
    virtual_texts: Vec<PluginVirtualText>,
    signs: Vec<SignMark>,
}

struct Highlight {
    start: LineColumn,
    end: LineColumn,
    style: DecorationStyle,
}

struct PluginVirtualText {
    line: usize,
    text: String,
    style: DecorationStyle,
}

struct SignMark {
    line: usize,
    text: String,
    style: DecorationStyle,
}

#[derive(Clone, Copy)]
struct LineColumn {
    line: usize,
    column: usize,
}

impl PluginDecorations {
    pub(crate) fn set(
        &mut self,
        plugin: &str,
        buffer: usize,
        revision: u64,
        decorations: &[ProtocolDecoration],
        snapshot: &Rope,
    ) -> Result<(), RpcError> {
        let set = DecorationSet::new(revision, decorations, snapshot)?;
        self.sets.insert((plugin.to_string(), buffer), set);
        Ok(())
    }

    pub(crate) fn remove_plugin(&mut self, plugin: &str) {
        self.sets.retain(|(name, _), _| name != plugin);
    }

    pub(crate) fn retain_live(&mut self, editor: &Editor) {
        self.sets
            .retain(|(_, buffer), set| editor.buffer_revision_at(*buffer) == Some(set.revision));
    }

    pub(crate) fn has_signs(&self, buffer: usize, revision: u64) -> bool {
        self.sets.iter().any(|((_, number), set)| {
            *number == buffer && set.revision == revision && !set.signs.is_empty()
        })
    }

    pub(crate) fn signs(&self, buffer: usize, revision: u64, rows: &[usize]) -> Vec<Option<Sign>> {
        let mut signs = vec![None; rows.len()];
        for ((_, number), set) in &self.sets {
            if *number != buffer || set.revision != revision {
                continue;
            }
            for mark in &set.signs {
                let Some(index) = rows.iter().position(|row| *row == mark.line) else {
                    continue;
                };
                if signs[index].is_none() {
                    signs[index] = Some(Sign::Plugin(PluginSign {
                        text: mark.text.clone(),
                        style: mark.style,
                    }));
                }
            }
        }
        signs
    }

    pub(crate) fn spans(
        &self,
        buffer: usize,
        revision: u64,
        rows: &[usize],
    ) -> Vec<DecorationSpan> {
        let mut spans = Vec::new();
        for ((_, number), set) in &self.sets {
            if *number != buffer || set.revision != revision {
                continue;
            }
            for highlight in &set.highlights {
                highlight.push_spans(rows, &mut spans);
            }
        }
        spans
    }

    pub(crate) fn virtual_texts(
        &self,
        buffer: usize,
        revision: u64,
        rows: &[usize],
    ) -> Vec<Vec<VirtualText>> {
        let mut texts = std::iter::repeat_with(Vec::new)
            .take(rows.len())
            .collect::<Vec<Vec<VirtualText>>>();
        for ((_, number), set) in &self.sets {
            if *number != buffer || set.revision != revision {
                continue;
            }
            for text in &set.virtual_texts {
                let Some(index) = rows.iter().position(|row| *row == text.line) else {
                    continue;
                };
                texts[index].push(VirtualText::plugin(text.text.clone(), text.style));
            }
        }
        texts
    }
}

impl DecorationSet {
    fn new(
        revision: u64,
        decorations: &[ProtocolDecoration],
        snapshot: &Rope,
    ) -> Result<Self, RpcError> {
        let mut highlights = Vec::new();
        let mut virtual_texts = Vec::new();
        let mut signs = Vec::new();
        for decoration in decorations {
            match decoration {
                ProtocolDecoration::Highlight { start, end, style } => {
                    if let Some(highlight) = Highlight::new(start, end, *style, snapshot)? {
                        highlights.push(highlight);
                    }
                }
                ProtocolDecoration::VirtualText { line, text, style } => {
                    if let Some(text) = PluginVirtualText::new(*line, text, *style, snapshot)? {
                        virtual_texts.push(text);
                    }
                }
                ProtocolDecoration::Sign { line, text, style } => {
                    if let Some(sign) = SignMark::new(*line, text, *style, snapshot)? {
                        signs.push(sign);
                    }
                }
            }
        }
        Ok(Self {
            revision,
            highlights,
            virtual_texts,
            signs,
        })
    }
}

impl Highlight {
    fn new(
        start: &Position,
        end: &Position,
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
            style: frame_style(style),
        }))
    }

    fn push_spans(&self, rows: &[usize], spans: &mut Vec<DecorationSpan>) {
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
}

impl PluginVirtualText {
    fn new(
        line: usize,
        text: &str,
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
            style: frame_style(style),
        }))
    }
}

impl SignMark {
    fn new(
        line: usize,
        text: &str,
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
            style: frame_style(style),
        }))
    }
}

fn line_column(snapshot: &Rope, offset: usize) -> LineColumn {
    let line = snapshot.char_to_line(offset);
    LineColumn {
        line,
        column: offset.saturating_sub(snapshot.line_to_char(line)),
    }
}

#[cfg(test)]
mod tests;
