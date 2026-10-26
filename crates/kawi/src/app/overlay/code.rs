use kawi_syntax::{highlight_snippet, LanguageId, SnippetSpan, SyntaxHighlight};
use kawi_tui::markdown::{MarkdownCodeBlock, MarkdownLine, MarkdownSpan, MarkdownStyle};
use kawi_tui::text::sanitize_char;

#[cfg(test)]
thread_local! {
    static HIGHLIGHT_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_highlight_calls() {
    HIGHLIGHT_CALLS.with(|calls| calls.set(0));
}

#[cfg(test)]
pub(crate) fn highlight_calls() -> usize {
    HIGHLIGHT_CALLS.with(std::cell::Cell::get)
}

pub(super) fn highlight_code_block(block: &mut MarkdownCodeBlock, fallback: Option<LanguageId>) {
    let Some(language) = code_language(block.language.as_deref(), fallback) else {
        return;
    };
    let text = block.text();
    #[cfg(test)]
    HIGHLIGHT_CALLS.with(|calls| calls.set(calls.get() + 1));
    let spans = highlight_snippet(language, &text);
    if !spans.is_empty() {
        block.lines = syntax_lines(&text, &spans);
    }
}

fn code_language(info: Option<&str>, fallback: Option<LanguageId>) -> Option<LanguageId> {
    match info {
        Some(alias) => LanguageId::from_alias(alias),
        None => fallback,
    }
}

pub(super) fn syntax_lines(text: &str, spans: &[SnippetSpan]) -> Vec<MarkdownLine> {
    if text.is_empty() {
        return Vec::new();
    }
    let source: Vec<&str> = text.split('\n').collect();
    let mut cells: Vec<Vec<MarkdownStyle>> = source
        .iter()
        .map(|line| vec![MarkdownStyle::Code; line.chars().count()])
        .collect();
    let ranges = line_ranges(text);
    for span in ordered_spans(spans) {
        apply_span(text, &source, &ranges, &mut cells, span);
    }
    source
        .iter()
        .zip(cells)
        .map(|(line, cells)| build_line(line, &cells))
        .collect()
}

fn ordered_spans(spans: &[SnippetSpan]) -> Vec<&SnippetSpan> {
    let mut ordered: Vec<_> = spans.iter().collect();
    ordered.sort_by(|left, right| {
        left.start
            .cmp(&right.start)
            .then_with(|| right.end.cmp(&left.end))
    });
    ordered
}

fn line_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            ranges.push((start, index));
            start = index + 1;
        }
    }
    ranges.push((start, text.len()));
    ranges
}

fn apply_span(
    text: &str,
    source: &[&str],
    ranges: &[(usize, usize)],
    cells: &mut [Vec<MarkdownStyle>],
    span: &SnippetSpan,
) {
    if span.start > span.end
        || span.end > text.len()
        || !text.is_char_boundary(span.start)
        || !text.is_char_boundary(span.end)
    {
        return;
    }
    for (index, (line, (start, end))) in source.iter().zip(ranges).enumerate() {
        if span.end <= *start || span.start >= *end {
            continue;
        }
        let from = span.start.max(*start).min(*end) - start;
        let to = span.end.min(*end) - start;
        fill_cells(line, &mut cells[index], from, to, span.highlight);
    }
}

fn fill_cells(
    line: &str,
    cells: &mut [MarkdownStyle],
    start: usize,
    end: usize,
    highlight: SyntaxHighlight,
) {
    let from = line[..start].chars().count();
    let to = line[..end].chars().count();
    let len = cells.len();
    for cell in &mut cells[from.min(len)..to.min(len)] {
        *cell = MarkdownStyle::Syntax(highlight);
    }
}

fn build_line(line: &str, cells: &[MarkdownStyle]) -> MarkdownLine {
    let mut spans = Vec::new();
    let mut text = String::new();
    let mut current = None;
    for (character, style) in line.chars().zip(cells) {
        if current != Some(*style) {
            push_span(&mut spans, &mut text, current);
            current = Some(*style);
        }
        text.push(sanitize_char(character));
    }
    push_span(&mut spans, &mut text, current);
    MarkdownLine { spans, code: true }
}

fn push_span(spans: &mut Vec<MarkdownSpan>, text: &mut String, style: Option<MarkdownStyle>) {
    let Some(style) = style else {
        return;
    };
    if !text.is_empty() {
        spans.push(MarkdownSpan {
            text: std::mem::take(text),
            style,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_syntax::registry;
    use kawi_tui::markdown::MarkdownLine;

    fn highlight(name: &str) -> SyntaxHighlight {
        registry().index_of(name).expect("registered capture")
    }

    fn line(text: &str) -> MarkdownLine {
        MarkdownLine {
            spans: vec![MarkdownSpan {
                text: text.to_string(),
                style: MarkdownStyle::Code,
            }],
            code: true,
        }
    }

    #[test]
    fn splits_spans_across_lines() {
        let text = "let value\nreturn value";
        let lines = syntax_lines(
            text,
            &[SnippetSpan {
                start: 0,
                end: text.len(),
                highlight: highlight("keyword"),
            }],
        );
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|line| {
            line.spans
                .iter()
                .all(|span| span.style == MarkdownStyle::Syntax(highlight("keyword")))
        }));
    }

    #[test]
    fn keeps_wide_characters_and_tabs_after_conversion() {
        let lines = syntax_lines(
            "\téx",
            &[SnippetSpan {
                start: 0,
                end: 4,
                highlight: highlight("type"),
            }],
        );
        assert_eq!(lines[0].text(), "\téx");
        assert_eq!(
            lines[0].spans[0].style,
            MarkdownStyle::Syntax(highlight("type"))
        );
    }

    #[test]
    fn nested_groups_use_the_inner_style() {
        let lines = syntax_lines(
            "abcdef",
            &[
                SnippetSpan {
                    start: 0,
                    end: 6,
                    highlight: highlight("keyword"),
                },
                SnippetSpan {
                    start: 2,
                    end: 4,
                    highlight: highlight("type"),
                },
            ],
        );
        assert_eq!(
            lines[0]
                .spans
                .iter()
                .map(|span| (span.text.as_str(), span.style))
                .collect::<Vec<_>>(),
            [
                ("ab", MarkdownStyle::Syntax(highlight("keyword"))),
                ("cd", MarkdownStyle::Syntax(highlight("type"))),
                ("ef", MarkdownStyle::Syntax(highlight("keyword"))),
            ]
        );
    }

    #[test]
    fn invalid_boundaries_keep_the_fallback_style() {
        let lines = syntax_lines(
            "é",
            &[SnippetSpan {
                start: 0,
                end: 1,
                highlight: highlight("keyword"),
            }],
        );
        assert_eq!(lines[0].text(), "é");
        assert_eq!(lines[0].spans[0].style, MarkdownStyle::Code);
    }

    #[test]
    fn oversized_code_keeps_the_fallback_style() {
        let text = "x".repeat(kawi_syntax::MAX_SNIPPET_BYTES + 1);
        let mut block = MarkdownCodeBlock {
            language: Some("rust".to_string()),
            lines: vec![line(&text)],
        };
        highlight_code_block(&mut block, None);
        assert_eq!(block.lines[0].text(), text);
        assert!(block.lines[0]
            .spans
            .iter()
            .all(|span| span.style == MarkdownStyle::Code));
    }

    #[test]
    fn overlays_keep_finer_capture_names() {
        let mut block = MarkdownCodeBlock {
            language: Some("python".to_string()),
            lines: vec![line("def greet(name: str):")],
        };
        highlight_code_block(&mut block, None);
        assert!(block.lines[0].spans.iter().any(|span| {
            let MarkdownStyle::Syntax(highlight) = span.style else {
                return false;
            };
            registry().name(highlight) == Some("variable.parameter")
        }));
    }

    #[test]
    fn tagged_code_uses_the_snippet_highlighter() {
        let mut block = MarkdownCodeBlock {
            language: Some("rust".to_string()),
            lines: vec![line("let value = 1;")],
        };
        highlight_code_block(&mut block, None);
        assert!(block.lines[0]
            .spans
            .iter()
            .any(|span| matches!(span.style, MarkdownStyle::Syntax(_))));
    }

    #[test]
    fn untagged_code_uses_the_fallback_language() {
        let mut block = MarkdownCodeBlock {
            language: None,
            lines: vec![line("let value = 1;")],
        };
        highlight_code_block(&mut block, Some(LanguageId::Rust));
        assert!(block.lines[0]
            .spans
            .iter()
            .any(|span| span.style == MarkdownStyle::Syntax(highlight("keyword"))));
    }

    #[test]
    fn untagged_code_without_fallback_keeps_the_code_style() {
        let mut block = MarkdownCodeBlock {
            language: None,
            lines: vec![line("raw")],
        };
        highlight_code_block(&mut block, None);
        assert_eq!(block.lines[0].spans[0].style, MarkdownStyle::Code);
    }

    #[test]
    fn wrapping_keeps_wide_characters_and_tabs_aligned() {
        let text = "中\t文";
        let lines = syntax_lines(
            text,
            &[SnippetSpan {
                start: 0,
                end: text.len(),
                highlight: highlight("type"),
            }],
        );
        let document = kawi_tui::markdown::MarkdownDocument {
            blocks: vec![kawi_tui::markdown::MarkdownBlock::Code(MarkdownCodeBlock {
                language: None,
                lines,
            })],
        };
        let wrapped = kawi_tui::markdown::render_document(&document, 1);
        assert_eq!(
            wrapped.iter().map(MarkdownLine::text).collect::<Vec<_>>(),
            ["中", " ", " ", " ", " ", " ", " ", "文"]
        );
        assert!(wrapped
            .iter()
            .flat_map(|line| &line.spans)
            .all(|span| span.style == MarkdownStyle::Syntax(highlight("type"))));
    }

    #[test]
    fn unknown_code_keeps_the_fallback_style() {
        let mut block = MarkdownCodeBlock {
            language: Some("brainfuck".to_string()),
            lines: vec![line("++")],
        };
        highlight_code_block(&mut block, Some(LanguageId::Rust));
        assert_eq!(block.lines[0].spans[0].style, MarkdownStyle::Code);
    }
}
