use super::*;
use crate::app::lsp::{LspUi, SymbolEvent};
use kawi_editor::Editor;
use std::time::Instant;

pub(crate) fn handle_event(
    event: SymbolEvent,
    session: &mut Option<SymbolSession>,
    editor: &Editor,
    lsp: &mut LspUi,
    cache: &mut SymbolCache,
    now: Instant,
) -> Option<SymbolJump> {
    if let SymbolEvent::Document(event) = &event {
        cache.insert(&event.path, event.version, event.symbols.clone());
    }
    let session = session.as_mut()?;
    match event {
        SymbolEvent::Document(event) => {
            let cursor = (editor.row(), editor.col());
            session.apply_document(event, cursor, now, editor, lsp);
        }
        SymbolEvent::Workspace(event) => {
            session.apply_workspace(event, now, editor, lsp);
        }
        SymbolEvent::Resolved(event) => {
            return session.apply_resolved(event, now, editor, lsp);
        }
        SymbolEvent::Error { request, message } => {
            session.apply_error(request, &message);
        }
    }
    None
}
