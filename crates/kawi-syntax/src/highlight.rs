use std::ops::Range;

use tree_sitter::{Parser, QueryCursor, Range as TreeRange, StreamingIterator, Tree};
use tree_sitter_highlight::HighlightConfiguration;

use crate::diff::{input_edit, LineIndex};
use crate::group::SyntaxGroup;
use crate::language::LanguageId;
use crate::limits::{MAX_CAPTURES, WINDOW_CONTEXT_LINES, WINDOW_PARSE_BYTES};
use crate::rows::{self, RowWindow};
use crate::span::{SyntaxRequest, SyntaxSpan};

pub(crate) struct Runtime {
    pub(crate) config: HighlightConfiguration,
    pub(crate) groups: Vec<Option<SyntaxGroup>>,
}

impl Runtime {
    pub(crate) fn new(language: LanguageId) -> Option<Self> {
        let config = HighlightConfiguration::new(
            language.language(),
            language.name(),
            &language.highlights(),
            "",
            "",
        )
        .ok()?;
        let groups = config
            .names()
            .iter()
            .map(|name| SyntaxGroup::from_capture(name))
            .collect();
        Some(Self { config, groups })
    }
}

pub(crate) struct Document {
    parser: Parser,
    text: String,
    tree: Option<Tree>,
    revision: u64,
    index: LineIndex,
    window_rows: Range<usize>,
    large: bool,
}

impl Document {
    pub(crate) fn new(language: LanguageId) -> Self {
        let mut parser = Parser::new();
        let _ = parser.set_language(&language.language());
        Self {
            parser,
            text: String::new(),
            tree: None,
            revision: 0,
            index: LineIndex::new(""),
            window_rows: 0..0,
            large: false,
        }
    }

    pub(crate) fn update(
        &mut self,
        request: &SyntaxRequest,
        runtime: &Runtime,
    ) -> (Range<usize>, Vec<SyntaxSpan>) {
        let large = request.text.len_bytes() > WINDOW_PARSE_BYTES;
        let revision_changed = request.key.revision != self.revision;
        if large {
            if revision_changed || self.tree.is_none() || !self.large {
                self.refresh_text(request);
            }
            if revision_changed
                || !self.large
                || self.tree.is_none()
                || !self.window_covers(request)
            {
                self.parse_window(request);
            }
        } else if self.tree.is_none() || revision_changed || self.large {
            self.reparse_full(request);
        }
        let window = if self.large {
            self.window_rows.clone()
        } else {
            window_bounds(&self.index, request.key.top, request.key.end)
        };
        let spans = self.spans(runtime, window.clone());
        (window, spans)
    }

    fn refresh_text(&mut self, request: &SyntaxRequest) {
        self.text = request.text.to_string();
        self.index = LineIndex::new(&self.text);
        self.revision = request.key.revision;
    }

    fn reparse_full(&mut self, request: &SyntaxRequest) {
        let text = request.text.to_string();
        let index = LineIndex::new(&text);
        let _ = self.parser.set_included_ranges(&[]);
        let old_tree = if self.large {
            self.tree.take();
            None
        } else {
            self.tree.take()
        };
        let tree = match old_tree {
            Some(mut tree) => match input_edit(&self.text, &text, &self.index, &index) {
                Some(edit) => {
                    tree.edit(&edit);
                    self.parser.parse(&text, Some(&tree))
                }
                None => Some(tree),
            },
            None => self.parser.parse(&text, None),
        };
        self.text = text;
        self.index = index;
        self.revision = request.key.revision;
        self.tree = tree;
        self.large = false;
        self.window_rows = 0..self.index.line_count();
    }

    fn parse_window(&mut self, request: &SyntaxRequest) {
        let window = window_bounds(&self.index, request.key.top, request.key.end);
        let start = window.start;
        let end = window.end;
        let start_byte = self.index.line_start(start);
        let end_byte = if end < self.index.line_count() {
            self.index.line_start(end)
        } else {
            self.text.len()
        };
        let range = TreeRange {
            start_byte,
            end_byte,
            start_point: self.index.point(start_byte),
            end_point: self.index.point(end_byte),
        };
        let _ = self.parser.set_included_ranges(&[range]);
        self.tree = self.parser.parse(&self.text, None);
        self.large = true;
        self.window_rows = window;
        self.revision = request.key.revision;
    }

    fn window_covers(&self, request: &SyntaxRequest) -> bool {
        self.window_rows.start <= request.key.top && request.key.end <= self.window_rows.end
    }

    fn spans(&self, runtime: &Runtime, window: Range<usize>) -> Vec<SyntaxSpan> {
        let Some(tree) = self.tree.as_ref() else {
            return Vec::new();
        };
        let rows = rows::visible_rows(&self.text, &self.index, window.start, window.end);
        if rows.is_empty() {
            return Vec::new();
        }
        let Some(captures) = self.captures(tree, runtime, &rows) else {
            return Vec::new();
        };
        paint_captures(&self.text, &rows, &captures)
    }

    fn captures(&self, tree: &Tree, runtime: &Runtime, rows: &[RowWindow]) -> Option<Vec<Capture>> {
        let mut captures = Vec::new();
        for block in rows::query_blocks(rows) {
            let mut cursor = QueryCursor::new();
            cursor.set_byte_range(block);
            let mut matches = cursor.captures(
                &runtime.config.query,
                tree.root_node(),
                self.text.as_bytes(),
            );
            while let Some((matched, capture_index)) = matches.next() {
                let Some(capture) = matched.captures().get(*capture_index) else {
                    continue;
                };
                let Some(group) = runtime
                    .groups
                    .get(capture.index as usize)
                    .copied()
                    .flatten()
                    .map(|group| refine_group(group, capture.node.kind()))
                else {
                    continue;
                };
                if captures.len() >= MAX_CAPTURES {
                    return None;
                }
                captures.push(Capture {
                    start: capture.node.start_byte(),
                    end: capture.node.end_byte(),
                    group,
                });
            }
        }
        captures.sort_by_key(|capture| capture.start);
        Some(captures)
    }
}

fn window_bounds(index: &LineIndex, top: usize, end: usize) -> Range<usize> {
    let start = top.saturating_sub(WINDOW_CONTEXT_LINES);
    let end = end
        .saturating_add(WINDOW_CONTEXT_LINES)
        .min(index.line_count());
    start..end
}

fn refine_group(group: SyntaxGroup, kind: &str) -> SyntaxGroup {
    let numeric = kind.contains("number")
        || kind.contains("integer")
        || kind.contains("float")
        || kind.contains("numeric");
    if group == SyntaxGroup::Constant && numeric {
        return SyntaxGroup::Number;
    }
    group
}

struct Capture {
    start: usize,
    end: usize,
    group: SyntaxGroup,
}

fn paint_captures(text: &str, rows: &[RowWindow], captures: &[Capture]) -> Vec<SyntaxSpan> {
    let mut cells: Vec<Vec<Option<SyntaxGroup>>> = rows
        .iter()
        .map(|row| vec![None; rows::row_char_len(text, row)])
        .collect();
    for capture in captures {
        for (position, row) in rows.iter().enumerate() {
            if row.end <= capture.start {
                continue;
            }
            if row.start >= capture.end {
                break;
            }
            let start = capture.start.max(row.start);
            let end = capture.end.min(row.text_end);
            if start >= end {
                continue;
            }
            let from = rows::char_column(text, row.start, start);
            let to = rows::char_column(text, row.start, end);
            let line = &mut cells[position];
            let from = from.min(line.len());
            let to = to.min(line.len());
            for cell in &mut line[from..to] {
                *cell = Some(capture.group);
            }
        }
    }
    merge_cells(rows, cells)
}

fn merge_cells(rows: &[RowWindow], cells: Vec<Vec<Option<SyntaxGroup>>>) -> Vec<SyntaxSpan> {
    let mut spans = Vec::new();
    for (row, cells) in rows.iter().zip(cells) {
        let mut start = 0usize;
        let mut current = None;
        for (column, cell) in cells.iter().enumerate() {
            if *cell == current {
                continue;
            }
            if let Some(group) = current {
                spans.push(SyntaxSpan {
                    row: row.row,
                    start,
                    end: column,
                    group,
                });
            }
            current = *cell;
            start = column;
        }
        if let Some(group) = current {
            spans.push(SyntaxSpan {
                row: row.row,
                start,
                end: cells.len(),
                group,
            });
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use ropey::Rope;

    use super::*;
    use crate::limits::{MAX_LINE_BYTES, WINDOW_PARSE_BYTES};
    use crate::span::SyntaxKey;

    fn request(text: &str, top: usize, end: usize, revision: u64) -> SyntaxRequest {
        SyntaxRequest {
            key: SyntaxKey {
                document: 1,
                revision,
                top,
                end,
                language: LanguageId::Rust,
            },
            text: Rope::from_str(text),
        }
    }

    fn highlight(language: LanguageId, text: &str, top: usize, end: usize) -> Vec<SyntaxSpan> {
        let runtime = Runtime::new(language).expect("runtime");
        let mut document = Document::new(language);
        document.update(&request(text, top, end, 1), &runtime).1
    }

    fn group_at(spans: &[SyntaxSpan], row: usize, column: usize) -> Option<SyntaxGroup> {
        spans
            .iter()
            .find(|span| span.row == row && span.start <= column && column < span.end)
            .map(|span| span.group)
    }

    #[test]
    fn highlights_rust_tokens() {
        let text = "// hello\nfn main() {\n    let value = 42;\n    let text = \"hi\";\n}\n";
        let spans = highlight(LanguageId::Rust, text, 0, 5);
        assert_eq!(group_at(&spans, 0, 0), Some(SyntaxGroup::Comment));
        assert_eq!(group_at(&spans, 1, 0), Some(SyntaxGroup::Keyword));
        assert_eq!(group_at(&spans, 1, 3), Some(SyntaxGroup::Function));
        assert_eq!(group_at(&spans, 2, 4), Some(SyntaxGroup::Keyword));
        assert_eq!(group_at(&spans, 2, 16), Some(SyntaxGroup::Number));
        assert_eq!(group_at(&spans, 3, 15), Some(SyntaxGroup::String));
    }

    #[test]
    fn highlights_toml_tokens() {
        let text = "# comment\nname = \"kawi\"\nversion = 1\nenabled = true\n";
        let spans = highlight(LanguageId::Toml, text, 0, 4);
        assert_eq!(group_at(&spans, 0, 0), Some(SyntaxGroup::Comment));
        assert_eq!(group_at(&spans, 1, 7), Some(SyntaxGroup::String));
        assert_eq!(group_at(&spans, 2, 10), Some(SyntaxGroup::Number));
        assert_eq!(group_at(&spans, 3, 10), Some(SyntaxGroup::Constant));
    }

    #[test]
    fn highlights_typescript_tokens() {
        let text =
            "const x: string = \"hi\";\nfunction greet(name: string) { return name; }\n// tail\n";
        let spans = highlight(LanguageId::TypeScript, text, 0, 3);
        assert_eq!(group_at(&spans, 0, 0), Some(SyntaxGroup::Keyword));
        assert_eq!(group_at(&spans, 0, 18), Some(SyntaxGroup::String));
        assert_eq!(group_at(&spans, 1, 0), Some(SyntaxGroup::Keyword));
        assert_eq!(group_at(&spans, 1, 9), Some(SyntaxGroup::Function));
        assert_eq!(group_at(&spans, 2, 0), Some(SyntaxGroup::Comment));
    }

    #[test]
    fn highlights_python_tokens() {
        let text = "def greet(name):\n    # comment\n    return \"hello\"\n";
        let spans = highlight(LanguageId::Python, text, 0, 3);
        assert_eq!(group_at(&spans, 0, 0), Some(SyntaxGroup::Keyword));
        assert_eq!(group_at(&spans, 0, 4), Some(SyntaxGroup::Function));
        assert_eq!(group_at(&spans, 1, 4), Some(SyntaxGroup::Comment));
        assert_eq!(group_at(&spans, 2, 11), Some(SyntaxGroup::String));
    }

    #[test]
    fn highlights_json_tokens() {
        let text = "{\"name\": \"kawi\", \"count\": 2}\n";
        let spans = highlight(LanguageId::Json, text, 0, 1);
        assert_eq!(group_at(&spans, 0, 1), Some(SyntaxGroup::String));
        assert_eq!(group_at(&spans, 0, 9), Some(SyntaxGroup::String));
        assert_eq!(group_at(&spans, 0, 26), Some(SyntaxGroup::Number));
    }

    #[test]
    fn highlights_bash_tokens() {
        let text = "#!/bin/bash\n# comment\necho \"hi\"\n";
        let spans = highlight(LanguageId::Bash, text, 0, 3);
        assert_eq!(group_at(&spans, 1, 0), Some(SyntaxGroup::Comment));
        assert_eq!(group_at(&spans, 2, 5), Some(SyntaxGroup::String));
    }

    #[test]
    fn builds_a_runtime_for_every_language() {
        for language in [
            LanguageId::Rust,
            LanguageId::Toml,
            LanguageId::TypeScript,
            LanguageId::Tsx,
            LanguageId::JavaScript,
            LanguageId::Jsx,
            LanguageId::Python,
            LanguageId::Json,
            LanguageId::Bash,
        ] {
            assert!(Runtime::new(language).is_some(), "{language:?}");
        }
    }

    #[test]
    fn highlights_javascript_tokens() {
        let text = "const value = \"hi\"; // tail\n";
        let spans = highlight(LanguageId::JavaScript, text, 0, 1);
        assert_eq!(group_at(&spans, 0, 0), Some(SyntaxGroup::Keyword));
        assert_eq!(group_at(&spans, 0, 14), Some(SyntaxGroup::String));
        assert_eq!(group_at(&spans, 0, 20), Some(SyntaxGroup::Comment));
    }

    #[test]
    fn highlights_jsx_tokens() {
        let text = "const view = <div className=\"x\">hi</div>;\n";
        let spans = highlight(LanguageId::Jsx, text, 0, 1);
        assert_eq!(group_at(&spans, 0, 0), Some(SyntaxGroup::Keyword));
        assert_eq!(group_at(&spans, 0, 18), Some(SyntaxGroup::Attribute));
        assert_eq!(group_at(&spans, 0, 30), Some(SyntaxGroup::String));
        assert_eq!(group_at(&spans, 0, 14), Some(SyntaxGroup::Tag));
    }

    #[test]
    fn highlights_tsx_tokens() {
        let text = "const view: string = <div className=\"x\">hi</div>;\n";
        let spans = highlight(LanguageId::Tsx, text, 0, 1);
        assert_eq!(group_at(&spans, 0, 0), Some(SyntaxGroup::Keyword));
        assert_eq!(group_at(&spans, 0, 27), Some(SyntaxGroup::Attribute));
        assert_eq!(group_at(&spans, 0, 36), Some(SyntaxGroup::String));
        assert_eq!(group_at(&spans, 0, 22), Some(SyntaxGroup::Tag));
    }

    #[test]
    fn multiline_comments_reach_the_visible_window() {
        let text = "fn main() {\n    /* first\n    second\n    third */\n    let x = 1;\n}\n";
        let spans = highlight(LanguageId::Rust, text, 2, 3);
        assert_eq!(group_at(&spans, 2, 4), Some(SyntaxGroup::Comment));
        assert_eq!(group_at(&spans, 1, 4), Some(SyntaxGroup::Comment));
    }

    #[test]
    fn incremental_parse_updates_later_rows() {
        let runtime = Runtime::new(LanguageId::Rust).expect("runtime");
        let mut document = Document::new(LanguageId::Rust);
        let first = "fn main() {\n    let x = 1;\n}\n";
        let spans = document.update(&request(first, 0, 3, 1), &runtime).1;
        assert_eq!(group_at(&spans, 1, 4), Some(SyntaxGroup::Keyword));
        let second = "/*\nfn main() {\n    let x = 1;\n}\n";
        let spans = document.update(&request(second, 0, 4, 2), &runtime).1;
        assert_eq!(group_at(&spans, 1, 0), Some(SyntaxGroup::Comment));
        assert_eq!(group_at(&spans, 2, 4), Some(SyntaxGroup::Comment));
    }

    #[test]
    fn unicode_columns_use_characters() {
        let text = "let s = \"é\"; // tail\n";
        let spans = highlight(LanguageId::Rust, text, 0, 1);
        let comment = text.find("//").expect("comment");
        let column = text[..comment].chars().count();
        assert_eq!(group_at(&spans, 0, column), Some(SyntaxGroup::Comment));
    }

    #[test]
    fn large_files_use_bounded_windows() {
        let text = "# marker\n".repeat(60_000);
        assert!(text.len() > WINDOW_PARSE_BYTES);
        let runtime = Runtime::new(LanguageId::Python).expect("runtime");
        let mut document = Document::new(LanguageId::Python);
        let key = SyntaxKey {
            document: 1,
            revision: 1,
            top: 30_000,
            end: 30_020,
            language: LanguageId::Python,
        };
        let first = SyntaxRequest {
            key,
            text: Rope::from_str(&text),
        };
        let (window, spans) = document.update(&first, &runtime);
        assert!(document.large);
        assert_eq!(window, 29_000..31_020);
        assert_eq!(document.window_rows, 29_000..31_020);
        assert_eq!(group_at(&spans, 30_000, 0), Some(SyntaxGroup::Comment));

        let second = SyntaxRequest {
            key: SyntaxKey {
                top: 40_000,
                end: 40_020,
                ..key
            },
            text: Rope::from_str(&text),
        };
        let (window, spans) = document.update(&second, &runtime);
        assert_eq!(window, 39_000..41_020);
        assert_eq!(document.window_rows, 39_000..41_020);
        assert_eq!(group_at(&spans, 40_000, 0), Some(SyntaxGroup::Comment));
    }

    #[test]
    fn large_file_edits_refresh_the_window() {
        let text = "# marker\n".repeat(60_000);
        let runtime = Runtime::new(LanguageId::Python).expect("runtime");
        let mut document = Document::new(LanguageId::Python);
        let key = SyntaxKey {
            document: 1,
            revision: 1,
            top: 0,
            end: 20,
            language: LanguageId::Python,
        };
        document.update(
            &SyntaxRequest {
                key,
                text: Rope::from_str(&text),
            },
            &runtime,
        );
        let edited = text.replacen("# marker", "# changed", 1);
        let (_, spans) = document.update(
            &SyntaxRequest {
                key: SyntaxKey { revision: 2, ..key },
                text: Rope::from_str(&edited),
            },
            &runtime,
        );
        assert!(document.large);
        assert_eq!(document.window_rows, 0..1_020);
        assert_eq!(group_at(&spans, 0, 0), Some(SyntaxGroup::Comment));
    }

    #[test]
    fn shrinking_a_large_document_restores_a_full_tree() {
        let text = "# marker\n".repeat(60_000);
        let runtime = Runtime::new(LanguageId::Python).expect("runtime");
        let mut document = Document::new(LanguageId::Python);
        let key = SyntaxKey {
            document: 1,
            revision: 1,
            top: 30_000,
            end: 30_020,
            language: LanguageId::Python,
        };
        document.update(
            &SyntaxRequest {
                key,
                text: Rope::from_str(&text),
            },
            &runtime,
        );
        let small = "def main():\n    return True\n";
        let (_, spans) = document.update(
            &SyntaxRequest {
                key: SyntaxKey {
                    revision: 2,
                    top: 0,
                    end: 2,
                    ..key
                },
                text: Rope::from_str(small),
            },
            &runtime,
        );
        assert!(!document.large);
        assert_eq!(document.window_rows, 0..document.index.line_count());
        assert_eq!(group_at(&spans, 0, 0), Some(SyntaxGroup::Keyword));
    }

    #[test]
    fn long_lines_are_left_plain() {
        let long = format!("{} // tail\n", "a".repeat(MAX_LINE_BYTES + 1));
        let text = format!("{long}fn main() {{}}\n");
        let spans = highlight(LanguageId::Rust, &text, 0, 2);
        assert_eq!(group_at(&spans, 0, 0), None);
        assert_eq!(group_at(&spans, 1, 0), Some(SyntaxGroup::Keyword));
    }
}

#[cfg(test)]
mod parity_tests {
    use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

    use super::*;
    use crate::rows::{char_column, row_char_len, visible_rows, RowWindow};
    use crate::span::SyntaxKey;

    #[test]
    fn rust_range_resolution_matches_the_reference_highlighter() {
        assert_reference(
            LanguageId::Rust,
            "// heading\n#[derive(Debug)]\nfn main() {\n    let raw = r#\"a\nb\"#;\n    let value = true;\n    /* block\n       comment */\n    println!(\"{value}\");\n}\n",
        );
    }

    #[test]
    fn toml_range_resolution_matches_the_reference_highlighter() {
        assert_reference(
            LanguageId::Toml,
            "# heading\nname = \"kawi\"\nenabled = true\n[section]\nkey = 'value'\n",
        );
    }

    #[test]
    fn typescript_range_resolution_matches_the_reference_highlighter() {
        assert_reference(
            LanguageId::TypeScript,
            "const value: string = \"hi\";\nfunction greet(name: string) {\n    return name;\n}\n// tail\n",
        );
    }

    #[test]
    fn python_range_resolution_matches_the_reference_highlighter() {
        assert_reference(
            LanguageId::Python,
            "def greet(name):\n    # comment\n    return \"hello\"\n",
        );
    }

    fn assert_reference(language: LanguageId, text: &str) {
        let (runtime, recognized) = configured_runtime(language);
        let row_end = text.lines().count();
        let request = SyntaxRequest {
            key: SyntaxKey {
                document: 1,
                revision: 1,
                top: 0,
                end: row_end,
                language,
            },
            text: ropey::Rope::from_str(text),
        };
        let mut document = Document::new(language);
        let spans = document.update(&request, &runtime).1;
        let rows = visible_rows(text, &LineIndex::new(text), 0, row_end);
        let ours = our_cells(text, &rows, &spans);
        let reference = reference_cells(text, &rows, &runtime.config, &recognized);
        assert_eq!(ours, reference);
    }

    fn configured_runtime(language: LanguageId) -> (Runtime, Vec<(String, SyntaxGroup)>) {
        let mut config = HighlightConfiguration::new(
            language.language(),
            language.name(),
            &language.highlights(),
            "",
            "",
        )
        .expect("config");
        let groups: Vec<Option<SyntaxGroup>> = config
            .names()
            .iter()
            .map(|name| SyntaxGroup::from_capture(name))
            .collect();
        let recognized: Vec<(String, SyntaxGroup)> = config
            .names()
            .iter()
            .zip(groups.iter())
            .filter_map(|(name, group)| group.map(|group| ((*name).to_string(), group)))
            .collect();
        let names: Vec<&str> = recognized.iter().map(|(name, _)| name.as_str()).collect();
        config.configure(&names);
        (Runtime { config, groups }, recognized)
    }

    fn our_cells(
        text: &str,
        rows: &[RowWindow],
        spans: &[SyntaxSpan],
    ) -> Vec<Vec<Option<SyntaxGroup>>> {
        let mut cells: Vec<Vec<Option<SyntaxGroup>>> = rows
            .iter()
            .map(|row| vec![None; row_char_len(text, row)])
            .collect();
        for span in spans {
            let Some(position) = rows.iter().position(|row| row.row == span.row) else {
                continue;
            };
            let line = &mut cells[position];
            let from = span.start.min(line.len());
            let to = span.end.min(line.len());
            for cell in &mut line[from..to] {
                *cell = Some(span.group);
            }
        }
        cells
    }

    fn reference_cells(
        text: &str,
        rows: &[RowWindow],
        config: &HighlightConfiguration,
        recognized: &[(String, SyntaxGroup)],
    ) -> Vec<Vec<Option<SyntaxGroup>>> {
        let mut cells: Vec<Vec<Option<SyntaxGroup>>> = rows
            .iter()
            .map(|row| vec![None; row_char_len(text, row)])
            .collect();
        let mut highlighter = Highlighter::new();
        let events = highlighter
            .highlight(config, text.as_bytes(), None, None, |_| None)
            .expect("highlight");
        let mut stack = Vec::new();
        for event in events {
            match event.expect("event") {
                HighlightEvent::Source { start, end } => {
                    if let Some(group) = stack.last() {
                        paint_source(text, rows, &mut cells, start, end, *group);
                    }
                }
                HighlightEvent::HighlightStart(index) => {
                    stack.push(recognized[index.0].1);
                }
                HighlightEvent::HighlightEnd => {
                    stack.pop();
                }
            }
        }
        cells
    }

    fn paint_source(
        text: &str,
        rows: &[RowWindow],
        cells: &mut [Vec<Option<SyntaxGroup>>],
        start: usize,
        end: usize,
        group: SyntaxGroup,
    ) {
        for (position, row) in rows.iter().enumerate() {
            let segment_start = start.max(row.start);
            let segment_end = end.min(row.text_end);
            if segment_start >= segment_end {
                continue;
            }
            let from = char_column(text, row.start, segment_start);
            let to = char_column(text, row.start, segment_end);
            let line = &mut cells[position];
            let from = from.min(line.len());
            let to = to.min(line.len());
            for cell in &mut line[from..to] {
                *cell = Some(group);
            }
        }
    }
}
