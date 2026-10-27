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
fn links_keep_their_targets() {
    let lines = render(&markdown("[Options](help:options)"), 80);
    assert_eq!(lines[0].text(), "Options");
    assert_eq!(lines[0].spans[0].link.as_deref(), Some("help:options"));
}

#[test]
fn code_spans_keep_underscores_and_emphasis_skips_words() {
    let lines = render(
        &markdown("`diagnostic_virtual_text` diagnostic_virtual_text"),
        80,
    );
    assert_eq!(
        lines[0].text(),
        "diagnostic_virtual_text diagnostic_virtual_text"
    );
}

#[test]
fn wrapped_spans_keep_link_targets() {
    let lines = render_spans(
        &[MarkdownSpan {
            text: "options".to_string(),
            style: MarkdownStyle::Normal,
            link: Some("help:options".to_string()),
        }],
        3,
    );
    assert_eq!(
        lines.iter().map(MarkdownLine::text).collect::<Vec<_>>(),
        ["opt", "ion", "s"]
    );
    assert!(lines.iter().all(|line| line
        .spans
        .iter()
        .all(|span| span.link.as_deref() == Some("help:options"))));
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
            link: None,
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
                link: None,
            },
            MarkdownSpan {
                text: "cd".to_string(),
                style: MarkdownStyle::Strong,
                link: None,
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

#[test]
fn code_blocks_sanitize_control_characters() {
    let document = parse(&markdown("```\nsafe\u{1b}[31m\n```"));
    assert_eq!(code_block(&document).text(), "safe [31m");
}

#[test]
fn raw_markdown_rendering_never_infers_untagged_languages() {
    let lines = render(&markdown("```\nlet answer = 1;\n```"), 80);
    assert!(lines[0]
        .spans
        .iter()
        .all(|span| span.style == MarkdownStyle::Code));
}

#[test]
fn code_lines_keep_their_background_across_blocks() {
    let document = parse(&markdown("```rust\na\nb\n```\n\ntext\n\n```\nc\n```"));
    let lines = render_document(&document, 20);
    let flags: Vec<bool> = lines.iter().map(|line| line.code).collect();
    assert_eq!(flags, [true, true, false, false, false, true]);
}

#[test]
fn expands_tabs_at_the_standard_tab_stop() {
    let lines = render_spans(
        &[MarkdownSpan {
            text: "a\tb".to_string(),
            style: MarkdownStyle::Normal,
            link: None,
        }],
        80,
    );
    assert_eq!(lines[0].text(), "a       b");
}

#[test]
fn wraps_wide_characters_by_display_width() {
    let spans = [MarkdownSpan {
        text: "中文".to_string(),
        style: MarkdownStyle::Normal,
        link: None,
    }];
    assert_eq!(
        render_spans(&spans, 4)
            .iter()
            .map(MarkdownLine::text)
            .collect::<Vec<_>>(),
        ["中文"]
    );
    assert_eq!(
        render_spans(&spans, 2)
            .iter()
            .map(MarkdownLine::text)
            .collect::<Vec<_>>(),
        ["中", "文"]
    );
}
