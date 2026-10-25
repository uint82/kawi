use kawi_completion::Documentation;
use kawi_lsp::MarkupDocument;
use kawi_syntax::LanguageId;
use kawi_tui::markdown::{self, MarkdownBlock, MarkdownDocument};

use super::code::highlight_code_block;

pub(crate) fn prepare_markup_document(
    document: &MarkupDocument,
    fallback: Option<LanguageId>,
) -> MarkdownDocument {
    prepare_documentation(&as_documentation(document), fallback)
}

pub(crate) fn prepare_markup_documents(
    documents: &[MarkupDocument],
    fallback: Option<LanguageId>,
) -> Vec<MarkdownDocument> {
    documents
        .iter()
        .map(|document| prepare_markup_document(document, fallback))
        .collect()
}

pub(crate) fn prepare_documentation(
    documentation: &Documentation,
    fallback: Option<LanguageId>,
) -> MarkdownDocument {
    let mut document = markdown::parse(documentation);
    for block in &mut document.blocks {
        if let MarkdownBlock::Code(code) = block {
            highlight_code_block(code, fallback);
        }
    }
    document
}

fn as_documentation(document: &MarkupDocument) -> Documentation {
    Documentation::from_markup(document.text.clone(), document.markdown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_syntax::SyntaxGroup;
    use kawi_tui::markdown::{MarkdownLine, MarkdownStyle};

    fn hover(text: &str) -> MarkdownDocument {
        prepare_markup_document(
            &MarkupDocument {
                text: text.to_string(),
                markdown: true,
            },
            Some(LanguageId::Rust),
        )
    }

    fn style_of(line: &MarkdownLine, text: &str) -> Option<MarkdownStyle> {
        line.spans
            .iter()
            .find(|span| span.text == text)
            .map(|span| span.style)
    }

    #[test]
    fn highlights_known_and_keeps_unknown_languages_plain() {
        let document = hover("```rust\nlet answer: u8 = 42;\n```\n\n```mystery\nraw text\n```");
        let lines = markdown::render_document(&document, 80);
        assert_eq!(
            style_of(&lines[0], "let"),
            Some(MarkdownStyle::Syntax(SyntaxGroup::Keyword))
        );
        assert_eq!(
            lines
                .iter()
                .map(MarkdownLine::text)
                .collect::<Vec<_>>()
                .join("\n"),
            "let answer: u8 = 42;\n\nraw text"
        );
        let raw = lines
            .iter()
            .find(|line| line.text() == "raw text")
            .expect("unknown block");
        assert!(raw
            .spans
            .iter()
            .all(|span| span.style == MarkdownStyle::Code));
    }

    #[test]
    fn untagged_fences_use_the_fallback_language() {
        let document = hover("```\nlet answer = 1;\n```");
        let lines = markdown::render_document(&document, 80);
        assert_eq!(
            style_of(&lines[0], "let"),
            Some(MarkdownStyle::Syntax(SyntaxGroup::Keyword))
        );
    }

    #[test]
    fn resize_rewraps_stored_syntax_spans() {
        let document = hover("```rust\nlet answer: u8 = 42;\n```");
        let wide = markdown::render_document(&document, 80);
        let narrow = markdown::render_document(&document, 8);
        assert_eq!(wide.len(), 1);
        assert!(narrow.len() > wide.len());
        assert!(narrow
            .iter()
            .flat_map(|line| &line.spans)
            .any(|span| matches!(span.style, MarkdownStyle::Syntax(_))));
    }

    #[test]
    fn plain_text_documents_are_not_highlighted() {
        let document = prepare_markup_document(
            &MarkupDocument {
                text: "let answer = 1;".to_string(),
                markdown: false,
            },
            Some(LanguageId::Rust),
        );
        let lines = markdown::render_document(&document, 80);
        assert_eq!(lines[0].spans[0].style, MarkdownStyle::Normal);
    }
}
