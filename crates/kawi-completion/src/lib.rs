use kawi_buffer::TextEdit;
use serde_json::Value;
use std::path::PathBuf;

mod kind;

pub use kind::CompletionKind;

pub type RequestId = u64;
pub type SourceId = String;
pub type ItemId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerKind {
    Invoked,
    Typing,
    TriggerCharacter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trigger {
    pub kind: TriggerKind,
    pub character: Option<String>,
}

impl Trigger {
    pub fn invoked() -> Self {
        Self {
            kind: TriggerKind::Invoked,
            character: None,
        }
    }

    pub fn typing() -> Self {
        Self {
            kind: TriggerKind::Typing,
            character: None,
        }
    }

    pub fn character(character: impl Into<String>) -> Self {
        Self {
            kind: TriggerKind::TriggerCharacter,
            character: Some(character.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub path: PathBuf,
    pub line: u32,
    pub column: u32,
    pub trigger: Trigger,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveRequest {
    pub item: Item,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub request: RequestId,
    pub path: PathBuf,
    pub version: i32,
    pub original_hash: u64,
    pub cursor: usize,
    pub word_start: usize,
    pub prefix: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedItem {
    pub request: RequestId,
    pub item_id: ItemId,
    pub detail: Option<String>,
    pub documentation: Option<Documentation>,
    pub additional: Vec<TextEdit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub source: SourceId,
    pub id: ItemId,
    pub label: String,
    pub kind: Option<CompletionKind>,
    pub filter_text: Option<String>,
    pub sort_text: Option<String>,
    pub detail: Option<String>,
    pub documentation: Option<Documentation>,
    pub deprecated: bool,
    pub preselect: bool,
    pub snippet: bool,
    pub resolve: bool,
    pub primary: TextEdit,
    pub additional: Vec<TextEdit>,
    pub command: Option<Command>,
}

impl Item {
    pub fn filter_key(&self) -> &str {
        self.filter_text.as_deref().unwrap_or(&self.label)
    }

    pub fn sort_key(&self) -> &str {
        self.sort_text.as_deref().unwrap_or(&self.label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub title: String,
    pub command: String,
    pub arguments: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkupKind {
    Markdown,
    PlainText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Documentation {
    pub text: String,
    pub kind: MarkupKind,
}

impl Documentation {
    pub fn from_markup(text: impl Into<String>, markdown: bool) -> Self {
        Self {
            text: text.into(),
            kind: if markdown {
                MarkupKind::Markdown
            } else {
                MarkupKind::PlainText
            },
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub completion: bool,
    pub resolve: bool,
    pub completion_triggers: Vec<String>,
    pub signature_help: bool,
    pub signature_triggers: Vec<String>,
}

pub trait Source {
    fn id(&self) -> SourceId;
    fn capabilities(&self) -> Capabilities;
    fn request(&mut self, request: Request) -> Result<RequestId, String>;
    fn resolve(&mut self, request: ResolveRequest) -> Result<RequestId, String>;
    fn cancel(&mut self, request_id: RequestId);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(label: &str) -> Item {
        Item {
            source: "test".to_string(),
            id: label.to_string(),
            label: label.to_string(),
            kind: None,
            filter_text: None,
            sort_text: None,
            detail: None,
            documentation: None,
            deprecated: false,
            preselect: false,
            snippet: false,
            resolve: false,
            primary: TextEdit {
                start: 0,
                end: 0,
                text: label.to_string(),
            },
            additional: Vec::new(),
            command: None,
        }
    }

    #[test]
    fn documentation_tracks_the_markup_flag() {
        assert_eq!(
            Documentation::from_markup("**value**", true).kind,
            MarkupKind::Markdown
        );
        assert_eq!(
            Documentation::from_markup("**value**", false).kind,
            MarkupKind::PlainText
        );
    }

    #[test]
    fn filter_and_sort_keys_fall_back_to_label() {
        let mut value = item("value");
        assert_eq!(value.filter_key(), "value");
        assert_eq!(value.sort_key(), "value");
        value.filter_text = Some("filter".to_string());
        value.sort_text = Some("sort".to_string());
        assert_eq!(value.filter_key(), "filter");
        assert_eq!(value.sort_key(), "sort");
    }
}
