use kawi_completion::Documentation;

use crate::text::{char_width, TAB_WIDTH};
use model::sanitize;

pub use model::{
    MarkdownBlock, MarkdownCodeBlock, MarkdownDocument, MarkdownLine, MarkdownSpan, MarkdownStyle,
};
pub use parse::parse;

mod model;
mod parse;

pub fn render_spans(spans: &[MarkdownSpan], width: usize) -> Vec<MarkdownLine> {
    let sanitized: Vec<MarkdownSpan> = spans
        .iter()
        .map(|span| MarkdownSpan {
            text: sanitize(&span.text),
            style: span.style,
        })
        .collect();
    let mut lines = Vec::new();
    push_wrapped(&mut lines, &sanitized, width.max(1), false);
    lines
}

pub fn render(document: &Documentation, width: usize) -> Vec<MarkdownLine> {
    render_document(&parse(document), width)
}

pub fn render_document(document: &MarkdownDocument, width: usize) -> Vec<MarkdownLine> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for block in &document.blocks {
        match block {
            MarkdownBlock::Text(source) => render_lines(source, width, &mut lines, false),
            MarkdownBlock::Code(code) => render_lines(&code.lines, width, &mut lines, true),
            MarkdownBlock::Rule => push_wrapped(
                &mut lines,
                &[MarkdownSpan {
                    text: "-".repeat(width.min(20)),
                    style: MarkdownStyle::Quote,
                }],
                width,
                false,
            ),
        }
    }
    lines
}

fn render_lines(source: &[MarkdownLine], width: usize, lines: &mut Vec<MarkdownLine>, code: bool) {
    for line in source {
        push_wrapped(lines, &line.spans, width, code);
    }
}

fn push_wrapped(lines: &mut Vec<MarkdownLine>, spans: &[MarkdownSpan], width: usize, code: bool) {
    if spans.is_empty() {
        lines.push(MarkdownLine {
            spans: Vec::new(),
            code,
        });
        return;
    }
    let mut writer = WrappedLine::new(lines, width, code, spans[0].style);
    for span in spans {
        for character in span.text.chars() {
            if character == '\t' {
                writer.push_tab(span.style);
            } else {
                writer.push_char(character, span.style);
            }
        }
    }
    writer.finish();
}

struct WrappedLine<'a> {
    lines: &'a mut Vec<MarkdownLine>,
    spans: Vec<MarkdownSpan>,
    current: MarkdownSpan,
    used: usize,
    width: usize,
    code: bool,
}

impl<'a> WrappedLine<'a> {
    fn new(
        lines: &'a mut Vec<MarkdownLine>,
        width: usize,
        code: bool,
        style: MarkdownStyle,
    ) -> Self {
        Self {
            lines,
            spans: Vec::new(),
            current: MarkdownSpan {
                text: String::new(),
                style,
            },
            used: 0,
            width,
            code,
        }
    }

    fn push_tab(&mut self, style: MarkdownStyle) {
        let spaces = TAB_WIDTH.saturating_sub(self.used % TAB_WIDTH);
        for _ in 0..spaces {
            self.push_cell(' ', style, 1);
        }
    }

    fn push_char(&mut self, character: char, style: MarkdownStyle) {
        self.push_cell(character, style, char_width(character));
    }

    fn push_cell(&mut self, character: char, style: MarkdownStyle, cells: usize) {
        if self.needs_wrap(cells) {
            self.wrap(style);
        }
        if self.current.style != style {
            self.flush_current();
            self.current.style = style;
        }
        self.current.text.push(character);
        self.used = self.used.saturating_add(cells);
    }

    fn needs_wrap(&self, cells: usize) -> bool {
        self.used >= self.width || (self.used > 0 && self.used.saturating_add(cells) > self.width)
    }

    fn wrap(&mut self, style: MarkdownStyle) {
        self.flush_current();
        self.lines.push(MarkdownLine {
            spans: std::mem::take(&mut self.spans),
            code: self.code,
        });
        self.current.style = style;
        self.used = 0;
    }

    fn finish(mut self) {
        self.flush_current();
        self.lines.push(MarkdownLine {
            spans: self.spans,
            code: self.code,
        });
    }

    fn flush_current(&mut self) {
        if !self.current.text.is_empty() {
            self.spans.push(MarkdownSpan {
                text: std::mem::take(&mut self.current.text),
                style: self.current.style,
            });
        }
    }
}

#[cfg(test)]
mod tests;
