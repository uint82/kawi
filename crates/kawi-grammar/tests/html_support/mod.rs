use std::sync::OnceLock;
use std::time::Duration;

use kawi_grammar::{GrammarLoader, LoadedGrammar, Pack};
use tree_sitter::Language;

use super::support::fixture_html_pack;

pub const PARSE_BUDGET: Duration = Duration::from_secs(10);

pub const HTML_SOURCE: &str = r#"<!doctype html>
<!-- shell -->
<html lang="en">
<head>
  <title>Kawi</title>
  <style>.box { color: red; }</style>
</head>
<body>
  <h1 id="top">Welcome</h1>
  <p class="lead" data-id=page>
    Hello <strong>bold</strong>, <em>soft</em>, <code>sample</code>, <s>stale</s>, <u>under</u>.
    <a href="/home" title="Home">Home</a>
    <img src="logo.png" alt="logo" />
    &amp;
  </p>
  <script>const value = 1;</script>
</body>
</html>
"#;

pub const INJECTION_SOURCE: &str =
    "<script>const value = 1;</script>\n<style>.box { color: red; }</style>\n";

pub const ERROR_SOURCE: &str = "<div>\n";

pub const CORPUS: &[(&str, &str)] = &[
    ("features", HTML_SOURCE),
    ("injections", INJECTION_SOURCE),
    ("errors", ERROR_SOURCE),
    ("mismatched_end_tag", "<div class=\"box\"></span>\n"),
    ("empty", ""),
    ("crlf", "<div>\r\n  <span>text</span>\r\n</div>\r\n"),
];

pub const HIGHLIGHTS: &str = include_str!("../../packs/html/0.23.2/queries/highlights.scm");
pub const INJECTIONS: &str = include_str!("../../packs/html/0.23.2/queries/injections.scm");

pub const NATIVE_HIGHLIGHTS: &str = include_str!("../../../kawi-syntax/queries/html.scm");
pub const NATIVE_INJECTIONS: &str = tree_sitter_html::INJECTIONS_QUERY;

pub fn grammar() -> &'static LoadedGrammar {
    static GRAMMAR: OnceLock<LoadedGrammar> = OnceLock::new();
    GRAMMAR.get_or_init(|| load(fixture_html_pack()))
}

pub fn native_language() -> Language {
    tree_sitter_html::LANGUAGE.into()
}

pub fn large_html_source(elements: usize) -> String {
    use std::fmt::Write;

    let mut source = String::with_capacity(elements * 128);
    for index in 0..elements {
        writeln!(source, "<section id=\"section-{index}\">").expect("write source");
        writeln!(source, "  <h2>Section {index}</h2>").expect("write source");
        writeln!(
            source,
            "  <p class=\"body\">Text <strong>bold</strong> and <em>soft</em>.</p>"
        )
        .expect("write source");
        writeln!(source, "</section>").expect("write source");
    }
    source
}

fn load(path: std::path::PathBuf) -> LoadedGrammar {
    let pack = Pack::open(path).unwrap_or_else(|error| panic!("open HTML pack: {error}"));
    GrammarLoader::new()
        .unwrap_or_else(|error| panic!("create loader: {error}"))
        .load(&pack)
        .unwrap_or_else(|error| panic!("load HTML pack: {error}"))
}
