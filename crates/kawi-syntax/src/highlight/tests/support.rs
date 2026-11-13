use ropey::Rope;

use super::super::*;
use crate::registry::registry;
use crate::span::SyntaxKey;

pub(super) fn request(text: &str, top: usize, end: usize, revision: u64) -> SyntaxRequest {
    request_for(LanguageId::Rust, text, top, end, revision)
}

pub(super) fn request_for(
    language: LanguageId,
    text: &str,
    top: usize,
    end: usize,
    revision: u64,
) -> SyntaxRequest {
    SyntaxRequest {
        base_revision: 0,
        edits: Vec::new(),
        key: SyntaxKey {
            document: 1,
            revision,
            top,
            end,
            language,
        },
        text: Rope::from_str(text),
    }
}

pub(super) fn highlight(
    language: LanguageId,
    text: &str,
    top: usize,
    end: usize,
) -> Vec<SyntaxSpan> {
    let runtime = Runtime::new(language).expect("runtime");
    let mut document = Document::new(language);
    document.update(&request(text, top, end, 1), &runtime).1
}

pub(super) fn name_at(spans: &[SyntaxSpan], row: usize, column: usize) -> Option<&'static str> {
    spans
        .iter()
        .find(|span| span.row == row && span.start <= column && column < span.end)
        .and_then(|span| registry().name(span.highlight))
}

pub(super) fn name_of(
    spans: &[SyntaxSpan],
    text: &str,
    row: usize,
    needle: &str,
) -> Option<&'static str> {
    let column = text.lines().nth(row)?.find(needle)?;
    name_at(spans, row, column)
}
