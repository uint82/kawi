use super::super::position::cursor_position;
use super::super::*;
use super::convert::{append_symbol_information, has_deprecated_tag, symbol_range};
use super::SymbolEntry;
use super::SymbolKind;
use lsp_types::{DocumentSymbolResponse, SymbolInformation};
use ropey::Rope;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

pub(super) fn document_symbols(
    documents: &HashMap<PathBuf, Document>,
    path: &Path,
    text: &Rope,
    value: &Value,
    encoding: &PositionEncodingKind,
) -> Result<Vec<SymbolEntry>, String> {
    let response = serde_json::from_value::<Option<DocumentSymbolResponse>>(value.clone())
        .map_err(|error| format!("Invalid document symbol response: {error}"))?;
    let Some(response) = response else {
        return Ok(Vec::new());
    };
    let mut entries = Vec::new();
    match response {
        DocumentSymbolResponse::Nested(symbols) => {
            for symbol in symbols {
                flatten_document_symbol(
                    &mut entries,
                    text,
                    path,
                    encoding,
                    symbol,
                    &mut Vec::new(),
                );
            }
        }
        DocumentSymbolResponse::Flat(symbols) => {
            flatten_symbol_information(documents, path, text, encoding, symbols, &mut entries)
        }
    }
    Ok(entries)
}

fn flatten_symbol_information(
    documents: &HashMap<PathBuf, Document>,
    path: &Path,
    text: &Rope,
    encoding: &PositionEncodingKind,
    symbols: Vec<SymbolInformation>,
    entries: &mut Vec<SymbolEntry>,
) {
    let mut cache = HashMap::from([(path.to_path_buf(), text.clone())]);
    append_symbol_information(entries, documents, &mut cache, encoding, symbols, None);
}

#[allow(deprecated)]
fn flatten_document_symbol(
    entries: &mut Vec<SymbolEntry>,
    text: &Rope,
    path: &Path,
    encoding: &PositionEncodingKind,
    symbol: lsp_types::DocumentSymbol,
    parents: &mut Vec<String>,
) {
    let range = symbol_range(text, symbol.range, encoding);
    let selection = cursor_position(text, symbol.selection_range.start, encoding);
    entries.push(SymbolEntry {
        name: symbol.name.clone(),
        detail: symbol.detail.as_deref().map(one_line_message),
        container: (!parents.is_empty()).then(|| parents.join(" > ")),
        kind: SymbolKind::from_lsp(symbol.kind),
        deprecated: has_deprecated_tag(symbol.tags.as_deref(), symbol.deprecated),
        path: path.to_path_buf(),
        range: Some(range),
        selection: Some(selection),
        server: None,
        resolve_id: None,
    });
    parents.push(symbol.name);
    for child in symbol.children.unwrap_or_default() {
        flatten_document_symbol(entries, text, path, encoding, child, parents);
    }
    parents.pop();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn document(path: &Path, text: &str) -> Document {
        Document {
            uri: crate::workspace::file_uri(path).expect("uri"),
            language_id: "rust".to_string(),
            text: Rope::from_str(text),
            version: 1,
            clients: Vec::new(),
        }
    }

    fn documents(path: &Path, text: &str) -> HashMap<PathBuf, Document> {
        HashMap::from([(path.to_path_buf(), document(path, text))])
    }

    #[test]
    fn flattens_nested_symbols_in_document_order() {
        let path = PathBuf::from("/tmp/kawi_symbols_nested.rs");
        let files = documents(&path, "struct Outer\nfn inner()\n");
        let text = Rope::from_str("struct Outer\nfn inner()\n");
        let value = serde_json::json!([{
            "name": "Outer",
            "kind": 23,
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 1, "character": 12}},
            "selectionRange": {"start": {"line": 0, "character": 7}, "end": {"line": 0, "character": 12}},
            "children": [{
                "name": "inner",
                "kind": 12,
                "range": {"start": {"line": 1, "character": 0}, "end": {"line": 1, "character": 12}},
                "selectionRange": {"start": {"line": 1, "character": 3}, "end": {"line": 1, "character": 8}}
            }]
        }]);
        let entries = document_symbols(&files, &path, &text, &value, &PositionEncodingKind::UTF16)
            .expect("symbols");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "Outer");
        assert_eq!(entries[0].selection.expect("selection").column, 7);
        assert_eq!(entries[1].name, "inner");
        assert_eq!(entries[1].container.as_deref(), Some("Outer"));
        assert_eq!(entries[1].selection.expect("selection").column, 3);
    }

    #[test]
    fn flat_symbols_jump_to_the_location_start() {
        let path = PathBuf::from("/tmp/kawi_symbols_flat.rs");
        let files = documents(&path, "one\ntwo\n");
        let text = Rope::from_str("one\ntwo\n");
        let uri = crate::workspace::file_uri(&path).expect("uri");
        let value = serde_json::json!([{
            "name": "two",
            "kind": 12,
            "location": {
                "uri": uri,
                "range": {
                    "start": {"line": 1, "character": 1},
                    "end": {"line": 1, "character": 3}
                }
            },
            "containerName": "root"
        }]);
        let entries = document_symbols(&files, &path, &text, &value, &PositionEncodingKind::UTF16)
            .expect("symbols");
        assert_eq!(entries[0].selection.expect("selection").line, 1);
        assert_eq!(entries[0].selection.expect("selection").column, 1);
        assert_eq!(entries[0].container.as_deref(), Some("root"));
    }

    #[test]
    fn null_and_empty_responses_have_no_symbols() {
        let path = PathBuf::from("/tmp/kawi_symbols_empty.rs");
        let files = documents(&path, "fn main() {}\n");
        let text = Rope::from_str("fn main() {}\n");
        for value in [Value::Null, serde_json::json!([])] {
            assert!(
                document_symbols(&files, &path, &text, &value, &PositionEncodingKind::UTF16)
                    .expect("symbols")
                    .is_empty()
            );
        }
    }
}
