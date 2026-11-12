use super::*;
use crate::registry::registry;

fn names(language: LanguageId, text: &str) -> Vec<&'static str> {
    highlight_snippet(language, text)
        .into_iter()
        .filter_map(|span| registry().name(span.highlight))
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
        (LanguageId::C, "int main(void) { return 0; }"),
        (LanguageId::Cpp, "int main() { return 0; }"),
        (LanguageId::Go, "package main\nfunc main() {}"),
        (
            LanguageId::Java,
            "class Main { public static void main(String[] args) {} }",
        ),
        (LanguageId::CSharp, "class Main { static void Main() {} }"),
        (LanguageId::Ruby, "def run\n  1\nend"),
        (LanguageId::Php, "<?php function run() { return 1; }"),
        (LanguageId::Lua, "local function run() return 1 end"),
        (LanguageId::Yaml, "key: value"),
        (LanguageId::Html, "<div class=\"x\"></div>"),
        (LanguageId::Css, ".x { color: red; }"),
        (LanguageId::Markdown, "# Title\n"),
    ];
    for (language, snippet) in snippets {
        assert!(
            !names(language, snippet).is_empty(),
            "{language:?} returned no groups"
        );
    }
}

#[test]
fn cpp_inherits_c_highlights() {
    assert!(names(LanguageId::Cpp, "int value = 0;").contains(&"type.builtin"));
}

#[test]
fn lua_conditionals_use_standard_captures() {
    assert!(names(LanguageId::Lua, "if true then end").contains(&"keyword.control"));
}

#[test]
fn markdown_headings_use_standard_captures() {
    assert!(names(LanguageId::Markdown, "# Title\n").contains(&"markup.heading.1"));
}

#[test]
fn highlights_a_bare_signature() {
    assert!(names(LanguageId::Rust, "fn push(&mut self, value: T)").contains(&"keyword.function"));
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
        (LanguageId::C, "return 0;"),
        (LanguageId::Cpp, "return 0;"),
        (LanguageId::Go, "func main() {}"),
        (LanguageId::Java, "return 1;"),
        (LanguageId::CSharp, "return 1;"),
        (LanguageId::Ruby, "value + 1"),
        (LanguageId::Php, "<?php return 1;"),
        (LanguageId::Lua, "return 1"),
        (LanguageId::Yaml, "key: value"),
        (LanguageId::Html, "<div class=\"x\"></div>"),
        (LanguageId::Css, ".x { color: red; }"),
        (LanguageId::Markdown, "# Title\n"),
    ];
    for (language, fragment) in fragments {
        assert!(
            !names(language, fragment).is_empty(),
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
    assert!(names(LanguageId::Rust, "fn main(").contains(&"keyword.function"));
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
