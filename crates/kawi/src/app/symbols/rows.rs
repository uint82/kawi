use kawi_lsp::SymbolEntry;
use kawi_picker::PickerRow;
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn picker_row(entry: &SymbolEntry, id: u64, root: &Path) -> PickerRow {
    let mut detail = Vec::new();
    if let Some(value) = entry.container.as_deref().filter(|value| !value.is_empty()) {
        detail.push(value.to_string());
    }
    if let Some(value) = entry.detail.as_deref().filter(|value| !value.is_empty()) {
        detail.push(value.to_string());
    }
    if let Some(server) = entry.server.as_deref() {
        detail.push(server.to_string());
        detail.push(relative_path(&entry.path, root));
    }
    PickerRow {
        display: format!("{} {}", entry.kind.label(), entry.name),
        path: entry.path.to_string_lossy().to_string(),
        line: entry.selection.map(|selection| selection.line as usize + 1),
        column: entry
            .selection
            .map(|selection| selection.column as usize + 1),
        buffer: None,
        secondary: (!detail.is_empty()).then(|| format!("  {}", detail.join("  "))),
        deprecated: entry.deprecated,
        id: Some(id),
    }
}

fn relative_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub(super) fn picker_rows(entries: &BTreeMap<u64, SymbolEntry>, root: &Path) -> Vec<PickerRow> {
    entries
        .iter()
        .map(|(id, entry)| picker_row(entry, *id, root))
        .collect()
}

pub(super) fn innermost(
    entries: &BTreeMap<u64, SymbolEntry>,
    line: usize,
    column: usize,
) -> Option<u64> {
    entries
        .iter()
        .filter(|(_, entry)| contains(entry, line, column))
        .min_by_key(|(_, entry)| span(entry))
        .map(|(id, _)| *id)
}

fn contains(entry: &SymbolEntry, line: usize, column: usize) -> bool {
    let Some(range) = entry.range else {
        return false;
    };
    let cursor = (line as u32, column as u32);
    (range.start.line, range.start.column) <= cursor && cursor <= (range.end.line, range.end.column)
}

fn span(entry: &SymbolEntry) -> (u32, u32) {
    let Some(range) = entry.range else {
        return (u32::MAX, u32::MAX);
    };
    (
        range.end.line.saturating_sub(range.start.line),
        range.end.column.saturating_sub(range.start.column),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_lsp::{SymbolEntry, SymbolKind, SymbolRange};
    use std::path::PathBuf;

    fn entry(name: &str, start: u32, end: u32) -> SymbolEntry {
        SymbolEntry {
            name: name.to_string(),
            detail: None,
            container: None,
            kind: SymbolKind::Function,
            deprecated: false,
            path: PathBuf::from("/workspace/src/main.rs"),
            range: Some(SymbolRange {
                start: kawi_lsp::CursorPosition {
                    line: start,
                    column: 0,
                },
                end: kawi_lsp::CursorPosition {
                    line: end,
                    column: 4,
                },
            }),
            selection: Some(kawi_lsp::CursorPosition {
                line: start,
                column: 2,
            }),
            server: None,
            resolve_id: None,
        }
    }

    #[test]
    fn rows_show_kind_name_and_workspace_details() {
        let mut symbol = entry("run", 0, 4);
        symbol.detail = Some("fn run()".to_string());
        symbol.container = Some("App".to_string());
        symbol.server = Some("rust-analyzer".to_string());
        let row = picker_row(&symbol, 7, Path::new("/workspace"));
        assert_eq!(row.display, "function run");
        assert_eq!(
            row.secondary.as_deref(),
            Some("  App  fn run()  rust-analyzer  src/main.rs")
        );
        assert_eq!(row.id, Some(7));
    }

    #[test]
    fn innermost_prefers_the_smallest_enclosing_symbol() {
        let entries = BTreeMap::from([(1, entry("outer", 0, 20)), (2, entry("inner", 4, 8))]);
        assert_eq!(innermost(&entries, 5, 1), Some(2));
        assert_eq!(innermost(&entries, 12, 1), Some(1));
        assert_eq!(innermost(&entries, 30, 1), None);
    }
}
