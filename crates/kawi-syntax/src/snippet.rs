use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::ControlFlow;

use tree_sitter::{
    ParseOptions, Parser, Point, QueryCursor, QueryCursorOptions, StreamingIterator, Tree,
};

use crate::group::SyntaxGroup;
use crate::highlight::{refine_group, Runtime};
use crate::language::LanguageId;
use crate::limits::MAX_SNIPPET_BYTES;
use crate::span::SnippetSpan;

thread_local! {
    static RUNTIMES: RefCell<HashMap<LanguageId, Option<SnippetRuntime>>> =
        RefCell::new(HashMap::new());
}

struct SnippetRuntime {
    parser: Parser,
    runtime: Runtime,
}

impl SnippetRuntime {
    fn new(language: LanguageId) -> Option<Self> {
        let mut parser = Parser::new();
        parser.set_language(&language.language()).ok()?;
        let runtime = Runtime::new(language)?;
        Some(Self { parser, runtime })
    }

    fn spans(&mut self, text: &str, is_cancelled: &mut dyn FnMut() -> bool) -> Vec<SnippetSpan> {
        let Some(tree) = self.parse(text, is_cancelled) else {
            return Vec::new();
        };
        if is_cancelled() {
            return Vec::new();
        }
        let mut cancelled = false;
        let captures = {
            let mut cursor = QueryCursor::new();
            let mut progress = |_state: &tree_sitter::QueryCursorState| {
                if is_cancelled() {
                    cancelled = true;
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            };
            let options = QueryCursorOptions::new().progress_callback(&mut progress);
            let mut matches = cursor.captures_with_options(
                &self.runtime.config.query,
                tree.root_node(),
                text.as_bytes(),
                options,
            );
            let mut captures = Vec::new();
            while let Some((matched, capture_index)) = matches.next() {
                let Some(capture) = matched.captures().get(*capture_index) else {
                    continue;
                };
                let Some(group) = self
                    .runtime
                    .groups
                    .get(capture.index as usize)
                    .copied()
                    .flatten()
                    .map(|group| refine_group(group, capture.node.kind()))
                else {
                    continue;
                };
                captures.push(SnippetSpan {
                    start: capture.node.start_byte(),
                    end: capture.node.end_byte(),
                    group,
                });
            }
            captures
        };
        if cancelled {
            return Vec::new();
        }
        paint_spans(text, &captures)
    }

    fn parse(&mut self, text: &str, is_cancelled: &mut dyn FnMut() -> bool) -> Option<Tree> {
        let mut cancelled = false;
        let mut input = |offset: usize, _: Point| -> &[u8] {
            text.as_bytes().get(offset..).unwrap_or_default()
        };
        let mut progress = |_state: &tree_sitter::ParseState| {
            if is_cancelled() {
                cancelled = true;
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        };
        let options = ParseOptions::new().progress_callback(&mut progress);
        let tree = self
            .parser
            .parse_with_options(&mut input, None, Some(options));
        if cancelled {
            None
        } else {
            tree
        }
    }
}

pub fn highlight_snippet(language: LanguageId, text: &str) -> Vec<SnippetSpan> {
    highlight_snippet_with_cancel(language, text, || false)
}

pub fn highlight_snippet_with_cancel(
    language: LanguageId,
    text: &str,
    mut is_cancelled: impl FnMut() -> bool,
) -> Vec<SnippetSpan> {
    if text.is_empty() || text.len() > MAX_SNIPPET_BYTES {
        return Vec::new();
    }
    RUNTIMES.with(|runtimes| {
        let mut runtimes = runtimes.borrow_mut();
        let runtime = runtimes
            .entry(language)
            .or_insert_with(|| SnippetRuntime::new(language));
        runtime
            .as_mut()
            .map(|runtime| runtime.spans(text, &mut is_cancelled))
            .unwrap_or_default()
    })
}

fn paint_spans(text: &str, captures: &[SnippetSpan]) -> Vec<SnippetSpan> {
    let indices = char_indices(text);
    let mut cells = vec![None; indices[text.len()]];
    for capture in captures {
        if capture.start > capture.end
            || capture.end > text.len()
            || !text.is_char_boundary(capture.start)
            || !text.is_char_boundary(capture.end)
        {
            continue;
        }
        let start = indices[capture.start];
        let end = indices[capture.end];
        for cell in &mut cells[start..end] {
            *cell = Some(capture.group);
        }
    }
    merge_spans(text, &cells)
}

fn char_indices(text: &str) -> Vec<usize> {
    let mut indices = vec![0; text.len().saturating_add(1)];
    let mut count = 0;
    for (byte, _) in text.char_indices() {
        indices[byte] = count;
        count += 1;
    }
    indices[text.len()] = count;
    indices
}

fn merge_spans(text: &str, cells: &[Option<SyntaxGroup>]) -> Vec<SnippetSpan> {
    let mut spans = Vec::new();
    let mut current = None;
    let mut start = 0;
    for (index, (byte, _)) in text.char_indices().enumerate() {
        if cells[index] == current {
            continue;
        }
        if let Some(group) = current {
            spans.push(SnippetSpan {
                start,
                end: byte,
                group,
            });
        }
        current = cells[index];
        start = byte;
    }
    if let Some(group) = current {
        spans.push(SnippetSpan {
            start,
            end: text.len(),
            group,
        });
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups(language: LanguageId, text: &str) -> Vec<SyntaxGroup> {
        highlight_snippet(language, text)
            .into_iter()
            .map(|span| span.group)
            .collect()
    }

    #[test]
    fn highlights_every_bundled_language() {
        let snippets = [
            (LanguageId::Rust, "fn main() { let value = 1; }"),
            (LanguageId::Toml, "name = \"kawi\""),
            (LanguageId::TypeScript, "const value: number = 1;"),
            (LanguageId::Tsx, "const view = <div />;"),
            (LanguageId::JavaScript, "const value = 1;"),
            (LanguageId::Jsx, "const view = <div />;"),
            (LanguageId::Python, "def run():\n    return 1\n"),
            (LanguageId::Json, "{\"value\": 1}"),
            (LanguageId::Bash, "echo \"hi\""),
        ];
        for (language, snippet) in snippets {
            assert!(
                !groups(language, snippet).is_empty(),
                "{language:?} returned no groups"
            );
        }
    }

    #[test]
    fn highlights_a_bare_signature() {
        assert!(groups(LanguageId::Rust, "fn push(&mut self, value: T)")
            .contains(&SyntaxGroup::Keyword));
    }

    #[test]
    fn highlights_a_single_expression_in_every_language() {
        let fragments = [
            (LanguageId::Rust, "self.items.push(value)"),
            (LanguageId::Toml, "name = \"kawi\""),
            (LanguageId::TypeScript, "value + 1"),
            (LanguageId::Tsx, "<div className=\"x\" />"),
            (LanguageId::JavaScript, "value + 1"),
            (LanguageId::Jsx, "<div className=\"x\" />"),
            (LanguageId::Python, "value + 1"),
            (LanguageId::Json, "{\"value\": 1}"),
            (LanguageId::Bash, "echo \"hi\""),
        ];
        for (language, fragment) in fragments {
            assert!(
                !groups(language, fragment).is_empty(),
                "{language:?} fragment returned no groups"
            );
        }
    }

    #[test]
    fn spans_stay_on_utf8_boundaries() {
        let text = "let café = \"héllo\";";
        for span in highlight_snippet(LanguageId::Rust, text) {
            assert!(text.is_char_boundary(span.start));
            assert!(text.is_char_boundary(span.end));
        }
    }

    #[test]
    fn highlights_fragments_with_syntax_errors() {
        assert!(groups(LanguageId::Rust, "fn main(").contains(&SyntaxGroup::Keyword));
    }

    #[test]
    fn empty_and_oversized_inputs_return_no_spans() {
        assert!(highlight_snippet(LanguageId::Rust, "").is_empty());
        let oversized = "a".repeat(MAX_SNIPPET_BYTES + 1);
        assert!(highlight_snippet(LanguageId::Rust, &oversized).is_empty());
    }

    #[test]
    fn cancellation_returns_no_spans() {
        let text = "fn main() {}\n".repeat(4_000);
        assert!(highlight_snippet_with_cancel(LanguageId::Rust, &text, || true).is_empty());
    }
}
