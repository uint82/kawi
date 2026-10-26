use crossterm::event::KeyEvent;

use kawi_lsp::MarkupDocument;
use kawi_syntax::LanguageId;
use kawi_tui::markdown::MarkdownDocument;

use super::overlay::document::prepare_markup_documents;

use super::lsp_navigation::HoverResult;
use super::App;

pub(super) struct HoverSession {
    documents: Vec<MarkdownDocument>,
    scroll: usize,
}

impl HoverSession {
    pub(super) fn new(documents: Vec<MarkupDocument>, fallback: Option<LanguageId>) -> Self {
        Self {
            documents: prepare_markup_documents(&documents, fallback),
            scroll: 0,
        }
    }

    pub(super) fn documents(&self) -> &[MarkdownDocument] {
        &self.documents
    }

    pub(super) fn scroll(&self) -> usize {
        self.scroll
    }

    fn scroll_by(&mut self, delta: isize, limit: usize) {
        self.scroll = self.scroll.saturating_add_signed(delta).min(limit);
    }
}

impl App {
    pub(super) fn handle_hover_result(&mut self, result: HoverResult) {
        if self.picker.is_some() || self.replace.is_some() {
            return;
        }
        if result.documents.is_empty() {
            self.show_message("No hover information".to_string());
            return;
        }
        self.pager = None;
        self.hover = Some(HoverSession::new(result.documents, result.language));
    }

    pub(super) fn handle_hover_key(&mut self, key: KeyEvent) -> anyhow::Result<Option<bool>> {
        let up = self.completion_options.keys.up.event();
        let down = self.completion_options.keys.down.event();
        let geometry = (key == up || key == down)
            .then(|| self.hover_scroll())
            .flatten();
        Ok(super::overlay_scroll::handle_scroll_key(
            super::overlay_scroll::ScrollKey {
                session: &mut self.hover,
                geometry,
                key,
                up,
                down,
            },
        ))
    }
}

impl super::overlay_scroll::Scrollable for HoverSession {
    fn scroll_by(&mut self, delta: isize, limit: usize) {
        HoverSession::scroll_by(self, delta, limit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Flavor;
    use crossterm::event::{KeyCode, KeyModifiers};
    use kawi_buffer::Buffer;

    fn hover_app() -> App {
        let mut app = App::new(Buffer::from_string("value\n"), Flavor::Vim);
        app.hover = Some(HoverSession::new(
            vec![MarkupDocument {
                text: "documentation".to_string(),
                markdown: false,
            }],
            None,
        ));
        app
    }

    #[test]
    fn scroll_keys_keep_the_hover_open() {
        let mut app = hover_app();
        let key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL);
        assert_eq!(app.handle_hover_key(key).expect("hover"), Some(false));
        assert!(app.hover.is_some());
    }

    #[test]
    fn other_keys_close_the_hover_and_fall_through() {
        let mut app = hover_app();
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(app.handle_hover_key(key).expect("hover"), None);
        assert!(app.hover.is_none());
    }

    #[test]
    fn dismiss_keys_close_the_hover() {
        let mut app = hover_app();
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(app.handle_hover_key(key).expect("hover"), Some(false));
        assert!(app.hover.is_none());
    }
}

#[cfg(test)]
mod syntax_tests {
    use super::*;
    use kawi_tui::markdown::{self, MarkdownStyle};

    #[test]
    fn hover_documents_are_prepared_with_the_buffer_language() {
        let session = HoverSession::new(
            vec![MarkupDocument {
                text: "```\nlet answer = 1;\n```".to_string(),
                markdown: true,
            }],
            Some(LanguageId::Rust),
        );
        let lines = markdown::render_document(&session.documents()[0], 80);
        assert!(lines[0]
            .spans
            .iter()
            .any(|span| matches!(span.style, MarkdownStyle::Syntax(_))));
    }
}
