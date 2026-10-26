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
    let runtime = configured_runtime(language);
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
    let reference = reference_cells(text, &rows, &runtime.config);
    assert_eq!(ours, reference);
}

fn configured_runtime(language: LanguageId) -> Runtime {
    Runtime::new(language).expect("runtime")
}

fn our_cells(
    text: &str,
    rows: &[RowWindow],
    spans: &[SyntaxSpan],
) -> Vec<Vec<Option<SyntaxHighlight>>> {
    let mut cells: Vec<Vec<Option<SyntaxHighlight>>> = rows
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
            *cell = Some(span.highlight);
        }
    }
    cells
}

fn reference_cells(
    text: &str,
    rows: &[RowWindow],
    config: &HighlightConfiguration,
) -> Vec<Vec<Option<SyntaxHighlight>>> {
    let mut cells: Vec<Vec<Option<SyntaxHighlight>>> = rows
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
                let name = registry().names().get(index.0).expect("highlight name");
                if let Some(highlight) = registry().resolve(name) {
                    stack.push(highlight);
                }
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
    cells: &mut [Vec<Option<SyntaxHighlight>>],
    start: usize,
    end: usize,
    highlight: SyntaxHighlight,
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
            *cell = Some(highlight);
        }
    }
}
