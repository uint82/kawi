use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_markdown_atx_and_setext_headings() {
    let text =
        "# One\n\n### Three ###\n\n#### Four\n\n##### Five\n\n###### Six\n\nSix\n===\n\nTwo\n---\n";
    let spans = highlight(LanguageId::Markdown, text, 0, 15);
    assert_eq!(name_of(&spans, text, 0, "One"), Some("markup.heading.1"));
    assert_eq!(name_at(&spans, 0, 0), Some("markup.heading.1"));
    assert_eq!(name_of(&spans, text, 2, "Three"), Some("markup.heading.3"));
    assert_eq!(name_of(&spans, text, 4, "Four"), Some("markup.heading.4"));
    assert_eq!(name_of(&spans, text, 6, "Five"), Some("markup.heading.5"));
    assert_eq!(name_of(&spans, text, 8, "Six"), Some("markup.heading.6"));
    assert_eq!(name_of(&spans, text, 10, "Six"), Some("markup.heading.1"));
    assert_eq!(name_of(&spans, text, 11, "==="), Some("markup.heading.1"));
    assert_eq!(name_of(&spans, text, 13, "Two"), Some("markup.heading.2"));
    assert_eq!(name_of(&spans, text, 14, "---"), Some("markup.heading.2"));
}

#[test]
fn highlights_markdown_tables() {
    let text = "| Name | Value |\n| ---- | ----- |\n| one  | two   |\n";
    let spans = highlight(LanguageId::Markdown, text, 0, 3);
    assert_eq!(name_of(&spans, text, 0, "Name"), Some("markup.heading"));
    assert_eq!(name_of(&spans, text, 0, "Value"), Some("markup.heading"));
    assert_eq!(name_at(&spans, 0, 0), Some("punctuation.special"));
    assert_eq!(
        name_of(&spans, text, 1, "----"),
        Some("punctuation.special")
    );
    assert_eq!(name_of(&spans, text, 2, "one"), None);
}

#[test]
fn highlights_markdown_code_blocks_and_info_strings() {
    let text = "    indented\n\n```rust\nfn main() {}\n```\n";
    let spans = highlight(LanguageId::Markdown, text, 0, 5);
    assert_eq!(
        name_of(&spans, text, 0, "indented"),
        Some("markup.raw.block")
    );
    assert_eq!(
        name_of(&spans, text, 2, "```"),
        Some("punctuation.delimiter")
    );
    assert_eq!(name_of(&spans, text, 2, "rust"), Some("label"));
    assert_eq!(name_of(&spans, text, 3, "fn"), Some("keyword.function"));
}

#[test]
fn highlights_markdown_link_definitions() {
    let text = "[guide]: https://example.com \"Guide\"\n\n[other]: /docs\n";
    let spans = highlight(LanguageId::Markdown, text, 0, 3);
    assert_eq!(name_of(&spans, text, 0, "guide"), Some("markup.link.label"));
    assert_eq!(name_of(&spans, text, 0, "https"), Some("markup.link.url"));
    assert_eq!(
        name_of(&spans, text, 0, "\"Guide\""),
        Some("markup.link.label")
    );
    let colon = text.lines().next().expect("row").find(':').expect(":");
    assert_eq!(name_at(&spans, 0, colon), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 2, "other"), Some("markup.link.label"));
    assert_eq!(name_of(&spans, text, 2, "/docs"), Some("markup.link.url"));
}

#[test]
fn highlights_markdown_lists_quotes_and_tasks() {
    let text = "- one\n+ two\n* three\n1. four\n2) five\n- [x] done\n- [ ] todo\n\n> quote\n> more\n\n---\n";
    let spans = highlight(LanguageId::Markdown, text, 0, 12);
    for (row, marker) in [(0, "-"), (1, "+"), (2, "*"), (3, "1."), (4, "2)")] {
        assert_eq!(
            name_of(&spans, text, row, marker),
            Some("markup.list"),
            "{marker}"
        );
    }
    assert_eq!(name_of(&spans, text, 5, "[x]"), Some("markup.list.checked"));
    assert_eq!(
        name_of(&spans, text, 6, "[ ]"),
        Some("markup.list.unchecked")
    );
    assert_eq!(name_of(&spans, text, 8, "quote"), Some("markup.quote"));
    assert_eq!(name_of(&spans, text, 8, ">"), Some("punctuation.special"));
    assert_eq!(
        name_of(&spans, text, 11, "---"),
        Some("punctuation.special")
    );
}

#[test]
fn highlights_markdown_metadata() {
    let text = "+++\nkey = \"value\"\n+++\n\n# Head\n";
    let spans = highlight(LanguageId::Markdown, text, 0, 5);
    assert_eq!(name_of(&spans, text, 0, "+++"), Some("keyword.directive"));
    assert_eq!(name_of(&spans, text, 1, "key"), Some("property"));
    assert_eq!(name_of(&spans, text, 4, "Head"), Some("markup.heading.1"));
}

#[test]
fn highlights_inline_markup() {
    let text = "Some *italic* and **bold** and `code` with [label](https://example.com).\n";
    let spans = highlight(LanguageId::Markdown, text, 0, 1);
    assert_eq!(name_of(&spans, text, 0, "italic"), Some("markup.italic"));
    assert_eq!(name_of(&spans, text, 0, "bold"), Some("markup.strong"));
    assert_eq!(name_of(&spans, text, 0, "code"), Some("markup.raw"));
    assert_eq!(name_of(&spans, text, 0, "label"), Some("markup.link.label"));
    assert_eq!(name_of(&spans, text, 0, "https"), Some("markup.link.url"));
}

#[test]
fn highlights_inline_html() {
    let text = "Text <span>inside</span> tail\n";
    let spans = highlight(LanguageId::Markdown, text, 0, 1);
    assert_eq!(name_of(&spans, text, 0, "span"), Some("tag"));
    assert_eq!(name_of(&spans, text, 0, "inside"), None);
}
