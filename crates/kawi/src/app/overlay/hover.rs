use kawi_tui::markdown::{self, MarkdownDocument, MarkdownLine};

use super::super::App;
use super::{page_height, panel_content_width, OverlayData, PanelData};

impl App {
    pub(super) fn hover_overlay(&self) -> Option<OverlayData<'_>> {
        let session = self.hover.as_ref()?;
        let layout = self.panel_layout(self.hover_lines(session.documents()));
        Some(OverlayData {
            list: None,
            panel: Some(PanelData {
                rect: layout.rect,
                lines: layout.lines,
                scroll: session.scroll().min(layout.max_scroll),
            }),
        })
    }

    pub(crate) fn hover_scroll(&self) -> Option<(usize, usize)> {
        let session = self.hover.as_ref()?;
        let layout = self.panel_layout(self.hover_lines(session.documents()));
        Some((layout.max_scroll, page_height(layout.rect)))
    }

    fn hover_lines(&self, documents: &[MarkdownDocument]) -> Vec<MarkdownLine> {
        let width = panel_content_width(self.window_area().width);
        let mut lines = Vec::new();
        for (index, document) in documents.iter().enumerate() {
            if index > 0 {
                lines.push(MarkdownLine::blank());
            }
            lines.extend(markdown::render_document(document, width));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::lsp_navigation::HoverResult;
    use crate::keymap::Flavor;
    use kawi_buffer::Buffer;
    use kawi_lsp::MarkupDocument;
    use kawi_syntax::LanguageId;
    use kawi_tui::markdown::{MarkdownLine, MarkdownStyle};
    use kawi_tui::theme::Theme;

    use super::super::code::{highlight_calls, reset_highlight_calls};

    fn hover_app() -> App {
        let mut app = App::new(Buffer::from_string("value\n"), Flavor::Vim);
        reset_highlight_calls();
        app.handle_hover_result(HoverResult {
            documents: vec![MarkupDocument {
                text: "```rust\nlet answer: u8 = 42;\n```".to_string(),
                markdown: true,
            }],
            language: Some(LanguageId::Rust),
        });
        assert_eq!(highlight_calls(), 1);
        app
    }

    fn has_syntax(lines: &[MarkdownLine]) -> bool {
        lines
            .iter()
            .flat_map(|line| &line.spans)
            .any(|span| matches!(span.style, MarkdownStyle::Syntax(_)))
    }

    #[test]
    fn repaint_resize_and_theme_do_not_reparse_hover_documents() {
        let mut app = hover_app();
        let first = {
            let overlay = app.hover_overlay().expect("overlay");
            overlay.panel.expect("panel").lines
        };
        assert!(has_syntax(&first));
        assert!(app.hover_overlay().is_some());
        assert_eq!(highlight_calls(), 1);

        app.set_screen_size(20, 10);
        let resized = {
            let overlay = app.hover_overlay().expect("overlay");
            overlay.panel.expect("panel").lines
        };
        assert!(resized.len() > first.len());
        assert!(has_syntax(&resized));
        assert_eq!(highlight_calls(), 1);

        app.theme = Theme::named("gruvbox-dark").expect("theme");
        let recolored = {
            let overlay = app.hover_overlay().expect("overlay");
            overlay.panel.expect("panel").lines
        };
        assert_eq!(recolored, resized);
        assert_eq!(highlight_calls(), 1);
    }
}
