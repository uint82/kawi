use kawi_completion::{
    Capabilities, Command, CompletionKind, Documentation, Event, Item, Request, RequestId,
    ResolveRequest, ResolvedItem, Source, SourceId, Trigger, TriggerKind,
};
use kawi_lsp::{
    CompletionEntry, CompletionEvent, CompletionItemKind, CompletionResolveEvent,
    CompletionTrigger, MarkupDocument,
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
        is_incomplete: event.is_incomplete,
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
        kind: neutral_kind(entry.kind),
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

fn neutral_kind(kind: Option<CompletionItemKind>) -> Option<CompletionKind> {
    let kind = kind?;
    Some(match kind {
        CompletionItemKind::TEXT => CompletionKind::Text,
        CompletionItemKind::METHOD => CompletionKind::Method,
        CompletionItemKind::FUNCTION => CompletionKind::Function,
        CompletionItemKind::CONSTRUCTOR => CompletionKind::Constructor,
        CompletionItemKind::FIELD => CompletionKind::Field,
        CompletionItemKind::VARIABLE => CompletionKind::Variable,
        CompletionItemKind::CLASS => CompletionKind::Class,
        CompletionItemKind::INTERFACE => CompletionKind::Interface,
        CompletionItemKind::MODULE => CompletionKind::Module,
        CompletionItemKind::PROPERTY => CompletionKind::Property,
        CompletionItemKind::UNIT => CompletionKind::Unit,
        CompletionItemKind::VALUE => CompletionKind::Value,
        CompletionItemKind::ENUM => CompletionKind::Enum,
        CompletionItemKind::KEYWORD => CompletionKind::Keyword,
        CompletionItemKind::SNIPPET => CompletionKind::Snippet,
        CompletionItemKind::COLOR => CompletionKind::Color,
        CompletionItemKind::FILE => CompletionKind::File,
        CompletionItemKind::REFERENCE => CompletionKind::Reference,
        CompletionItemKind::FOLDER => CompletionKind::Folder,
        CompletionItemKind::ENUM_MEMBER => CompletionKind::EnumMember,
        CompletionItemKind::CONSTANT => CompletionKind::Constant,
        CompletionItemKind::STRUCT => CompletionKind::Struct,
        CompletionItemKind::EVENT => CompletionKind::Event,
        CompletionItemKind::OPERATOR => CompletionKind::Operator,
        CompletionItemKind::TYPE_PARAMETER => CompletionKind::TypeParameter,
        _ => return None,
    })
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
        TriggerKind::Incomplete => CompletionTrigger::incomplete(),
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
            kind: None,
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
            snippet: None,
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
            is_incomplete: false,
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
        assert_eq!(
            lsp_trigger(Trigger::character(".")).kind,
            CompletionTriggerKind::TriggerCharacter
        );
    }

    #[test]
    fn incomplete_trigger_survives_conversion() {
        assert_eq!(
            lsp_trigger(Trigger::incomplete()).kind,
            CompletionTriggerKind::Incomplete
        );
    }

    #[test]
    fn incomplete_lists_survive_neutralization() {
        let mut source = event(vec![entry(None, "value")]);
        source.is_incomplete = true;
        assert!(neutral_event(source).is_incomplete);
    }

    #[test]
    fn completion_kinds_are_translated() {
        let pairs = [
            (CompletionItemKind::TEXT, CompletionKind::Text),
            (CompletionItemKind::METHOD, CompletionKind::Method),
            (CompletionItemKind::FUNCTION, CompletionKind::Function),
            (CompletionItemKind::CONSTRUCTOR, CompletionKind::Constructor),
            (CompletionItemKind::FIELD, CompletionKind::Field),
            (CompletionItemKind::VARIABLE, CompletionKind::Variable),
            (CompletionItemKind::CLASS, CompletionKind::Class),
            (CompletionItemKind::INTERFACE, CompletionKind::Interface),
            (CompletionItemKind::MODULE, CompletionKind::Module),
            (CompletionItemKind::PROPERTY, CompletionKind::Property),
            (CompletionItemKind::UNIT, CompletionKind::Unit),
            (CompletionItemKind::VALUE, CompletionKind::Value),
            (CompletionItemKind::ENUM, CompletionKind::Enum),
            (CompletionItemKind::KEYWORD, CompletionKind::Keyword),
            (CompletionItemKind::SNIPPET, CompletionKind::Snippet),
            (CompletionItemKind::COLOR, CompletionKind::Color),
            (CompletionItemKind::FILE, CompletionKind::File),
            (CompletionItemKind::REFERENCE, CompletionKind::Reference),
            (CompletionItemKind::FOLDER, CompletionKind::Folder),
            (CompletionItemKind::ENUM_MEMBER, CompletionKind::EnumMember),
            (CompletionItemKind::CONSTANT, CompletionKind::Constant),
            (CompletionItemKind::STRUCT, CompletionKind::Struct),
            (CompletionItemKind::EVENT, CompletionKind::Event),
            (CompletionItemKind::OPERATOR, CompletionKind::Operator),
            (
                CompletionItemKind::TYPE_PARAMETER,
                CompletionKind::TypeParameter,
            ),
        ];
        for (lsp_kind, expected) in pairs {
            let mut source = entry(None, "value");
            source.kind = Some(lsp_kind);
            let event = neutral_event(event(vec![source]));
            assert_eq!(event.items[0].kind, Some(expected));
        }
    }

    #[test]
    fn unknown_completion_kinds_are_ignored() {
        let mut source = entry(None, "value");
        source.kind = serde_json::from_value(serde_json::json!(99)).ok();
        let event = neutral_event(event(vec![source]));
        assert_eq!(event.items[0].kind, None);
    }
}
