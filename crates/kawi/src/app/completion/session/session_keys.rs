use crossterm::event::KeyEvent;

use super::super::super::lsp_menu::{menu_key, MenuKey};
use super::{CompletionOutcome, CompletionSession};

impl CompletionSession {
    pub(crate) fn handle_key(
        &mut self,
        key: KeyEvent,
        docs_up: KeyEvent,
        docs_down: KeyEvent,
        docs_scroll: Option<(usize, usize)>,
    ) -> CompletionOutcome {
        if let Some((limit, page)) = docs_scroll {
            if self.selected_document().is_some() {
                let page = isize::try_from(page).unwrap_or(isize::MAX);
                if key == docs_up {
                    self.scroll_docs(-page, limit);
                    return CompletionOutcome::Consumed;
                }
                if key == docs_down {
                    self.scroll_docs(page, limit);
                    return CompletionOutcome::Consumed;
                }
            }
        }
        match menu_key(key) {
            MenuKey::Cancel => CompletionOutcome::Cancel,
            MenuKey::Move(delta) => {
                self.selection.move_by(delta);
                self.docs_scroll = 0;
                self.prepare_selected_document();
                CompletionOutcome::Consumed
            }
            MenuKey::Accept => CompletionOutcome::Accept,
            MenuKey::Other => CompletionOutcome::Fallthrough,
        }
    }
}
