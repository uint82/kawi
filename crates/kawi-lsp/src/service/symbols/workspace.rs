use super::super::*;
use super::convert::{append_symbol_information, has_deprecated_tag, workspace_symbol_entry};
use super::{StoredWorkspaceSymbol, SymbolEntry, SymbolKind, SymbolResponse};
use crate::service::navigation_results::path_for_uri;
use lsp_types::WorkspaceSymbolResponse;
use std::collections::HashMap;

#[allow(deprecated)]
pub(super) fn workspace_symbols(
    stored: &mut HashMap<u64, StoredWorkspaceSymbol>,
    next_symbol: &mut u64,
    documents: &HashMap<PathBuf, Document>,
    response: SymbolResponse,
    resolve_support: bool,
    advertised_resolve: bool,
) -> Result<Vec<SymbolEntry>, String> {
    let parsed = serde_json::from_value::<Option<WorkspaceSymbolResponse>>(response.value.clone())
        .map_err(|error| format!("Invalid workspace symbol response: {error}"))?;
    let Some(parsed) = parsed else {
        return Ok(Vec::new());
    };
    let mut cache = HashMap::new();
    match parsed {
        WorkspaceSymbolResponse::Flat(symbols) => {
            let mut entries = Vec::new();
            append_symbol_information(
                &mut entries,
                documents,
                &mut cache,
                &response.encoding,
                symbols,
                Some(&response.server),
            );
            Ok(entries)
        }
        WorkspaceSymbolResponse::Nested(symbols) => Ok(nested_entries(
            NestedContext {
                stored,
                next_symbol,
                documents,
                cache: &mut cache,
                response: &response,
                resolve_support,
                advertised_resolve,
            },
            symbols,
        )),
    }
}

struct NestedContext<'a> {
    stored: &'a mut HashMap<u64, StoredWorkspaceSymbol>,
    next_symbol: &'a mut u64,
    documents: &'a HashMap<PathBuf, Document>,
    cache: &'a mut HashMap<PathBuf, Rope>,
    response: &'a SymbolResponse,
    resolve_support: bool,
    advertised_resolve: bool,
}

#[allow(deprecated)]
fn nested_entries(
    context: NestedContext<'_>,
    symbols: Vec<lsp_types::WorkspaceSymbol>,
) -> Vec<SymbolEntry> {
    let mut entries = Vec::new();
    for symbol in symbols {
        let Some(path) = symbol_path(&symbol) else {
            continue;
        };
        let has_range = matches!(symbol.location, lsp_types::OneOf::Left(_));
        if !has_range && (!context.resolve_support || !context.advertised_resolve) {
            continue;
        }
        if has_range {
            if let Some(entry) = workspace_symbol_entry(
                context.documents,
                context.cache,
                &path,
                &context.response.encoding,
                symbol,
                &context.response.server,
                None,
            ) {
                entries.push(entry);
            }
            continue;
        }
        entries.push(stored_entry(
            context.stored,
            context.next_symbol,
            context.response,
            path,
            symbol,
        ));
    }
    entries
}

fn symbol_path(symbol: &lsp_types::WorkspaceSymbol) -> Option<PathBuf> {
    match &symbol.location {
        lsp_types::OneOf::Left(location) => path_for_uri(&location.uri),
        lsp_types::OneOf::Right(location) => path_for_uri(&location.uri),
    }
}

#[allow(deprecated)]
fn stored_entry(
    stored: &mut HashMap<u64, StoredWorkspaceSymbol>,
    next_symbol: &mut u64,
    response: &SymbolResponse,
    path: PathBuf,
    symbol: lsp_types::WorkspaceSymbol,
) -> SymbolEntry {
    *next_symbol = next_symbol.saturating_add(1);
    let resolve_id = *next_symbol;
    stored.insert(
        resolve_id,
        StoredWorkspaceSymbol {
            key: response.key.clone(),
            symbol: symbol.clone(),
            path: path.clone(),
        },
    );
    SymbolEntry {
        name: symbol.name,
        detail: None,
        container: symbol.container_name,
        kind: SymbolKind::from_lsp(symbol.kind),
        deprecated: has_deprecated_tag(symbol.tags.as_deref(), None),
        path,
        range: None,
        selection: None,
        server: Some(response.server.clone()),
        resolve_id: Some(resolve_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(value: serde_json::Value) -> SymbolResponse {
        SymbolResponse {
            server: "fake".to_string(),
            key: ClientKey {
                server: "fake".to_string(),
                root: PathBuf::from("/tmp"),
            },
            value,
            encoding: PositionEncodingKind::UTF16,
        }
    }

    #[test]
    fn non_file_workspace_symbols_are_skipped() {
        let mut stored = HashMap::new();
        let mut next_symbol = 0;
        let entries = workspace_symbols(
            &mut stored,
            &mut next_symbol,
            &HashMap::new(),
            response(serde_json::json!([{
                "name": "remote",
                "kind": 12,
                "location": {"uri": "jar:file:///tmp/x.jar!/Main.class"}
            }])),
            false,
            false,
        )
        .expect("symbols");
        assert!(entries.is_empty());
    }

    #[test]
    fn nested_symbols_without_resolve_support_are_skipped() {
        let mut stored = HashMap::new();
        let mut next_symbol = 0;
        let entries = workspace_symbols(
            &mut stored,
            &mut next_symbol,
            &HashMap::new(),
            response(serde_json::json!([{
                "name": "lazy",
                "kind": 12,
                "location": {"uri": "file:///tmp/kawi_lazy.rs"}
            }])),
            false,
            false,
        )
        .expect("symbols");
        assert!(entries.is_empty());
        assert!(stored.is_empty());
    }
}
