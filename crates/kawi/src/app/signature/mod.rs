mod lifecycle;
mod session;

pub(in crate::app) use lifecycle::SignatureAuto;
pub(in crate::app) use session::SignatureSession;

pub(super) const DEFAULT_OPEN: char = '(';

use crossterm::event::KeyEvent;

use super::App;

impl App {
    pub(super) fn handle_signature_key(&mut self, key: KeyEvent) -> Option<bool> {
        if self.signature.is_none() || !self.editor.mode().is_insert() {
            return None;
        }
        let up = self.completion_options.keys.up.event();
        let down = self.completion_options.keys.down.event();
        if key != up && key != down {
            return None;
        }
        let (limit, page) = self.signature_scroll()?;
        let mut session = self.signature.take()?;
        let page = isize::try_from(page).unwrap_or(isize::MAX);
        session.scroll_by(if key == up { -page } else { page }, limit);
        self.signature = Some(session);
        Some(false)
    }

    #[cfg(test)]
    pub(super) fn signature_event_for_test(request: u64) -> kawi_lsp::SignatureHelpEvent {
        use kawi_lsp::{MarkupDocument, SignatureEntry};

        kawi_lsp::SignatureHelpEvent {
            request,
            server: "test".to_string(),
            signatures: vec![SignatureEntry {
                label: "run(value: usize)".to_string(),
                documentation: Some(MarkupDocument {
                    text: "Runs the thing.".to_string(),
                    markdown: false,
                }),
                parameters: Vec::new(),
                active_parameter: None,
            }],
            active_signature: Some(0),
            active_parameter: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_buffer::Buffer;
    use kawi_lsp::MarkupDocument;
    use kawi_syntax::LanguageId;
    use kawi_tui::markdown::{self, MarkdownStyle};

    use crate::keymap::Flavor;
    use crossterm::event::{KeyCode, KeyModifiers};
    use kawi_lsp::CompletionCapabilitiesEvent;

    fn grant_signature_help(app: &mut crate::app::App) {
        app.lsp
            .set_completion_capabilities_for_test(CompletionCapabilitiesEvent {
                server: "test".to_string(),
                completion: false,
                completion_triggers: Vec::new(),
                completion_resolve: false,
                signature_help: true,
                signature_triggers: vec!["(".to_string()],
                signature_retriggers: vec![",".to_string()],
            });
    }

    #[test]
    fn signature_documentation_is_prepared_with_the_buffer_language() {
        let event = kawi_lsp::SignatureHelpEvent {
            request: 1,
            server: "test".to_string(),
            signatures: vec![kawi_lsp::SignatureEntry {
                label: "run()".to_string(),
                documentation: Some(MarkupDocument {
                    text: "```\nfn run() {}\n```".to_string(),
                    markdown: true,
                }),
                parameters: Vec::new(),
                active_parameter: None,
            }],
            active_signature: Some(0),
            active_parameter: None,
        };
        let session = SignatureSession::new(event, Some(LanguageId::Rust), '(', 1);
        let lines = markdown::render_document(session.document(0).expect("document"), 80);
        assert!(lines[0]
            .spans
            .iter()
            .any(|span| matches!(span.style, MarkdownStyle::Syntax(_))));
    }

    #[test]
    fn typing_an_auto_paired_final_closer_dismisses_signature_help() {
        let mut app = App::new(Buffer::from_string("call()"), Flavor::Vim);
        grant_signature_help(&mut app);
        app.editor.apply(kawi_editor::Action::JumpTo(0, 5));
        app.editor.apply(kawi_editor::Action::EnterInsert);
        app.signature = Some(SignatureSession::new(
            App::signature_event_for_test(1),
            None,
            DEFAULT_OPEN,
            app.editor.buffer_number(),
        ));
        let revision = app.editor.buffer_revision();
        app.handle_mapped_key(KeyEvent::new(KeyCode::Char(')'), KeyModifiers::NONE))
            .expect("closing paren");
        assert_eq!(app.editor.buffer_revision(), revision);
        assert!(app.signature.is_none());
    }

    fn open_signature(app: &mut App) {
        app.signature = Some(SignatureSession::new(
            App::signature_event_for_test(1),
            None,
            DEFAULT_OPEN,
            app.editor.buffer_number(),
        ));
    }

    #[test]
    fn typing_inside_the_call_schedules_a_refresh() {
        let mut app = App::new(Buffer::from_string("call(na)"), Flavor::Vim);
        grant_signature_help(&mut app);
        app.editor.apply(kawi_editor::Action::JumpTo(0, 6));
        app.editor.apply(kawi_editor::Action::EnterInsert);
        open_signature(&mut app);
        app.handle_mapped_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE))
            .expect("typing");
        assert!(app.signature.is_some());
        assert!(app.signature_auto.wait_timeout().is_some());
    }

    #[test]
    fn leaving_the_call_closes_signature_help() {
        let mut app = App::new(Buffer::from_string("call() "), Flavor::Vim);
        grant_signature_help(&mut app);
        app.editor.apply(kawi_editor::Action::JumpTo(0, 5));
        app.editor.apply(kawi_editor::Action::EnterInsert);
        open_signature(&mut app);
        let revision = app.editor.buffer_revision();
        app.handle_mapped_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))
            .expect("move right");
        assert_eq!(app.editor.buffer_revision(), revision);
        assert!(app.signature.is_none());
        assert!(app.signature_auto.wait_timeout().is_none());
    }

    #[test]
    fn opening_completion_dismisses_signature_help() {
        let mut app = App::new(Buffer::from_string("call("), Flavor::Vim);
        grant_signature_help(&mut app);
        app.editor.apply(kawi_editor::Action::EnterInsert);
        open_signature(&mut app);
        app.request_completion().expect("completion request");
        assert!(app.signature.is_none());
    }

    #[test]
    fn disabling_signature_help_dismisses_the_session() {
        let mut app = App::new(Buffer::from_string("call("), Flavor::Vim);
        grant_signature_help(&mut app);
        app.editor.apply(kawi_editor::Action::EnterInsert);
        open_signature(&mut app);
        app.completion_options.signature = false;
        app.handle_mapped_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE))
            .expect("typing");
        assert!(app.signature.is_none());
    }

    #[test]
    fn a_failing_signature_request_clears_the_session() {
        let mut app = App::new(Buffer::from_string("call()"), Flavor::Vim);
        grant_signature_help(&mut app);
        app.editor.apply(kawi_editor::Action::JumpTo(0, 5));
        app.editor.apply(kawi_editor::Action::EnterInsert);
        app.handle_mapped_key(KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE))
            .expect("opening paren");
        assert!(app.signature.is_none());
        assert!(app.signature_tick());
        assert!(app.signature.is_none());
    }

    #[test]
    fn an_inner_closer_keeps_the_outer_call_context() {
        let mut app = App::new(Buffer::from_string("call(inner())"), Flavor::Vim);
        grant_signature_help(&mut app);
        app.editor.apply(kawi_editor::Action::JumpTo(0, 11));
        app.editor.apply(kawi_editor::Action::EnterInsert);
        app.signature = Some(SignatureSession::new(
            App::signature_event_for_test(1),
            None,
            DEFAULT_OPEN,
            app.editor.buffer_number(),
        ));
        app.handle_mapped_key(KeyEvent::new(KeyCode::Char(')'), KeyModifiers::NONE))
            .expect("closing paren");
        assert!(app.signature.is_some());
        assert!(app.signature_auto.wait_timeout().is_some());
    }
}
