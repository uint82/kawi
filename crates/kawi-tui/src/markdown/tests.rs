use super::*;

fn markdown(text: &str) -> Documentation {
    Documentation {
        text: text.to_string(),
        kind: MarkupKind::Markdown,
    }
}

#[test]
fn strips_inline_markers_and_keeps_styles() {
    let lines = render(&markdown("**bold** `code` [link](https://example.com)"), 80);
    assert_eq!(lines[0].text(), "bold code link");
    assert!(lines[0]
        .spans
        .iter()
        .any(|span| span.style == MarkdownStyle::Strong));
    assert!(lines[0]
        .spans
        .iter()
        .any(|span| span.style == MarkdownStyle::Code));
}

#[test]
fn renders_fenced_code_without_markers() {
    let lines = render(&markdown("```rust\nfn main()\n```"), 80);
    assert_eq!(lines[0].text(), "fn main()");
    assert_eq!(lines[0].spans[0].style, MarkdownStyle::Code);
}

#[test]
fn wraps_at_the_requested_width() {
    let lines = render(&markdown("abcdef"), 3);
    assert_eq!(
        lines.iter().map(MarkdownLine::text).collect::<Vec<_>>(),
        ["abc", "def"]
    );
}

#[test]
fn plaintext_is_not_interpreted() {
    let lines = render(
        &Documentation {
            text: "**literal**".to_string(),
            kind: MarkupKind::PlainText,
        },
        80,
    );
    assert_eq!(lines[0].text(), "**literal**");
}

#[test]
fn styled_spans_are_not_parsed_as_markdown() {
    let lines = render_spans(
        &[MarkdownSpan {
            text: "fn value(x: *const u8)".to_string(),
            style: MarkdownStyle::Normal,
        }],
        80,
    );
    assert_eq!(lines[0].text(), "fn value(x: *const u8)");
    assert_eq!(lines[0].spans[0].style, MarkdownStyle::Normal);
}

#[test]
fn styled_spans_wrap_and_keep_styles() {
    let lines = render_spans(
        &[
            MarkdownSpan {
                text: "ab".to_string(),
                style: MarkdownStyle::Normal,
            },
            MarkdownSpan {
                text: "cd".to_string(),
                style: MarkdownStyle::Strong,
            },
        ],
        2,
    );
    assert_eq!(
        lines.iter().map(MarkdownLine::text).collect::<Vec<_>>(),
        ["ab", "cd"]
    );
    assert_eq!(lines[1].spans[0].style, MarkdownStyle::Strong);
}

#[test]
fn control_characters_are_replaced() {
    let lines = render(&markdown("safe\u{1b}[31m"), 80);
    assert_eq!(lines[0].text(), "safe [31m");
}
