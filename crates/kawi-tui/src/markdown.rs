use kawi_completion::Documentation;

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
    push_wrapped(&mut lines, &sanitized, width.max(1));
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
            MarkdownBlock::Text(source) => render_lines(source, width, &mut lines),
            MarkdownBlock::Code(code) => render_lines(&code.lines, width, &mut lines),
            MarkdownBlock::Rule => push_wrapped(
                &mut lines,
                &[MarkdownSpan {
                    text: "-".repeat(width.min(20)),
                    style: MarkdownStyle::Quote,
                }],
                width,
            ),
        }
    }
    lines
}

fn render_lines(source: &[MarkdownLine], width: usize, lines: &mut Vec<MarkdownLine>) {
    for line in source {
        push_wrapped(lines, &line.spans, width);
    }
}

fn push_wrapped(lines: &mut Vec<MarkdownLine>, spans: &[MarkdownSpan], width: usize) {
    if spans.is_empty() {
        lines.push(MarkdownLine::blank());
        return;
    }
    let mut line = Vec::new();
    let mut current = MarkdownSpan {
        text: String::new(),
        style: spans[0].style,
    };
    let mut used = 0;
    for span in spans {
        for character in span.text.chars() {
            if used == width {
                push_char(&mut line, &mut current);
                lines.push(MarkdownLine { spans: line });
                line = Vec::new();
                current = MarkdownSpan {
                    text: String::new(),
                    style: span.style,
                };
                used = 0;
            }
            if current.style != span.style {
                push_char(&mut line, &mut current);
                current.style = span.style;
            }
            current.text.push(character);
            used += 1;
        }
    }
    push_char(&mut line, &mut current);
    lines.push(MarkdownLine { spans: line });
}

fn push_char(line: &mut Vec<MarkdownSpan>, current: &mut MarkdownSpan) {
    if !current.text.is_empty() {
        line.push(MarkdownSpan {
            text: std::mem::take(&mut current.text),
            style: current.style,
        });
    }
}

#[cfg(test)]
mod tests;
