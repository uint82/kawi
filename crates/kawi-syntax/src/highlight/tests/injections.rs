use super::support::{highlight, name_of};
use super::*;

#[test]
fn highlights_html_script_blocks() {
    let text = "<script>const value = 1;</script>\n";
    let spans = highlight(LanguageId::Html, text, 0, 1);
    assert_eq!(name_of(&spans, text, 0, "const"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "value"), Some("variable"));
    assert_eq!(name_of(&spans, text, 0, "1"), Some("number"));
}

#[test]
fn highlights_html_style_blocks() {
    let text = "<style>.box { color: red; }</style>\n";
    let spans = highlight(LanguageId::Html, text, 0, 1);
    assert_eq!(name_of(&spans, text, 0, "box"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "color"), Some("property"));
}

#[test]
fn highlights_markdown_fenced_code_blocks() {
    let text = "# Title\n\n```javascript\nconst value = 1;\n```\n";
    let spans = highlight(LanguageId::Markdown, text, 0, 7);
    assert_eq!(name_of(&spans, text, 3, "const"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 3, "value"), Some("variable"));
    assert_eq!(name_of(&spans, text, 3, "1"), Some("number"));
}

#[test]
fn highlights_nested_markdown_injections() {
    let text = "```html\n<script>const value = 1;</script>\n```\n";
    let spans = highlight(LanguageId::Markdown, text, 0, 4);
    assert_eq!(name_of(&spans, text, 1, "const"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 1, "value"), Some("variable"));
}

#[test]
fn large_documents_highlight_visible_injections() {
    let filler = "word\n".repeat(120_000);
    let text = format!("{filler}```rust\nfn main() {{}}\n```\n");
    let first = 120_000;
    let spans = highlight(LanguageId::Markdown, &text, first, first + 4);
    assert_eq!(
        name_of(&spans, &text, first + 1, "fn"),
        Some("keyword.function")
    );
}
