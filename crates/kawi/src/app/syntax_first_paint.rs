use kawi_syntax::{SyntaxRequest, SyntaxService};

use super::App;

impl App {
    pub(super) fn paint_syntax_first(&mut self, request: SyntaxRequest) {
        let key = request.key;
        self.syntax_text = request.text.clone();
        self.syntax_text_revision = key.revision;
        let wake = self.events.wake();
        let service = self
            .syntax
            .get_or_insert_with(|| SyntaxService::with_wake(wake));
        let result = service.highlight_now(request);
        self.syntax_latest = result.generation;
        self.syntax_pending = false;
        self.syntax_key = Some(key);
        self.apply_syntax_result(result);
    }
}

#[cfg(test)]
mod tests;
