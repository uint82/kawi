use super::*;
use kawi_completion::MarkupKind;

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

fn code_block(document: &MarkdownDocument) -> &MarkdownCodeBlock {
    document
        .blocks
        .iter()
        .find_map(|block| match block {
            MarkdownBlock::Code(code) => Some(code),
            _ => None,
        })
        .expect("code block")
}

#[test]
fn keeps_tagged_fence_language_and_content() {
    let document = parse(&markdown("```rust\nfn main() {}\n```"));
    let code = code_block(&document);
    assert_eq!(code.language.as_deref(), Some("rust"));
    assert_eq!(code.text(), "fn main() {}");
}

#[test]
fn untagged_fences_have_no_language() {
    let document = parse(&markdown("```\nplain\n```"));
    let code = code_block(&document);
    assert_eq!(code.language, None);
    assert_eq!(code.text(), "plain");
}

#[test]
fn keeps_unknown_fence_languages_for_later_fallback() {
    let document = parse(&markdown("```brainfuck\n++\n```"));
    assert_eq!(code_block(&document).language.as_deref(), Some("brainfuck"));
}

#[test]
fn takes_the_first_info_token() {
    let document = parse(&markdown(
        "```rust,ignore\nfn main() {}\n```\n\n```ts title=\"x\"\nlet x = 1;\n```",
    ));
    let languages: Vec<_> = document
        .blocks
        .iter()
        .filter_map(|block| match block {
            MarkdownBlock::Code(code) => code.language.as_deref(),
            _ => None,
        })
        .collect();
    assert_eq!(languages, ["rust", "ts"]);
}

#[test]
fn unclosed_fences_keep_their_content() {
    let document = parse(&markdown("```rust\nfn main() {}\n"));
    assert_eq!(code_block(&document).text(), "fn main() {}");
}

#[test]
fn supports_tilde_fences_and_longer_closers() {
    let document = parse(&markdown("~~~python\nprint('hi')\n~~~~~\nafter\n"));
    assert_eq!(code_block(&document).language.as_deref(), Some("python"));
    assert_eq!(code_block(&document).text(), "print('hi')");
    let rendered = render(&markdown("~~~python\nprint('hi')\n~~~~~\nafter\n"), 80);
    assert_eq!(
        rendered.iter().map(MarkdownLine::text).collect::<Vec<_>>(),
        ["print('hi')", "after"]
    );
}
