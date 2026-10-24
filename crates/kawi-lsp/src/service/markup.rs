use lsp_types::Documentation;

use super::MarkupDocument;

pub(super) fn markup_document(documentation: Documentation) -> MarkupDocument {
    match documentation {
        Documentation::String(value) => plain_document(&value),
        Documentation::MarkupContent(markup) => MarkupDocument {
            text: sanitize_documentation(&markup.value),
            markdown: markup.kind == lsp_types::MarkupKind::Markdown,
        },
    }
}

pub(super) fn plain_document(value: &str) -> MarkupDocument {
    MarkupDocument {
        text: sanitize_documentation(value),
        markdown: false,
    }
}

pub(super) fn sanitize_documentation(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character == '\n' || character == '\t' || !character.is_control() {
                character
            } else {
                ' '
            }
        })
        .collect()
}
