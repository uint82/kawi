use kawi_completion::{
    Capabilities, Command, Documentation, Event, Item, Request, RequestId, ResolveRequest,
    ResolvedItem, Source, SourceId, Trigger, TriggerKind,
};
use kawi_lsp::{
    CompletionEntry, CompletionEvent, CompletionResolveEvent, CompletionTrigger, MarkupDocument,
};
use serde_json::Value;

use super::super::lsp::LspUi;

pub(super) fn neutral_event(event: CompletionEvent) -> Event {
    let items = event
        .items
        .into_iter()
        .enumerate()
        .map(|(index, entry)| neutral_item(entry, &event.server, index))
        .collect();
    Event {
        request: event.request,
        path: event.path,
        version: event.version,
        original_hash: event.original_hash,
        cursor: event.cursor,
        word_start: event.word_start,
        prefix: event.prefix,
        items,
    }
}

pub(super) fn neutral_resolved(event: CompletionResolveEvent) -> ResolvedItem {
    ResolvedItem {
        request: event.request,
        item_id: event.item_id.to_string(),
        detail: event.detail,
        documentation: event.documentation.map(neutral_documentation),
        additional: event.additional,
    }
}

fn neutral_item(entry: CompletionEntry, server: &str, index: usize) -> Item {
    let id = entry.resolve_id.map_or_else(
        || format!("{server}:{index}:{}", entry.label),
        |id| id.to_string(),
    );
    Item {
        source: server.to_string(),
        id,
        label: entry.label,
        filter_text: entry.filter_text,
        sort_text: entry.sort_text,
        detail: entry.detail,
        documentation: entry.documentation.map(neutral_documentation),
        deprecated: entry.deprecated,
        preselect: entry.preselect,
        snippet: entry.snippet,
        resolve: entry.resolve_id.is_some(),
        primary: entry.primary,
        additional: entry.additional,
        command: entry.command.map(|command| Command {
            title: command.title,
            command: command.command,
            arguments: command.arguments.map(Value::Array),
        }),
    }
}

fn neutral_documentation(document: MarkupDocument) -> Documentation {
    Documentation::from_markup(document.text, document.markdown)
}

pub(super) fn lsp_trigger(trigger: Trigger) -> CompletionTrigger {
    match trigger.kind {
        TriggerKind::Invoked | TriggerKind::Typing => CompletionTrigger::invoked(),
        TriggerKind::TriggerCharacter => trigger
            .character
            .map(CompletionTrigger::character)
            .unwrap_or_else(CompletionTrigger::invoked),
    }
}

impl Source for LspUi {
    fn id(&self) -> SourceId {
        "lsp".to_string()
    }

    fn capabilities(&self) -> Capabilities {
        LspUi::completion_capabilities(self)
    }

    fn request(&mut self, request: Request) -> Result<RequestId, String> {
        let position = kawi_lsp::CursorPosition {
            line: request.line,
            column: request.column,
        };
        LspUi::completion_at(self, &request.path, position, lsp_trigger(request.trigger))
    }

    fn resolve(&mut self, request: ResolveRequest) -> Result<RequestId, String> {
        let item_id = request
            .item
            .id
            .parse::<u64>()
            .map_err(|_| "completion item cannot be resolved".to_string())?;
        LspUi::resolve_completion(self, item_id)
    }

    fn cancel(&mut self, request_id: RequestId) {
        self.cancel_request(request_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_lsp::{CompletionTriggerKind, TextChange};
    use std::path::PathBuf;

    fn entry(id: Option<u64>, label: &str) -> CompletionEntry {
        CompletionEntry {
            server: "fake".to_string(),
            label: label.to_string(),
            filter_text: None,
            sort_text: None,
            detail: None,
            documentation: None,
            primary: TextChange {
                start: 0,
                end: 0,
                text: label.to_string(),
            },
            additional: Vec::new(),
            deprecated: false,
            preselect: false,
            snippet: false,
            resolve_id: id,
            command: None,
        }
    }

    fn event(entries: Vec<CompletionEntry>) -> CompletionEvent {
        CompletionEvent {
            request: 3,
            server: "fake".to_string(),
            path: PathBuf::from("/tmp/source.rs"),
            version: 1,
            original_hash: 0,
            cursor: 0,
            word_start: 0,
            prefix: String::new(),
            items: entries,
        }
    }

    #[test]
    fn resolvable_items_use_the_resolve_id() {
        let event = neutral_event(event(vec![entry(Some(8), "value")]));
        assert_eq!(event.items[0].id, "8");
        assert!(event.items[0].resolve);
    }

    #[test]
    fn plain_items_get_a_source_scoped_id() {
        let event = neutral_event(event(vec![entry(None, "value")]));
        assert_eq!(event.items[0].id, "fake:0:value");
        assert!(!event.items[0].resolve);
    }

    #[test]
    fn trigger_characters_survive_conversion() {
        match lsp_trigger(Trigger::character(".")).kind {
            CompletionTriggerKind::TriggerCharacter => {}
            CompletionTriggerKind::Invoked => panic!("expected a trigger character"),
        }
    }
}
