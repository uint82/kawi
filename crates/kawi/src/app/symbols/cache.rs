use kawi_lsp::SymbolEntry;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(crate) struct SymbolCache {
    documents: BTreeMap<(PathBuf, u64), Vec<SymbolEntry>>,
}

impl SymbolCache {
    pub(super) fn get(&self, path: &Path, version: u64) -> Option<&[SymbolEntry]> {
        self.documents
            .get(&(path.to_path_buf(), version))
            .map(Vec::as_slice)
    }

    pub(super) fn latest(&self, path: &Path) -> Option<&[SymbolEntry]> {
        self.documents
            .iter()
            .find(|((cached, _), _)| cached.as_path() == path)
            .map(|(_, symbols)| symbols.as_slice())
    }

    pub(super) fn insert(&mut self, path: &Path, version: u64, symbols: Vec<SymbolEntry>) {
        self.documents
            .retain(|(cached, _), _| cached.as_path() != path);
        self.documents
            .insert((path.to_path_buf(), version), symbols);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_share_one_cached_document() {
        let path = Path::new("/tmp/kawi_symbols_cache.rs");
        let mut cache = SymbolCache::default();
        cache.insert(path, 1, Vec::new());
        assert!(cache.get(path, 1).is_some());
        assert!(cache.get(path, 2).is_none());
        assert!(cache.latest(path).is_some());
        cache.insert(path, 2, Vec::new());
        assert!(cache.get(path, 1).is_none());
        assert!(cache.get(path, 2).is_some());
    }
}
