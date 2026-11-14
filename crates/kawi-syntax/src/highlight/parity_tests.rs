use std::collections::HashMap;
use std::sync::OnceLock;

use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use super::*;
use crate::registry::{registry, SyntaxHighlight};
use crate::rows::{visible_rows, RowWindow};
use crate::span::SyntaxKey;
use crate::text::{char_column, row_char_len};

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

#[test]
fn javascript_locals_match_the_reference_highlighter() {
    assert_reference(
        LanguageId::JavaScript,
        "function greet(name) {\n    const value = name;\n    return value;\n}\n",
    );
}

#[test]
fn ruby_locals_match_the_reference_highlighter() {
    assert_reference(
        LanguageId::Ruby,
        "def greet(name)\n  value = name\n  value\nend\n",
    );
}

#[test]
fn lua_locals_match_the_reference_highlighter() {
    assert_reference(
        LanguageId::Lua,
        "local function greet(name)\n  local value = name\n  return value\nend\n",
    );
}

#[test]
fn ada_locals_match_the_reference_highlighter() {
    assert_reference(
        LanguageId::Ada,
        "procedure Greet is\n   Value : Integer := 1;\nbegin\n   null;\nend Greet;\n",
    );
}

#[test]
fn locals_respect_scope_and_declaration_order() {
    let cases = [
        (
            LanguageId::JavaScript,
            "const value = 1;\n{ const value = 2; console.log(value); }\nconsole.log(value);\n",
        ),
        (
            LanguageId::Lua,
            "local value = 1\ndo\n  local value = 2\n  print(value)\nend\nprint(value)\n",
        ),
        (LanguageId::Ruby, "value = 1\ndef show\n  value\nend\n"),
        (
            LanguageId::TypeScript,
            "const value = 1;\nfunction show(value: number) { return value; }\n",
        ),
    ];
    for (language, text) in cases {
        assert_reference(language, text);
    }
}

#[test]
fn remaining_languages_match_the_reference_highlighter() {
    let cases = [
        (
            LanguageId::Tsx,
            "const view = <div className=\"box\">{value}</div>;\n",
        ),
        (LanguageId::Jsx, "const view = <div className=\"box\">{value}</div>;\n"),
        (
            LanguageId::Json,
            "{\"name\": \"kawi\", \"enabled\": true, \"items\": [1, null]}\n",
        ),
        (
            LanguageId::Bash,
            "#!/bin/bash\nname=\"kawi\"\nfor item in one two; do\n  echo \"$item\"\ndone\n",
        ),
        (
            LanguageId::C,
            "#include <stdio.h>\nint main(void) { // comment\n  return 0;\n}\n",
        ),
        (
            LanguageId::Cpp,
            "#include <string>\nclass Box { public: std::string name; };\nint main() { Box box; return 0; }\n",
        ),
        (
            LanguageId::Go,
            "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tvalue := 1\n\tfmt.Println(value)\n}\n",
        ),
        (
            LanguageId::Java,
            "class Main {\n  public static void main(String[] args) {\n    int value = 1;\n    System.out.println(value);\n  }\n}\n",
        ),
        (
            LanguageId::CSharp,
            "class Program {\n  static void Main(string[] args) {\n    int value = 1;\n    Console.WriteLine(value);\n  }\n}\n",
        ),
        (
            LanguageId::Php,
            "<?php\nfunction greet(string $name): void {\n    echo $name;\n}\n",
        ),
        (
            LanguageId::Yaml,
            "name: kawi\nenabled: true\nitems:\n  - one\n  - two\n",
        ),
        (LanguageId::Html, "<div class=\"box\">text</div>\n"),
        (LanguageId::Css, ".box { color: red; display: flex; }\n"),
        (
            LanguageId::MarkdownInline,
            "Some *italic* and **bold** text.\n",
        ),
        (
            LanguageId::Markdown,
            "# Title\n\nSome *italic* and `code`.\n\n<span>inside</span>\n\n```javascript\nconst value = 1;\n```\n",
        ),
        (
            LanguageId::Php,
            "<div <?php echo $attrs; ?>class=\"box\">body</div>\n",
        ),
        (
            LanguageId::Html,
            "<script>const value = 1;</script>\n",
        ),
    ];
    for (language, text) in cases {
        assert_reference(language, text);
    }
}

fn assert_reference(language: LanguageId, text: &str) {
    let runtime = configured_runtime(language);
    let row_end = text.lines().count();
    let request = SyntaxRequest {
        base_revision: 0,
        edits: Vec::new(),
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
    let text = ropey::Rope::from_str(text);
    let rows = visible_rows(&text, 0, row_end);
    let ours = our_cells(&text, &rows, &spans);
    let configs = reference_configs();
    let reference = reference_cells(&text, &rows, &configs[&language], configs);
    assert_eq!(ours, reference);
}

fn reference_configs() -> &'static HashMap<LanguageId, HighlightConfiguration> {
    static CONFIGS: OnceLock<HashMap<LanguageId, HighlightConfiguration>> = OnceLock::new();
    CONFIGS.get_or_init(|| {
        LanguageId::ALL
            .into_iter()
            .map(|language| (language, reference_config(language)))
            .collect()
    })
}

fn reference_config(language: LanguageId) -> HighlightConfiguration {
    let mut config = HighlightConfiguration::new(
        language.language(),
        language.name(),
        language.highlights(),
        &language.injections(),
        language.locals(),
    )
    .expect("reference configuration");
    config.configure(registry().names());
    config
}

fn configured_runtime(language: LanguageId) -> Runtime {
    Runtime::new(language).expect("runtime")
}

fn our_cells(
    text: &ropey::Rope,
    rows: &[RowWindow],
    spans: &[SyntaxSpan],
) -> Vec<Vec<Option<SyntaxHighlight>>> {
    let mut cells: Vec<Vec<Option<SyntaxHighlight>>> = rows
        .iter()
        .map(|row| vec![None; row_char_len(text, row.row)])
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
    text: &ropey::Rope,
    rows: &[RowWindow],
    config: &HighlightConfiguration,
    configs: &HashMap<LanguageId, HighlightConfiguration>,
) -> Vec<Vec<Option<SyntaxHighlight>>> {
    let mut cells: Vec<Vec<Option<SyntaxHighlight>>> = rows
        .iter()
        .map(|row| vec![None; row_char_len(text, row.row)])
        .collect();
    let mut highlighter = Highlighter::new();
    let source = text.to_string();
    let events = highlighter
        .highlight(config, source.as_bytes(), None, None, |name| {
            LanguageId::from_alias(name).and_then(|language| configs.get(&language))
        })
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
    text: &ropey::Rope,
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
