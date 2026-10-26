use super::super::*;
use super::{SymbolEntry, SymbolKind, SymbolRange};
use crate::service::navigation_results::{path_for_uri, resolve_position};
use crate::text;
use lsp_types::{Position, SymbolInformation, SymbolTag, WorkspaceSymbol};
use ropey::Rope;
use std::collections::HashMap;
use std::path::Path;

pub(super) fn range_at(
    documents: &HashMap<PathBuf, Document>,
    cache: &mut HashMap<PathBuf, Rope>,
    path: &Path,
    range: lsp_types::Range,
    encoding: &PositionEncodingKind,
) -> Option<SymbolRange> {
    let start = resolve_position(documents, cache, path, range.start, encoding)?;
    let end = resolve_position(documents, cache, path, range.end, encoding)?;
    Some(SymbolRange {
        start: CursorPosition {
            line: start.line,
            column: start.column,
        },
        end: CursorPosition {
            line: end.line,
            column: end.column,
        },
    })
}

pub(super) fn symbol_range(
    text: &Rope,
    range: lsp_types::Range,
    encoding: &PositionEncodingKind,
) -> SymbolRange {
    SymbolRange {
        start: cursor_position(text, range.start, encoding),
        end: cursor_position(text, range.end, encoding),
    }
}

pub(super) fn cursor_position(
    text: &Rope,
    position: Position,
    encoding: &PositionEncodingKind,
) -> CursorPosition {
    let line = text::clamped_line(text, position.line);
    CursorPosition {
        line: u32::try_from(line).unwrap_or(u32::MAX),
        column: text::line_character(
            text,
            u32::try_from(line).unwrap_or(u32::MAX),
            position.character,
            encoding,
        ),
    }
}

#[allow(deprecated)]
pub(super) fn has_deprecated_tag(tags: Option<&[SymbolTag]>, deprecated: Option<bool>) -> bool {
    tags.is_some_and(|tags| tags.contains(&SymbolTag::DEPRECATED)) || deprecated.unwrap_or(false)
}

#[allow(deprecated)]
pub(super) fn append_symbol_information(
    entries: &mut Vec<SymbolEntry>,
    documents: &HashMap<PathBuf, Document>,
    cache: &mut HashMap<PathBuf, Rope>,
    encoding: &PositionEncodingKind,
    symbols: Vec<SymbolInformation>,
    server: Option<&str>,
) {
    for symbol in symbols {
        let Some(path) = path_for_uri(&symbol.location.uri) else {
            continue;
        };
        let range = range_at(documents, cache, &path, symbol.location.range, encoding);
        entries.push(SymbolEntry {
            name: symbol.name,
            detail: None,
            container: symbol.container_name,
            kind: SymbolKind::from_lsp(symbol.kind),
            deprecated: has_deprecated_tag(symbol.tags.as_deref(), symbol.deprecated),
            path,
            range,
            selection: range.map(|range| range.start),
            server: server.map(str::to_string),
            resolve_id: None,
        });
    }
}

#[allow(deprecated)]
pub(super) fn workspace_symbol_entry(
    documents: &HashMap<PathBuf, Document>,
    cache: &mut HashMap<PathBuf, Rope>,
    path: &Path,
    encoding: &PositionEncodingKind,
    symbol: WorkspaceSymbol,
    server: &str,
    resolve_id: Option<u64>,
) -> Option<SymbolEntry> {
    let range = match symbol.location {
        lsp_types::OneOf::Left(location) => {
            range_at(documents, cache, path, location.range, encoding)
        }
        lsp_types::OneOf::Right(_) => None,
    };
    let selection = range.map(|range| range.start);
    selection?;
    Some(SymbolEntry {
        name: symbol.name,
        detail: None,
        container: symbol.container_name,
        kind: SymbolKind::from_lsp(symbol.kind),
        deprecated: has_deprecated_tag(symbol.tags.as_deref(), None),
        path: path.to_path_buf(),
        range,
        selection,
        server: Some(server.to_string()),
        resolve_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace;

    #[test]
    fn converts_negotiated_offsets_to_character_columns() {
        let path = PathBuf::from("/tmp/kawi_symbols_encoding.rs");
        let text = Rope::from_str("aé€b\n");
        let documents = HashMap::from([(
            path.clone(),
            Document {
                uri: workspace::file_uri(&path).expect("uri"),
                language_id: "rust".to_string(),
                text: text.clone(),
                version: 1,
                clients: Vec::new(),
            },
        )]);
        let cases = [
            (PositionEncodingKind::UTF8, 6, 3),
            (PositionEncodingKind::UTF16, 3, 3),
            (PositionEncodingKind::UTF32, 3, 3),
        ];
        for (encoding, encoded_character, expected_column) in cases {
            let mut cache = HashMap::new();
            let range = range_at(
                &documents,
                &mut cache,
                &path,
                lsp_types::Range {
                    start: Position {
                        line: 0,
                        character: encoded_character,
                    },
                    end: Position {
                        line: 0,
                        character: encoded_character,
                    },
                },
                &encoding,
            )
            .expect("range");
            assert_eq!(range.start.column, expected_column);
        }
    }
}
